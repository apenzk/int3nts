#!/bin/bash
# Shared utilities for testnet deployment and configuration scripts.
# Source this file: source "$(dirname "$0")/lib/env-utils.sh"

# Update or add a variable in .env.testnet
# Usage: update_env_var <file> <KEY> <value>
update_env_var() {
    local file="$1"
    local key="$2"
    local value="$3"

    if grep -q "^${key}=" "$file" 2>/dev/null; then
        # macOS and GNU sed have different -i syntax
        if [[ "$OSTYPE" == "darwin"* ]]; then
            sed -i '' "s|^${key}=.*|${key}=${value}|" "$file"
        else
            sed -i "s|^${key}=.*|${key}=${value}|" "$file"
        fi
    else
        echo "${key}=${value}" >> "$file"
    fi
}

# Pad a hex address to 32 bytes (64 hex chars), stripping 0x prefix.
# Returns the padded hex WITHOUT 0x prefix.
# Usage: pad_address_32 "0xabc123"
pad_address_32() {
    local addr="$1"
    local clean=$(echo "$addr" | sed 's/^0x//')
    printf "%064s" "$clean" | tr ' ' '0'
}

# Require a variable to be set, exit with error if not.
# Usage: require_var "VAR_NAME" "$VAR_VALUE" "description"
require_var() {
    local name="$1"
    local value="$2"
    local desc="${3:-$name}"

    if [ -z "$value" ]; then
        echo "ERROR: ${name} not set in .env.testnet (${desc})"
        exit 1
    fi
}

# Verify a Movement view function returns a non-empty result.
# Exits with error if the result is empty/null/0x.
# Usage: verify_movement_view <rpc_url> <function_id> <arguments_json> <description>
verify_movement_view() {
    local rpc_url="$1"
    local function_id="$2"
    local args_json="$3"
    local description="$4"

    local response=$(curl -s --max-time 10 -X POST "${rpc_url}/view" \
        -H "Content-Type: application/json" \
        -d "{\"function\":\"${function_id}\",\"type_arguments\":[],\"arguments\":${args_json}}" \
        2>/dev/null)

    local result=$(echo "$response" | jq -r '.[0] // ""' 2>/dev/null)

    if [ -z "$result" ] || [ "$result" = "" ] || [ "$result" = "null" ] || [ "$result" = "0x" ]; then
        echo "FATAL: Verification failed - ${description}"
        echo "   View function: ${function_id}"
        echo "   Arguments: ${args_json}"
        echo "   Response: ${response}"
        exit 1
    fi

    echo "   Verified on-chain: ${description}"
}

# Verify a Solana program has an account matching discriminator + size.
# Exits with error if no matching account found.
# Usage: verify_solana_has_account <program_id> <rpc_url> <disc_base64> <data_size> <description>
verify_solana_has_account() {
    local program_id="$1"
    local rpc_url="$2"
    local disc_base64="$3"
    local data_size="$4"
    local description="$5"

    local response=$(curl -s --max-time 10 -X POST "$rpc_url" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"getProgramAccounts\",\"params\":[\"$program_id\",{\"encoding\":\"base64\",\"dataSlice\":{\"offset\":0,\"length\":0},\"filters\":[{\"dataSize\":$data_size},{\"memcmp\":{\"offset\":0,\"bytes\":\"$disc_base64\",\"encoding\":\"base64\"}}]}],\"id\":1}" \
        2>/dev/null)

    local count=$(echo "$response" | jq -r '.result | length // 0' 2>/dev/null)

    if [ "$count" -gt 0 ] 2>/dev/null; then
        echo "   Verified on-chain: ${description}"
    else
        echo "FATAL: Verification failed - ${description}"
        echo "   Program: ${program_id}"
        echo "   Discriminator (base64): ${disc_base64}"
        echo "   Expected data size: ${data_size}"
        echo "   Response: ${response}"
        exit 1
    fi
}
