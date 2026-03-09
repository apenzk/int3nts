#!/bin/bash

# Start Integrated-GMP for SVM E2E Tests
#
# Configures and starts the integrated-gmp service (GMP message relay, port 3334, HAS keys).

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"

setup_project_root
setup_logging "integrated-gmp-start-svm"
cd "$PROJECT_ROOT"

log ""
log " Configuring and starting Integrated-GMP..."
log "========================================="
log_and_echo " All output logged to: $LOG_FILE"
log ""

log " Configuring integrated-gmp..."
source "$PROJECT_ROOT/testing-infra/ci-e2e/chain-hub/configure-integrated-gmp.sh"
source "$PROJECT_ROOT/testing-infra/ci-e2e/chain-connected-svm/configure-integrated-gmp.sh"

log ""
log "   Starting integrated-gmp service..."
start_integrated_gmp "$LOG_DIR/integrated-gmp.log" "info"

log ""
log_and_echo " Integrated-GMP started (port 3334)"
