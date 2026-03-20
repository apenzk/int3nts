#!/bin/bash

# Check Mainnet Balances Script
# Checks balances for all accounts in .env.mainnet
# Supports:
#   - Movement Mainnet (MOVE)
#   - Base Mainnet (ETH)
#   - HyperEVM Mainnet (HYPE)
#
# Asset addresses are read from testing-infra/networks/mainnet/config/mainnet-assets.toml

# Get the script directory and project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../.." && pwd )"
export PROJECT_ROOT

# Source utilities (for error handling only, not logging)
source "$PROJECT_ROOT/testing-infra/ci-e2e/util.sh" 2>/dev/null || true

echo " Checking Mainnet Balances"
echo "============================"
echo ""

# Load .env.mainnet
MAINNET_KEYS_FILE="$SCRIPT_DIR/.env.mainnet"

if [ ! -f "$MAINNET_KEYS_FILE" ]; then
    echo "ERROR: .env.mainnet not found at $MAINNET_KEYS_FILE"
    echo "   Create it from env.mainnet.example in this directory"
    exit 1
fi

source "$MAINNET_KEYS_FILE"

# Load assets configuration
ASSETS_CONFIG_FILE="$SCRIPT_DIR/config/mainnet-assets.toml"

if [ ! -f "$ASSETS_CONFIG_FILE" ]; then
    echo "ERROR: mainnet-assets.toml not found at $ASSETS_CONFIG_FILE"
    exit 1
fi

