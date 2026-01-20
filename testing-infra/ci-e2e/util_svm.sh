#!/bin/bash

# SVM-specific utilities for testing infrastructure scripts
# This file MUST be sourced AFTER util.sh
# Usage:
#   source "$(dirname "$0")/../util.sh"
#   source "$(dirname "$0")/../util_svm.sh"

set -e

# Run a Solana CLI command inside nix develop ./nix
# Usage: svm_cmd "<command>"
svm_cmd() {
    local cmd="$1"
    if [ -z "$cmd" ]; then
        log_and_echo "❌ ERROR: svm_cmd requires a command"
        exit 1
    fi
    if [ -z "$PROJECT_ROOT" ]; then
        setup_project_root
    fi
    # Filter nix develop banners from stdout while preserving the command status.
    local status
    set +e
    NIX_CONFIG="warn-dirty = false" nix develop "$PROJECT_ROOT/nix" -c bash -c "$cmd" \
        | sed -e '/^\[nix\] Dev shell ready:/d' -e '/^warning: Git tree/d'
    status=${PIPESTATUS[0]}
    set -e
    return "$status"
}

# Check if SVM chain is running
# Usage: check_svm_chain_running [rpc_url]
check_svm_chain_running() {
    local rpc_url="${1:-http://127.0.0.1:8899}"
    if curl -s -X POST "$rpc_url" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"getHealth","params":[],"id":1}' \
        | grep -q '"result":"ok"'; then
        return 0
    fi
    return 1
}

# Ensure a keypair exists at the given path
# Usage: ensure_svm_keypair <path>
ensure_svm_keypair() {
    local keypair_path="$1"
    if [ -z "$keypair_path" ]; then
        log_and_echo "❌ ERROR: ensure_svm_keypair requires a keypair path"
        exit 1
    fi

    if [ ! -f "$keypair_path" ]; then
        log "   Generating keypair: $keypair_path"
        svm_cmd "solana-keygen new --no-bip39-passphrase --silent -o \"$keypair_path\""
    else
        log "   ✅ Keypair already exists: $keypair_path"
    fi
}

# Get base58 pubkey for a keypair file
# Usage: get_svm_pubkey <keypair_path>
get_svm_pubkey() {
    local keypair_path="$1"
    # nix develop prints a banner; capture only the actual pubkey
    svm_cmd "solana-keygen pubkey \"$keypair_path\"" | tail -n 1
}

# Convert base58 pubkey to 0x-hex string
# Usage: svm_pubkey_to_hex <base58_pubkey>
svm_pubkey_to_hex() {
    # Use node inside nix develop ./nix to avoid relying on system python
    if [ -z "$PROJECT_ROOT" ]; then
        setup_project_root
    fi
    svm_cmd "node \"$PROJECT_ROOT/testing-infra/ci-e2e/base58.js\" decode-base58-to-hex \"$1\"" | tail -n 1
}

# Convert base64-encoded public key bytes to base58
# Usage: svm_base64_to_base58 <base64_pubkey>
svm_base64_to_base58() {
    if [ -z "$PROJECT_ROOT" ]; then
        setup_project_root
    fi
    svm_cmd "node \"$PROJECT_ROOT/testing-infra/ci-e2e/base58.js\" encode-base64 \"$1\"" | tail -n 1
}

# Convert keypair JSON file (byte array) to base58 private key
# Usage: svm_keypair_to_base58 <keypair_json_path>
svm_keypair_to_base58() {
    local keypair_path="$1"
    if [ -z "$PROJECT_ROOT" ]; then
        setup_project_root
    fi
    svm_cmd "node \"$PROJECT_ROOT/testing-infra/ci-e2e/base58.js\" encode-keypair-json \"$keypair_path\"" | tail -n 1
}

# Airdrop SOL to a pubkey
# Usage: airdrop_svm <pubkey> <amount> [rpc_url]
airdrop_svm() {
    local pubkey="$1"
    local amount="${2:-10}"
    local rpc_url="${3:-http://127.0.0.1:8899}"
    svm_cmd "solana airdrop \"$amount\" \"$pubkey\" --url \"$rpc_url\" >/dev/null"
}

# Create an SPL token mint
# Usage: create_svm_mint <payer_keypair> [rpc_url]
create_svm_mint() {
    local payer_keypair="$1"
    local rpc_url="${2:-http://127.0.0.1:8899}"
    svm_cmd "spl-token create-token --decimals 6 --url \"$rpc_url\" --fee-payer \"$payer_keypair\" --mint-authority \"$payer_keypair\" \
        | awk '/Creating token/ {print \$3}'" | tail -n 1
}

# Create an SPL token account
# Usage: create_svm_token_account <mint> <owner_pubkey> <payer_keypair> [rpc_url]
create_svm_token_account() {
    local mint="$1"
    local owner="$2"
    local payer_keypair="$3"
    local rpc_url="${4:-http://127.0.0.1:8899}"
    local ata
    ata=$(get_svm_associated_token_address "$mint" "$owner" "$rpc_url")
    svm_cmd "spl-token create-account \"$mint\" --owner \"$owner\" --url \"$rpc_url\" --fee-payer \"$payer_keypair\" >/dev/null"
    echo "$ata"
}

# Get the associated token address (ATA) for an owner and mint
# Usage: get_svm_associated_token_address <mint> <owner_pubkey> [rpc_url]
get_svm_associated_token_address() {
    local mint="$1"
    local owner="$2"
    local rpc_url="${3:-http://127.0.0.1:8899}"
    svm_cmd "spl-token address --verbose --owner \"$owner\" --token \"$mint\" --url \"$rpc_url\" \
        | awk '/Associated token address:/ {print \$NF} /Address:/ {print \$NF}'" | tail -n 1
}

# Mint tokens to an account
# Usage: mint_svm_tokens <mint> <amount> <account> <payer_keypair> [rpc_url]
mint_svm_tokens() {
    local mint="$1"
    local amount="$2"
    local account="$3"
    local payer_keypair="$4"
    local rpc_url="${5:-http://127.0.0.1:8899}"
    svm_cmd "spl-token mint \"$mint\" \"$amount\" \"$account\" --url \"$rpc_url\" --fee-payer \"$payer_keypair\" --mint-authority \"$payer_keypair\""
}
