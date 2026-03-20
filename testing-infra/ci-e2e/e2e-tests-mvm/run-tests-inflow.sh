#!/bin/bash

# E2E Integration Test Runner - INFLOW (MVM)
#
# Runs inflow tests against both MVM instances sequentially.
# Hub balances shift after each iteration, so expected values differ per instance.
#
# Usage: ./run-tests-inflow.sh [--no-build]
#   --no-build  Skip full rebuild; only build binaries that are missing

# -e: exit on error; -o pipefail: fail pipeline if ANY command fails (not just the last).
# Without pipefail, `grep ... | sed ...` silently succeeds even when grep finds no match.
set -eo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../e2e-common.sh"
source "$SCRIPT_DIR/../chain-connected-mvm/utils.sh"
# "$@" forwards this script's CLI args (e.g. --no-build) into e2e_init for flag parsing
e2e_init "mvm" "inflow" "$@"

e2e_cleanup_pre

e2e_build

generate_integrated_gmp_keys

e2e_setup_chains

e2e_start_services

# --- Instance 2 inflow ---
export MVM_INSTANCE=2
mvm_instance_vars 2

log_and_echo ""
log_and_echo " INFLOW test against MVM instance 2 (chain ID $MVM_CHAIN_ID)"
log_and_echo "========================================================================="

log_and_echo ""
log_and_echo " Submitting cross-chain intents via coordinator negotiation routing..."
log_and_echo "========================================================================="
./testing-infra/ci-e2e/e2e-tests-mvm/inflow-submit-hub-intent.sh
log_and_echo ""
log_and_echo " Pre-Escrow Balance Validation (instance 2)"
log_and_echo "=========================================="
# Pre: solver_hub=2000000, requester_hub=2000000, solver_mvm2=2000000, requester_mvm2=2000000
./testing-infra/ci-e2e/e2e-tests-mvm/balance-check.sh 2000000 2000000 2000000 2000000

./testing-infra/ci-e2e/e2e-tests-mvm/inflow-submit-escrow.sh
e2e_wait_for_fulfillment "inflow" 20

./testing-infra/ci-e2e/e2e-tests-mvm/wait-for-escrow-release.sh

log_and_echo ""
log_and_echo " Final Balance Validation (instance 2)"
log_and_echo "=========================================="
# Post: solver_hub=1015000, requester_hub=2985000, solver_mvm2=3000000, requester_mvm2=1000000
./testing-infra/ci-e2e/e2e-tests-mvm/balance-check.sh 1015000 2985000 3000000 1000000

log_and_echo "✅ INFLOW test passed for MVM instance 2"

# --- Instance 3 inflow ---
export MVM_INSTANCE=3
mvm_instance_vars 3

log_and_echo ""
log_and_echo " INFLOW test against MVM instance 3 (chain ID $MVM_CHAIN_ID)"
log_and_echo "========================================================================="

./testing-infra/ci-e2e/e2e-tests-mvm/inflow-submit-hub-intent.sh
log_and_echo ""
log_and_echo " Pre-Escrow Balance Validation (instance 3)"
log_and_echo "=========================================="
# Pre: hub balances carried from instance 2; mvm3 is fresh
# solver_hub=1015000, requester_hub=2985000, solver_mvm3=2000000, requester_mvm3=2000000
./testing-infra/ci-e2e/e2e-tests-mvm/balance-check.sh 1015000 2985000 2000000 2000000

./testing-infra/ci-e2e/e2e-tests-mvm/inflow-submit-escrow.sh
e2e_wait_for_fulfillment "inflow" 20

./testing-infra/ci-e2e/e2e-tests-mvm/wait-for-escrow-release.sh

log_and_echo ""
log_and_echo " Final Balance Validation (instance 3)"
log_and_echo "=========================================="
# Post: solver_hub=30000, requester_hub=3970000, solver_mvm3=3000000, requester_mvm3=1000000
./testing-infra/ci-e2e/e2e-tests-mvm/balance-check.sh 30000 3970000 3000000 1000000

log_and_echo "✅ INFLOW test passed for MVM instance 3"

# --- Reject insufficient liquidity (solver depleted after two inflows) ---
# Solver hub balance is now 30000 — far too low for another intent
./testing-infra/ci-e2e/e2e-tests-mvm/reject-insufficient-liquidity.sh

e2e_cleanup_post

