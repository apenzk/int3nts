#!/bin/bash

# Start Trusted-GMP for SVM E2E Tests
#
# Configures and starts the trusted-gmp service (validation and signing, port 3334, HAS keys).

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"

setup_project_root
setup_logging "trusted-gmp-start-svm"
cd "$PROJECT_ROOT"

log ""
log " Configuring and starting Trusted-GMP..."
log "========================================="
log_and_echo " All output logged to: $LOG_FILE"
log ""

log " Configuring trusted-gmp..."
source "$PROJECT_ROOT/testing-infra/ci-e2e/chain-hub/configure-trusted-gmp.sh"
source "$PROJECT_ROOT/testing-infra/ci-e2e/chain-connected-svm/configure-trusted-gmp.sh"

log ""
log "   Starting trusted-gmp service..."
start_trusted_gmp "$LOG_DIR/trusted-gmp.log" "info"

log ""
log_and_echo " Trusted-GMP started (port 3334)"
