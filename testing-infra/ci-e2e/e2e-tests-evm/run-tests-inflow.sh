#!/bin/bash

# E2E Integration Test Runner (Mixed-Chain: hub + EVM Escrow)
# 
# This script runs the mixed-chain E2E flow:
# - Hub: Intent creation and fulfillment
# - Chain 3 (EVM): Escrow operations
# - Coordinator + Integrated-GMP: Negotiation routing and chain monitoring

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
    log_and_echo " Step 1: Build if missing (--no-build)"
    log_and_echo "========================================"
    build_common_bins_if_missing
    build_if_missing "$PROJECT_ROOT/integrated-gmp" "cargo build --bin get_approver_eth_address" \
        "Integrated-GMP: get_approver_eth_address" \
        "$PROJECT_ROOT/integrated-gmp/target/debug/get_approver_eth_address"
    build_if_missing "$PROJECT_ROOT/solver" "cargo build --bin sign_intent" \
        "Solver: sign_intent" \
        "$PROJECT_ROOT/solver/target/debug/sign_intent"
else
    log_and_echo " Step 1: Build bins and pre-pull docker images"
    log_and_echo "========================================"
    # Delete existing binaries to ensure fresh build
    rm -f "$PROJECT_ROOT/target/debug/integrated-gmp" "$PROJECT_ROOT/target/debug/solver" "$PROJECT_ROOT/target/debug/coordinator"
    rm -f "$PROJECT_ROOT/target/release/integrated-gmp" "$PROJECT_ROOT/target/release/solver" "$PROJECT_ROOT/target/release/coordinator"

    pushd "$PROJECT_ROOT/coordinator" > /dev/null
    cargo build --bin coordinator 2>&1 | tail -5
    popd > /dev/null
    log_and_echo "   ✅ Coordinator: coordinator"

    pushd "$PROJECT_ROOT/integrated-gmp" > /dev/null
    cargo build --bin integrated-gmp --bin generate_keys --bin get_approver_eth_address 2>&1 | tail -5
    popd > /dev/null
    log_and_echo "   ✅ Integrated-GMP: integrated-gmp, generate_keys, get_approver_eth_address"

    pushd "$PROJECT_ROOT/solver" > /dev/null
    cargo build --bin solver --bin sign_intent 2>&1 | tail -5
    popd > /dev/null
    log_and_echo "   ✅ Solver: solver, sign_intent"
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
./testing-infra/ci-e2e/chain-connected-evm/setup-chain.sh
./testing-infra/ci-e2e/chain-connected-evm/setup-requester-solver.sh
./testing-infra/ci-e2e/chain-hub/deploy-contracts.sh
./testing-infra/ci-e2e/chain-connected-evm/deploy-contract.sh

log_and_echo ""
log_and_echo " Step 4: Starting coordinator and integrated-gmp..."
log_and_echo "=========================================================================="
./testing-infra/ci-e2e/e2e-tests-evm/start-coordinator.sh
./testing-infra/ci-e2e/e2e-tests-evm/start-integrated-gmp.sh

# Start solver service for automatic signing and fulfillment
log_and_echo ""
log_and_echo " Step 4b: Starting solver service..."
log_and_echo "======================================="
./testing-infra/ci-e2e/e2e-tests-evm/start-solver.sh

# Verify solver and integrated-gmp started successfully
./testing-infra/ci-e2e/verify-solver-running.sh
./testing-infra/ci-e2e/verify-integrated-gmp-running.sh

log_and_echo ""
log_and_echo " Step 5: Submitting cross-chain intents via coordinator negotiation routing..."
log_and_echo "============================================================================="
./testing-infra/ci-e2e/e2e-tests-evm/inflow-submit-hub-intent.sh
log_and_echo ""
log_and_echo " Pre-Escrow Balance Validation"
log_and_echo "=========================================="
log_and_echo "   Nobody should have moved funds yet; all four actors start with 2 USDhub/USDcon tokens on each chain"
./testing-infra/ci-e2e/e2e-tests-evm/balance-check.sh 2000000 2000000 2000000 2000000

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
log_and_echo "   3. Integrated-GMP will detect fulfillment and generate approval"
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
./testing-infra/ci-e2e/e2e-tests-evm/balance-check.sh 1000000 3000000 3000000 1000000

log_and_echo ""
log_and_echo " Step 6: Verify solver rejects intent when liquidity is insufficient..."
log_and_echo "=========================================================================="
log_and_echo "   Solver started with 2,000,000 USDhub on hub, spent 1,000,000 fulfilling intent 1."
log_and_echo "   Remaining: 1,000,000. Second intent requests 1,000,000."
log_and_echo "   Liquidity check: available >= requested + min_balance => 1,000,000 >= 1,000,000 + 1 => false."
log_and_echo "   Solver must reject: not enough to cover the request AND retain the min_balance threshold."

CONNECTED_CHAIN_ID=31337
HUB_CHAIN_ID=1
HUB_MODULE_ADDR=$(get_profile_address "intent-account-chain1")
TEST_TOKENS_HUB=$(get_profile_address "test-tokens-chain1")
REQUESTER_HUB_ADDR=$(get_profile_address "requester-chain1")
USDHUB_METADATA_HUB=$(get_usdxyz_metadata_addr "0x$TEST_TOKENS_HUB" "1")

source "$PROJECT_ROOT/.tmp/chain-info.env" 2>/dev/null || true
EVM_TOKEN_ADDR_NO_PREFIX="${USD_EVM_ADDR#0x}"
EVM_TOKEN_ADDR_LOWER=$(echo "$EVM_TOKEN_ADDR_NO_PREFIX" | tr '[:upper:]' '[:lower:]')
OFFERED_METADATA_EVM="0x000000000000000000000000${EVM_TOKEN_ADDR_LOWER}"

EXPIRY_TIME=$(date -d "+1 hour" +%s)
SECOND_INTENT_ID="0x$(openssl rand -hex 32)"
DRAFT_DATA=$(build_draft_data \
    "$OFFERED_METADATA_EVM" \
    "1000000" \
    "$CONNECTED_CHAIN_ID" \
    "$USDHUB_METADATA_HUB" \
    "1000000" \
    "$HUB_CHAIN_ID" \
    "$EXPIRY_TIME" \
    "$SECOND_INTENT_ID" \
    "$REQUESTER_HUB_ADDR" \
    "{\"chain_addr\": \"$HUB_MODULE_ADDR\", \"flow_type\": \"inflow\", \"connected_chain_type\": \"evm\"}")

assert_solver_rejects_draft "$REQUESTER_HUB_ADDR" "$DRAFT_DATA" "$EXPIRY_TIME"
log_and_echo "✅ Solver correctly rejected second intent due to insufficient liquidity!"

log_and_echo ""
log_and_echo "✅ E2E inflow test completed!"

log_and_echo ""
log_and_echo " Step 7: Cleaning up chains, accounts and processes..."
log_and_echo "========================================================"
./testing-infra/ci-e2e/chain-connected-evm/cleanup.sh