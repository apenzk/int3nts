#!/bin/bash

# E2E Integration Test Runner - INFLOW (MVM)
#
# Usage: ./run-tests-inflow.sh [--no-build]
#   --no-build  Skip full rebuild; only build binaries that are missing

# -e: exit on error; -o pipefail: fail pipeline if ANY command fails (not just the last).
# Without pipefail, `grep ... | sed ...` silently succeeds even when grep finds no match.
set -eo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../e2e-common.sh"
# "$@" forwards this script's CLI args (e.g. --no-build) into e2e_init for flag parsing
e2e_init "mvm" "inflow" "$@"

e2e_cleanup_pre

e2e_build

generate_integrated_gmp_keys

e2e_setup_chains

e2e_start_services

log_and_echo ""
log_and_echo " Testing INFLOW intents (connected chain → hub chain)..."
log_and_echo "============================================================"
log_and_echo "   Submitting inflow cross-chain intents via coordinator negotiation routing..."
./testing-infra/ci-e2e/e2e-tests-mvm/inflow-submit-hub-intent.sh
log_and_echo ""
log_and_echo " Pre-Escrow Balance Validation"
log_and_echo "=========================================="
# Nobody should have done anything yet: all four actors start with 2 USDhub/USDcon on each chain
./testing-infra/ci-e2e/e2e-tests-mvm/balance-check.sh 2000000 2000000 2000000 2000000

./testing-infra/ci-e2e/e2e-tests-mvm/inflow-submit-escrow.sh

e2e_wait_for_fulfillment "inflow" 20

# Wait for escrow auto-release (verifies FulfillmentProof triggered release)
./testing-infra/ci-e2e/e2e-tests-mvm/wait-for-escrow-release.sh

log_and_echo ""
log_and_echo " Final Balance Validation"
log_and_echo "=========================================="
# Inflow: Solver sends 985,000 (desired) to requester on hub, receives 1,000,000 (offered) from escrow
#         Fee = 15,000 embedded in exchange rate (solver keeps the spread)
./testing-infra/ci-e2e/e2e-tests-mvm/balance-check.sh 1015000 2985000 3000000 1000000

./testing-infra/ci-e2e/e2e-tests-mvm/reject-insufficient-liquidity.sh

e2e_cleanup_post

