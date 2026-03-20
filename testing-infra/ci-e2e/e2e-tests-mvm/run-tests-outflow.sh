#!/bin/bash

# E2E Integration Test Runner - OUTFLOW (MVM)
#
# Runs outflow tests against both MVM instances sequentially.
# Hub balances shift after each iteration, so expected values differ per instance.
#
# Usage: ./run-tests-outflow.sh [--no-build]
#   --no-build  Skip full rebuild; only build binaries that are missing

# -e: exit on error; -o pipefail: fail pipeline if ANY command fails (not just the last).
# Without pipefail, `grep ... | sed ...` silently succeeds even when grep finds no match.
set -eo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../e2e-common.sh"
source "$SCRIPT_DIR/../chain-connected-mvm/utils.sh"
# "$@" forwards this script's CLI args (e.g. --no-build) into e2e_init for flag parsing
e2e_init "mvm" "outflow" "$@"

e2e_cleanup_pre

e2e_build

generate_integrated_gmp_keys

e2e_setup_chains

# Load chain info for balance assertions
source "$PROJECT_ROOT/.tmp/chain-info.env"

log_and_echo ""
log_and_echo " Starting coordinator and integrated-gmp..."
log_and_echo "======================================================"
./testing-infra/ci-e2e/e2e-tests-mvm/start-coordinator.sh
./testing-infra/ci-e2e/e2e-tests-mvm/start-integrated-gmp.sh

# Assert solver has USDcon on both instances before starting
for n in 2 3; do
    mvm_instance_vars "$n"
    load_mvm_chain_info "$n"
    local_usd_addr=$(get_profile_address "test-tokens-chain${n}")
    assert_usdxyz_balance "solver-chain${n}" "$n" "0x${local_usd_addr}" "2000000" "pre-solver-start-mvm${n}"
done
log_and_echo "   [DEBUG] Balance assertions completed, continuing..."

# Start solver service for automatic signing and fulfillment
log_and_echo ""
log_and_echo " Starting solver service..."
log_and_echo "======================================="
./testing-infra/ci-e2e/e2e-tests-mvm/start-solver.sh

# Verify solver and integrated-gmp started successfully
./testing-infra/ci-e2e/verify-solver-running.sh
./testing-infra/ci-e2e/verify-integrated-gmp-running.sh

# --- Instance 2 outflow ---
export MVM_INSTANCE=2
mvm_instance_vars 2

log_and_echo ""
log_and_echo " OUTFLOW test against MVM instance 2 (chain ID $MVM_CHAIN_ID)"
log_and_echo "================================================================="

log_and_echo ""
log_and_echo " Testing OUTFLOW intents (hub chain -> connected MVM chain)..."
log_and_echo "================================================================="
log_and_echo "   Submitting outflow cross-chain intents via coordinator negotiation routing..."
log_and_echo ""
log_and_echo " Pre-Intent Balance Validation (instance 2)"
log_and_echo "=========================================="
# Pre: solver_hub=2000000, requester_hub=2000000, solver_mvm2=2000000, requester_mvm2=2000000
./testing-infra/ci-e2e/e2e-tests-mvm/balance-check.sh 2000000 2000000 2000000 2000000

./testing-infra/ci-e2e/e2e-tests-mvm/outflow-submit-hub-intent.sh

e2e_wait_for_fulfillment "outflow" 40

log_and_echo ""
log_and_echo " Final Balance View (instance 2)"
log_and_echo "=========================================="
# Post: solver_hub=3000000, requester_hub=1000000, solver_mvm2=1015000, requester_mvm2=2985000
./testing-infra/ci-e2e/e2e-tests-mvm/balance-check.sh 3000000 1000000 1015000 2985000

log_and_echo "✅ OUTFLOW test passed for MVM instance 2"

# --- Instance 3 outflow ---
export MVM_INSTANCE=3
mvm_instance_vars 3

log_and_echo ""
log_and_echo " OUTFLOW test against MVM instance 3 (chain ID $MVM_CHAIN_ID)"
log_and_echo "================================================================="

log_and_echo ""
log_and_echo " Pre-Intent Balance Validation (instance 3)"
log_and_echo "=========================================="
# Pre: hub balances carried from instance 2; mvm3 is fresh
# solver_hub=3000000, requester_hub=1000000, solver_mvm3=2000000, requester_mvm3=2000000
./testing-infra/ci-e2e/e2e-tests-mvm/balance-check.sh 3000000 1000000 2000000 2000000

./testing-infra/ci-e2e/e2e-tests-mvm/outflow-submit-hub-intent.sh

e2e_wait_for_fulfillment "outflow" 40

log_and_echo ""
log_and_echo " Final Balance View (instance 3)"
log_and_echo "=========================================="
# Post: solver_hub=4000000, requester_hub=0, solver_mvm3=1015000, requester_mvm3=2985000
./testing-infra/ci-e2e/e2e-tests-mvm/balance-check.sh 4000000 0 1015000 2985000

log_and_echo "✅ OUTFLOW test passed for MVM instance 3"

# --- Reject insufficient liquidity (requester depleted after two outflows) ---
./testing-infra/ci-e2e/e2e-tests-mvm/reject-insufficient-liquidity.sh

e2e_cleanup_post

