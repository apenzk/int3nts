#!/bin/bash

# E2E Integration Test Runner - OUTFLOW (SVM)
#
# Runs outflow tests against both SVM instances sequentially.
# Hub balances shift after each iteration, so expected values differ per instance.
#
# Usage: ./run-tests-outflow.sh [--no-build]
#   --no-build  Skip full rebuild; only build binaries that are missing

# -e: exit on error; -o pipefail: fail pipeline if ANY command fails (not just the last).
# Without pipefail, `grep ... | sed ...` silently succeeds even when grep finds no match.
set -eo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../e2e-common.sh"
source "$SCRIPT_DIR/../chain-connected-svm/utils.sh"
# "$@" forwards this script's CLI args (e.g. --no-build) into e2e_init for flag parsing
e2e_init "svm" "outflow" "$@"

e2e_cleanup_pre

e2e_build

generate_integrated_gmp_keys

e2e_setup_chains

e2e_start_services

# --- Instance 2 outflow ---
export SVM_INSTANCE=2
svm_instance_vars 2

log_and_echo ""
log_and_echo " OUTFLOW test against SVM instance 2 (chain ID $SVM_CHAIN_ID)"
log_and_echo "================================================================="

log_and_echo ""
log_and_echo " Testing OUTFLOW intents (hub chain -> connected SVM chain)..."
log_and_echo "================================================================="
log_and_echo "   Submitting outflow cross-chain intents via coordinator negotiation routing..."
log_and_echo ""
log_and_echo " Pre-Intent Balance Validation (instance 2)"
log_and_echo "=========================================="
# Pre: solver_hub=2000000, requester_hub=2000000, solver_svm2=2000000, requester_svm2=2000000
./testing-infra/ci-e2e/e2e-tests-svm/balance-check.sh 2000000 2000000 2000000 2000000

./testing-infra/ci-e2e/e2e-tests-svm/outflow-submit-hub-intent.sh

e2e_wait_for_fulfillment "outflow" 40

log_and_echo ""
log_and_echo " Final Balance View (instance 2)"
log_and_echo "=========================================="
# Post: solver_hub=3000000, requester_hub=1000000, solver_svm2=1015000, requester_svm2=2985000
./testing-infra/ci-e2e/e2e-tests-svm/balance-check.sh 3000000 1000000 1015000 2985000

log_and_echo "✅ OUTFLOW test passed for SVM instance 2"

# --- Instance 3 outflow ---
export SVM_INSTANCE=3
svm_instance_vars 3

log_and_echo ""
log_and_echo " OUTFLOW test against SVM instance 3 (chain ID $SVM_CHAIN_ID)"
log_and_echo "================================================================="

log_and_echo ""
log_and_echo " Pre-Intent Balance Validation (instance 3)"
log_and_echo "=========================================="
# Pre: hub balances carried from instance 2; svm3 is fresh
# solver_hub=3000000, requester_hub=1000000, solver_svm3=2000000, requester_svm3=2000000
./testing-infra/ci-e2e/e2e-tests-svm/balance-check.sh 3000000 1000000 2000000 2000000

./testing-infra/ci-e2e/e2e-tests-svm/outflow-submit-hub-intent.sh

e2e_wait_for_fulfillment "outflow" 40

log_and_echo ""
log_and_echo " Final Balance View (instance 3)"
log_and_echo "=========================================="
# Post: solver_hub=4000000, requester_hub=0, solver_svm3=1015000, requester_svm3=2985000
./testing-infra/ci-e2e/e2e-tests-svm/balance-check.sh 4000000 0 1015000 2985000

log_and_echo "✅ OUTFLOW test passed for SVM instance 3"

# --- Reject insufficient liquidity (requester depleted after two outflows) ---
./testing-infra/ci-e2e/e2e-tests-svm/reject-insufficient-liquidity.sh

e2e_cleanup_post
