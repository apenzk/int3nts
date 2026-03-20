#!/bin/bash

# E2E Integration Test Runner - INFLOW (EVM)
#
# Runs inflow tests against both EVM instances sequentially.
# Hub balances shift after each iteration, so expected values differ per instance.
#
# Usage: ./run-tests-inflow.sh [--no-build]
#   --no-build  Skip full rebuild; only build binaries that are missing

# -e: exit on error; -o pipefail: fail pipeline if ANY command fails (not just the last).
# Without pipefail, `grep ... | sed ...` silently succeeds even when grep finds no match.
set -eo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../e2e-common.sh"
source "$SCRIPT_DIR/../chain-connected-evm/utils.sh"
# "$@" forwards this script's CLI args (e.g. --no-build) into e2e_init for flag parsing
e2e_init "evm" "inflow" "$@"

e2e_cleanup_pre

e2e_build

generate_integrated_gmp_keys

e2e_setup_chains

e2e_start_services

# --- Instance 2 inflow ---
export EVM_INSTANCE=2
evm_instance_vars 2

log_and_echo ""
log_and_echo " INFLOW test against EVM instance 2 (chain ID $EVM_CHAIN_ID)"
log_and_echo "========================================================================="

log_and_echo ""
log_and_echo " Submitting cross-chain intents via coordinator negotiation routing..."
log_and_echo "========================================================================="
./testing-infra/ci-e2e/e2e-tests-evm/inflow-submit-hub-intent.sh
log_and_echo ""
log_and_echo " Pre-Escrow Balance Validation (instance 2)"
log_and_echo "=========================================="
# Pre: solver_hub=2000000, requester_hub=2000000, solver_evm2=2000000, requester_evm2=2000000
./testing-infra/ci-e2e/e2e-tests-evm/balance-check.sh 2000000 2000000 2000000 2000000

./testing-infra/ci-e2e/e2e-tests-evm/inflow-submit-escrow.sh
e2e_wait_for_fulfillment "inflow" 20

./testing-infra/ci-e2e/e2e-tests-evm/wait-for-escrow-release.sh

log_and_echo ""
log_and_echo " Final Balance Validation (instance 2)"
log_and_echo "=========================================="
# Post: solver_hub=1015000, requester_hub=2985000, solver_evm2=3000000, requester_evm2=1000000
./testing-infra/ci-e2e/e2e-tests-evm/balance-check.sh 1015000 2985000 3000000 1000000

log_and_echo "✅ INFLOW test passed for EVM instance 2"

# --- Instance 3 inflow ---
export EVM_INSTANCE=3
evm_instance_vars 3

log_and_echo ""
log_and_echo " INFLOW test against EVM instance 3 (chain ID $EVM_CHAIN_ID)"
log_and_echo "========================================================================="

./testing-infra/ci-e2e/e2e-tests-evm/inflow-submit-hub-intent.sh
log_and_echo ""
log_and_echo " Pre-Escrow Balance Validation (instance 3)"
log_and_echo "=========================================="
# Pre: hub balances carried from instance 2; evm3 is fresh
# solver_hub=1015000, requester_hub=2985000, solver_evm3=2000000, requester_evm3=2000000
./testing-infra/ci-e2e/e2e-tests-evm/balance-check.sh 1015000 2985000 2000000 2000000

./testing-infra/ci-e2e/e2e-tests-evm/inflow-submit-escrow.sh
e2e_wait_for_fulfillment "inflow" 20

./testing-infra/ci-e2e/e2e-tests-evm/wait-for-escrow-release.sh

log_and_echo ""
log_and_echo " Final Balance Validation (instance 3)"
log_and_echo "=========================================="
# Post: solver_hub=30000, requester_hub=3970000, solver_evm3=3000000, requester_evm3=1000000
./testing-infra/ci-e2e/e2e-tests-evm/balance-check.sh 30000 3970000 3000000 1000000

log_and_echo "✅ INFLOW test passed for EVM instance 3"

# --- Reject insufficient liquidity (solver depleted after two inflows) ---
# Solver hub balance is now 30000 — far too low for another intent
./testing-infra/ci-e2e/e2e-tests-evm/reject-insufficient-liquidity.sh

e2e_cleanup_post
