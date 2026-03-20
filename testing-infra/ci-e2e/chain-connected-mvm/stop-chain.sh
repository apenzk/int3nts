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

setup_logging "stop-chain-mvm${MVM_INSTANCE}"
cd "$PROJECT_ROOT"

log " STOPPING CONNECTED CHAIN (Chain $MVM_INSTANCE)"
log "======================================"

log " Stopping Chain $MVM_INSTANCE..."
COMPOSE_FILE="$PROJECT_ROOT/.tmp/docker-compose-mvm${MVM_INSTANCE}.yml"
if [ -f "$COMPOSE_FILE" ]; then
    docker-compose -f "$COMPOSE_FILE" -p "$MVM_DOCKER_PROJECT" down -v
else
    # Fallback to legacy compose file for instance 2
    docker-compose -f testing-infra/ci-e2e/chain-connected-mvm/docker-compose-connected-chain-mvm.yml -p "$MVM_DOCKER_PROJECT" down -v 2>/dev/null || true
fi

# Force remove any orphaned containers
log " Force removing any orphaned containers..."
if docker ps -a --format '{{.Names}}' | grep -q "^aptos-localnet-chain${MVM_INSTANCE}$"; then
    log "   Found orphaned container aptos-localnet-chain${MVM_INSTANCE}, removing..."
    docker rm -f "aptos-localnet-chain${MVM_INSTANCE}"
    log "   ✅ Orphaned container removed"
else
    log "   No orphaned containers found"
fi

# Remove any remaining volumes
log " Removing Docker volumes..."
if docker volume ls --format '{{.Name}}' | grep -q "^${MVM_DOCKER_PROJECT}_aptos-data-chain${MVM_INSTANCE}$"; then
    docker volume rm "${MVM_DOCKER_PROJECT}_aptos-data-chain${MVM_INSTANCE}"
    log "   ✅ Volume ${MVM_DOCKER_PROJECT}_aptos-data-chain${MVM_INSTANCE} removed"
else
    log "   No volume found"
fi

log ""
log " Cleaning up Chain $MVM_INSTANCE Aptos CLI profiles..."
cleanup_aptos_profile "requester-chain${MVM_INSTANCE}" "$LOG_FILE"
cleanup_aptos_profile "solver-chain${MVM_INSTANCE}" "$LOG_FILE"
cleanup_aptos_profile "test-tokens-chain${MVM_INSTANCE}" "$LOG_FILE"
cleanup_aptos_profile "intent-account-chain${MVM_INSTANCE}" "$LOG_FILE"

log ""
log_and_echo "✅ Connected chain $MVM_INSTANCE stopped and accounts cleaned up!"
