#!/bin/bash

# Check Testnet Preparedness Script
# Checks balances and deployed contracts for testnet readiness
# 
# Checks:
#   1. Account balances (MOVE, ETH, USDC/USDC.e)
#   2. Deployed contracts (Movement Intent Module, Base Escrow)
#
# Supports:
#   - Movement Bardock Testnet (MOVE, USDC.e)
#   - Base Sepolia (ETH, USDC)
#   - Ethereum Sepolia (ETH, USDC)
# 
# Assets Config: testing-infra/testnet/config/testnet-assets.toml
# Service Configs: verifier/config/verifier_testnet.toml, solver/config/solver_testnet.toml (gitignored)
# Keys: .env.testnet

# Get the script directory and project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"
export PROJECT_ROOT

# Source utilities (for error handling only, not logging)
source "$PROJECT_ROOT/testing-infra/ci-e2e/util.sh" 2>/dev/null || true

echo " Checking Testnet Preparedness"
echo "================================="
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
    echo "❌ Base Sepolia USDC address not found in testnet-assets.toml"
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
    echo "❌ Ethereum Sepolia USDC address not found in testnet-assets.toml"
    echo "   Ethereum Sepolia USDC balance checks will be skipped"
elif [ -z "$SEPOLIA_USDC_DECIMALS" ]; then
    echo "❌ ERROR: Ethereum Sepolia USDC decimals not found in testnet-assets.toml"
    echo "   Add usdc_decimals = 6 to [ethereum_sepolia] section"
    exit 1
fi

# Extract Movement USDC address and decimals
MOVEMENT_USDC_ADDR=$(grep -A 20 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^usdc = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
MOVEMENT_USDC_DECIMALS=$(grep -A 20 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^usdc_decimals = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "")
if [ -n "$MOVEMENT_USDC_ADDR" ] && [ -z "$MOVEMENT_USDC_DECIMALS" ]; then
    echo "❌ ERROR: Movement USDC.e address configured but decimals not found in testnet-assets.toml"
    echo "   Add usdc_decimals = 6 to [movement_bardock_testnet] section"
    exit 1
fi

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
    echo "❌ Movement RPC URL not found in testnet-assets.toml"
    echo "   Movement balance checks will fail"
fi

BASE_RPC_URL=$(grep -A 5 "^\[base_sepolia\]" "$ASSETS_CONFIG_FILE" | grep "^rpc_url = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
if [ -z "$BASE_RPC_URL" ]; then
    echo "❌ Base Sepolia RPC URL not found in testnet-assets.toml"
    echo "   Base Sepolia balance checks will fail"
fi

# Substitute API key in Base Sepolia RPC URL if placeholder is present
if [[ "$BASE_RPC_URL" == *"ALCHEMY_API_KEY"* ]]; then
    if [ -n "$ALCHEMY_BASE_SEPOLIA_API_KEY" ]; then
        BASE_RPC_URL="${BASE_RPC_URL/ALCHEMY_API_KEY/$ALCHEMY_BASE_SEPOLIA_API_KEY}"
    else
        echo "❌ ALCHEMY_BASE_SEPOLIA_API_KEY not set in .env.testnet"
        echo "   Base Sepolia balance checks will fail"
    fi
fi

SEPOLIA_RPC_URL=$(grep -A 5 "^\[ethereum_sepolia\]" "$ASSETS_CONFIG_FILE" | grep "^rpc_url = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
if [ -z "$SEPOLIA_RPC_URL" ]; then
    echo "❌ Ethereum Sepolia RPC URL not found in testnet-assets.toml"
    echo "   Ethereum Sepolia balance checks will fail"
fi

# Substitute API key in Sepolia RPC URL if placeholder is present
if [[ "$SEPOLIA_RPC_URL" == *"ALCHEMY_API_KEY"* ]]; then
    if [ -n "$ALCHEMY_ETH_SEPOLIA_API_KEY" ]; then
        SEPOLIA_RPC_URL="${SEPOLIA_RPC_URL/ALCHEMY_API_KEY/$ALCHEMY_ETH_SEPOLIA_API_KEY}"
    else
        echo "❌ ALCHEMY_ETH_SEPOLIA_API_KEY not set in .env.testnet"
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

# Function to get Movement USDC balance (Fungible Asset)
get_movement_usdc_balance() {
    local address="$1"
    # Ensure address has 0x prefix
    if [[ ! "$address" =~ ^0x ]]; then
        address="0x${address}"
    fi
    
    # If USDC address is not configured, return 0
    if [ -z "$MOVEMENT_USDC_ADDR" ] || [ "$MOVEMENT_USDC_ADDR" = "" ]; then
        echo "0"
        return
    fi
    
    # Query USDC.e balance via view function API (Fungible Asset)
    # USDC.e is deployed as a Fungible Asset, use primary_fungible_store::balance
    local balance=$(curl -s --max-time 10 -X POST "${MOVEMENT_RPC_URL}/view" \
        -H "Content-Type: application/json" \
        -d "{\"function\":\"0x1::primary_fungible_store::balance\",\"type_arguments\":[\"0x1::fungible_asset::Metadata\"],\"arguments\":[\"$address\",\"${MOVEMENT_USDC_ADDR}\"]}" \
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

# Format balance for display
format_balance() {
    local balance="$1"
    local decimals="$2"
    local symbol="${3:-}"
    
    # Convert from smallest unit to human-readable
    # Decimals must be provided (read from testnet-assets.toml config)
    local divisor
    case "$decimals" in
        18) divisor="1000000000000000000" ;;
        9)  divisor="1000000000" ;;
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
            9)  printf "%.6f SOL" "$formatted" ;;
            8)  printf "%.6f MOVE" "$formatted" ;;
            6)  printf "%.6f USDC" "$formatted" ;;
            *)  printf "%s" "$balance" ;;
        esac
    fi
}

