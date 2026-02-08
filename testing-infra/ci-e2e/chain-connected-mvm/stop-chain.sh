#!/bin/bash

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"

# Setup project root and logging
setup_project_root
setup_logging "stop-chain"
cd "$PROJECT_ROOT"

log " STOPPING CONNECTED CHAIN (Chain 2)"
log "======================================"

log " Stopping Chain 2..."
docker-compose -f testing-infra/ci-e2e/chain-connected-mvm/docker-compose-connected-chain-mvm.yml -p aptos-chain2 down -v

# Force remove any orphaned containers
log " Force removing any orphaned containers..."
if docker ps -a --format '{{.Names}}' | grep -q "^aptos-localnet-chain2$"; then
    log "   Found orphaned container aptos-localnet-chain2, removing..."
    docker rm -f aptos-localnet-chain2
    log "   ✅ Orphaned container removed"
else
    log "   No orphaned containers found"
fi

# Remove any remaining volumes
log " Removing Docker volumes..."
if docker volume ls --format '{{.Name}}' | grep -q "^aptos-chain2_aptos-data-chain2$"; then
    docker volume rm aptos-chain2_aptos-data-chain2
    log "   ✅ Volume aptos-chain2_aptos-data-chain2 removed"
else
    log "   No volume found"
fi

log ""
log " Cleaning up Chain 2 Aptos CLI profiles..."
cleanup_aptos_profile "requester-chain2" "$LOG_FILE"
cleanup_aptos_profile "solver-chain2" "$LOG_FILE"
cleanup_aptos_profile "test-tokens-chain2" "$LOG_FILE"
cleanup_aptos_profile "intent-account-chain2" "$LOG_FILE"

log ""
log_and_echo "✅ Connected chain stopped and accounts cleaned up!"

