#!/bin/bash

# Check Testnet Balances Script
# Checks balances for all accounts in .env.testnet
# Supports:
#   - Movement Bardock Testnet (MOVE, USDC.e, USDC, USDT, WETH)
#   - Base Sepolia (ETH, USDC)
#   - Ethereum Sepolia (ETH, USDC)
# 
# Asset addresses are read from testing-infra/testnet/config/testnet-assets.toml

# Get the script directory and project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"
export PROJECT_ROOT

# Source utilities (for error handling only, not logging)
source "$PROJECT_ROOT/testing-infra/ci-e2e/util.sh" 2>/dev/null || true

echo " Checking Testnet Balances"
echo "============================"
echo ""

# Load .env.testnet
TESTNET_KEYS_FILE="$SCRIPT_DIR/.env.testnet"

if [ ! -f "$TESTNET_KEYS_FILE" ]; then
    echo "❌ ERROR: .env.testnet not found at $TESTNET_KEYS_FILE"
    echo "   Create it from env.testnet.example in this directory"
    exit 1
fi

# Source the keys file
source "$TESTNET_KEYS_FILE"

# Load assets configuration
ASSETS_CONFIG_FILE="$PROJECT_ROOT/testing-infra/testnet/config/testnet-assets.toml"

if [ ! -f "$ASSETS_CONFIG_FILE" ]; then
    echo "❌ ERROR: testnet-assets.toml not found at $ASSETS_CONFIG_FILE"
    echo "   Asset addresses must be configured in testing-infra/testnet/config/testnet-assets.toml"
    exit 1
fi