# Function to get Solana SOL balance (lamports)
get_solana_balance() {
    local address="$1"
    local rpc_url="$2"

    local balance=$(curl -s --max-time 10 -X POST "$rpc_url" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"getBalance\",\"params\":[\"$address\"],\"id\":1}" \
        | jq -r '.result.value // "0"' 2>/dev/null)

    if [ -z "$balance" ] || [ "$balance" = "null" ]; then
        echo "0"
    else
        echo "$balance"
    fi
}

# Function to get Solana SPL token balance (raw amount)
get_solana_token_balance() {
    local owner="$1"
    local mint="$2"
    local rpc_url="$3"

    if [ -z "$mint" ]; then
        echo "0"
        return
    fi

    local total=$(curl -s --max-time 10 -X POST "$rpc_url" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"getTokenAccountsByOwner\",\"params\":[\"$owner\",{\"mint\":\"$mint\"},{\"encoding\":\"jsonParsed\"}],\"id\":1}" \
        | jq -r '[.result.value[].account.data.parsed.info.tokenAmount.amount] | map(tonumber) | add // 0' 2>/dev/null)

    if [ -z "$total" ] || [ "$total" = "null" ]; then
        echo "0"
    else
        echo "$total"
    fi
}

# Check Movement balances
movement_ready="❌"
if [ -n "$MOVEMENT_DEPLOYER_ADDR" ] && [ -n "$MOVEMENT_REQUESTER_ADDR" ] && [ -n "$MOVEMENT_SOLVER_ADDR" ]; then
    movement_ready="✅"
fi
echo " $movement_ready Movement Bardock Testnet"
echo "----------------------------"
echo "   RPC: $MOVEMENT_RPC_URL"

if [ -z "$MOVEMENT_DEPLOYER_ADDR" ]; then
    echo "   ❌ MOVEMENT_DEPLOYER_ADDR not set in .env.testnet"
else
    balance=$(get_movement_balance "$MOVEMENT_DEPLOYER_ADDR")
    formatted=$(format_balance "$balance" "$MOVEMENT_NATIVE_DECIMALS")
    usdc_balance=$(get_movement_usdc_balance "$MOVEMENT_DEPLOYER_ADDR")
    echo "   Deployer  ($MOVEMENT_DEPLOYER_ADDR)"
    if [ -n "$MOVEMENT_USDC_ADDR" ]; then
        usdc_formatted=$(format_balance "$usdc_balance" "$MOVEMENT_USDC_DECIMALS" "USDC.e")
        echo "             $formatted, $usdc_formatted"
    else
        echo "             $formatted (USDC.e n/a)"
    fi
fi

if [ -z "$MOVEMENT_REQUESTER_ADDR" ]; then
    echo "   ❌ MOVEMENT_REQUESTER_ADDR not set in .env.testnet"
