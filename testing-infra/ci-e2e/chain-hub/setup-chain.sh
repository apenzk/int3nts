#!/bin/bash

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"

# Setup project root and logging
setup_project_root
setup_logging "setup-chain"
cd "$PROJECT_ROOT"

log " HUB SETUP"
log "============================="
log_and_echo " All output logged to: $LOG_FILE"

# Stop any existing container
log " Stopping existing Hub container..."
docker-compose -f testing-infra/ci-e2e/chain-hub/docker-compose-hub-chain.yml -p aptos-chain1 down 2>/dev/null || true

log ""
log " Starting Hub (ports 8080/8081)..."
docker-compose -f testing-infra/ci-e2e/chain-hub/docker-compose-hub-chain.yml -p aptos-chain1 up -d

log ""
log "⏳ Waiting for Hub to start..."

# Wait for Hub
wait_for_mvm_chain_ready "1"

log ""
log " Verifying Hub..."

# Verify Hub services
verify_mvm_chain_services "1"

# Show chain status
log ""
log " Hub Status:"
CHAIN1_INFO=$(curl -s http://127.0.0.1:8080/v1 2>/dev/null)
CHAIN1_ID=$(echo "$CHAIN1_INFO" | jq -r '.chain_id // "unknown"' 2>/dev/null)
CHAIN1_HEIGHT=$(echo "$CHAIN1_INFO" | jq -r '.block_height // "unknown"' 2>/dev/null)
CHAIN1_ROLE=$(echo "$CHAIN1_INFO" | jq -r '.node_role // "unknown"' 2>/dev/null)
log "   Hub: ID=$CHAIN1_ID, Height=$CHAIN1_HEIGHT, Role=$CHAIN1_ROLE"

log ""
log " Hub setup complete!"
log "   Hub is running on ports 8080 (REST) and 8081 (faucet)"

