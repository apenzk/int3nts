#!/bin/bash

# E2E Integration Test Runner - OUTFLOW (SVM)

set -eo pipefail

# Parse flags
SKIP_BUILD=false
for arg in "$@"; do
    case "$arg" in
        --no-build) SKIP_BUILD=true ;;
    esac
done

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_svm.sh"

setup_project_root
setup_logging "run-tests-svm-outflow"
cd "$PROJECT_ROOT"

log_and_echo " E2E Test for Connected SVM Chain - OUTFLOW"
log_and_echo "============================================="
log_and_echo " All output logged to: $LOG_FILE"
log_and_echo ""

log_and_echo " Step 0: Cleaning up any existing chains, accounts and processes..."
log_and_echo "=========================================================="
./testing-infra/ci-e2e/chain-connected-svm/cleanup.sh
./testing-infra/ci-e2e/chain-hub/stop-chain.sh || true

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

    pushd "$PROJECT_ROOT/intent-frameworks/svm" > /dev/null
    ./scripts/build-with-docker.sh 2>&1 | tail -5
    popd > /dev/null
    log_and_echo "   ✅ SVM: on-chain programs (intent_inflow_escrow, intent_gmp, intent_outflow_validator)"

    pushd "$PROJECT_ROOT/coordinator" > /dev/null
    cargo build --bin coordinator 2>&1 | tail -5
    popd > /dev/null
    log_and_echo "   ✅ Coordinator: coordinator"

    pushd "$PROJECT_ROOT/trusted-gmp" > /dev/null
    cargo build --bin trusted-gmp --bin generate_keys 2>&1 | tail -5
    popd > /dev/null
    log_and_echo "   ✅ Trusted-GMP: trusted-gmp, generate_keys"

    pushd "$PROJECT_ROOT/solver" > /dev/null
    cargo build --bin solver 2>&1 | tail -5
    popd > /dev/null
    log_and_echo "   ✅ Solver: solver"

    pushd "$PROJECT_ROOT/intent-frameworks/svm" > /dev/null
    cargo build -p intent_escrow_cli 2>&1 | tail -5
    popd > /dev/null
    log_and_echo "   ✅ SVM: intent_escrow_cli"
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
./testing-infra/ci-e2e/chain-connected-svm/setup-chain.sh
./testing-infra/ci-e2e/chain-connected-svm/setup-requester-solver.sh
./testing-infra/ci-e2e/chain-hub/deploy-contracts.sh
./testing-infra/ci-e2e/chain-connected-svm/deploy-contract.sh

log_and_echo ""
log_and_echo " Step 4: Configuring and starting coordinator and trusted-gmp (for negotiation routing)..."
log_and_echo "=========================================================================="
./testing-infra/ci-e2e/e2e-tests-svm/start-coordinator.sh
./testing-infra/ci-e2e/e2e-tests-svm/start-trusted-gmp.sh

log_and_echo ""
log_and_echo " Step 4b: Starting solver service..."
log_and_echo "======================================="
./testing-infra/ci-e2e/e2e-tests-svm/start-solver.sh

./testing-infra/ci-e2e/verify-solver-running.sh
./testing-infra/ci-e2e/verify-trusted-gmp-running.sh

log_and_echo ""
log_and_echo " Step 5: Testing OUTFLOW intents (hub chain → connected SVM chain)..."
log_and_echo "====================================================================="
log_and_echo "   Submitting outflow cross-chain intents via coordinator negotiation routing..."
log_and_echo ""
log_and_echo " Pre-Intent Balance Validation"
log_and_echo "=========================================="
./testing-infra/ci-e2e/e2e-tests-svm/balance-check.sh 1000000 1000000 1000000 1000000

./testing-infra/ci-e2e/e2e-tests-svm/outflow-submit-hub-intent.sh

if ! load_intent_info "INTENT_ID"; then
    log_and_echo "❌ ERROR: Failed to load intent info"
    exit 1
fi

log_and_echo ""
log_and_echo " Step 5b: Waiting for solver to automatically fulfill..."
log_and_echo "==========================================================="

if ! wait_for_solver_fulfillment "$INTENT_ID" "outflow" 40; then
    log_and_echo "❌ ERROR: Solver did not fulfill the intent automatically"
    display_service_logs "Solver fulfillment timeout"
    exit 1
fi

log_and_echo "✅ Solver fulfilled the intent automatically!"
log_and_echo ""

log_and_echo " Final Balance Validation"
log_and_echo "=========================================="
./testing-infra/ci-e2e/e2e-tests-svm/balance-check.sh 2000000 0 0 2000000

log_and_echo ""
log_and_echo "✅ E2E outflow test completed!"
log_and_echo ""

log_and_echo " Step 6: Cleaning up chains, accounts and processes..."
log_and_echo "========================================================"
./testing-infra/ci-e2e/chain-connected-svm/cleanup.sh