else
    balance=$(get_movement_balance "$MOVEMENT_REQUESTER_ADDR")
    formatted=$(format_balance "$balance" "$MOVEMENT_NATIVE_DECIMALS")
    usdc_balance=$(get_movement_usdc_balance "$MOVEMENT_REQUESTER_ADDR")
    echo "   Requester ($MOVEMENT_REQUESTER_ADDR)"
    if [ -n "$MOVEMENT_USDC_ADDR" ]; then
        usdc_formatted=$(format_balance "$usdc_balance" "$MOVEMENT_USDC_DECIMALS" "USDC.e")
        echo "             $formatted, $usdc_formatted"
    else
        echo "             $formatted (USDC.e n/a)"
    fi
fi

if [ -z "$MOVEMENT_SOLVER_ADDR" ]; then
    echo "   ❌ MOVEMENT_SOLVER_ADDR not set in .env.testnet"
else
    balance=$(get_movement_balance "$MOVEMENT_SOLVER_ADDR")
    formatted=$(format_balance "$balance" "$MOVEMENT_NATIVE_DECIMALS")
    usdc_balance=$(get_movement_usdc_balance "$MOVEMENT_SOLVER_ADDR")
    echo "   Solver    ($MOVEMENT_SOLVER_ADDR)"
    if [ -n "$MOVEMENT_USDC_ADDR" ]; then
        usdc_formatted=$(format_balance "$usdc_balance" "$MOVEMENT_USDC_DECIMALS" "USDC.e")
        echo "             $formatted, $usdc_formatted"
    else
        echo "             $formatted (USDC.e n/a)"
    fi
fi

echo ""

# Check Solana Devnet balances
solana_ready="❌"
if [ -n "$SOLANA_DEPLOYER_ADDR" ] && [ -n "$SOLANA_REQUESTER_ADDR" ] && [ -n "$SOLANA_SOLVER_ADDR" ]; then
    solana_ready="✅"
fi
echo " $solana_ready Solana Devnet"
echo "----------------"

SOLANA_RPC_URL="${SOLANA_RPC_URL:-https://api.devnet.solana.com}"
echo "   RPC: $SOLANA_RPC_URL"

if [ -z "$SOLANA_DEPLOYER_ADDR" ]; then
    echo "   ❌ SOLANA_DEPLOYER_ADDR not set in .env.testnet"
else
    sol_balance=$(get_solana_balance "$SOLANA_DEPLOYER_ADDR" "$SOLANA_RPC_URL")
    sol_formatted=$(format_balance "$sol_balance" 9 "SOL")
    echo "   Deployer  ($SOLANA_DEPLOYER_ADDR)"
    if [ -n "$SOLANA_USDC_MINT" ]; then
        usdc_balance=$(get_solana_token_balance "$SOLANA_DEPLOYER_ADDR" "$SOLANA_USDC_MINT" "$SOLANA_RPC_URL")
        usdc_formatted=$(format_balance "$usdc_balance" 6 "USDC")
        echo "             $sol_formatted, $usdc_formatted"
    else
        echo "             $sol_formatted (USDC n/a)"
    fi
fi

if [ -z "$SOLANA_REQUESTER_ADDR" ]; then
    echo "   ❌ SOLANA_REQUESTER_ADDR not set in .env.testnet"
else
    sol_balance=$(get_solana_balance "$SOLANA_REQUESTER_ADDR" "$SOLANA_RPC_URL")
    sol_formatted=$(format_balance "$sol_balance" 9 "SOL")
    echo "   Requester ($SOLANA_REQUESTER_ADDR)"
    if [ -n "$SOLANA_USDC_MINT" ]; then
        usdc_balance=$(get_solana_token_balance "$SOLANA_REQUESTER_ADDR" "$SOLANA_USDC_MINT" "$SOLANA_RPC_URL")
        usdc_formatted=$(format_balance "$usdc_balance" 6 "USDC")
        echo "             $sol_formatted, $usdc_formatted"
    else
        echo "             $sol_formatted (USDC n/a)"
    fi
fi

if [ -z "$SOLANA_SOLVER_ADDR" ]; then
    echo "   ❌ SOLANA_SOLVER_ADDR not set in .env.testnet"
else
    sol_balance=$(get_solana_balance "$SOLANA_SOLVER_ADDR" "$SOLANA_RPC_URL")
    sol_formatted=$(format_balance "$sol_balance" 9 "SOL")
    echo "   Solver    ($SOLANA_SOLVER_ADDR)"
    if [ -n "$SOLANA_USDC_MINT" ]; then
        usdc_balance=$(get_solana_token_balance "$SOLANA_SOLVER_ADDR" "$SOLANA_USDC_MINT" "$SOLANA_RPC_URL")
        usdc_formatted=$(format_balance "$usdc_balance" 6 "USDC")
        echo "             $sol_formatted, $usdc_formatted"
    else
        echo "             $sol_formatted (USDC n/a)"
    fi