# Parse TOML config (simple grep-based parser)
# Extract Base Sepolia USDC address and decimals
BASE_USDC_ADDR=$(grep -A 20 "^\[base_sepolia\]" "$ASSETS_CONFIG_FILE" | grep "^usdc = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
BASE_USDC_DECIMALS=$(grep -A 20 "^\[base_sepolia\]" "$ASSETS_CONFIG_FILE" | grep "^usdc_decimals = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "")
if [ -z "$BASE_USDC_ADDR" ]; then
    echo "️  WARNING: Base Sepolia USDC address not found in testnet-assets.toml"
    echo "   Base Sepolia USDC balance checks will be skipped"
elif [ -z "$BASE_USDC_DECIMALS" ]; then
    echo "❌ ERROR: Base Sepolia USDC decimals not found in testnet-assets.toml"
    echo "   Add usdc_decimals = 6 to [base_sepolia] section"
    exit 1
fi

# Extract Ethereum Sepolia USDC address and decimals
SEPOLIA_USDC_ADDR=$(grep -A 20 "^\[ethereum_sepolia\]" "$ASSETS_CONFIG_FILE" | grep "^usdc = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
SEPOLIA_USDC_DECIMALS=$(grep -A 20 "^\[ethereum_sepolia\]" "$ASSETS_CONFIG_FILE" | grep "^usdc_decimals = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "")
if [ -z "$SEPOLIA_USDC_ADDR" ]; then
    echo "️  WARNING: Ethereum Sepolia USDC address not found in testnet-assets.toml"
    echo "   Ethereum Sepolia USDC balance checks will be skipped"
elif [ -z "$SEPOLIA_USDC_DECIMALS" ]; then
    echo "❌ ERROR: Ethereum Sepolia USDC decimals not found in testnet-assets.toml"
    echo "   Add usdc_decimals = 6 to [ethereum_sepolia] section"
    exit 1
fi

# Extract Movement USDC.e address and decimals
MOVEMENT_USDC_E_ADDR=$(grep -A 20 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^usdc_e = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
MOVEMENT_USDC_E_DECIMALS=$(grep -A 20 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^usdc_e_decimals = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "")
if [ -n "$MOVEMENT_USDC_E_ADDR" ] && [ -z "$MOVEMENT_USDC_E_DECIMALS" ]; then
    echo "❌ ERROR: Movement USDC.e address configured but decimals not found in testnet-assets.toml"
    echo "   Add usdc_e_decimals = 6 to [movement_bardock_testnet] section"
    exit 1
fi

# Extract new Movement tokens (USDC, USDT, WETH) - FA metadata addresses
MOVEMENT_USDC=$(grep -A 30 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^usdc = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
MOVEMENT_USDC_DECIMALS=$(grep -A 30 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^usdc_decimals = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "6")
MOVEMENT_USDT=$(grep -A 30 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^usdt = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
MOVEMENT_USDT_DECIMALS=$(grep -A 30 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^usdt_decimals = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "6")
MOVEMENT_WETH=$(grep -A 30 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^weth = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
MOVEMENT_WETH_DECIMALS=$(grep -A 30 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^weth_decimals = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "18")

# Coin types for CoinStore balance checking (tokens may be in CoinStore instead of FA)
MOVEMENT_USDC_COIN_TYPE="0xa6cc575a28e9c97d1cec569392fe6f698c593990e7029ef49fed6740a36a31b0::tokens::USDC"
MOVEMENT_USDT_COIN_TYPE="0xa6cc575a28e9c97d1cec569392fe6f698c593990e7029ef49fed6740a36a31b0::tokens::USDT"
MOVEMENT_WETH_COIN_TYPE="0xa6cc575a28e9c97d1cec569392fe6f698c593990e7029ef49fed6740a36a31b0::tokens::WETH"

# Extract native token decimals
MOVEMENT_NATIVE_DECIMALS=$(grep -A 10 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^native_token_decimals = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "")
if [ -z "$MOVEMENT_NATIVE_DECIMALS" ]; then
    echo "❌ ERROR: Movement native token decimals not found in testnet-assets.toml"
    echo "   Add native_token_decimals = 8 to [movement_bardock_testnet] section"
    exit 1
fi

BASE_NATIVE_DECIMALS=$(grep -A 10 "^\[base_sepolia\]" "$ASSETS_CONFIG_FILE" | grep "^native_token_decimals = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "")
if [ -z "$BASE_NATIVE_DECIMALS" ]; then
    echo "❌ ERROR: Base Sepolia native token decimals not found in testnet-assets.toml"
    echo "   Add native_token_decimals = 18 to [base_sepolia] section"
    exit 1
fi

SEPOLIA_NATIVE_DECIMALS=$(grep -A 10 "^\[ethereum_sepolia\]" "$ASSETS_CONFIG_FILE" | grep "^native_token_decimals = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "")
if [ -z "$SEPOLIA_NATIVE_DECIMALS" ]; then
    echo "❌ ERROR: Ethereum Sepolia native token decimals not found in testnet-assets.toml"
    echo "   Add native_token_decimals = 18 to [ethereum_sepolia] section"
    exit 1
fi

# Extract RPC URLs
MOVEMENT_RPC_URL=$(grep -A 5 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^rpc_url = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
if [ -z "$MOVEMENT_RPC_URL" ]; then
    echo "️  WARNING: Movement RPC URL not found in testnet-assets.toml"
    echo "   Movement balance checks will fail"
fi

BASE_RPC_URL=$(grep -A 5 "^\[base_sepolia\]" "$ASSETS_CONFIG_FILE" | grep "^rpc_url = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
if [ -z "$BASE_RPC_URL" ]; then
    echo "️  WARNING: Base Sepolia RPC URL not found in testnet-assets.toml"
    echo "   Base Sepolia balance checks will fail"
fi

# Substitute API key in Base Sepolia RPC URL if placeholder is present
if [[ "$BASE_RPC_URL" == *"ALCHEMY_API_KEY"* ]]; then
    if [ -n "$ALCHEMY_BASE_SEPOLIA_API_KEY" ]; then
        BASE_RPC_URL="${BASE_RPC_URL/ALCHEMY_API_KEY/$ALCHEMY_BASE_SEPOLIA_API_KEY}"
    else
        echo "️  WARNING: ALCHEMY_BASE_SEPOLIA_API_KEY not set in .env.testnet"
        echo "   Base Sepolia balance checks will fail"
    fi
fi

SEPOLIA_RPC_URL=$(grep -A 5 "^\[ethereum_sepolia\]" "$ASSETS_CONFIG_FILE" | grep "^rpc_url = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
if [ -z "$SEPOLIA_RPC_URL" ]; then
    echo "️  WARNING: Ethereum Sepolia RPC URL not found in testnet-assets.toml"
    echo "   Ethereum Sepolia balance checks will fail"
fi

# Substitute API key in Sepolia RPC URL if placeholder is present
if [[ "$SEPOLIA_RPC_URL" == *"ALCHEMY_API_KEY"* ]]; then
    if [ -n "$ALCHEMY_ETH_SEPOLIA_API_KEY" ]; then
        SEPOLIA_RPC_URL="${SEPOLIA_RPC_URL/ALCHEMY_API_KEY/$ALCHEMY_ETH_SEPOLIA_API_KEY}"
    else
        echo "️  WARNING: ALCHEMY_ETH_SEPOLIA_API_KEY not set in .env.testnet"
        echo "   Ethereum Sepolia balance checks will fail"
    fi
fi

# Function to get Movement balance (MOVE tokens)
# Uses the view function API to get balance (works with both CoinStore and FA systems)
get_movement_balance() {
    local address="$1"
    # Ensure address has 0x prefix
    if [[ ! "$address" =~ ^0x ]]; then
        address="0x${address}"
    fi
    
    # Query balance via view function API (with 10 second timeout)
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

# Function to get Movement USDC.e balance (Fungible Asset)
get_movement_usdc_e_balance() {
    local address="$1"
    # Ensure address has 0x prefix
    if [[ ! "$address" =~ ^0x ]]; then
        address="0x${address}"
    fi
    
    # If USDC.e address is not configured, return 0
    if [ -z "$MOVEMENT_USDC_E_ADDR" ] || [ "$MOVEMENT_USDC_E_ADDR" = "" ]; then
        echo "0"
        return
    fi
    
    # Query USDC.e balance via view function API (Fungible Asset)
    # USDC.e is deployed as a Fungible Asset, use primary_fungible_store::balance
    local balance=$(curl -s --max-time 10 -X POST "${MOVEMENT_RPC_URL}/view" \
        -H "Content-Type: application/json" \
        -d "{\"function\":\"0x1::primary_fungible_store::balance\",\"type_arguments\":[\"0x1::fungible_asset::Metadata\"],\"arguments\":[\"$address\",\"${MOVEMENT_USDC_E_ADDR}\"]}" \
        | jq -r '.[0] // "0"' 2>/dev/null)
    
    if [ -z "$balance" ] || [ "$balance" = "null" ]; then
        echo "0"
    else
        echo "$balance"
    fi
}

# Function to get Movement token FA balance (primary_fungible_store)
get_movement_fa_balance() {
    local address="$1"
    local metadata_addr="$2"  # FA metadata address
    # Ensure address has 0x prefix
    if [[ ! "$address" =~ ^0x ]]; then
        address="0x${address}"
    fi
    
    if [ -z "$metadata_addr" ] || [ "$metadata_addr" = "" ]; then
        echo "0"
        return
    fi
    
    local balance=$(curl -s --max-time 10 -X POST "${MOVEMENT_RPC_URL}/view" \
        -H "Content-Type: application/json" \
        -d "{\"function\":\"0x1::primary_fungible_store::balance\",\"type_arguments\":[\"0x1::fungible_asset::Metadata\"],\"arguments\":[\"$address\",\"${metadata_addr}\"]}" \
        | jq -r '.[0] // "0"' 2>/dev/null)
    
    if [ -z "$balance" ] || [ "$balance" = "null" ]; then
        echo "0"
    else
        echo "$balance"
    fi
}

# Function to get Movement token Coin balance (CoinStore)
# This checks if CoinStore resource actually exists, not using coin::balance
# which falls back to FA after migration
get_movement_coin_balance() {
    local address="$1"
    local coin_type="$2"  # Coin type, e.g., "0xa6cc...::tokens::USDC"
    # Ensure address has 0x prefix
    if [[ ! "$address" =~ ^0x ]]; then
        address="0x${address}"
    fi
    
    if [ -z "$coin_type" ] || [ "$coin_type" = "" ]; then
        echo "0"
        return
    fi
    
    # Check if CoinStore resource exists by querying the resource directly
    # If CoinStore was destroyed (after migration), this will return null/error
    # URL-encode the < and > characters
    local coin_store_type="0x1::coin::CoinStore%3C${coin_type}%3E"
    local resource=$(curl -s --max-time 10 "${MOVEMENT_RPC_URL}/accounts/${address}/resource/${coin_store_type}" 2>/dev/null)
    
    # Check if we got a valid response with coin value
    local balance=$(echo "$resource" | jq -r '.data.coin.value // "0"' 2>/dev/null)
    
    if [ -z "$balance" ] || [ "$balance" = "null" ] || [ "$balance" = "0" ]; then
        # CoinStore doesn't exist or is empty
        echo "0"
    else
        echo "$balance"
    fi
}

# Function to get EVM ETH balance (works for any EVM chain)
get_evm_eth_balance() {
    local address="$1"
    local rpc_url="$2"
    
    # Ensure address has 0x prefix
    if [[ ! "$address" =~ ^0x ]]; then
        address="0x${address}"
    fi
    
    # Query balance via JSON-RPC (with 10 second timeout)
    local balance_hex=$(curl -s --max-time 10 -X POST "$rpc_url" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBalance\",\"params\":[\"$address\",\"latest\"],\"id\":1}" \
        | jq -r '.result // "0x0"' 2>/dev/null)
    
    if [ -z "$balance_hex" ] || [ "$balance_hex" = "null" ] || [ "$balance_hex" = "0x0" ]; then
        echo "0"
    else
        # Convert hex to decimal (remove 0x, uppercase, use bc for large numbers)
        local hex_no_prefix="${balance_hex#0x}"
        local hex_upper=$(echo "$hex_no_prefix" | tr '[:lower:]' '[:upper:]')
        echo "obase=10; ibase=16; $hex_upper" | bc 2>/dev/null || echo "0"
    fi
}

# Function to get ERC20 token balance (works for any EVM chain)
get_evm_token_balance() {
    local address="$1"
    local token_addr="$2"
    local rpc_url="$3"
    
    # Ensure addresses have 0x prefix
    if [[ ! "$address" =~ ^0x ]]; then
        address="0x${address}"
    fi
    if [[ ! "$token_addr" =~ ^0x ]]; then
        token_addr="0x${token_addr}"
    fi
    
    # ERC20 balanceOf(address) - function selector: 0x70a08231
    # Pad address to 64 hex characters (32 bytes) with leading zeros
    local addr_no_prefix="${address#0x}"
    local addr_padded=$(printf "%064s" "$addr_no_prefix" | sed 's/ /0/g')
    local data="0x70a08231$addr_padded"
    
    local balance_hex=$(curl -s --max-time 10 -X POST "$rpc_url" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_call\",\"params\":[{\"to\":\"$token_addr\",\"data\":\"$data\"},\"latest\"],\"id\":1}" \
        | jq -r '.result // "0x0"' 2>/dev/null)
    
    if [ -z "$balance_hex" ] || [ "$balance_hex" = "null" ] || [ "$balance_hex" = "0x0" ]; then
        echo "0"
    else
        # Convert hex to decimal (remove 0x, uppercase, use bc for large numbers)
        local hex_no_prefix="${balance_hex#0x}"
        local hex_upper=$(echo "$hex_no_prefix" | tr '[:lower:]' '[:upper:]')
        echo "obase=10; ibase=16; $hex_upper" | bc 2>/dev/null || echo "0"
    fi
}

# Wrapper functions for backwards compatibility
get_base_eth_balance() {
    get_evm_eth_balance "$1" "$BASE_RPC_URL"
}

get_base_token_balance() {
    get_evm_token_balance "$1" "$2" "$BASE_RPC_URL"
}

# Format balance for display (number only, no symbol)
format_balance_number() {
    local balance="$1"
    local decimals="$2"
    
    local divisor
    case "$decimals" in
        18) divisor="1000000000000000000" ;;
        8)  divisor="100000000" ;;
        6)  divisor="1000000" ;;
        *)  divisor="1" ;;
    esac
    
    local formatted=$(echo "scale=6; $balance / $divisor" | bc 2>/dev/null || echo "0")
    printf "%.6f" "$formatted"
}

# Format balance for display (with optional symbol)
format_balance() {
    local balance="$1"
    local decimals="$2"
    local symbol="${3:-}"
    
    # Convert from smallest unit to human-readable
    # Decimals must be provided (read from testnet-assets.toml config)
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
        case "$decimals" in
            18) printf "%.6f ETH" "$formatted" ;;
            8)  printf "%.6f MOVE" "$formatted" ;;
            6)  printf "%.6f USDC" "$formatted" ;;
            *)  printf "%s" "$balance" ;;
        esac
    fi
}

# Check Movement balances
echo " Movement Bardock Testnet"
echo "----------------------------"
echo "   RPC: $MOVEMENT_RPC_URL"

if [ -z "$MOVEMENT_DEPLOYER_ADDR" ]; then
    echo "️  MOVEMENT_DEPLOYER_ADDR not set in .env.testnet"
else
    echo "   Deployer  ($MOVEMENT_DEPLOYER_ADDR)"
    # MOVE (native)
    balance=$(get_movement_balance "$MOVEMENT_DEPLOYER_ADDR")
    formatted=$(format_balance "$balance" "$MOVEMENT_NATIVE_DECIMALS")
    echo "             MOVE: $formatted"
    # USDC.e (FA only)
    if [ -n "$MOVEMENT_USDC_E_ADDR" ]; then
        usdc_e_balance=$(get_movement_usdc_e_balance "$MOVEMENT_DEPLOYER_ADDR")
        usdc_e_formatted=$(format_balance_number "$usdc_e_balance" "$MOVEMENT_USDC_E_DECIMALS")
        echo "             USDC.e: $usdc_e_formatted"
    fi
    # USDC (FA/Coin)
    if [ -n "$MOVEMENT_USDC" ]; then
        usdc_fa=$(get_movement_fa_balance "$MOVEMENT_DEPLOYER_ADDR" "$MOVEMENT_USDC")
        usdc_coin=$(get_movement_coin_balance "$MOVEMENT_DEPLOYER_ADDR" "$MOVEMENT_USDC_COIN_TYPE")
        usdc_fa_fmt=$(format_balance_number "$usdc_fa" "$MOVEMENT_USDC_DECIMALS")
        usdc_coin_fmt=$(format_balance_number "$usdc_coin" "$MOVEMENT_USDC_DECIMALS")
        echo "             USDC: $usdc_fa_fmt FA / $usdc_coin_fmt Coin"
    fi
    # USDT (FA/Coin)
    if [ -n "$MOVEMENT_USDT" ]; then
        usdt_fa=$(get_movement_fa_balance "$MOVEMENT_DEPLOYER_ADDR" "$MOVEMENT_USDT")
        usdt_coin=$(get_movement_coin_balance "$MOVEMENT_DEPLOYER_ADDR" "$MOVEMENT_USDT_COIN_TYPE")
        usdt_fa_fmt=$(format_balance_number "$usdt_fa" "$MOVEMENT_USDT_DECIMALS")
        usdt_coin_fmt=$(format_balance_number "$usdt_coin" "$MOVEMENT_USDT_DECIMALS")
        echo "             USDT: $usdt_fa_fmt FA / $usdt_coin_fmt Coin"
    fi
    # WETH (FA/Coin)
    if [ -n "$MOVEMENT_WETH" ]; then
        weth_fa=$(get_movement_fa_balance "$MOVEMENT_DEPLOYER_ADDR" "$MOVEMENT_WETH")
        weth_coin=$(get_movement_coin_balance "$MOVEMENT_DEPLOYER_ADDR" "$MOVEMENT_WETH_COIN_TYPE")
        weth_fa_fmt=$(format_balance_number "$weth_fa" "$MOVEMENT_WETH_DECIMALS")
        weth_coin_fmt=$(format_balance_number "$weth_coin" "$MOVEMENT_WETH_DECIMALS")
        echo "             WETH: $weth_fa_fmt FA / $weth_coin_fmt Coin"
    fi
fi

if [ -z "$MOVEMENT_REQUESTER_ADDR" ]; then
    echo "️  MOVEMENT_REQUESTER_ADDR not set in .env.testnet"
else
    echo "   Requester ($MOVEMENT_REQUESTER_ADDR)"
    # MOVE (native)
    balance=$(get_movement_balance "$MOVEMENT_REQUESTER_ADDR")
    formatted=$(format_balance "$balance" "$MOVEMENT_NATIVE_DECIMALS")
    echo "             MOVE: $formatted"
    # USDC.e (FA only)
    if [ -n "$MOVEMENT_USDC_E_ADDR" ]; then
        usdc_e_balance=$(get_movement_usdc_e_balance "$MOVEMENT_REQUESTER_ADDR")
        usdc_e_formatted=$(format_balance_number "$usdc_e_balance" "$MOVEMENT_USDC_E_DECIMALS")
        echo "             USDC.e: $usdc_e_formatted"
    fi
    # USDC (FA/Coin)
    if [ -n "$MOVEMENT_USDC" ]; then
        usdc_fa=$(get_movement_fa_balance "$MOVEMENT_REQUESTER_ADDR" "$MOVEMENT_USDC")
        usdc_coin=$(get_movement_coin_balance "$MOVEMENT_REQUESTER_ADDR" "$MOVEMENT_USDC_COIN_TYPE")
        usdc_fa_fmt=$(format_balance_number "$usdc_fa" "$MOVEMENT_USDC_DECIMALS")
        usdc_coin_fmt=$(format_balance_number "$usdc_coin" "$MOVEMENT_USDC_DECIMALS")
        echo "             USDC: $usdc_fa_fmt FA / $usdc_coin_fmt Coin"
    fi
    # USDT (FA/Coin)
    if [ -n "$MOVEMENT_USDT" ]; then
        usdt_fa=$(get_movement_fa_balance "$MOVEMENT_REQUESTER_ADDR" "$MOVEMENT_USDT")
        usdt_coin=$(get_movement_coin_balance "$MOVEMENT_REQUESTER_ADDR" "$MOVEMENT_USDT_COIN_TYPE")
        usdt_fa_fmt=$(format_balance_number "$usdt_fa" "$MOVEMENT_USDT_DECIMALS")
        usdt_coin_fmt=$(format_balance_number "$usdt_coin" "$MOVEMENT_USDT_DECIMALS")
        echo "             USDT: $usdt_fa_fmt FA / $usdt_coin_fmt Coin"
    fi
    # WETH (FA/Coin)
    if [ -n "$MOVEMENT_WETH" ]; then
        weth_fa=$(get_movement_fa_balance "$MOVEMENT_REQUESTER_ADDR" "$MOVEMENT_WETH")
        weth_coin=$(get_movement_coin_balance "$MOVEMENT_REQUESTER_ADDR" "$MOVEMENT_WETH_COIN_TYPE")
        weth_fa_fmt=$(format_balance_number "$weth_fa" "$MOVEMENT_WETH_DECIMALS")
        weth_coin_fmt=$(format_balance_number "$weth_coin" "$MOVEMENT_WETH_DECIMALS")
        echo "             WETH: $weth_fa_fmt FA / $weth_coin_fmt Coin"
    fi
fi

if [ -z "$MOVEMENT_SOLVER_ADDR" ]; then
    echo "️  MOVEMENT_SOLVER_ADDR not set in .env.testnet"
else
    echo "   Solver    ($MOVEMENT_SOLVER_ADDR)"
    # MOVE (native)
    balance=$(get_movement_balance "$MOVEMENT_SOLVER_ADDR")
    formatted=$(format_balance "$balance" "$MOVEMENT_NATIVE_DECIMALS")
    echo "             MOVE: $formatted"
    # USDC.e (FA only)
    if [ -n "$MOVEMENT_USDC_E_ADDR" ]; then
        usdc_e_balance=$(get_movement_usdc_e_balance "$MOVEMENT_SOLVER_ADDR")
        usdc_e_formatted=$(format_balance_number "$usdc_e_balance" "$MOVEMENT_USDC_E_DECIMALS")
        echo "             USDC.e: $usdc_e_formatted"
    fi
    # USDC (FA/Coin)
    if [ -n "$MOVEMENT_USDC" ]; then
        usdc_fa=$(get_movement_fa_balance "$MOVEMENT_SOLVER_ADDR" "$MOVEMENT_USDC")
        usdc_coin=$(get_movement_coin_balance "$MOVEMENT_SOLVER_ADDR" "$MOVEMENT_USDC_COIN_TYPE")
        usdc_fa_fmt=$(format_balance_number "$usdc_fa" "$MOVEMENT_USDC_DECIMALS")
        usdc_coin_fmt=$(format_balance_number "$usdc_coin" "$MOVEMENT_USDC_DECIMALS")
        echo "             USDC: $usdc_fa_fmt FA / $usdc_coin_fmt Coin"
    fi
    # USDT (FA/Coin)
    if [ -n "$MOVEMENT_USDT" ]; then
        usdt_fa=$(get_movement_fa_balance "$MOVEMENT_SOLVER_ADDR" "$MOVEMENT_USDT")
        usdt_coin=$(get_movement_coin_balance "$MOVEMENT_SOLVER_ADDR" "$MOVEMENT_USDT_COIN_TYPE")
        usdt_fa_fmt=$(format_balance_number "$usdt_fa" "$MOVEMENT_USDT_DECIMALS")
        usdt_coin_fmt=$(format_balance_number "$usdt_coin" "$MOVEMENT_USDT_DECIMALS")
        echo "             USDT: $usdt_fa_fmt FA / $usdt_coin_fmt Coin"
    fi
    # WETH (FA/Coin)
    if [ -n "$MOVEMENT_WETH" ]; then
        weth_fa=$(get_movement_fa_balance "$MOVEMENT_SOLVER_ADDR" "$MOVEMENT_WETH")
        weth_coin=$(get_movement_coin_balance "$MOVEMENT_SOLVER_ADDR" "$MOVEMENT_WETH_COIN_TYPE")
        weth_fa_fmt=$(format_balance_number "$weth_fa" "$MOVEMENT_WETH_DECIMALS")
        weth_coin_fmt=$(format_balance_number "$weth_coin" "$MOVEMENT_WETH_DECIMALS")
        echo "             WETH: $weth_fa_fmt FA / $weth_coin_fmt Coin"
    fi
fi

echo ""

# Check Base Sepolia balances
echo " Base Sepolia"
echo "---------------"
echo "   RPC: $BASE_RPC_URL"

if [ -z "$BASE_DEPLOYER_ADDR" ]; then
    echo "️  BASE_DEPLOYER_ADDR not set in .env.testnet"
else
    eth_balance=$(get_base_eth_balance "$BASE_DEPLOYER_ADDR")
    eth_formatted=$(format_balance "$eth_balance" "$BASE_NATIVE_DECIMALS")
    echo "   Deployer  ($BASE_DEPLOYER_ADDR)"
    if [ -n "$BASE_USDC_ADDR" ]; then
        usdc_balance=$(get_base_token_balance "$BASE_DEPLOYER_ADDR" "$BASE_USDC_ADDR")
        usdc_formatted=$(format_balance "$usdc_balance" "$BASE_USDC_DECIMALS" "USDC")
        echo "             $eth_formatted, $usdc_formatted"
    else
        echo "             $eth_formatted (USDC n/a)"
    fi
fi

if [ -z "$BASE_REQUESTER_ADDR" ]; then
    echo "️  BASE_REQUESTER_ADDR not set in .env.testnet"
else
    eth_balance=$(get_base_eth_balance "$BASE_REQUESTER_ADDR")
    eth_formatted=$(format_balance "$eth_balance" "$BASE_NATIVE_DECIMALS")
    echo "   Requester ($BASE_REQUESTER_ADDR)"
    if [ -n "$BASE_USDC_ADDR" ]; then
        usdc_balance=$(get_base_token_balance "$BASE_REQUESTER_ADDR" "$BASE_USDC_ADDR")
        usdc_formatted=$(format_balance "$usdc_balance" "$BASE_USDC_DECIMALS" "USDC")
        echo "             $eth_formatted, $usdc_formatted"
    else
        echo "             $eth_formatted (USDC n/a)"
    fi
fi

if [ -z "$BASE_SOLVER_ADDR" ]; then
    echo "️  BASE_SOLVER_ADDR not set in .env.testnet"
else
    eth_balance=$(get_base_eth_balance "$BASE_SOLVER_ADDR")
    eth_formatted=$(format_balance "$eth_balance" "$BASE_NATIVE_DECIMALS")
    echo "   Solver    ($BASE_SOLVER_ADDR)"
    if [ -n "$BASE_USDC_ADDR" ]; then
        usdc_balance=$(get_base_token_balance "$BASE_SOLVER_ADDR" "$BASE_USDC_ADDR")
        usdc_formatted=$(format_balance "$usdc_balance" "$BASE_USDC_DECIMALS" "USDC")
        echo "             $eth_formatted, $usdc_formatted"
    else
        echo "             $eth_formatted (USDC n/a)"
    fi
fi

echo ""

# Check Ethereum Sepolia balances (using same addresses as Base - EVM addresses work across chains)
echo " Ethereum Sepolia"
echo "-------------------"
echo "   RPC: $SEPOLIA_RPC_URL"
echo "   (Using same addresses as Base Sepolia)"

if [ -z "$BASE_DEPLOYER_ADDR" ]; then
    echo "️  BASE_DEPLOYER_ADDR not set in .env.testnet"
else
    eth_balance=$(get_evm_eth_balance "$BASE_DEPLOYER_ADDR" "$SEPOLIA_RPC_URL")
    eth_formatted=$(format_balance "$eth_balance" "$SEPOLIA_NATIVE_DECIMALS")
    echo "   Deployer  ($BASE_DEPLOYER_ADDR)"
    if [ -n "$SEPOLIA_USDC_ADDR" ]; then
        usdc_balance=$(get_evm_token_balance "$BASE_DEPLOYER_ADDR" "$SEPOLIA_USDC_ADDR" "$SEPOLIA_RPC_URL")
        usdc_formatted=$(format_balance "$usdc_balance" "$SEPOLIA_USDC_DECIMALS" "USDC")
        echo "             $eth_formatted, $usdc_formatted"
    else
        echo "             $eth_formatted (USDC n/a)"
    fi
fi

if [ -z "$BASE_REQUESTER_ADDR" ]; then
    echo "️  BASE_REQUESTER_ADDR not set in .env.testnet"
else
    eth_balance=$(get_evm_eth_balance "$BASE_REQUESTER_ADDR" "$SEPOLIA_RPC_URL")
    eth_formatted=$(format_balance "$eth_balance" "$SEPOLIA_NATIVE_DECIMALS")
    echo "   Requester ($BASE_REQUESTER_ADDR)"
    if [ -n "$SEPOLIA_USDC_ADDR" ]; then
        usdc_balance=$(get_evm_token_balance "$BASE_REQUESTER_ADDR" "$SEPOLIA_USDC_ADDR" "$SEPOLIA_RPC_URL")
        usdc_formatted=$(format_balance "$usdc_balance" "$SEPOLIA_USDC_DECIMALS" "USDC")
        echo "             $eth_formatted, $usdc_formatted"
    else
        echo "             $eth_formatted (USDC n/a)"
    fi
fi

if [ -z "$BASE_SOLVER_ADDR" ]; then
    echo "️  BASE_SOLVER_ADDR not set in .env.testnet"
else
    eth_balance=$(get_evm_eth_balance "$BASE_SOLVER_ADDR" "$SEPOLIA_RPC_URL")
    eth_formatted=$(format_balance "$eth_balance" "$SEPOLIA_NATIVE_DECIMALS")
    echo "   Solver    ($BASE_SOLVER_ADDR)"
    if [ -n "$SEPOLIA_USDC_ADDR" ]; then
        usdc_balance=$(get_evm_token_balance "$BASE_SOLVER_ADDR" "$SEPOLIA_USDC_ADDR" "$SEPOLIA_RPC_URL")
        usdc_formatted=$(format_balance "$usdc_balance" "$SEPOLIA_USDC_DECIMALS" "USDC")
        echo "             $eth_formatted, $usdc_formatted"
    else
        echo "             $eth_formatted (USDC n/a)"
    fi
fi

echo ""
if [ -z "$MOVEMENT_USDC_E_ADDR" ] || [ "$MOVEMENT_USDC_E_ADDR" = "" ]; then
    echo " Note: Movement USDC.e address not configured in testnet-assets.toml"
    echo "   Add usdc_e deployment address to check Movement USDC.e balances"
fi
echo "   Config file: $ASSETS_CONFIG_FILE"
echo ""
echo "✅ Balance check complete!"

