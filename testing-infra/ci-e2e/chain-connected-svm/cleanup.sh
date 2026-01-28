#!/bin/bash

# Cleanup for SVM E2E tests

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"

setup_project_root
setup_logging "cleanup-svm"
cd "$PROJECT_ROOT"

log_and_echo " Cleaning up SVM chain and temp files..."

stop_coordinator || true
stop_trusted_gmp || true
stop_solver || true

./testing-infra/ci-e2e/chain-connected-svm/stop-chain.sh || true

rm -rf "$PROJECT_ROOT/.tmp/solana-test-validator"
rm -rf "$PROJECT_ROOT/.tmp/svm-e2e"

log_and_echo "✅ SVM cleanup complete"
