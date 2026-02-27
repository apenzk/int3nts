#!/bin/bash

# E2E Integration Test Runner - OUTFLOW
# 
# This script runs the outflow E2E tests that require Docker chains.
# It sets up chains, deploys contracts, starts coordinator and integrated-gmp for negotiation routing,
# submits outflow intents via coordinator, then runs the tests.

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

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"

# Setup project root
setup_project_root
cd "$PROJECT_ROOT"

echo " E2E Test with Connected Move VM Chain - OUTFLOW"
echo "=================================================="
echo ""

echo " Step 0: Cleaning up any existing chains, accounts and processes..."
echo "================================================================"
./testing-infra/ci-e2e/chain-connected-mvm/cleanup.sh

echo ""
if [ "$SKIP_BUILD" = "true" ]; then
    echo " Step 1: Build if missing (--no-build)"
    echo "========================================"
    build_common_bins_if_missing
    build_if_missing "$PROJECT_ROOT/solver" "cargo build --bin sign_intent" \
        "Solver: sign_intent" \
        "$PROJECT_ROOT/solver/target/debug/sign_intent"
else
    echo " Step 1: Build bins and pre-pull docker images"
    echo "========================================"
    # Delete existing binaries to ensure fresh build
    rm -f "$PROJECT_ROOT/target/debug/integrated-gmp" "$PROJECT_ROOT/target/debug/solver" "$PROJECT_ROOT/target/debug/coordinator"
    rm -f "$PROJECT_ROOT/target/release/integrated-gmp" "$PROJECT_ROOT/target/release/solver" "$PROJECT_ROOT/target/release/coordinator"

    pushd "$PROJECT_ROOT/coordinator" > /dev/null
    cargo build --bin coordinator 2>&1 | tail -5
    popd > /dev/null
    echo "   ✅ Coordinator: coordinator"

    pushd "$PROJECT_ROOT/integrated-gmp" > /dev/null
    cargo build --bin integrated-gmp --bin generate_keys 2>&1 | tail -5
    popd > /dev/null
    echo "   ✅ Integrated-GMP: integrated-gmp, generate_keys"

    pushd "$PROJECT_ROOT/solver" > /dev/null
    cargo build --bin solver --bin sign_intent 2>&1 | tail -5
    popd > /dev/null
    echo "   ✅ Solver: solver, sign_intent"
fi

echo ""
docker pull "$APTOS_DOCKER_IMAGE"

echo " Step 2: Generating integrated-gmp keys..."
echo "======================================="
generate_integrated_gmp_keys
echo ""

echo " Step 3: Setting up chains, deploying contracts, funding accounts"
echo "===================================================================="
./testing-infra/ci-e2e/chain-hub/setup-chain.sh
./testing-infra/ci-e2e/chain-hub/setup-requester-solver.sh
./testing-infra/ci-e2e/chain-connected-mvm/setup-chain.sh
./testing-infra/ci-e2e/chain-connected-mvm/setup-requester-solver.sh
./testing-infra/ci-e2e/chain-hub/deploy-contracts.sh
./testing-infra/ci-e2e/chain-connected-mvm/deploy-contracts.sh

# Load chain info for balance assertions
source "$PROJECT_ROOT/.tmp/chain-info.env"

echo ""
echo " Step 4: Configuring and starting coordinator and integrated-gmp (for negotiation routing)..."
echo "=========================================================================="
./testing-infra/ci-e2e/e2e-tests-mvm/start-coordinator.sh
./testing-infra/ci-e2e/e2e-tests-mvm/start-integrated-gmp.sh

# Assert solver has USDcon before starting (should have 1 USDcon from deploy)
assert_usdxyz_balance "solver-chain2" "2" "$USD_MVMCON_MODULE_ADDR" "2000000" "pre-solver-start"
echo "   [DEBUG] Balance assertion completed, continuing..."

# Start solver service for automatic signing and fulfillment
echo ""
echo " Step 4b: Starting solver service..."
echo "======================================="
./testing-infra/ci-e2e/e2e-tests-mvm/start-solver.sh

# Verify solver and integrated-gmp started successfully
./testing-infra/ci-e2e/verify-solver-running.sh
./testing-infra/ci-e2e/verify-integrated-gmp-running.sh

