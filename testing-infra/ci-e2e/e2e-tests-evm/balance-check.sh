#!/bin/bash

# Balance Check Script for EVM E2E Tests
# Displays and validates final balances for Hub (USDhub) and Connected EVM (USDcon)
# Usage: balance-check.sh <solver_chain_hub> <requester_chain_hub> <solver_chain_connected> <requester_chain_connected>
#   - Pass -1 for any parameter to skip that check
#   - Values are in 10e-6.USDhub / 10e-6.USDcon units (e.g., 2000000 = 2 USDhub or 2 USDcon)
#   - Respects EVM_INSTANCE env var for multi-instance testing

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/../util_evm.sh"
source "$SCRIPT_DIR/../chain-connected-evm/utils.sh"

# Setup project root
setup_project_root

# Load EVM instance vars
evm_instance_vars "${EVM_INSTANCE:-2}"
source "$EVM_CHAIN_INFO_FILE" 2>/dev/null || true

# Parse expected balance parameters
SOLVER_CHAIN_HUB_EXPECTED="${1:-}"
REQUESTER_CHAIN_HUB_EXPECTED="${2:-}"
SOLVER_CHAIN_CONNECTED_EXPECTED="${3:-}"
REQUESTER_CHAIN_CONNECTED_EXPECTED="${4:-}"

# Get test tokens addresses
TEST_TOKENS_HUB=$(get_profile_address "test-tokens-chain1" 2>/dev/null) || true
USD_EVM_ADDR="$USD_EVM_ADDR"

# Display balances
if [ -z "$TEST_TOKENS_HUB" ]; then
    echo "️  Warning: test-tokens-chain1 profile not found, skipping USDhub balances"
    display_balances_hub
else
    display_balances_hub "0x$TEST_TOKENS_HUB"
fi

if [ -z "$USD_EVM_ADDR" ]; then
    echo "️  Warning: USD_EVM_ADDR not found, skipping USDcon balances"
    display_balances_connected_evm
else
    display_balances_connected_evm "$USD_EVM_ADDR"
fi

# Validate solver balance on Hub
if [ -n "$SOLVER_CHAIN_HUB_EXPECTED" ] && [ "$SOLVER_CHAIN_HUB_EXPECTED" != "-1" ] && [ -n "$TEST_TOKENS_HUB" ]; then
    SOLVER_CHAIN_HUB_ADDR=$(get_profile_address "solver-chain1" 2>/dev/null || echo "")
    if [ -n "$SOLVER_CHAIN_HUB_ADDR" ]; then
        SOLVER_CHAIN_HUB_ACTUAL=$(get_usdxyz_balance "solver-chain1" "1" "0x$TEST_TOKENS_HUB" 2>/dev/null || echo "0")

        if [ "$SOLVER_CHAIN_HUB_ACTUAL" != "$SOLVER_CHAIN_HUB_EXPECTED" ]; then
            log_and_echo "❌ ERROR: Solver balance mismatch on Hub!"
            log_and_echo "   Actual:   $SOLVER_CHAIN_HUB_ACTUAL 10e-6.USDhub"
            log_and_echo "   Expected: $SOLVER_CHAIN_HUB_EXPECTED 10e-6.USDhub"
            display_service_logs "Solver balance mismatch on Hub"
            exit 1
        fi
        log_and_echo "✅ Solver balance validated on Hub: $SOLVER_CHAIN_HUB_ACTUAL 10e-6.USDhub"
    fi
fi

