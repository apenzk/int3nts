#!/bin/bash

# E2E Integration Test Runner - OUTFLOW (SVM)

# -e: exit on error; -o pipefail: fail pipeline if ANY command fails (not just the last).
# Without pipefail, `grep ... | sed ...` silently succeeds even when grep finds no match.
set -eo pipefail

# Parse flags
SKIP_BUILD=false
for arg in "$@"; do
    case "$arg" in
        --no-build) SKIP_BUILD=true ;;
    esac
done
export SKIP_BUILD

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
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
    log_and_echo " Step 1: Build if missing (--no-build)"
    log_and_echo "========================================"
    # SVM on-chain programs (docker build)
    if [ ! -f "$PROJECT_ROOT/intent-frameworks/svm/target/deploy/intent_inflow_escrow.so" ] || \
       [ ! -f "$PROJECT_ROOT/intent-frameworks/svm/target/deploy/intent_gmp.so" ] || \
       [ ! -f "$PROJECT_ROOT/intent-frameworks/svm/target/deploy/intent_outflow_validator.so" ]; then
        pushd "$PROJECT_ROOT/intent-frameworks/svm" > /dev/null
        ./scripts/build-with-docker.sh 2>&1 | tail -5
        popd > /dev/null
        log_and_echo "   ✅ SVM: on-chain programs (built)"
    else
        log_and_echo "   ✅ SVM: on-chain programs (exists)"
    fi
    build_common_bins_if_missing
    build_if_missing "$PROJECT_ROOT/intent-frameworks/svm" "cargo build -p intent_escrow_cli" \
        "SVM: intent_escrow_cli" \
        "$PROJECT_ROOT/intent-frameworks/svm/target/debug/intent_escrow_cli"
else
    log_and_echo " Step 1: Build bins and pre-pull docker images"
    log_and_echo "========================================"
    # Delete existing binaries to ensure fresh build
    rm -f "$PROJECT_ROOT/target/debug/integrated-gmp" "$PROJECT_ROOT/target/debug/solver" "$PROJECT_ROOT/target/debug/coordinator"
    rm -f "$PROJECT_ROOT/target/release/integrated-gmp" "$PROJECT_ROOT/target/release/solver" "$PROJECT_ROOT/target/release/coordinator"

    pushd "$PROJECT_ROOT/intent-frameworks/svm" > /dev/null
    ./scripts/build-with-docker.sh 2>&1 | tail -5
    popd > /dev/null
    log_and_echo "   ✅ SVM: on-chain programs (intent_inflow_escrow, intent_gmp, intent_outflow_validator)"

    pushd "$PROJECT_ROOT/coordinator" > /dev/null
    cargo build --bin coordinator 2>&1 | tail -5
    popd > /dev/null
    log_and_echo "   ✅ Coordinator: coordinator"

    pushd "$PROJECT_ROOT/integrated-gmp" > /dev/null
    cargo build --bin integrated-gmp --bin generate_keys 2>&1 | tail -5
    popd > /dev/null
    log_and_echo "   ✅ Integrated-GMP: integrated-gmp, generate_keys"

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

log_and_echo " Step 2: Generating integrated-gmp keys..."
log_and_echo "======================================="
generate_integrated_gmp_keys
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
log_and_echo " Step 4: Configuring and starting coordinator and integrated-gmp (for negotiation routing)..."
log_and_echo "=========================================================================="
./testing-infra/ci-e2e/e2e-tests-svm/start-coordinator.sh
./testing-infra/ci-e2e/e2e-tests-svm/start-integrated-gmp.sh

log_and_echo ""
log_and_echo " Step 4b: Starting solver service..."
log_and_echo "======================================="
./testing-infra/ci-e2e/e2e-tests-svm/start-solver.sh

./testing-infra/ci-e2e/verify-solver-running.sh
./testing-infra/ci-e2e/verify-integrated-gmp-running.sh

log_and_echo ""
log_and_echo " Step 5: Testing OUTFLOW intents (hub chain → connected SVM chain)..."
log_and_echo "====================================================================="
log_and_echo "   Submitting outflow cross-chain intents via coordinator negotiation routing..."
log_and_echo ""
log_and_echo " Pre-Intent Balance Validation"
log_and_echo "=========================================="
./testing-infra/ci-e2e/e2e-tests-svm/balance-check.sh 2000000 2000000 2000000 2000000

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
./testing-infra/ci-e2e/e2e-tests-svm/balance-check.sh 3000000 1000000 1000000 3000000

log_and_echo ""
log_and_echo " Step 6: Verify solver rejects intent when liquidity is insufficient..."
log_and_echo "=========================================================================="
log_and_echo "   Solver started with 2,000,000 USDcon on connected SVM, spent 1,000,000 fulfilling intent 1."
log_and_echo "   Remaining: 1,000,000. Second intent requests 1,000,000."
log_and_echo "   Liquidity check: available >= requested + min_balance => 1,000,000 >= 1,000,000 + 1 => false."
log_and_echo "   Solver must reject: not enough to cover the request AND retain the min_balance threshold."

HUB_CHAIN_ID=1
CONNECTED_CHAIN_ID=901
HUB_MODULE_ADDR=$(get_profile_address "intent-account-chain1")
TEST_TOKENS_HUB=$(get_profile_address "test-tokens-chain1")
REQUESTER_HUB_ADDR=$(get_profile_address "requester-chain1")
USDHUB_METADATA_HUB=$(get_usdxyz_metadata_addr "0x$TEST_TOKENS_HUB" "1")

source "$PROJECT_ROOT/.tmp/chain-info.env" 2>/dev/null || true
REQUESTER_SVM_ADDR=$(svm_pubkey_to_hex "$REQUESTER_SVM_PUBKEY")
SVM_TOKEN_HEX=$(svm_pubkey_to_hex "$USD_SVM_MINT_ADDR")

EXPIRY_TIME=$(date -d "+1 hour" +%s)
SECOND_INTENT_ID="0x$(openssl rand -hex 32)"
DRAFT_DATA=$(build_draft_data \
    "$USDHUB_METADATA_HUB" \
    "1000000" \
    "$HUB_CHAIN_ID" \
    "$SVM_TOKEN_HEX" \
    "1000000" \
    "$CONNECTED_CHAIN_ID" \
    "$EXPIRY_TIME" \
    "$SECOND_INTENT_ID" \
    "$REQUESTER_HUB_ADDR" \
    "{\"chain_addr\": \"$HUB_MODULE_ADDR\", \"flow_type\": \"outflow\", \"connected_chain_type\": \"svm\", \"requester_addr_connected_chain\": \"$REQUESTER_SVM_ADDR\"}")

assert_solver_rejects_draft "$REQUESTER_HUB_ADDR" "$DRAFT_DATA" "$EXPIRY_TIME"
log_and_echo "✅ Solver correctly rejected second intent due to insufficient liquidity!"

log_and_echo ""
log_and_echo "✅ E2E outflow test completed!"
log_and_echo ""

log_and_echo " Step 7: Cleaning up chains, accounts and processes..."
log_and_echo "========================================================"
./testing-infra/ci-e2e/chain-connected-svm/cleanup.sh