# Extract native token decimals
MOVEMENT_NATIVE_DECIMALS=$(grep -A 10 "^\[movement_mainnet\]" "$ASSETS_CONFIG_FILE" | grep "^native_token_decimals = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "")
if [ -z "$MOVEMENT_NATIVE_DECIMALS" ]; then
    echo "ERROR: Movement native token decimals not found in mainnet-assets.toml"
    exit 1
fi

BASE_NATIVE_DECIMALS=$(grep -A 10 "^\[base_mainnet\]" "$ASSETS_CONFIG_FILE" | grep "^native_token_decimals = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "")
if [ -z "$BASE_NATIVE_DECIMALS" ]; then
    echo "ERROR: Base native token decimals not found in mainnet-assets.toml"
    exit 1
fi

HYPERLIQUID_NATIVE_DECIMALS=$(grep -A 10 "^\[hyperliquid_mainnet\]" "$ASSETS_CONFIG_FILE" | grep "^native_token_decimals = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "")
if [ -z "$HYPERLIQUID_NATIVE_DECIMALS" ]; then
    echo "ERROR: HyperEVM native token decimals not found in mainnet-assets.toml"
    exit 1
fi

# Extract RPC URLs
MOVEMENT_RPC_URL="https://mainnet.movementnetwork.xyz/v1"

if [ -z "$BASE_RPC_URL" ]; then
    BASE_RPC_URL=$(grep -A 5 "^\[base_mainnet\]" "$ASSETS_CONFIG_FILE" | grep "^rpc_url = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
fi
if [ -z "$BASE_RPC_URL" ]; then
    echo "WARNING: BASE_RPC_URL not set and not in mainnet-assets.toml"
    echo "   Base balance checks will fail"
fi

if [ -z "$HYPERLIQUID_RPC_URL" ]; then
    HYPERLIQUID_RPC_URL=$(grep -A 5 "^\[hyperliquid_mainnet\]" "$ASSETS_CONFIG_FILE" | grep "^rpc_url = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
fi
if [ -z "$HYPERLIQUID_RPC_URL" ]; then
    echo "WARNING: HYPERLIQUID_RPC_URL not set and not in mainnet-assets.toml"
    echo "   HyperEVM balance checks will fail"
fi

# Function to get Movement balance (MOVE tokens)
get_movement_balance() {
    local address="$1"
    if [[ ! "$address" =~ ^0x ]]; then
        address="0x${address}"
    fi

    local balance=$(curl -s --max-time 10 -X POST "${MOVEMENT_RPC_URL}/view" \
        -H "Content-Type: application/json" \
        -d "{\"function\":\"0x1::coin::balance\",\"type_arguments\":[\"0x1::aptos_coin::AptosCoin\"],\"arguments\":[\"$address\"]}" \
        | jq -r '.[0] // "0"' 2>/dev/null)

    if [ -z "$balance" ] || [ "$balance" = "null" ]; then
        echo "0"
    else
        echo "$balance"
    fi
}

# Function to get EVM ETH balance (works for any EVM chain)
get_evm_eth_balance() {
    local address="$1"
    local rpc_url="$2"

    if [[ ! "$address" =~ ^0x ]]; then
        address="0x${address}"
    fi

    local balance_hex=$(curl -s --max-time 10 -X POST "$rpc_url" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBalance\",\"params\":[\"$address\",\"latest\"],\"id\":1}" \
        | jq -r '.result // "0x0"' 2>/dev/null)

    if [ -z "$balance_hex" ] || [ "$balance_hex" = "null" ] || [ "$balance_hex" = "0x0" ]; then
        echo "0"
    else
        local hex_no_prefix="${balance_hex#0x}"
        local hex_upper=$(echo "$hex_no_prefix" | tr '[:lower:]' '[:upper:]')
        echo "obase=10; ibase=16; $hex_upper" | bc 2>/dev/null || echo "0"
    fi
}

# Format balance for display
format_balance() {
    local balance="$1"
    local decimals="$2"
    local symbol="${3:-}"

    local divisor
    case "$decimals" in
        18) divisor="1000000000000000000" ;;
        8)  divisor="100000000" ;;
        6)  divisor="1000000" ;;
        *)  divisor="1" ;;
    esac

    local formatted=$(echo "scale=6; $balance / $divisor" | bc 2>/dev/null || echo "0")

    if [ -n "$symbol" ]; then
        printf "%.6f %s" "$formatted" "$symbol"
    else
        printf "%.6f" "$formatted"
    fi
}

# ============================================================================
# Movement Mainnet
# ============================================================================
echo " Movement Mainnet"
echo "----------------------------"
echo "   RPC: $MOVEMENT_RPC_URL"

for role_var in MOVEMENT_DEPLOYER_ADDR MOVEMENT_REQUESTER_ADDR MOVEMENT_SOLVER_ADDR; do
    addr="${!role_var}"
    label="${role_var#MOVEMENT_}"
    label="${label%_ADDR}"
    label=$(echo "$label" | tr '[:upper:]' '[:lower:]' | sed 's/^./\U&/')

    if [ -z "$addr" ]; then
        echo "   ${role_var} not set in .env.mainnet"
    else
        balance=$(get_movement_balance "$addr")
        formatted=$(format_balance "$balance" "$MOVEMENT_NATIVE_DECIMALS" "MOVE")
        printf "   %-10s (%s)\n" "$label" "$addr"
        echo "             $formatted"
    fi
done

echo ""

# ============================================================================
# Base Mainnet
# ============================================================================
echo " Base Mainnet"
echo "---------------"
echo "   RPC: $BASE_RPC_URL"

for role_var in BASE_DEPLOYER_ADDR BASE_REQUESTER_ADDR BASE_SOLVER_ADDR; do
    addr="${!role_var}"
    label="${role_var#BASE_}"
    label="${label%_ADDR}"
    label=$(echo "$label" | tr '[:upper:]' '[:lower:]' | sed 's/^./\U&/')

    if [ -z "$addr" ]; then
        echo "   ${role_var} not set in .env.mainnet"
    else
        eth_balance=$(get_evm_eth_balance "$addr" "$BASE_RPC_URL")
        eth_formatted=$(format_balance "$eth_balance" "$BASE_NATIVE_DECIMALS" "ETH")
        printf "   %-10s (%s)\n" "$label" "$addr"
        echo "             $eth_formatted"
    fi
done

echo ""

# ============================================================================
# HyperEVM Mainnet
# ============================================================================
echo " HyperEVM Mainnet"
echo "-------------------"
echo "   RPC: $HYPERLIQUID_RPC_URL"

for role_var in HYPERLIQUID_DEPLOYER_ADDR HYPERLIQUID_REQUESTER_ADDR HYPERLIQUID_SOLVER_ADDR; do
    addr="${!role_var}"
    label="${role_var#HYPERLIQUID_}"
    label="${label%_ADDR}"
    label=$(echo "$label" | tr '[:upper:]' '[:lower:]' | sed 's/^./\U&/')

    if [ -z "$addr" ]; then
        echo "   ${role_var} not set in .env.mainnet"
    else
        eth_balance=$(get_evm_eth_balance "$addr" "$HYPERLIQUID_RPC_URL")
        eth_formatted=$(format_balance "$eth_balance" "$HYPERLIQUID_NATIVE_DECIMALS" "HYPE")
        printf "   %-10s (%s)\n" "$label" "$addr"
        echo "             $eth_formatted"
    fi
done

echo ""
echo "   Config file: $ASSETS_CONFIG_FILE"
echo ""
echo "Balance check complete!"