fi

echo ""

# Check Base Sepolia balances
base_ready="❌"
if [ -n "$BASE_DEPLOYER_ADDR" ] && [ -n "$BASE_REQUESTER_ADDR" ] && [ -n "$BASE_SOLVER_ADDR" ]; then
    base_ready="✅"
fi
echo " $base_ready Base Sepolia"
echo "---------------"
echo "   RPC: $BASE_RPC_URL"

if [ -z "$BASE_DEPLOYER_ADDR" ]; then
    echo "   ❌ BASE_DEPLOYER_ADDR not set in .env.testnet"
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
    echo "   ❌ BASE_REQUESTER_ADDR not set in .env.testnet"
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
    echo "   ❌ BASE_SOLVER_ADDR not set in .env.testnet"
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
sepolia_ready="❌"
if [ -n "$BASE_DEPLOYER_ADDR" ] && [ -n "$BASE_REQUESTER_ADDR" ] && [ -n "$BASE_SOLVER_ADDR" ]; then
    sepolia_ready="✅"
fi
echo " $sepolia_ready Ethereum Sepolia"
echo "-------------------"
echo "   RPC: $SEPOLIA_RPC_URL"
echo "   (Using same addresses as Base Sepolia)"

if [ -z "$BASE_DEPLOYER_ADDR" ]; then
    echo "   ❌ BASE_DEPLOYER_ADDR not set in .env.testnet"
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
    echo "   ❌ BASE_REQUESTER_ADDR not set in .env.testnet"
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
    echo "   ❌ BASE_SOLVER_ADDR not set in .env.testnet"
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

# =============================================================================
# CONTRACT DEPLOYMENT STATUS
# =============================================================================

echo " Deployed Contracts"
echo "---------------------"

# Check Movement Intent Module
check_movement_module() {
    local module_addr="$1"
    
    # Ensure address has 0x prefix
    if [[ ! "$module_addr" =~ ^0x ]]; then
        module_addr="0x${module_addr}"
    fi
    
    # Query account modules to check if intent module exists
    local response=$(curl -s --max-time 10 "${MOVEMENT_RPC_URL}/accounts/${module_addr}/modules" 2>/dev/null)
    
    if echo "$response" | jq -e '.[].abi.name' 2>/dev/null | grep -q "intent"; then
        echo "✅"
    else
        echo "❌"
    fi
}

# Check Base Escrow Contract (EVM)
check_evm_contract() {
    local contract_addr="$1"
    local rpc_url="$2"
    
    # Ensure address has 0x prefix
    if [[ ! "$contract_addr" =~ ^0x ]]; then
        contract_addr="0x${contract_addr}"
    fi
    
    # Query contract code
    local code=$(curl -s --max-time 10 -X POST "$rpc_url" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getCode\",\"params\":[\"$contract_addr\",\"latest\"],\"id\":1}" \
        | jq -r '.result // "0x"' 2>/dev/null)
    
    if [ -n "$code" ] && [ "$code" != "0x" ] && [ "$code" != "null" ]; then
        echo "✅"
    else
        echo "❌"
    fi
}

# Movement Intent Module
# Read from verifier_testnet.toml (gitignored config file)
VERIFIER_CONFIG="$PROJECT_ROOT/verifier/config/verifier_testnet.toml"
if [ -f "$VERIFIER_CONFIG" ]; then
    MOVEMENT_INTENT_MODULE_ADDR=$(grep -A5 "\[hub_chain\]" "$VERIFIER_CONFIG" | grep "intent_module_addr" | sed 's/.*= *"\(.*\)".*/\1/' | tr -d '"' || echo "")
fi

if [ -z "$MOVEMENT_INTENT_MODULE_ADDR" ] || [ "$MOVEMENT_INTENT_MODULE_ADDR" = "" ]; then
    echo "   Movement Intent Module: ❌ Not configured (check verifier_testnet.toml)"
else
    status=$(check_movement_module "$MOVEMENT_INTENT_MODULE_ADDR")
    echo "   Movement Intent Module ($MOVEMENT_INTENT_MODULE_ADDR)"
    echo "             Status: $status Deployed"
fi

# Base Escrow Contract
# Read from verifier_testnet.toml (gitignored config file)
if [ -f "$VERIFIER_CONFIG" ]; then
    BASE_ESCROW_CONTRACT_ADDR=$(grep -A5 "\[connected_chain_evm\]" "$VERIFIER_CONFIG" | grep "escrow_contract_addr" | sed 's/.*= *"\(.*\)".*/\1/' | tr -d '"' || echo "")
