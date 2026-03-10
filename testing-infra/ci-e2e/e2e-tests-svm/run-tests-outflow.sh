#!/bin/bash

# E2E Integration Test Runner - OUTFLOW (SVM)
#
# Usage: ./run-tests-outflow.sh [--no-build]
#   --no-build  Skip full rebuild; only build binaries that are missing

# -e: exit on error; -o pipefail: fail pipeline if ANY command fails (not just the last).
# Without pipefail, `grep ... | sed ...` silently succeeds even when grep finds no match.
set -eo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../e2e-common.sh"
# "$@" forwards this script's CLI args (e.g. --no-build) into e2e_init for flag parsing
e2e_init "svm" "outflow" "$@"

e2e_cleanup_pre

e2e_build

generate_integrated_gmp_keys

e2e_setup_chains

e2e_start_services

log_and_echo ""
log_and_echo " Testing OUTFLOW intents (hub chain → connected SVM chain)..."
log_and_echo "================================================================="
log_and_echo "   Submitting outflow cross-chain intents via coordinator negotiation routing..."
log_and_echo ""
log_and_echo " Pre-Intent Balance Validation"
log_and_echo "=========================================="
./testing-infra/ci-e2e/e2e-tests-svm/balance-check.sh 2000000 2000000 2000000 2000000

./testing-infra/ci-e2e/e2e-tests-svm/outflow-submit-hub-intent.sh

e2e_wait_for_fulfillment "outflow" 40

log_and_echo " Final Balance Validation"
log_and_echo "=========================================="
./testing-infra/ci-e2e/e2e-tests-svm/balance-check.sh 3000000 1000000 1015000 2985000

./testing-infra/ci-e2e/e2e-tests-svm/reject-insufficient-liquidity.sh

e2e_cleanup_post
