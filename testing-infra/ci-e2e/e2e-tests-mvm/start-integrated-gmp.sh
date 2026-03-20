#!/bin/bash

# Start Integrated-GMP for MVM E2E Tests
#
# Configures and starts the integrated-gmp service (GMP message relay, port 3334, HAS keys).
# Configures both MVM instances.

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"

setup_project_root
setup_logging "integrated-gmp-start"
cd "$PROJECT_ROOT"

log ""
log " Configuring and starting Integrated-GMP..."
log "========================================="
log_and_echo " All output logged to: $LOG_FILE"
log ""

log " Configuring integrated-gmp..."
source "$PROJECT_ROOT/testing-infra/ci-e2e/chain-hub/configure-integrated-gmp.sh"
source "$PROJECT_ROOT/testing-infra/ci-e2e/chain-connected-mvm/configure-integrated-gmp.sh" 2
source "$PROJECT_ROOT/testing-infra/ci-e2e/chain-connected-mvm/configure-integrated-gmp.sh" 3

log ""
log "   Starting integrated-gmp service..."
start_integrated_gmp "$LOG_DIR/integrated-gmp.log" "info"

log ""
log_and_echo " Integrated-GMP started (port 3334)"
