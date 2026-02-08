#!/bin/bash

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"

# Setup project root and logging
setup_project_root
setup_logging "stop-chain"
cd "$PROJECT_ROOT"

log " STOPPING HUB"
log "================================"

log " Stopping Hub..."
docker-compose -f testing-infra/ci-e2e/chain-hub/docker-compose-hub-chain.yml -p aptos-chain1 down -v

# Force remove any orphaned containers
log " Force removing any orphaned containers..."
if docker ps -a --format '{{.Names}}' | grep -q "^aptos-localnet-chain1$"; then
    log "   Found orphaned container aptos-localnet-chain1, removing..."
    docker rm -f aptos-localnet-chain1
    log "   ✅ Orphaned container removed"
else
    log "   No orphaned containers found"
fi

# Remove any remaining volumes
log " Removing Docker volumes..."
if docker volume ls --format '{{.Name}}' | grep -q "^aptos-chain1_aptos-data$"; then
    docker volume rm aptos-chain1_aptos-data
    log "   ✅ Volume aptos-chain1_aptos-data removed"
else
    log "   No volume found"
fi

log ""
log " Cleaning up Hub Aptos CLI profiles..."
cleanup_aptos_profile "requester-chain1" "$LOG_FILE"
cleanup_aptos_profile "solver-chain1" "$LOG_FILE"
cleanup_aptos_profile "test-tokens-chain1" "$LOG_FILE"
cleanup_aptos_profile "intent-account-chain1" "$LOG_FILE"

log ""
log_and_echo "✅ Hub stopped and accounts cleaned up!"

