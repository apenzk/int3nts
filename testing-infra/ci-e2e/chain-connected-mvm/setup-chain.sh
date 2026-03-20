#!/bin/bash

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/utils.sh"

# Setup project root and logging
setup_project_root

# Accept instance number as argument (default: 2)
mvm_instance_vars "${1:-2}"

setup_logging "setup-chain-mvm${MVM_INSTANCE}"
cd "$PROJECT_ROOT"

log " CONNECTED CHAIN SETUP (Chain $MVM_INSTANCE)"
log "==================================="
log_and_echo " All output logged to: $LOG_FILE"

# Generate instance-specific docker-compose file
COMPOSE_FILE="$PROJECT_ROOT/.tmp/docker-compose-mvm${MVM_INSTANCE}.yml"
mkdir -p "$(dirname "$COMPOSE_FILE")"
cat > "$COMPOSE_FILE" << EOF
# APTOS_DOCKER_IMAGE is defined in util.sh and exported before docker-compose runs
services:
  aptos-localnet-chain${MVM_INSTANCE}:
    image: \${APTOS_DOCKER_IMAGE}
    container_name: aptos-localnet-chain${MVM_INSTANCE}
    ports:
      - "${MVM_REST_PORT}:8080"
      - "${MVM_FAUCET_PORT}:8081"
    volumes:
      - aptos-data-chain${MVM_INSTANCE}:/aptos/config
    command: >
      sh -c "
        echo 'Starting Aptos Localnet Chain ${MVM_INSTANCE} with faucet (fresh start)...' &&
        aptos node run-localnet --with-faucet --force-restart --assume-yes
      "
    restart: unless-stopped

volumes:
  aptos-data-chain${MVM_INSTANCE}:
EOF

# Stop any existing container
log " Stopping existing Chain $MVM_INSTANCE container..."
docker-compose -f "$COMPOSE_FILE" -p "$MVM_DOCKER_PROJECT" down 2>/dev/null || true

log ""
log " Starting Chain $MVM_INSTANCE (ports $MVM_REST_PORT/$MVM_FAUCET_PORT)..."
docker-compose -f "$COMPOSE_FILE" -p "$MVM_DOCKER_PROJECT" up -d

log ""
log "⏳ Waiting for Chain $MVM_INSTANCE to start..."

# Wait for chain
wait_for_mvm_chain_ready "$MVM_INSTANCE"

log ""
log " Verifying Chain $MVM_INSTANCE..."

# Verify chain services
verify_mvm_chain_services "$MVM_INSTANCE"

# Show chain status
log ""
log " Chain $MVM_INSTANCE Status:"
CHAIN_INFO=$(curl -s "http://127.0.0.1:${MVM_REST_PORT}/v1" 2>/dev/null)
MVMCON_CHAIN_ID=$(echo "$CHAIN_INFO" | jq -r '.chain_id // "unknown"' 2>/dev/null)
CHAIN_HEIGHT=$(echo "$CHAIN_INFO" | jq -r '.block_height // "unknown"' 2>/dev/null)
CHAIN_ROLE=$(echo "$CHAIN_INFO" | jq -r '.node_role // "unknown"' 2>/dev/null)
log "   Chain $MVM_INSTANCE: ID=$MVMCON_CHAIN_ID, Height=$CHAIN_HEIGHT, Role=$CHAIN_ROLE"

log ""
log " Connected chain setup complete!"
log "   Chain $MVM_INSTANCE is running on ports $MVM_REST_PORT (REST) and $MVM_FAUCET_PORT (faucet)"
