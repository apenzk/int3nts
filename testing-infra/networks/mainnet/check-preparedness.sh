#!/bin/bash

# Check Mainnet Preparedness Script
# Checks balances and deployed contracts for mainnet readiness
#
# Checks:
#   1. Account balances (native tokens)
#   2. Deployed contracts (Movement Intent Module, Base Escrow, HyperEVM Escrow)
#
# Supports:
#   - Movement Mainnet (MOVE)
#   - Base Mainnet (ETH)
#   - HyperEVM Mainnet (HYPE)
#
# Assets Config: testing-infra/networks/mainnet/config/mainnet-assets.toml
# Service Configs: coordinator/config/coordinator_mainnet.toml, integrated-gmp/config/integrated-gmp_mainnet.toml, solver/config/solver_mainnet.toml (gitignored)
# Keys: .env.mainnet

# Get the script directory and project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../.." && pwd )"
export PROJECT_ROOT

# Source utilities (for error handling only, not logging)
source "$PROJECT_ROOT/testing-infra/ci-e2e/util.sh" 2>/dev/null || true

echo " Checking Mainnet Preparedness"
echo "================================="
echo ""

# Load .env.mainnet
MAINNET_KEYS_FILE="$SCRIPT_DIR/.env.mainnet"

if [ ! -f "$MAINNET_KEYS_FILE" ]; then
    echo "ERROR: .env.mainnet not found at $MAINNET_KEYS_FILE"
    echo "   Create it from env.mainnet.example in this directory"
    exit 1
fi

# Source the keys file
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

