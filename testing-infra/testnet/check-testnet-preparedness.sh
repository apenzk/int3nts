#!/bin/bash

# Check Testnet Preparedness Script
# Checks balances and deployed contracts for testnet readiness
#
# Checks:
#   1. Account balances (native + tokens)
#   2. Deployed contracts (Movement Intent Module, Base Escrow)
#
# Supports:
#   - Movement Bardock Testnet (MOVE, USDC.e, USDC, USDT, WETH)
#   - Base Sepolia (ETH, USDC)
#   - Ethereum Sepolia (ETH, USDC)
#   - Solana Devnet (SOL, USDC)
# 
# Assets Config: testing-infra/testnet/config/testnet-assets.toml
# Service Configs: coordinator/config/coordinator_testnet.toml, integrated-gmp/config/integrated-gmp_testnet.toml, solver/config/solver_testnet.toml (gitignored)
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

# Extract Movement token addresses and decimals
# USDC.e
MOVEMENT_USDC_E_ADDR=$(grep -A 30 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^usdc_e = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
MOVEMENT_USDC_E_DECIMALS=$(grep -A 30 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^usdc_e_decimals = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "6")

# USDC
MOVEMENT_USDC_ADDR=$(grep -A 30 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^usdc = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
MOVEMENT_USDC_DECIMALS=$(grep -A 30 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^usdc_decimals = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "6")

# USDT
MOVEMENT_USDT_ADDR=$(grep -A 30 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^usdt = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
MOVEMENT_USDT_DECIMALS=$(grep -A 30 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^usdt_decimals = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "6")

# WETH
MOVEMENT_WETH_ADDR=$(grep -A 30 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^weth = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")
MOVEMENT_WETH_DECIMALS=$(grep -A 30 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^weth_decimals = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "8")

# WBTC skipped - no paired FA metadata yet

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

# Extract chain IDs for GMP trusted remote checks
MOVEMENT_CHAIN_ID=$(grep -A 5 "^\[movement_bardock_testnet\]" "$ASSETS_CONFIG_FILE" | grep "^chain_id = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "")
BASE_CHAIN_ID=$(grep -A 5 "^\[base_sepolia\]" "$ASSETS_CONFIG_FILE" | grep "^chain_id = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "")
SVM_CHAIN_ID="${SVM_CHAIN_ID:-4}"  # Solana devnet chain ID (not in testnet-assets.toml)

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

# Function to get Movement FA token balance (generic for any Fungible Asset)
get_movement_fa_balance() {
    local address="$1"
    local token_addr="$2"

    # Ensure address has 0x prefix
    if [[ ! "$address" =~ ^0x ]]; then
        address="0x${address}"
    fi

    # If token address is not configured or empty, return 0
    if [ -z "$token_addr" ] || [ "$token_addr" = "" ]; then
        echo "0"
        return
    fi

    # Query balance via view function API (Fungible Asset)
    local balance=$(curl -s --max-time 10 -X POST "${MOVEMENT_RPC_URL}/view" \
        -H "Content-Type: application/json" \
        -d "{\"function\":\"0x1::primary_fungible_store::balance\",\"type_arguments\":[\"0x1::fungible_asset::Metadata\"],\"arguments\":[\"$address\",\"${token_addr}\"]}" \
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

# Helper to display all Movement token balances for an address
display_movement_balances() {
    local addr="$1"
    local move_bal=$(get_movement_balance "$addr")
    local move_fmt=$(format_balance "$move_bal" "$MOVEMENT_NATIVE_DECIMALS")

    # Build token balance string
    local tokens=""
    if [ -n "$MOVEMENT_USDC_E_ADDR" ]; then
        local bal=$(get_movement_fa_balance "$addr" "$MOVEMENT_USDC_E_ADDR")
        local fmt=$(format_balance "$bal" "$MOVEMENT_USDC_E_DECIMALS" "USDC.e")
        tokens="$tokens $fmt,"
    fi
    if [ -n "$MOVEMENT_USDC_ADDR" ]; then
        local bal=$(get_movement_fa_balance "$addr" "$MOVEMENT_USDC_ADDR")
        local fmt=$(format_balance "$bal" "$MOVEMENT_USDC_DECIMALS" "USDC")
        tokens="$tokens $fmt,"
    fi
    if [ -n "$MOVEMENT_USDT_ADDR" ]; then
        local bal=$(get_movement_fa_balance "$addr" "$MOVEMENT_USDT_ADDR")
        local fmt=$(format_balance "$bal" "$MOVEMENT_USDT_DECIMALS" "USDT")
        tokens="$tokens $fmt,"
    fi
    if [ -n "$MOVEMENT_WETH_ADDR" ]; then
        local bal=$(get_movement_fa_balance "$addr" "$MOVEMENT_WETH_ADDR")
        local fmt=$(format_balance "$bal" "$MOVEMENT_WETH_DECIMALS" "WETH")
        tokens="$tokens $fmt,"
    fi
    # Remove trailing comma
    tokens="${tokens%,}"
    echo "             $move_fmt,$tokens"
}

if [ -z "$MOVEMENT_DEPLOYER_ADDR" ]; then
    echo "   ❌ MOVEMENT_DEPLOYER_ADDR not set in .env.testnet"
else
    echo "   Deployer  ($MOVEMENT_DEPLOYER_ADDR)"
    display_movement_balances "$MOVEMENT_DEPLOYER_ADDR"
fi

if [ -z "$MOVEMENT_REQUESTER_ADDR" ]; then
    echo "   ❌ MOVEMENT_REQUESTER_ADDR not set in .env.testnet"
else
    echo "   Requester ($MOVEMENT_REQUESTER_ADDR)"
    display_movement_balances "$MOVEMENT_REQUESTER_ADDR"
fi

if [ -z "$MOVEMENT_SOLVER_ADDR" ]; then
    echo "   ❌ MOVEMENT_SOLVER_ADDR not set in .env.testnet"
else
    echo "   Solver    ($MOVEMENT_SOLVER_ADDR)"
    display_movement_balances "$MOVEMENT_SOLVER_ADDR"
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
echo "   Legend:"
echo "      Deployed:             Contract bytecode exists on-chain"
echo "      On-chain Configured:  Contract state is set up (e.g., approver set)"
echo "      Locally Configured:   Address is in local config file (TOML/.env)"

# Check Movement Intent Module
check_movement_module() {
    local module_addr="$1"

    # Ensure address has 0x prefix
    if [[ ! "$module_addr" =~ ^0x ]]; then
        module_addr="0x${module_addr}"
    fi

    # Query account modules to check if intent module exists
    local response=$(curl -s --max-time 10 "${MOVEMENT_RPC_URL}/accounts/${module_addr}/modules" 2>/dev/null)

    if echo "$response" | jq -e '.[].abi.name' 2>/dev/null | grep -q "fa_intent"; then
        echo "✅"
    else
        echo "❌"
    fi
}

# Check Movement GMP Module (bundled with Intent Module at same address)
check_movement_gmp_module() {
    local module_addr="$1"

    # Ensure address has 0x prefix
    if [[ ! "$module_addr" =~ ^0x ]]; then
        module_addr="0x${module_addr}"
    fi

    # Query account modules to check if intent_gmp module exists
    local response=$(curl -s --max-time 10 "${MOVEMENT_RPC_URL}/accounts/${module_addr}/modules" 2>/dev/null)

    if echo "$response" | jq -e '.[].abi.name' 2>/dev/null | grep -q "intent_gmp"; then
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

# Check Solana program exists on-chain
check_solana_program() {
    local program_id="$1"
    local rpc_url="$2"

    # Query program account info
    local response=$(curl -s --max-time 10 -X POST "$rpc_url" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"getAccountInfo\",\"params\":[\"$program_id\",{\"encoding\":\"base64\"}],\"id\":1}" \
        2>/dev/null)

    # Check if account exists and is executable (program)
    local executable=$(echo "$response" | jq -r '.result.value.executable // false' 2>/dev/null)

    if [ "$executable" = "true" ]; then
        echo "✅"
    else
        echo "❌"
    fi
}

# Check if Movement module is initialized (has resources)
check_movement_initialized() {
    local module_addr="$1"
    local resource_type="$2"  # e.g., "fa_intent::ChainConfig"

    if [[ ! "$module_addr" =~ ^0x ]]; then
        module_addr="0x${module_addr}"
    fi

    local response=$(curl -s --max-time 10 "${MOVEMENT_RPC_URL}/accounts/${module_addr}/resource/${module_addr}::${resource_type}" 2>/dev/null)

    if echo "$response" | jq -e '.data' &>/dev/null; then
        echo "✅"
    else
        echo "❌"
    fi
}

# Check if a GMP trusted remote is set for a given chain ID
# Uses the view function to call get_trusted_remote(chain_id)
check_gmp_trusted_remote() {
    local module_addr="$1"
    local module_name="$2"  # e.g., "intent_gmp" or "intent_gmp_hub"
    local chain_id="$3"

    if [[ ! "$module_addr" =~ ^0x ]]; then
        module_addr="0x${module_addr}"
    fi

    local response=$(curl -s --max-time 10 -X POST "${MOVEMENT_RPC_URL}/view" \
        -H "Content-Type: application/json" \
        -d "{\"function\":\"${module_addr}::${module_name}::get_trusted_remote\",\"type_arguments\":[],\"arguments\":[$chain_id]}" \
        2>/dev/null)

    # If successful, response contains a non-empty byte vector
    local result=$(echo "$response" | jq -r '.[0] // ""' 2>/dev/null)

    if [ -n "$result" ] && [ "$result" != "" ] && [ "$result" != "null" ] && [ "$result" != "0x" ]; then
        echo "✅"
    else
        echo "❌"
    fi
}

# Generic: Check if an EVM view function (no args) returns a non-zero result
# Usage: check_evm_nonzero_result <contract_addr> <rpc_url> <4-byte selector>
check_evm_nonzero_result() {
    local contract_addr="$1"
    local rpc_url="$2"
    local selector="$3"

    if [[ ! "$contract_addr" =~ ^0x ]]; then
        contract_addr="0x${contract_addr}"
    fi

    local result=$(curl -s --max-time 10 -X POST "$rpc_url" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_call\",\"params\":[{\"to\":\"$contract_addr\",\"data\":\"$selector\"},\"latest\"],\"id\":1}" \
        | jq -r '.result // "0x"' 2>/dev/null)

    if [ -n "$result" ] && [ "$result" != "0x" ] && [ "$result" != "0x0000000000000000000000000000000000000000000000000000000000000000" ]; then
        echo "✅"
    else
        echo "❌"
    fi
}

# Check if EVM GMP endpoint has a trusted remote for a given chain ID
# Calls hasTrustedRemote(uint32) → selector 0x48cc1a3a
check_evm_has_trusted_remote() {
    local contract_addr="$1"
    local rpc_url="$2"
    local chain_id="$3"

    if [[ ! "$contract_addr" =~ ^0x ]]; then
        contract_addr="0x${contract_addr}"
    fi

    local chain_id_hex=$(printf "%064x" "$chain_id")
    local data="0x48cc1a3a${chain_id_hex}"

    local result=$(curl -s --max-time 10 -X POST "$rpc_url" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_call\",\"params\":[{\"to\":\"$contract_addr\",\"data\":\"$data\"},\"latest\"],\"id\":1}" \
        | jq -r '.result // "0x"' 2>/dev/null)

    if [ -n "$result" ] && [ "$result" != "0x" ] && [ "$result" != "0x0000000000000000000000000000000000000000000000000000000000000000" ]; then
        echo "✅"
    else
        echo "❌"
    fi
}

# Check if Solana program has accounts matching discriminator + size
# Uses memcmp at offset 0 with base64-encoded discriminator bytes
check_solana_has_account() {
    local program_id="$1"
    local rpc_url="$2"
    local disc_base64="$3"  # base64-encoded discriminator bytes
    local data_size="$4"    # expected account size

    local response=$(curl -s --max-time 10 -X POST "$rpc_url" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"getProgramAccounts\",\"params\":[\"$program_id\",{\"encoding\":\"base64\",\"dataSlice\":{\"offset\":0,\"length\":0},\"filters\":[{\"dataSize\":$data_size},{\"memcmp\":{\"offset\":0,\"bytes\":\"$disc_base64\",\"encoding\":\"base64\"}}]}],\"id\":1}" \
        2>/dev/null)

    local count=$(echo "$response" | jq -r '.result | length // 0' 2>/dev/null)

    if [ "$count" -gt 0 ] 2>/dev/null; then
        echo "✅"
    else
        echo "❌"
    fi
}

# Read config files
COORDINATOR_CONFIG="$PROJECT_ROOT/coordinator/config/coordinator_testnet.toml"
INTEGRATED_GMP_CONFIG="$PROJECT_ROOT/integrated-gmp/config/integrated-gmp_testnet.toml"

# Extract all config values first
if [ -f "$COORDINATOR_CONFIG" ]; then
    MOVEMENT_INTENT_MODULE_ADDR=$(grep -A5 "\[hub_chain\]" "$COORDINATOR_CONFIG" | grep "intent_module_addr" | sed 's/.*= *"\(.*\)".*/\1/' | tr -d '"' || echo "")
    BASE_ESCROW_CONTRACT_ADDR=$(grep -A5 "\[connected_chain_evm\]" "$COORDINATOR_CONFIG" | grep "escrow_contract_addr" | sed 's/.*= *"\(.*\)".*/\1/' | tr -d '"' || echo "")
fi

if [ -f "$INTEGRATED_GMP_CONFIG" ]; then
    BASE_GMP_ENDPOINT_ADDR=$(grep -A10 "\[connected_chain_evm\]" "$INTEGRATED_GMP_CONFIG" | grep "gmp_endpoint_addr" | sed 's/.*= *"\(.*\)".*/\1/' | tr -d '"' || echo "")
    SOLANA_GMP_PROGRAM_ID=$(grep -A10 "\[connected_chain_svm\]" "$INTEGRATED_GMP_CONFIG" | grep "gmp_endpoint_program_id" | sed 's/.*= *"\(.*\)".*/\1/' | tr -d '"' || echo "")
fi

# Track overall pass/fail - set to false on any ❌
all_ok=true
mark() { [ "$1" != "✅" ] && all_ok=false; }

# -----------------------------------------------------------------------------
# Movement Bardock (Hub)
# -----------------------------------------------------------------------------
echo ""
echo "   Movement Bardock (Hub)"
echo "   ----------------------"

# Intent Module (fa_intent)
echo "   Intent Module (fa_intent):"
if [ -z "$MOVEMENT_INTENT_MODULE_ADDR" ] || [ "$MOVEMENT_INTENT_MODULE_ADDR" = "" ]; then
    all_ok=false
    echo "      Deployed:             ❌ (not locally configured)"
    echo "      On-chain Configured:  ❌"
    echo "      Locally Configured:   ❌ (not set in coordinator_testnet.toml)"
else
    deployed_status=$(check_movement_module "$MOVEMENT_INTENT_MODULE_ADDR"); mark "$deployed_status"
    init_status=$(check_movement_initialized "$MOVEMENT_INTENT_MODULE_ADDR" "fa_intent::ChainInfo"); mark "$init_status"
    echo "      Deployed:             $deployed_status"
    echo "      On-chain Configured:"
    echo "         ChainInfo:         $init_status"
    echo "      Locally Configured:   ✅ $MOVEMENT_INTENT_MODULE_ADDR"
fi

# GMP Module (intent_gmp) - bundled at same address as Intent Module
echo "   GMP Module (intent_gmp):"
if [ -z "$MOVEMENT_INTENT_MODULE_ADDR" ] || [ "$MOVEMENT_INTENT_MODULE_ADDR" = "" ]; then
    all_ok=false
    echo "      Deployed:             ❌ (not locally configured)"
    echo "      On-chain Configured:  ❌"
    echo "      Locally Configured:   ❌ (same address as Intent Module)"
else
    deployed_status=$(check_movement_gmp_module "$MOVEMENT_INTENT_MODULE_ADDR"); mark "$deployed_status"
    init_status=$(check_movement_initialized "$MOVEMENT_INTENT_MODULE_ADDR" "intent_gmp::EndpointConfig"); mark "$init_status"
    tr_base=$(check_gmp_trusted_remote "$MOVEMENT_INTENT_MODULE_ADDR" "intent_gmp" "$BASE_CHAIN_ID"); mark "$tr_base"
    tr_svm=$(check_gmp_trusted_remote "$MOVEMENT_INTENT_MODULE_ADDR" "intent_gmp" "$SVM_CHAIN_ID"); mark "$tr_svm"
    echo "      Deployed:             $deployed_status"
    echo "      On-chain Configured:"
    echo "         EndpointConfig:    $init_status"
    echo "         Trusted Remote (Base Sepolia $BASE_CHAIN_ID): $tr_base"
    echo "         Trusted Remote (Solana Devnet $SVM_CHAIN_ID): $tr_svm"
    echo "      Locally Configured:   ✅ (bundled at $MOVEMENT_INTENT_MODULE_ADDR)"
fi

# GMP Hub (intent_gmp_hub) - bundled at same address
echo "   GMP Hub (intent_gmp_hub):"
if [ -z "$MOVEMENT_INTENT_MODULE_ADDR" ] || [ "$MOVEMENT_INTENT_MODULE_ADDR" = "" ]; then
    all_ok=false
    echo "      On-chain Configured:  ❌"
else
    hub_init=$(check_movement_initialized "$MOVEMENT_INTENT_MODULE_ADDR" "intent_gmp_hub::GmpHubConfig"); mark "$hub_init"
    hub_tr_base=$(check_gmp_trusted_remote "$MOVEMENT_INTENT_MODULE_ADDR" "intent_gmp_hub" "$BASE_CHAIN_ID"); mark "$hub_tr_base"
    hub_tr_svm=$(check_gmp_trusted_remote "$MOVEMENT_INTENT_MODULE_ADDR" "intent_gmp_hub" "$SVM_CHAIN_ID"); mark "$hub_tr_svm"
    echo "      On-chain Configured:"
    echo "         GmpHubConfig:      $hub_init"
    echo "         Trusted Remote (Base Sepolia $BASE_CHAIN_ID): $hub_tr_base"
    echo "         Trusted Remote (Solana Devnet $SVM_CHAIN_ID): $hub_tr_svm"
fi

# GMP Sender (gmp_sender)
echo "   GMP Sender (gmp_sender):"
if [ -z "$MOVEMENT_INTENT_MODULE_ADDR" ] || [ "$MOVEMENT_INTENT_MODULE_ADDR" = "" ]; then
    all_ok=false
    echo "      On-chain Configured:  ❌"
else
    sender_init=$(check_movement_initialized "$MOVEMENT_INTENT_MODULE_ADDR" "gmp_sender::SenderConfig"); mark "$sender_init"
    echo "      On-chain Configured:"
    echo "         SenderConfig:      $sender_init"
fi

# -----------------------------------------------------------------------------
# Base Sepolia (EVM)
# -----------------------------------------------------------------------------
echo ""
echo "   Base Sepolia (EVM)"
echo "   ------------------"

# Escrow Contract (IntentInflowEscrow)
echo "   Escrow Contract (IntentInflowEscrow):"
if [ -z "$BASE_ESCROW_CONTRACT_ADDR" ] || [ "$BASE_ESCROW_CONTRACT_ADDR" = "" ]; then
    all_ok=false
    echo "      Deployed:             ❌ (not locally configured)"
    echo "      On-chain Configured:  ❌"
    echo "      Locally Configured:   ❌ (not set in coordinator_testnet.toml)"
else
    deployed_status=$(check_evm_contract "$BASE_ESCROW_CONTRACT_ADDR" "$BASE_RPC_URL"); mark "$deployed_status"
    gmp_ep=$(check_evm_nonzero_result "$BASE_ESCROW_CONTRACT_ADDR" "$BASE_RPC_URL" "0xb2ed7d86"); mark "$gmp_ep"
    hub_cid=$(check_evm_nonzero_result "$BASE_ESCROW_CONTRACT_ADDR" "$BASE_RPC_URL" "0x929f5840"); mark "$hub_cid"
    hub_addr=$(check_evm_nonzero_result "$BASE_ESCROW_CONTRACT_ADDR" "$BASE_RPC_URL" "0x46112810"); mark "$hub_addr"
    echo "      Deployed:             $deployed_status"
    echo "      On-chain Configured:"
    echo "         gmpEndpoint:       $gmp_ep"
    echo "         hubChainId:        $hub_cid"
    echo "         trustedHubAddr:    $hub_addr"
    echo "      Locally Configured:   ✅ $BASE_ESCROW_CONTRACT_ADDR"
fi

# GMP Endpoint (IntentGmp)
echo "   GMP Endpoint (IntentGmp):"
if [ -z "$BASE_GMP_ENDPOINT_ADDR" ] || [ "$BASE_GMP_ENDPOINT_ADDR" = "" ]; then
    all_ok=false
    echo "      Deployed:             ❌ (not locally configured)"
    echo "      On-chain Configured:  ❌"
    echo "      Locally Configured:   ❌ (not set in integrated-gmp_testnet.toml)"
else
    deployed_status=$(check_evm_contract "$BASE_GMP_ENDPOINT_ADDR" "$BASE_RPC_URL"); mark "$deployed_status"
    escrow_h=$(check_evm_nonzero_result "$BASE_GMP_ENDPOINT_ADDR" "$BASE_RPC_URL" "0x87ad8f87"); mark "$escrow_h"
    outflow_h=$(check_evm_nonzero_result "$BASE_GMP_ENDPOINT_ADDR" "$BASE_RPC_URL" "0xa80693bc"); mark "$outflow_h"
    tr_hub=$(check_evm_has_trusted_remote "$BASE_GMP_ENDPOINT_ADDR" "$BASE_RPC_URL" "$MOVEMENT_CHAIN_ID"); mark "$tr_hub"
    echo "      Deployed:             $deployed_status"
    echo "      On-chain Configured:"
    echo "         escrowHandler:     $escrow_h"
    echo "         outflowHandler:    $outflow_h"
    echo "         Trusted Remote (Movement $MOVEMENT_CHAIN_ID): $tr_hub"
    echo "      Locally Configured:   ✅ $BASE_GMP_ENDPOINT_ADDR"
fi

# -----------------------------------------------------------------------------
# Solana Devnet (SVM)
# -----------------------------------------------------------------------------
echo ""
echo "   Solana Devnet (SVM)"
echo "   -------------------"

# Escrow Program
echo "   Escrow Program:"
if [ -z "$SOLANA_PROGRAM_ID" ] || [ "$SOLANA_PROGRAM_ID" = "" ]; then
    all_ok=false
    echo "      Deployed:             ❌ (not locally configured)"
    echo "      On-chain Configured:  ❌"
    echo "      Locally Configured:   ❌ (not set in .env.testnet)"
else
    deployed_status=$(check_solana_program "$SOLANA_PROGRAM_ID" "$SOLANA_RPC_URL"); mark "$deployed_status"
    # EscrowState: disc="ESCROWST" base64=RVNDUk9XU1Q=, size=40
    state_pda=$(check_solana_has_account "$SOLANA_PROGRAM_ID" "$SOLANA_RPC_URL" "RVNDUk9XU1Q=" 40); mark "$state_pda"
    # GmpConfig: disc="GMPCONFG" base64=R01QQ09ORkc=, size=109
    gmp_cfg=$(check_solana_has_account "$SOLANA_PROGRAM_ID" "$SOLANA_RPC_URL" "R01QQ09ORkc=" 109); mark "$gmp_cfg"
    echo "      Deployed:             $deployed_status"
    echo "      On-chain Configured:"
    echo "         State PDA:         $state_pda"
    echo "         GMP Config PDA:    $gmp_cfg"
    echo "      Locally Configured:   ✅ $SOLANA_PROGRAM_ID"
fi

# GMP Endpoint (intent-gmp program)
echo "   GMP Endpoint (intent-gmp):"
if [ -z "$SOLANA_GMP_PROGRAM_ID" ] || [ "$SOLANA_GMP_PROGRAM_ID" = "" ]; then
    all_ok=false
    echo "      Deployed:             ❌ (not locally configured)"
    echo "      On-chain Configured:  ❌"
    echo "      Locally Configured:   ❌ (not set in integrated-gmp_testnet.toml)"
else
    deployed_status=$(check_solana_program "$SOLANA_GMP_PROGRAM_ID" "$SOLANA_RPC_URL"); mark "$deployed_status"
    # ConfigAccount: disc=1 base64=AQ==, size=38
    config_pda=$(check_solana_has_account "$SOLANA_GMP_PROGRAM_ID" "$SOLANA_RPC_URL" "AQ==" 38); mark "$config_pda"
    # TrustedRemoteAccount: disc=3 base64=Aw==, size=38
    trusted_remote=$(check_solana_has_account "$SOLANA_GMP_PROGRAM_ID" "$SOLANA_RPC_URL" "Aw==" 38); mark "$trusted_remote"
    # RoutingConfig: disc=6 base64=Bg==, size=66
    routing_cfg=$(check_solana_has_account "$SOLANA_GMP_PROGRAM_ID" "$SOLANA_RPC_URL" "Bg==" 66); mark "$routing_cfg"
    echo "      Deployed:             $deployed_status"
    echo "      On-chain Configured:"
    echo "         Config PDA:        $config_pda"
    echo "         Trusted Remote (Movement $MOVEMENT_CHAIN_ID): $trusted_remote"
    echo "         Routing Config:    $routing_cfg"
    echo "      Locally Configured:   ✅ $SOLANA_GMP_PROGRAM_ID"
fi

echo ""

# =============================================================================
# SUMMARY
# =============================================================================

echo " Summary"
echo "----------"
echo ""
echo "   Assets Config: $ASSETS_CONFIG_FILE"
echo "   Service Configs: coordinator_testnet.toml, integrated-gmp_testnet.toml, solver_testnet.toml (gitignored)"
echo "   Keys:   $TESTNET_KEYS_FILE"
echo ""
if [ "$all_ok" = true ]; then
    echo "✅ Preparedness check success."
else
    echo "❌ Preparedness check failure."
    echo "   Fix the failing checks above before testnet runs."
fi

