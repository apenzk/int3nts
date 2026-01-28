#!/bin/bash

# Start Coordinator for EVM E2E Tests
#
# Configures and starts the coordinator service (monitoring and negotiation, port 3333, NO keys).

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"

setup_project_root
setup_logging "coordinator-start-evm"
cd "$PROJECT_ROOT"

log ""
log " Configuring and starting Coordinator..."
log "========================================="
log_and_echo " All output logged to: $LOG_FILE"
log ""

log " Configuring coordinator..."
source "$PROJECT_ROOT/testing-infra/ci-e2e/chain-hub/configure-coordinator.sh"
source "$PROJECT_ROOT/testing-infra/ci-e2e/chain-connected-evm/configure-coordinator.sh"

log ""
log "   Starting coordinator service..."
start_coordinator "$LOG_DIR/coordinator.log" "info"

log ""
log_and_echo " Coordinator started (port 3333)"