# Extract chain IDs for remote GMP endpoint checks
MOVEMENT_CHAIN_ID=$(grep -A 5 "^\[movement_mainnet\]" "$ASSETS_CONFIG_FILE" | grep "^chain_id = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "")
BASE_CHAIN_ID=$(grep -A 5 "^\[base_mainnet\]" "$ASSETS_CONFIG_FILE" | grep "^chain_id = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "")
HYPERLIQUID_CHAIN_ID_VAL=$(grep -A 5 "^\[hyperliquid_mainnet\]" "$ASSETS_CONFIG_FILE" | grep "^chain_id = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "")

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

# Check Movement balances
movement_ready="❌"
if [ -n "$MOVEMENT_DEPLOYER_ADDR" ] && [ -n "$MOVEMENT_REQUESTER_ADDR" ] && [ -n "$MOVEMENT_SOLVER_ADDR" ]; then
    movement_ready="✅"
fi
echo " $movement_ready Movement Mainnet (Hub)"
echo "----------------------------"
echo "   RPC: $MOVEMENT_RPC_URL"

for role_var in MOVEMENT_DEPLOYER_ADDR MOVEMENT_REQUESTER_ADDR MOVEMENT_SOLVER_ADDR INTEGRATED_GMP_MVM_ADDR; do
    addr="${!role_var}"
    label="${role_var#MOVEMENT_}"
    label="${label#INTEGRATED_GMP_}"
    label="${label%_ADDR}"
    label=$(echo "$label" | tr '[:upper:]' '[:lower:]' | sed 's/^./\U&/')

    if [ "$role_var" = "INTEGRATED_GMP_MVM_ADDR" ]; then
        label="Relay"
    fi

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

# Check Base Mainnet balances
base_ready="❌"
if [ -n "$BASE_DEPLOYER_ADDR" ] && [ -n "$BASE_REQUESTER_ADDR" ] && [ -n "$BASE_SOLVER_ADDR" ]; then
    base_ready="✅"
fi
echo " $base_ready Base Mainnet"
echo "---------------"
echo "   RPC: $BASE_RPC_URL"

# Relay EVM address: from .env.mainnet or from integrated-gmp config
COORDINATOR_CONFIG="$PROJECT_ROOT/coordinator/config/coordinator_mainnet.toml"
INTEGRATED_GMP_CONFIG="$PROJECT_ROOT/integrated-gmp/config/integrated-gmp_mainnet.toml"

GMP_RELAY_EVM_ADDR="${INTEGRATED_GMP_EVM_PUBKEY_HASH:-}"
if [ -z "$GMP_RELAY_EVM_ADDR" ] && [ -f "$INTEGRATED_GMP_CONFIG" ]; then
    GMP_RELAY_EVM_ADDR=$(grep -A10 "\[connected_chain_evm\]" "$INTEGRATED_GMP_CONFIG" | grep "approver_evm_pubkey_hash" | head -1 | sed 's/.*= *"\(.*\)".*/\1/' | tr -d '"' || echo "")
fi

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

if [ -z "$GMP_RELAY_EVM_ADDR" ]; then
    echo "   INTEGRATED_GMP_EVM_PUBKEY_HASH not set in .env.mainnet"
else
    eth_balance=$(get_evm_eth_balance "$GMP_RELAY_EVM_ADDR" "$BASE_RPC_URL")
    eth_formatted=$(format_balance "$eth_balance" "$BASE_NATIVE_DECIMALS" "ETH")
    echo "   Relay     ($GMP_RELAY_EVM_ADDR)"
    echo "             $eth_formatted"
fi

echo ""

# Check HyperEVM Mainnet balances
hyper_ready="❌"
if [ -n "$HYPERLIQUID_DEPLOYER_ADDR" ] && [ -n "$HYPERLIQUID_REQUESTER_ADDR" ] && [ -n "$HYPERLIQUID_SOLVER_ADDR" ]; then
    hyper_ready="✅"
fi
echo " $hyper_ready HyperEVM Mainnet"
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

if [ -n "$GMP_RELAY_EVM_ADDR" ]; then
    eth_balance=$(get_evm_eth_balance "$GMP_RELAY_EVM_ADDR" "$HYPERLIQUID_RPC_URL")
    eth_formatted=$(format_balance "$eth_balance" "$HYPERLIQUID_NATIVE_DECIMALS" "HYPE")
    echo "   Relay     ($GMP_RELAY_EVM_ADDR)"
    echo "             $eth_formatted"
fi

echo ""

# =============================================================================
# CONTRACT DEPLOYMENT STATUS
# =============================================================================

echo " Deployed Contracts"
echo "---------------------"
echo "   ✅/❌ = on-chain check passed/failed, followed by the configured value"

# Check Movement Intent Module
check_movement_module() {
    local module_addr="$1"

    if [[ ! "$module_addr" =~ ^0x ]]; then
        module_addr="0x${module_addr}"
    fi

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

    if [[ ! "$module_addr" =~ ^0x ]]; then
        module_addr="0x${module_addr}"
    fi

    local response=$(curl -s --max-time 10 "${MOVEMENT_RPC_URL}/accounts/${module_addr}/modules" 2>/dev/null)

    if echo "$response" | jq -e '.[].abi.name' 2>/dev/null | grep -q "intent_gmp"; then
        echo "✅"
    else
        echo "❌"
    fi
}

# Check EVM Contract
check_evm_contract() {
    local contract_addr="$1"
    local rpc_url="$2"

    if [[ ! "$contract_addr" =~ ^0x ]]; then
        contract_addr="0x${contract_addr}"
    fi

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

# Check if Movement module is initialized (has resources)
check_movement_initialized() {
    local module_addr="$1"
    local resource_type="$2"

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

# Check if a remote GMP endpoint is set for a given chain ID
check_gmp_remote_endpoint() {
    local module_addr="$1"
    local module_name="$2"
    local chain_id="$3"

    if [[ ! "$module_addr" =~ ^0x ]]; then
        module_addr="0x${module_addr}"
    fi

    local view_fn="get_remote_gmp_endpoint_addrs"
    if [ "$module_name" = "intent_gmp_hub" ]; then
        view_fn="get_remote_gmp_endpoint_addr"
    fi

    local response=$(curl -s --max-time 10 -X POST "${MOVEMENT_RPC_URL}/view" \
        -H "Content-Type: application/json" \
        -d "{\"function\":\"${module_addr}::${module_name}::${view_fn}\",\"type_arguments\":[],\"arguments\":[$chain_id]}" \
        2>/dev/null)

    local result=$(echo "$response" | jq -r '.[0] | if type == "array" then .[0] else . end // ""' 2>/dev/null)

    if [ -n "$result" ] && [ "$result" != "" ] && [ "$result" != "null" ] && [ "$result" != "0x" ]; then
        echo "✅ $result"
    else
        echo "❌"
    fi
}

# Generic: Check if an EVM view function (no args) returns a non-zero result
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
        echo "✅ $result"
    else
        echo "❌"
    fi
}

# Check EVM remote GMP endpoint for a given chain ID
# Calls getRemoteGmpEndpointAddrs(uint32) selector 0xfaa36825, returns bytes32[] (dynamic array)
check_evm_has_remote_gmp_endpoint() {
    local contract_addr="$1"
    local rpc_url="$2"
    local chain_id="$3"

    if [[ ! "$contract_addr" =~ ^0x ]]; then
        contract_addr="0x${contract_addr}"
    fi

    local chain_id_hex=$(printf "%064x" "$chain_id")
    local data="0xfaa36825${chain_id_hex}"

    local result=$(curl -s --max-time 10 -X POST "$rpc_url" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_call\",\"params\":[{\"to\":\"$contract_addr\",\"data\":\"$data\"},\"latest\"],\"id\":1}" \
        | jq -r '.result // "0x"' 2>/dev/null)

    # getRemoteGmpEndpointAddrs returns a dynamic bytes32 array, ABI-encoded as:
    #   0x20 (offset) + length (32 bytes) + element0 (32 bytes) + ...
    # Empty array: offset + length=0 → 0x0000...0020 0000...0000 (128 hex chars)
    # Extract first element: skip offset (64 chars) + length (64 chars) = chars 130..194
    if [ -n "$result" ] && [ ${#result} -gt 130 ]; then
        local first_element="0x${result:130:64}"
        if [ "$first_element" != "0x0000000000000000000000000000000000000000000000000000000000000000" ]; then
            echo "✅ $first_element"
        else
            echo "❌"
        fi
    else
        echo "❌"
    fi
}

# Check if MVM relay is authorized via is_relay_authorized(address) view function
check_mvm_relay_authorized() {
    local module_addr="$1"
    local relay_addr="$2"

    if [[ ! "$module_addr" =~ ^0x ]]; then
        module_addr="0x${module_addr}"
    fi
    if [[ ! "$relay_addr" =~ ^0x ]]; then
        relay_addr="0x${relay_addr}"
    fi

    local response=$(curl -s --max-time 10 -X POST "${MOVEMENT_RPC_URL}/view" \
        -H "Content-Type: application/json" \
        -d "{\"function\":\"${module_addr}::intent_gmp::is_relay_authorized\",\"type_arguments\":[],\"arguments\":[\"$relay_addr\"]}" \
        2>/dev/null)

    local result=$(echo "$response" | jq -r '.[0] // "false"' 2>/dev/null)

    if [ "$result" = "true" ]; then
        echo "✅"
    else
        echo "❌"
    fi
}

# Check if EVM relay is authorized via isRelayAuthorized(address)
# Selector: 0xe4082869
check_evm_relay_authorized() {
    local contract_addr="$1"
    local rpc_url="$2"
    local relay_addr="$3"

    if [[ ! "$contract_addr" =~ ^0x ]]; then
        contract_addr="0x${contract_addr}"
    fi
    if [[ ! "$relay_addr" =~ ^0x ]]; then
        relay_addr="0x${relay_addr}"
    fi

    local addr_no_prefix="${relay_addr#0x}"
    local addr_padded=$(printf "%064s" "$addr_no_prefix" | sed 's/ /0/g')
    local data="0xe4082869${addr_padded}"

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

# Read config files
MOVEMENT_INTENT_MODULE_ADDR_CFG=""
BASE_ESCROW_CONTRACT_ADDR=""
HYPERLIQUID_ESCROW_CONTRACT_ADDR=""
BASE_GMP_ENDPOINT_ADDR_CFG=""
HYPERLIQUID_GMP_ENDPOINT_ADDR_CFG=""

if [ -f "$COORDINATOR_CONFIG" ]; then
    MOVEMENT_INTENT_MODULE_ADDR_CFG=$(grep -A5 "\[hub_chain\]" "$COORDINATOR_CONFIG" | grep "intent_module_addr" | sed 's/.*= *"\(.*\)".*/\1/' | tr -d '"' || echo "")
    BASE_ESCROW_CONTRACT_ADDR=$(grep -A5 "connected_chain_evm" "$COORDINATOR_CONFIG" | grep "escrow_contract_addr" | head -1 | sed 's/.*= *"\(.*\)".*/\1/' | tr -d '"' || echo "")
fi

if [ -f "$INTEGRATED_GMP_CONFIG" ]; then
    BASE_GMP_ENDPOINT_ADDR_CFG=$(grep -A10 "connected_chain_evm" "$INTEGRATED_GMP_CONFIG" | grep "gmp_endpoint_addr" | head -1 | sed 's/.*= *"\(.*\)".*/\1/' | tr -d '"' || echo "")
fi

# Use env vars if available, fall back to config
if [ -z "$MOVEMENT_INTENT_MODULE_ADDR_CFG" ]; then
    MOVEMENT_INTENT_MODULE_ADDR_CFG="$MOVEMENT_INTENT_MODULE_ADDR"
fi

# ANSI colors
BLUE='\033[1;34m'
GREY='\033[90m'
NC='\033[0m'

# Track overall pass/fail - set to false on any ❌
all_ok=true
mark() { [[ "$1" != ✅* ]] && all_ok=false; }

# Format 32-byte hex result as EVM address (last 20 bytes)
fmt_addr() {
    local hex="${1#0x}"
    [ ${#hex} -ge 40 ] && echo "0x${hex: -40}" || echo "0x${hex}"
}

# Format hex to decimal
fmt_uint() {
    local hex="${1#0x}"
    hex=$(echo "$hex" | sed 's/^0*//')
    [ -z "$hex" ] && echo "0" && return
    echo "obase=10; ibase=16; $(echo "$hex" | tr 'a-f' 'A-F')" | bc 2>/dev/null || echo "0"
}

# Print a check result line
print_check() {
    local label="$1"
    local result="$2"
    local format="${3:-}"
    local indent="${4:-         }"
    local desc="${5:-}"

    local desc_suffix=""
    if [ -n "$desc" ]; then
        desc_suffix=" ${GREY}— ${desc}${NC}"
    fi

    if [[ "$result" == ✅* ]]; then
        local raw="${result#✅ }"
        if [ "$raw" = "✅" ] || [ -z "$format" ]; then
            echo -e "${indent}✅ $label${desc_suffix}"
        else
            local formatted=""
            case "$format" in
                address) formatted=$(fmt_addr "$raw") ;;
                uint) formatted=$(fmt_uint "$raw") ;;
                hex) formatted="$raw" ;;
                *) formatted="" ;;
            esac
            if [ -n "$formatted" ]; then
                echo -e "${indent}✅ $label: ${GREY}${formatted}${NC}${desc_suffix}"
            else
                echo -e "${indent}✅ $label${desc_suffix}"
            fi
        fi
    else
        echo -e "${indent}❌ $label${desc_suffix}"
    fi
}

# -----------------------------------------------------------------------------
# Movement Mainnet (Hub)
# -----------------------------------------------------------------------------
echo ""
echo "   Movement Mainnet (Hub)"
echo "   ----------------------"

# Intent Module (fa_intent)
echo -e "   ${BLUE}Intent Module (fa_intent):${NC}"
if [ -z "$MOVEMENT_INTENT_MODULE_ADDR_CFG" ] || [ "$MOVEMENT_INTENT_MODULE_ADDR_CFG" = "" ]; then
    all_ok=false
    echo "      ❌ Deployed (not locally configured)"
    echo "      ❌ Locally Configured (not set in coordinator_mainnet.toml)"
else
    deployed_status=$(check_movement_module "$MOVEMENT_INTENT_MODULE_ADDR_CFG"); mark "$deployed_status"
    init_status=$(check_movement_initialized "$MOVEMENT_INTENT_MODULE_ADDR_CFG" "fa_intent::ChainInfo"); mark "$init_status"
    print_check "Deployed" "$deployed_status" "" "      " "module bytecode on-chain"
    print_check "ChainInfo" "$init_status" "" "      " "chain identity and config initialized"
    echo -e "      ✅ Locally Configured: ${GREY}$MOVEMENT_INTENT_MODULE_ADDR_CFG${NC}"
fi

# GMP Module (intent_gmp) - bundled at same address as Intent Module
echo -e "   ${BLUE}GMP Module (intent_gmp):${NC}"
if [ -z "$MOVEMENT_INTENT_MODULE_ADDR_CFG" ] || [ "$MOVEMENT_INTENT_MODULE_ADDR_CFG" = "" ]; then
    all_ok=false
    echo "      ❌ Deployed (not locally configured)"
else
    deployed_status=$(check_movement_gmp_module "$MOVEMENT_INTENT_MODULE_ADDR_CFG"); mark "$deployed_status"
    init_status=$(check_movement_initialized "$MOVEMENT_INTENT_MODULE_ADDR_CFG" "intent_gmp::EndpointConfig"); mark "$init_status"
    tr_base=$(check_gmp_remote_endpoint "$MOVEMENT_INTENT_MODULE_ADDR_CFG" "intent_gmp" "$BASE_CHAIN_ID"); mark "$tr_base"
    tr_hyper=$(check_gmp_remote_endpoint "$MOVEMENT_INTENT_MODULE_ADDR_CFG" "intent_gmp" "$HYPERLIQUID_CHAIN_ID_VAL"); mark "$tr_hyper"
    print_check "Deployed" "$deployed_status" "" "      " "module bytecode on-chain"
    print_check "EndpointConfig" "$init_status" "" "      " "GMP endpoint initialized with chain ID"
    print_check "Remote GMP Endpoint (Base $BASE_CHAIN_ID)" "$tr_base" "hex" "      " "only accepts GMP from this Base address"
    print_check "Remote GMP Endpoint (HyperEVM $HYPERLIQUID_CHAIN_ID_VAL)" "$tr_hyper" "hex" "      " "only accepts GMP from this HyperEVM address"
    if [ -n "$INTEGRATED_GMP_MVM_ADDR" ]; then
        relay_auth=$(check_mvm_relay_authorized "$MOVEMENT_INTENT_MODULE_ADDR_CFG" "$INTEGRATED_GMP_MVM_ADDR"); mark "$relay_auth"
        print_check "Relay Authorized ${GREY}($INTEGRATED_GMP_MVM_ADDR)${NC}" "$relay_auth" "" "      " "relay can deliver cross-chain messages"
    fi
    echo -e "      ✅ Locally Configured: ${GREY}bundled at $MOVEMENT_INTENT_MODULE_ADDR_CFG${NC}"
fi

# GMP Hub (intent_gmp_hub) - bundled at same address
echo -e "   ${BLUE}GMP Hub (intent_gmp_hub):${NC}"
if [ -z "$MOVEMENT_INTENT_MODULE_ADDR_CFG" ] || [ "$MOVEMENT_INTENT_MODULE_ADDR_CFG" = "" ]; then
    all_ok=false
    echo "      ❌ GmpHubConfig"
else
    hub_init=$(check_movement_initialized "$MOVEMENT_INTENT_MODULE_ADDR_CFG" "intent_gmp_hub::GmpHubConfig"); mark "$hub_init"
    hub_tr_base=$(check_gmp_remote_endpoint "$MOVEMENT_INTENT_MODULE_ADDR_CFG" "intent_gmp_hub" "$BASE_CHAIN_ID"); mark "$hub_tr_base"
    hub_tr_hyper=$(check_gmp_remote_endpoint "$MOVEMENT_INTENT_MODULE_ADDR_CFG" "intent_gmp_hub" "$HYPERLIQUID_CHAIN_ID_VAL"); mark "$hub_tr_hyper"
    print_check "GmpHubConfig" "$hub_init" "" "      " "hub routing for cross-chain messages"
    print_check "Remote GMP Endpoint (Base $BASE_CHAIN_ID)" "$hub_tr_base" "hex" "      " "only accepts GMP from this Base address"
    print_check "Remote GMP Endpoint (HyperEVM $HYPERLIQUID_CHAIN_ID_VAL)" "$hub_tr_hyper" "hex" "      " "only accepts GMP from this HyperEVM address"
fi

# GMP Sender (gmp_sender)
echo -e "   ${BLUE}GMP Sender (gmp_sender):${NC}"
if [ -z "$MOVEMENT_INTENT_MODULE_ADDR_CFG" ] || [ "$MOVEMENT_INTENT_MODULE_ADDR_CFG" = "" ]; then
    all_ok=false
    echo "      ❌ SenderConfig"
else
    sender_init=$(check_movement_initialized "$MOVEMENT_INTENT_MODULE_ADDR_CFG" "gmp_sender::SenderConfig"); mark "$sender_init"
    print_check "SenderConfig" "$sender_init" "" "      " "outbound message sender initialized"
fi

# -----------------------------------------------------------------------------
# Base Mainnet (EVM)
# -----------------------------------------------------------------------------
echo ""
echo "   Base Mainnet (EVM)"
echo "   ------------------"

# Escrow Contract (IntentInflowEscrow)
echo -e "   ${BLUE}Escrow Contract (IntentInflowEscrow):${NC}"
BASE_ESCROW_ADDR="${BASE_ESCROW_CONTRACT_ADDR:-$BASE_INFLOW_ESCROW_ADDR}"
if [ -z "$BASE_ESCROW_ADDR" ] || [ "$BASE_ESCROW_ADDR" = "" ]; then
    all_ok=false
    echo "      ❌ Deployed (not locally configured)"
    echo "      ❌ Locally Configured (not set in coordinator_mainnet.toml)"
else
    deployed_status=$(check_evm_contract "$BASE_ESCROW_ADDR" "$BASE_RPC_URL"); mark "$deployed_status"
    gmp_ep=$(check_evm_nonzero_result "$BASE_ESCROW_ADDR" "$BASE_RPC_URL" "0xb2ed7d86"); mark "$gmp_ep"
    hub_cid=$(check_evm_nonzero_result "$BASE_ESCROW_ADDR" "$BASE_RPC_URL" "0x929f5840"); mark "$hub_cid"
    hub_addr=$(check_evm_nonzero_result "$BASE_ESCROW_ADDR" "$BASE_RPC_URL" "0xa227f5dd"); mark "$hub_addr"
    print_check "Deployed" "$deployed_status" "" "      " "contract bytecode on-chain"
    print_check "gmpEndpoint" "$gmp_ep" "address" "      " "GMP contract for cross-chain messaging"
    print_check "hubChainId" "$hub_cid" "uint" "      " "hub chain for outbound messages"
    print_check "hubGmpEndpointAddr" "$hub_addr" "hex" "      " "hub GMP endpoint address for inbound messages"
    echo -e "      ✅ Locally Configured: ${GREY}$BASE_ESCROW_ADDR${NC}"
fi

# GMP Endpoint (IntentGmp)
echo -e "   ${BLUE}GMP Endpoint (IntentGmp):${NC}"
BASE_GMP_ADDR="${BASE_GMP_ENDPOINT_ADDR_CFG:-$BASE_GMP_ENDPOINT_ADDR}"
if [ -z "$BASE_GMP_ADDR" ] || [ "$BASE_GMP_ADDR" = "" ]; then
    all_ok=false
    echo "      ❌ Deployed (not locally configured)"
    echo "      ❌ Locally Configured (not set in integrated-gmp_mainnet.toml)"
else
    deployed_status=$(check_evm_contract "$BASE_GMP_ADDR" "$BASE_RPC_URL"); mark "$deployed_status"
    escrow_h=$(check_evm_nonzero_result "$BASE_GMP_ADDR" "$BASE_RPC_URL" "0x87ad8f87"); mark "$escrow_h"
    outflow_h=$(check_evm_nonzero_result "$BASE_GMP_ADDR" "$BASE_RPC_URL" "0xa80693bc"); mark "$outflow_h"
    tr_hub=$(check_evm_has_remote_gmp_endpoint "$BASE_GMP_ADDR" "$BASE_RPC_URL" "$MOVEMENT_CHAIN_ID"); mark "$tr_hub"
    print_check "Deployed" "$deployed_status" "" "      " "contract bytecode on-chain"
    print_check "escrowHandler" "$escrow_h" "address" "      " "receives inbound token transfers"
    print_check "outflowHandler" "$outflow_h" "address" "      " "validates outbound fulfillments"
    print_check "Remote GMP Endpoint (Movement $MOVEMENT_CHAIN_ID)" "$tr_hub" "hex" "      " "only accepts GMP from this hub address"
    if [ -n "$GMP_RELAY_EVM_ADDR" ]; then
        relay_auth=$(check_evm_relay_authorized "$BASE_GMP_ADDR" "$BASE_RPC_URL" "$GMP_RELAY_EVM_ADDR"); mark "$relay_auth"
        print_check "Relay Authorized ${GREY}($GMP_RELAY_EVM_ADDR)${NC}" "$relay_auth" "" "      " "relay can deliver cross-chain messages"
    fi
    echo -e "      ✅ Locally Configured: ${GREY}$BASE_GMP_ADDR${NC}"
fi

# -----------------------------------------------------------------------------
# HyperEVM Mainnet (EVM)
# -----------------------------------------------------------------------------
echo ""
echo "   HyperEVM Mainnet (EVM)"
echo "   ----------------------"

# Escrow Contract (IntentInflowEscrow)
echo -e "   ${BLUE}Escrow Contract (IntentInflowEscrow):${NC}"
HYPER_ESCROW_ADDR="${HYPERLIQUID_INFLOW_ESCROW_ADDR:-}"
if [ -z "$HYPER_ESCROW_ADDR" ] || [ "$HYPER_ESCROW_ADDR" = "" ]; then
    all_ok=false
    echo "      ❌ Deployed (not locally configured)"
    echo "      ❌ Locally Configured (not set in .env.mainnet)"
else
    deployed_status=$(check_evm_contract "$HYPER_ESCROW_ADDR" "$HYPERLIQUID_RPC_URL"); mark "$deployed_status"
    gmp_ep=$(check_evm_nonzero_result "$HYPER_ESCROW_ADDR" "$HYPERLIQUID_RPC_URL" "0xb2ed7d86"); mark "$gmp_ep"
    hub_cid=$(check_evm_nonzero_result "$HYPER_ESCROW_ADDR" "$HYPERLIQUID_RPC_URL" "0x929f5840"); mark "$hub_cid"
    hub_addr=$(check_evm_nonzero_result "$HYPER_ESCROW_ADDR" "$HYPERLIQUID_RPC_URL" "0xa227f5dd"); mark "$hub_addr"
    print_check "Deployed" "$deployed_status" "" "      " "contract bytecode on-chain"
    print_check "gmpEndpoint" "$gmp_ep" "address" "      " "GMP contract for cross-chain messaging"
    print_check "hubChainId" "$hub_cid" "uint" "      " "hub chain for outbound messages"
    print_check "hubGmpEndpointAddr" "$hub_addr" "hex" "      " "hub GMP endpoint address for inbound messages"
    echo -e "      ✅ Locally Configured: ${GREY}$HYPER_ESCROW_ADDR${NC}"
fi

# GMP Endpoint (IntentGmp)
echo -e "   ${BLUE}GMP Endpoint (IntentGmp):${NC}"
HYPER_GMP_ADDR="${HYPERLIQUID_GMP_ENDPOINT_ADDR:-}"
if [ -z "$HYPER_GMP_ADDR" ] || [ "$HYPER_GMP_ADDR" = "" ]; then
    all_ok=false
    echo "      ❌ Deployed (not locally configured)"
    echo "      ❌ Locally Configured (not set in .env.mainnet)"
else
    deployed_status=$(check_evm_contract "$HYPER_GMP_ADDR" "$HYPERLIQUID_RPC_URL"); mark "$deployed_status"
    escrow_h=$(check_evm_nonzero_result "$HYPER_GMP_ADDR" "$HYPERLIQUID_RPC_URL" "0x87ad8f87"); mark "$escrow_h"
    outflow_h=$(check_evm_nonzero_result "$HYPER_GMP_ADDR" "$HYPERLIQUID_RPC_URL" "0xa80693bc"); mark "$outflow_h"
    tr_hub=$(check_evm_has_remote_gmp_endpoint "$HYPER_GMP_ADDR" "$HYPERLIQUID_RPC_URL" "$MOVEMENT_CHAIN_ID"); mark "$tr_hub"
    print_check "Deployed" "$deployed_status" "" "      " "contract bytecode on-chain"
    print_check "escrowHandler" "$escrow_h" "address" "      " "receives inbound token transfers"
    print_check "outflowHandler" "$outflow_h" "address" "      " "validates outbound fulfillments"
    print_check "Remote GMP Endpoint (Movement $MOVEMENT_CHAIN_ID)" "$tr_hub" "hex" "      " "only accepts GMP from this hub address"
    if [ -n "$GMP_RELAY_EVM_ADDR" ]; then
        relay_auth=$(check_evm_relay_authorized "$HYPER_GMP_ADDR" "$HYPERLIQUID_RPC_URL" "$GMP_RELAY_EVM_ADDR"); mark "$relay_auth"
        print_check "Relay Authorized ${GREY}($GMP_RELAY_EVM_ADDR)${NC}" "$relay_auth" "" "      " "relay can deliver cross-chain messages"
    fi
    echo -e "      ✅ Locally Configured: ${GREY}$HYPER_GMP_ADDR${NC}"
fi

echo ""

# =============================================================================
# SUMMARY
# =============================================================================

echo " Summary"
echo "----------"
echo ""
echo "   Assets Config: $ASSETS_CONFIG_FILE"
echo "   Service Configs: coordinator_mainnet.toml, integrated-gmp_mainnet.toml, solver_mainnet.toml (gitignored)"
echo "   Keys:   $MAINNET_KEYS_FILE"
echo ""
if [ "$all_ok" = true ]; then
    echo "✅ Preparedness check success."
else
    echo "❌ Preparedness check failure."
    echo "   Fix the failing checks above before mainnet runs."
fi
