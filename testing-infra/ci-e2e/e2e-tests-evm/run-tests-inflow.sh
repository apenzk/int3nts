#!/bin/bash

# E2E Integration Test Runner (Mixed-Chain: hub + EVM Escrow)
# 
# This script runs the mixed-chain E2E flow:
# - Hub: Intent creation and fulfillment
# - Chain 3 (EVM): Escrow operations
# - Coordinator + Trusted-GMP: Negotiation routing and chain monitoring

set -eo pipefail

# Parse flags
SKIP_BUILD=false
for arg in "$@"; do
    case "$arg" in
        --no-build) SKIP_BUILD=true ;;
    esac
done

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/../util_evm.sh"

# Setup project root and logging
setup_project_root
setup_logging "run-tests-evm"
cd "$PROJECT_ROOT"

log_and_echo " E2E Test for Connected EVM Chain - INFLOW"
log_and_echo "============================================="
log_and_echo " All output logged to: $LOG_FILE"
log_and_echo ""

log_and_echo " Step 0: Cleaning up any existing chains, accounts and processes..."
log_and_echo "=========================================================="
./testing-infra/ci-e2e/chain-connected-evm/cleanup.sh

log_and_echo ""
if [ "$SKIP_BUILD" = "true" ]; then
    log_and_echo " Step 1: Skipping build (--no-build)"
    log_and_echo "========================================"
else
    log_and_echo " Step 1: Build bins and pre-pull docker images"
    log_and_echo "========================================"
    # Delete existing binaries to ensure fresh build
    rm -f "$PROJECT_ROOT/target/debug/trusted-gmp" "$PROJECT_ROOT/target/debug/solver" "$PROJECT_ROOT/target/debug/coordinator"
    rm -f "$PROJECT_ROOT/target/release/trusted-gmp" "$PROJECT_ROOT/target/release/solver" "$PROJECT_ROOT/target/release/coordinator"

    pushd "$PROJECT_ROOT/coordinator" > /dev/null
    cargo build --bin coordinator 2>&1 | tail -5
    popd > /dev/null
    log_and_echo "   ✅ Coordinator: coordinator"

    pushd "$PROJECT_ROOT/trusted-gmp" > /dev/null
    cargo build --bin trusted-gmp --bin generate_keys --bin get_approver_eth_address 2>&1 | tail -5
    popd > /dev/null
    log_and_echo "   ✅ Trusted-GMP: trusted-gmp, generate_keys, get_approver_eth_address"

    pushd "$PROJECT_ROOT/solver" > /dev/null
    cargo build --bin solver --bin sign_intent 2>&1 | tail -5
    popd > /dev/null
    log_and_echo "   ✅ Solver: solver, sign_intent"
fi

log_and_echo ""
docker pull "$APTOS_DOCKER_IMAGE"

log_and_echo " Step 2: Generating trusted-gmp keys..."
log_and_echo "======================================="
generate_trusted_gmp_keys
log_and_echo ""

log_and_echo " Step 3: Setting up chains and deploying contracts..."
log_and_echo "======================================================"
./testing-infra/ci-e2e/chain-hub/setup-chain.sh
./testing-infra/ci-e2e/chain-hub/setup-requester-solver.sh
./testing-infra/ci-e2e/chain-connected-evm/setup-chain.sh
./testing-infra/ci-e2e/chain-connected-evm/setup-requester-solver.sh
./testing-infra/ci-e2e/chain-hub/deploy-contracts.sh
./testing-infra/ci-e2e/chain-connected-evm/deploy-contract.sh

log_and_echo ""
log_and_echo " Step 4: Starting coordinator and trusted-gmp..."
log_and_echo "=========================================================================="
./testing-infra/ci-e2e/e2e-tests-evm/start-coordinator.sh
./testing-infra/ci-e2e/e2e-tests-evm/start-trusted-gmp.sh

# Start solver service for automatic signing and fulfillment
log_and_echo ""
log_and_echo " Step 4b: Starting solver service..."
log_and_echo "======================================="
./testing-infra/ci-e2e/e2e-tests-evm/start-solver.sh

# Verify solver and trusted-gmp started successfully
./testing-infra/ci-e2e/verify-solver-running.sh
./testing-infra/ci-e2e/verify-trusted-gmp-running.sh

log_and_echo ""
log_and_echo " Step 5: Submitting cross-chain intents via coordinator negotiation routing..."
log_and_echo "============================================================================="
./testing-infra/ci-e2e/e2e-tests-evm/inflow-submit-hub-intent.sh
log_and_echo ""
log_and_echo " Pre-Escrow Balance Validation"
log_and_echo "=========================================="
log_and_echo "   Nobody should have moved funds yet; all four actors start with 1 USDhub/USDcon token on each chain"
./testing-infra/ci-e2e/e2e-tests-evm/balance-check.sh 1000000 1000000 1000000 1000000

./testing-infra/ci-e2e/e2e-tests-evm/inflow-submit-escrow.sh
# Load intent ID for solver fulfillment wait
if ! load_intent_info "INTENT_ID"; then
    log_and_echo "❌ ERROR: Failed to load intent info"
    exit 1
fi

log_and_echo ""
log_and_echo " Step 5b: Waiting for solver to automatically fulfill..."
log_and_echo "==========================================================="
log_and_echo "   The solver service is running and will:"
log_and_echo "   1. Detect the escrow on connected EVM chain"
log_and_echo "   2. Fulfill the intent on hub chain"
log_and_echo "   3. Trusted-GMP will detect fulfillment and generate approval"
log_and_echo ""

if ! wait_for_solver_fulfillment "$INTENT_ID" "inflow" 20; then
    log_and_echo "❌ ERROR: Solver did not fulfill the intent automatically"
    display_service_logs "Solver fulfillment timeout"
    exit 1
fi

log_and_echo "✅ Solver fulfilled the intent automatically!"
log_and_echo ""

# Wait for escrow release (happens automatically when fulfillment proof arrives)
./testing-infra/ci-e2e/e2e-tests-evm/wait-for-escrow-release.sh

log_and_echo ""
log_and_echo " Final Balance Validation"
log_and_echo "=========================================="
# Inflow: Solver transfers to hub requester (0 on hub, 2000000 on EVM from escrow)
#         Requester receives on hub (2000000 on hub, 0 on EVM locked in escrow)
./testing-infra/ci-e2e/e2e-tests-evm/balance-check.sh 0 2000000 2000000 0

log_and_echo ""
log_and_echo "✅ E2E inflow test completed!"

log_and_echo ""
log_and_echo " Step 6: Cleaning up chains, accounts and processes..."
log_and_echo "========================================================"
./testing-infra/ci-e2e/chain-connected-evm/cleanup.sh