fi

if [ -z "$BASE_ESCROW_CONTRACT_ADDR" ] || [ "$BASE_ESCROW_CONTRACT_ADDR" = "" ]; then
    echo "   Base Escrow Contract:   ❌ Not configured (check verifier_testnet.toml)"
else
    status=$(check_evm_contract "$BASE_ESCROW_CONTRACT_ADDR" "$BASE_RPC_URL")
    echo "   Base Escrow Contract ($BASE_ESCROW_CONTRACT_ADDR)"
    echo "             Status: $status Deployed"
fi

# Solana Intent Escrow Program
if [ -z "$SOLANA_PROGRAM_ID" ] || [ "$SOLANA_PROGRAM_ID" = "" ]; then
    echo "   Solana Intent Escrow:   ❌ Not configured (set SOLANA_PROGRAM_ID in .env.testnet)"
else
    echo "   Solana Intent Escrow ($SOLANA_PROGRAM_ID)"
    echo "             Status: ✅ Configured"
fi

echo ""

# =============================================================================
# SUMMARY
# =============================================================================

echo " Summary"
echo "----------"

# Count readiness
ready_count=0
total_count=9

# Check balances
if [ -n "$MOVEMENT_DEPLOYER_ADDR" ]; then
    balance=$(get_movement_balance "$MOVEMENT_DEPLOYER_ADDR")
    if [ "$balance" != "0" ] && [ -n "$balance" ]; then
        ((ready_count++))
    fi
fi

if [ -n "$BASE_DEPLOYER_ADDR" ]; then
    balance=$(get_base_eth_balance "$BASE_DEPLOYER_ADDR")
    if [ "$balance" != "0" ] && [ -n "$balance" ]; then
        ((ready_count++))
    fi
fi

# Check requester/solver have funds
if [ -n "$MOVEMENT_REQUESTER_ADDR" ]; then
    balance=$(get_movement_balance "$MOVEMENT_REQUESTER_ADDR")
    if [ "$balance" != "0" ] && [ -n "$balance" ]; then
        ((ready_count++))
    fi
fi

if [ -n "$BASE_REQUESTER_ADDR" ]; then
    balance=$(get_base_eth_balance "$BASE_REQUESTER_ADDR")
    if [ "$balance" != "0" ] && [ -n "$balance" ]; then
        ((ready_count++))
    fi
fi

# Check contracts deployed
if [ -n "$MOVEMENT_INTENT_MODULE_ADDR" ] && [ "$MOVEMENT_INTENT_MODULE_ADDR" != "" ]; then
    ((ready_count++))
fi

if [ -n "$BASE_ESCROW_CONTRACT_ADDR" ] && [ "$BASE_ESCROW_CONTRACT_ADDR" != "" ]; then
    ((ready_count++))
fi

# Check Solana balances and program
if [ -n "$SOLANA_DEPLOYER_ADDR" ]; then
    balance=$(get_solana_balance "$SOLANA_DEPLOYER_ADDR" "$SOLANA_RPC_URL")
    if [ "$balance" != "0" ] && [ -n "$balance" ]; then
        ((ready_count++))
    fi
fi

if [ -n "$SOLANA_REQUESTER_ADDR" ]; then
    balance=$(get_solana_balance "$SOLANA_REQUESTER_ADDR" "$SOLANA_RPC_URL")
    if [ "$balance" != "0" ] && [ -n "$balance" ]; then
        ((ready_count++))
    fi
fi

if [ -n "$SOLANA_PROGRAM_ID" ]; then
    ((ready_count++))
fi

echo "   Readiness: $ready_count/$total_count checks passed"

if [ -z "$MOVEMENT_USDC_ADDR" ] || [ "$MOVEMENT_USDC_ADDR" = "" ]; then
    echo ""
    echo " Note: Movement USDC.e address not configured in testnet-assets.toml"
fi

echo ""
echo "   Assets Config: $ASSETS_CONFIG_FILE"
echo "   Service Configs: verifier_testnet.toml, solver_testnet.toml (gitignored)"
echo "   Keys:   $TESTNET_KEYS_FILE"
echo ""
if [ "$ready_count" -eq "$total_count" ]; then
    echo "✅ Preparedness check success."
else
    echo "❌ Preparedness check failure ($ready_count/$total_count)."
    echo "   Fix the missing checks above before testnet runs."
fi