echo ""
echo " Step 5: Testing OUTFLOW intents (hub chain → connected chain)..."
echo "===================================================================="
echo "   Submitting outflow cross-chain intents via coordinator negotiation routing..."
echo ""
echo " Pre-Intent Balance Validation"
echo "=========================================="
# Everybody starts with 2 USDhub/USDcon on each chain
./testing-infra/ci-e2e/e2e-tests-mvm/balance-check.sh 2000000 2000000 2000000 2000000

./testing-infra/ci-e2e/e2e-tests-mvm/outflow-submit-hub-intent.sh

# Load intent ID for solver fulfillment wait
if ! load_intent_info "INTENT_ID"; then
    echo "❌ ERROR: Failed to load intent info"
    exit 1
fi

echo ""
echo " Step 5b: Waiting for solver to automatically fulfill..."
echo "==========================================================="
echo "   The solver service is running and will:"
echo "   1. Detect the intent on hub chain"
echo "   2. Transfer tokens to requester on connected MVM chain"
echo "   3. Call integrated-gmp to validate and get approval signature"
echo "   4. Fulfill the hub intent with approval"
echo ""

if ! wait_for_solver_fulfillment "$INTENT_ID" "outflow" 40; then
    echo "❌ ERROR: Solver did not fulfill the intent automatically"
    display_service_logs "Solver fulfillment timeout"
    exit 1
fi

echo "✅ Solver fulfilled the intent automatically!"

echo ""
echo " Final Balance View"
echo "=========================================="
# Outflow: Solver gets from hub intent (2000000 on hub, 0 on MVM transferred to requester)
#          Requester receives on MVM (0 on hub locked in intent, 2000000 on MVM)
./testing-infra/ci-e2e/e2e-tests-mvm/balance-check.sh 3000000 1000000 1000000 3000000

echo ""
echo " Step 6: Verify solver rejects intent when liquidity is insufficient..."
echo "=========================================================================="
echo "   Solver started with 2,000,000 USDcon on connected MVM, spent 1,000,000 fulfilling intent 1."
echo "   Remaining: 1,000,000. Second intent requests 1,000,000."
echo "   Liquidity check: available >= requested + min_balance => 1,000,000 >= 1,000,000 + 1 => false."
echo "   Solver must reject: not enough to cover the request AND retain the min_balance threshold."

# Resolve chain addresses for the second draft
CONNECTED_CHAIN_ID=2
HUB_CHAIN_ID=1
HUB_MODULE_ADDR=$(get_profile_address "intent-account-chain1")
TEST_TOKENS_HUB=$(get_profile_address "test-tokens-chain1")
USD_MVMCON_MODULE_ADDR=$(get_profile_address "test-tokens-chain2")
REQUESTER_HUB_ADDR=$(get_profile_address "requester-chain1")
REQUESTER_MVMCON_ADDR=$(get_profile_address "requester-chain2")
USDHUB_METADATA_HUB=$(get_usdxyz_metadata_addr "0x$TEST_TOKENS_HUB" "1")
USD_MVMCON_ADDR=$(get_usdxyz_metadata_addr "0x$USD_MVMCON_MODULE_ADDR" "2")
EXPIRY_TIME=$(date -d "+1 hour" +%s)

SECOND_INTENT_ID="0x$(openssl rand -hex 32)"
DRAFT_DATA=$(build_draft_data \
    "$USDHUB_METADATA_HUB" \
    "1000000" \
    "$HUB_CHAIN_ID" \
    "$USD_MVMCON_ADDR" \
    "1000000" \
    "$CONNECTED_CHAIN_ID" \
    "$EXPIRY_TIME" \
    "$SECOND_INTENT_ID" \
    "$REQUESTER_HUB_ADDR" \
    "{\"chain_addr\": \"$HUB_MODULE_ADDR\", \"flow_type\": \"outflow\", \"requester_addr_connected_chain\": \"$REQUESTER_MVMCON_ADDR\"}")

assert_solver_rejects_draft "$REQUESTER_HUB_ADDR" "$DRAFT_DATA" "$EXPIRY_TIME"
echo "✅ Solver correctly rejected second intent due to insufficient liquidity!"

echo ""
echo "✅ E2E outflow test completed!"
echo ""

echo " Step 7: Cleaning up chains, accounts and processes..."
echo "========================================================"
./testing-infra/ci-e2e/chain-connected-mvm/cleanup.sh

