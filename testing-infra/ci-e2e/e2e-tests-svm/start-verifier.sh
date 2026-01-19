#!/bin/bash

# Start Trusted Verifier Service for SVM E2E Tests

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"

setup_project_root
setup_logging "verifier-start-svm"
cd "$PROJECT_ROOT"

log ""
log " Starting Trusted Verifier Service..."
log "========================================"
log_and_echo " All output logged to: $LOG_FILE"
log ""

log " Configuring verifier..."
source "$PROJECT_ROOT/testing-infra/ci-e2e/chain-hub/configure-verifier.sh"
source "$PROJECT_ROOT/testing-infra/ci-e2e/chain-connected-svm/configure-verifier.sh"

log ""
log "   Starting verifier service..."
start_verifier "$LOG_DIR/verifier.log" "info"

log ""
log_and_echo "✅ Verifier configured and started successfully"
