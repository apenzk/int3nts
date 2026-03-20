#!/bin/bash
# Shared utilities for deployment and configuration scripts.
# Used by both testnet and mainnet. Network-specific env-utils.sh files
# source this and add their own functions/defaults.

# Update or add a variable in an env file.
# Usage: update_env_var <file> <KEY> <value>
update_env_var() {
    local file="$1"
    local key="$2"
    local value="$3"

    if grep -q "^${key}=" "$file" 2>/dev/null; then
        # Detect sed flavor (BSD vs GNU) — OSTYPE is unreliable inside nix shells
        if sed --version >/dev/null 2>&1; then
            # GNU sed
            sed -i "s|^${key}=.*|${key}=${value}|" "$file"
        else
            # BSD sed (macOS)
            sed -i '' "s|^${key}=.*|${key}=${value}|" "$file"
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
# The ENV_FILE_NAME variable controls which env file is named in the error message.
# Set it before sourcing this file (defaults to ".env").
require_var() {
    local name="$1"
    local value="$2"
    local desc="${3:-$name}"

    if [ -z "$value" ]; then
        echo "ERROR: ${name} not set in ${ENV_FILE_NAME:-.env} (${desc})"
        exit 1
    fi
}

# Read a chain_id from an assets TOML config by section name.
# Usage: get_chain_id "solana_devnet" "/path/to/assets.toml"
# Exits with error if section or chain_id not found.
get_chain_id() {
    local section="$1"
    local config_file="$2"

    if [ -z "$config_file" ]; then
        echo "ERROR: get_chain_id requires a config file path" >&2
        exit 1
    fi

    if [ ! -f "$config_file" ]; then
        echo "ERROR: assets config not found at $config_file" >&2
        exit 1
    fi

    local chain_id
    chain_id=$(grep -A 5 "^\[${section}\]" "$config_file" | grep "^chain_id = " | sed 's/.*= \([0-9]*\).*/\1/' || echo "")

    if [ -z "$chain_id" ]; then
        echo "ERROR: chain_id not found for [${section}] in $config_file" >&2
        exit 1
    fi

    echo "$chain_id"
}

# Load an env file, skipping if DEPLOY_ENV_SOURCED is already set (parent script already loaded it).
# Exits with error if the file does not exist.
# Usage: load_env_file "$SCRIPT_DIR/../.env.testnet"
load_env_file() {
    local env_file="$1"

    if [ ! -f "$env_file" ]; then
        echo "ERROR: ${ENV_FILE_NAME:-.env} not found at $env_file"
        echo "   Create it from the example file in this directory"
        exit 1
    fi

    if [ "${DEPLOY_ENV_SOURCED:-}" != "1" ]; then
        source "$env_file"
    fi
}

# Verify an EVM contract is deployed by checking eth_getCode.
# Exits with FATAL error if contract has no code at the address.
# Usage: verify_evm_contract <rpc_url> <contract_addr> <contract_name>
verify_evm_contract() {
    local rpc_url="$1"
    local contract_addr="$2"
    local contract_name="$3"

    local code
    code=$(curl -s --max-time 10 -X POST "$rpc_url" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getCode\",\"params\":[\"$contract_addr\",\"latest\"],\"id\":1}" \
        | jq -r '.result // ""' 2>/dev/null)

    if [ -z "$code" ] || [ "$code" = "0x" ] || [ "$code" = "" ]; then
        echo "FATAL: $contract_name contract not found at $contract_addr"
        exit 1
    fi
    echo "   $contract_name ($contract_addr): deployed"
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

# Run a Solana CLI command, tolerating "already initialized" errors for idempotent re-runs.
# Any other failure is fatal. Captures stderr+stdout to distinguish error types.
# Usage: run_solana_idempotent <description> <command> [args...]
run_solana_idempotent() {
    local desc="$1"
    shift

    local output
    set +e
    output=$("$@" 2>&1)
    local exit_code=$?
    set -e

    if [ $exit_code -eq 0 ]; then
        echo "$output"
        return 0
    fi

    # Tolerate "already initialized" — account exists from a previous run
    if echo "$output" | grep -q "uninitialized account\|already in use\|custom program error: 0x0"; then
        echo "   Already configured (idempotent), skipping: ${desc}"
        return 0
    fi

    # Any other error is fatal
    echo "$output"
    echo "FATAL: ${desc} failed (exit code $exit_code)"
    exit 1
}

# Verify a Solana program has an account matching discriminator + size.
# Exits with error if no matching account found.
# Usage: verify_solana_has_account <program_id> <rpc_url> <disc_base64> <data_size> <description> [<memcmp_offset> <memcmp_bytes_base58>]
# Optional memcmp filter checks for specific data at a given offset (e.g., relay pubkey at offset 1).
verify_solana_has_account() {
    local program_id="$1"
    local rpc_url="$2"
    local disc_base64="$3"
    local data_size="$4"
    local description="$5"
    local extra_memcmp_offset="${6:-}"
    local extra_memcmp_bytes="${7:-}"

    local filters="[{\"dataSize\":$data_size},{\"memcmp\":{\"offset\":0,\"bytes\":\"$disc_base64\",\"encoding\":\"base64\"}}"
    if [ -n "$extra_memcmp_offset" ] && [ -n "$extra_memcmp_bytes" ]; then
        filters="${filters},{\"memcmp\":{\"offset\":$extra_memcmp_offset,\"bytes\":\"$extra_memcmp_bytes\"}}"
    fi
    filters="${filters}]"

    local response=$(curl -s --max-time 10 -X POST "$rpc_url" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"getProgramAccounts\",\"params\":[\"$program_id\",{\"encoding\":\"base64\",\"dataSlice\":{\"offset\":0,\"length\":0},\"filters\":$filters}],\"id\":1}" \
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
