#!/bin/bash

# Balance Check Script for SVM E2E Tests
# Usage: balance-check.sh <solver_hub> <requester_hub> <solver_svm> <requester_svm>
# Pass -1 to skip a check. Amounts are in 10e-6 units (1 token = 1_000_000).

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/../util_svm.sh"

setup_project_root

SOLVER_CHAIN_HUB_EXPECTED="${1:-}"
REQUESTER_CHAIN_HUB_EXPECTED="${2:-}"
SOLVER_CHAIN_CONNECTED_EXPECTED="${3:-}"
REQUESTER_CHAIN_CONNECTED_EXPECTED="${4:-}"

TEST_TOKENS_HUB=$(get_profile_address "test-tokens-chain1" 2>/dev/null) || true
source "$PROJECT_ROOT/.tmp/chain-info.env" 2>/dev/null || true

if [ -n "$TEST_TOKENS_HUB" ]; then
    display_balances_hub "0x$TEST_TOKENS_HUB"
else
    display_balances_hub
fi

get_svm_balance() {
    local token_account="$1"
    SVM_TOKEN_ACCOUNT="$token_account" SVM_RPC_URL="${SVM_RPC_URL:-http://127.0.0.1:8899}" \
        bash "$PROJECT_ROOT/svm-intent-framework/scripts/get-token-balance.sh" \
        | grep -Eo 'Balance: [0-9]+' | awk '{print $2}' | tail -1 | tr -d '\n'
}

get_svm_sol_balance() {
    local pubkey="$1"
    if [ -z "$pubkey" ]; then
        echo "0"
        return
    fi
    svm_cmd "solana balance \"$pubkey\" --url \"${SVM_RPC_URL:-http://127.0.0.1:8899}\"" \
        | awk '{print $1}' | tail -n 1
}

if [ -z "$SOLVER_SVM_TOKEN_ACCOUNT" ] || [ -z "$REQUESTER_SVM_TOKEN_ACCOUNT" ]; then
    log_and_echo "❌ ERROR: SVM token accounts not found in chain-info.env"
    log_and_echo "   SOLVER_SVM_TOKEN_ACCOUNT: ${SOLVER_SVM_TOKEN_ACCOUNT:-missing}"
    log_and_echo "   REQUESTER_SVM_TOKEN_ACCOUNT: ${REQUESTER_SVM_TOKEN_ACCOUNT:-missing}"
    exit 1
fi

SOLVER_CHAIN_CONNECTED_ACTUAL=$(get_svm_balance "$SOLVER_SVM_TOKEN_ACCOUNT")
REQUESTER_CHAIN_CONNECTED_ACTUAL=$(get_svm_balance "$REQUESTER_SVM_TOKEN_ACCOUNT")
REQUESTER_SVM_SOL=$(get_svm_sol_balance "$REQUESTER_SVM_PUBKEY")
SOLVER_SVM_SOL=$(get_svm_sol_balance "$SOLVER_SVM_PUBKEY")

log_and_echo ""
log_and_echo "   Chain 4 (Connected SVM):"
log_and_echo "      Requester: ${REQUESTER_SVM_SOL} SOL, ${REQUESTER_CHAIN_CONNECTED_ACTUAL} 10e-6.USDsvm"
log_and_echo "      Solver:   ${SOLVER_SVM_SOL} SOL, ${SOLVER_CHAIN_CONNECTED_ACTUAL} 10e-6.USDsvm"

if [ -n "$SOLVER_CHAIN_CONNECTED_EXPECTED" ] && [ "$SOLVER_CHAIN_CONNECTED_EXPECTED" != "-1" ]; then
    if [ "$SOLVER_CHAIN_CONNECTED_ACTUAL" != "$SOLVER_CHAIN_CONNECTED_EXPECTED" ]; then
        log_and_echo "❌ ERROR: Solver balance mismatch on Connected SVM!"
        log_and_echo "   Actual:   $SOLVER_CHAIN_CONNECTED_ACTUAL"
        log_and_echo "   Expected: $SOLVER_CHAIN_CONNECTED_EXPECTED"
        display_service_logs "Solver balance mismatch on Connected SVM"
        exit 1
    fi
    log_and_echo "✅ Solver balance validated on Connected SVM: $SOLVER_CHAIN_CONNECTED_ACTUAL"
fi

if [ -n "$REQUESTER_CHAIN_CONNECTED_EXPECTED" ] && [ "$REQUESTER_CHAIN_CONNECTED_EXPECTED" != "-1" ]; then
    if [ "$REQUESTER_CHAIN_CONNECTED_ACTUAL" != "$REQUESTER_CHAIN_CONNECTED_EXPECTED" ]; then
        log_and_echo "❌ ERROR: Requester balance mismatch on Connected SVM!"
        log_and_echo "   Actual:   $REQUESTER_CHAIN_CONNECTED_ACTUAL"
        log_and_echo "   Expected: $REQUESTER_CHAIN_CONNECTED_EXPECTED"
        display_service_logs "Requester balance mismatch on Connected SVM"
        exit 1
    fi
    log_and_echo "✅ Requester balance validated on Connected SVM: $REQUESTER_CHAIN_CONNECTED_ACTUAL"
fi

# Hub checks (optional)
if [ -n "$SOLVER_CHAIN_HUB_EXPECTED" ] && [ "$SOLVER_CHAIN_HUB_EXPECTED" != "-1" ] && [ -n "$TEST_TOKENS_HUB" ]; then
    SOLVER_CHAIN_HUB_ACTUAL=$(get_usdxyz_balance "solver-chain1" "1" "0x$TEST_TOKENS_HUB" 2>/dev/null || echo "0")
    if [ "$SOLVER_CHAIN_HUB_ACTUAL" != "$SOLVER_CHAIN_HUB_EXPECTED" ]; then
        log_and_echo "❌ ERROR: Solver balance mismatch on Hub!"
        log_and_echo "   Actual:   $SOLVER_CHAIN_HUB_ACTUAL"
        log_and_echo "   Expected: $SOLVER_CHAIN_HUB_EXPECTED"
        display_service_logs "Solver balance mismatch on Hub"
        exit 1
    fi
    log_and_echo "✅ Solver balance validated on Hub: $SOLVER_CHAIN_HUB_ACTUAL"
fi

if [ -n "$REQUESTER_CHAIN_HUB_EXPECTED" ] && [ "$REQUESTER_CHAIN_HUB_EXPECTED" != "-1" ] && [ -n "$TEST_TOKENS_HUB" ]; then
    REQUESTER_CHAIN_HUB_ACTUAL=$(get_usdxyz_balance "requester-chain1" "1" "0x$TEST_TOKENS_HUB" 2>/dev/null || echo "0")
    if [ "$REQUESTER_CHAIN_HUB_ACTUAL" != "$REQUESTER_CHAIN_HUB_EXPECTED" ]; then
        log_and_echo "❌ ERROR: Requester balance mismatch on Hub!"
        log_and_echo "   Actual:   $REQUESTER_CHAIN_HUB_ACTUAL"
        log_and_echo "   Expected: $REQUESTER_CHAIN_HUB_EXPECTED"
        display_service_logs "Requester balance mismatch on Hub"
        exit 1
    fi
    log_and_echo "✅ Requester balance validated on Hub: $REQUESTER_CHAIN_HUB_ACTUAL"
fi

exit 0