# Validate requester balance on Hub
if [ -n "$REQUESTER_CHAIN_HUB_EXPECTED" ] && [ "$REQUESTER_CHAIN_HUB_EXPECTED" != "-1" ] && [ -n "$TEST_TOKENS_HUB" ]; then
    REQUESTER_CHAIN_HUB_ADDR=$(get_profile_address "requester-chain1" 2>/dev/null || echo "")
    if [ -n "$REQUESTER_CHAIN_HUB_ADDR" ]; then
        REQUESTER_CHAIN_HUB_ACTUAL=$(get_usdxyz_balance "requester-chain1" "1" "0x$TEST_TOKENS_HUB" 2>/dev/null || echo "0")

        if [ "$REQUESTER_CHAIN_HUB_ACTUAL" != "$REQUESTER_CHAIN_HUB_EXPECTED" ]; then
            log_and_echo "❌ ERROR: Requester balance mismatch on Hub!"
            log_and_echo "   Actual:   $REQUESTER_CHAIN_HUB_ACTUAL 10e-6.USDhub"
            log_and_echo "   Expected: $REQUESTER_CHAIN_HUB_EXPECTED 10e-6.USDhub"
            display_service_logs "Requester balance mismatch on Hub"
            exit 1
        fi
        log_and_echo "✅ Requester balance validated on Hub: $REQUESTER_CHAIN_HUB_ACTUAL 10e-6.USDhub"
    fi
fi

# Validate solver balance on Connected EVM
if [ -n "$SOLVER_CHAIN_CONNECTED_EXPECTED" ] && [ "$SOLVER_CHAIN_CONNECTED_EXPECTED" != "-1" ] && [ -n "$USD_EVM_ADDR" ]; then
    SOLVER_EVM_ADDR=$(get_hardhat_account_address "2" "$EVM_NETWORK")

    if [ -n "$SOLVER_EVM_ADDR" ]; then
        SOLVER_CHAIN_CONNECTED_ACTUAL=$(get_usdcon_balance_evm "$SOLVER_EVM_ADDR" "$USD_EVM_ADDR")

        if [ "$SOLVER_CHAIN_CONNECTED_ACTUAL" != "$SOLVER_CHAIN_CONNECTED_EXPECTED" ]; then
            log_and_echo "❌ ERROR: Solver balance mismatch on Connected EVM (instance $EVM_INSTANCE)!"
            log_and_echo "   Actual:   $SOLVER_CHAIN_CONNECTED_ACTUAL 10e-6.USDcon"
            log_and_echo "   Expected: $SOLVER_CHAIN_CONNECTED_EXPECTED 10e-6.USDcon"
            display_service_logs "Solver balance mismatch on Connected EVM"
            exit 1
        fi
        log_and_echo "✅ Solver balance validated on Connected EVM (instance $EVM_INSTANCE): $SOLVER_CHAIN_CONNECTED_ACTUAL 10e-6.USDcon"
    fi
fi

# Validate requester balance on Connected EVM
if [ -n "$REQUESTER_CHAIN_CONNECTED_EXPECTED" ] && [ "$REQUESTER_CHAIN_CONNECTED_EXPECTED" != "-1" ] && [ -n "$USD_EVM_ADDR" ]; then
    REQUESTER_EVM_ADDR=$(get_hardhat_account_address "1" "$EVM_NETWORK")

    if [ -n "$REQUESTER_EVM_ADDR" ]; then
        REQUESTER_CHAIN_CONNECTED_ACTUAL=$(get_usdcon_balance_evm "$REQUESTER_EVM_ADDR" "$USD_EVM_ADDR")

        if [ "$REQUESTER_CHAIN_CONNECTED_ACTUAL" != "$REQUESTER_CHAIN_CONNECTED_EXPECTED" ]; then
            log_and_echo "❌ ERROR: Requester balance mismatch on Connected EVM (instance $EVM_INSTANCE)!"
            log_and_echo "   Actual:   $REQUESTER_CHAIN_CONNECTED_ACTUAL 10e-6.USDcon"
            log_and_echo "   Expected: $REQUESTER_CHAIN_CONNECTED_EXPECTED 10e-6.USDcon"
            display_service_logs "Requester balance mismatch on Connected EVM"
            exit 1
        fi
        log_and_echo "✅ Requester balance validated on Connected EVM (instance $EVM_INSTANCE): $REQUESTER_CHAIN_CONNECTED_ACTUAL 10e-6.USDcon"
    fi
fi

# Explicit success exit (prevents set -e issues from log_and_echo return code)
exit 0
