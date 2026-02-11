#!/bin/bash

# Configure Solana Devnet - Set up cross-chain GMP routing
#
# This script configures the Solana GMP programs with cross-chain links
# to the Movement hub. Must be run AFTER all deployments are complete.
#
# Steps:
#   1. Set hub as trusted remote on GMP endpoint
#   2. Initialize outflow validator with hub config
#   3. Configure escrow GMP with hub address
#   4. Set GMP routing between programs
#   5. Add GMP relay authorization (if INTEGRATED_GMP_SVM_ADDR is set)
#
# Requires:
#   - .env.testnet with:
#     - SOLANA_DEPLOYER_PRIVATE_KEY
#     - MOVEMENT_INTENT_MODULE_ADDR (from deploy-to-movement-testnet.sh)
#     - SOLANA_GMP_ID, SOLANA_PROGRAM_ID, SOLANA_OUTFLOW_ID (from deploy-to-solana-devnet.sh)
#     - INTEGRATED_GMP_SVM_ADDR (optional, for relay authorization)
#   - Node.js (for base58 conversion)
#   - Solana CLI

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../.." && pwd )"

# Re-exec inside nix develop if not already in a nix shell
if [ -z "${IN_NIX_SHELL:-}" ]; then
    exec nix develop "$PROJECT_ROOT/nix" --command bash "$SCRIPT_DIR/configure-solana-devnet.sh" "$@"
fi

source "$SCRIPT_DIR/../lib/env-utils.sh"

echo " Configuring Solana Devnet (Cross-Chain GMP)"
echo "=============================================="
echo ""

# Load .env.testnet
TESTNET_KEYS_FILE="$SCRIPT_DIR/../.env.testnet"
if [ ! -f "$TESTNET_KEYS_FILE" ]; then
    echo "ERROR: .env.testnet not found at $TESTNET_KEYS_FILE"
    exit 1
fi
if [ "${DEPLOY_ENV_SOURCED:-}" != "1" ]; then
    source "$TESTNET_KEYS_FILE"
fi

require_var "SOLANA_DEPLOYER_PRIVATE_KEY" "$SOLANA_DEPLOYER_PRIVATE_KEY"
require_var "MOVEMENT_INTENT_MODULE_ADDR" "$MOVEMENT_INTENT_MODULE_ADDR" "Run deploy-to-movement-testnet.sh first"
require_var "SOLANA_GMP_ID" "$SOLANA_GMP_ID" "Run deploy-to-solana-devnet.sh first"
require_var "SOLANA_PROGRAM_ID" "$SOLANA_PROGRAM_ID" "Run deploy-to-solana-devnet.sh first"
require_var "SOLANA_OUTFLOW_ID" "$SOLANA_OUTFLOW_ID" "Run deploy-to-solana-devnet.sh first"

SOLANA_RPC_URL="${SOLANA_RPC_URL:-https://api.devnet.solana.com}"
HUB_CHAIN_ID="${HUB_CHAIN_ID:-250}"

echo " Configuration:"
echo "   Hub Chain ID: $HUB_CHAIN_ID"
echo "   Movement Module: $MOVEMENT_INTENT_MODULE_ADDR"
echo "   Solana GMP:      $SOLANA_GMP_ID"
echo "   Solana Escrow:   $SOLANA_PROGRAM_ID"
echo "   Solana Outflow:  $SOLANA_OUTFLOW_ID"
echo ""

# Create temporary keypair file from base58 private key
TEMP_KEYPAIR_DIR=$(mktemp -d)
DEPLOYER_KEYPAIR="$TEMP_KEYPAIR_DIR/deployer.json"

echo " Converting deployer private key to keypair file..."

# Use Node.js to convert base58 private key to JSON keypair format
node -e "
const ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
function b58decode(str) {
    const bytes = [];
    for (let i = 0; i < str.length; i++) {
        const idx = ALPHABET.indexOf(str[i]);
        if (idx < 0) throw new Error('Invalid base58 character');
        let carry = idx;
        for (let j = 0; j < bytes.length; j++) {
            carry += bytes[j] * 58;
            bytes[j] = carry & 0xff;
            carry >>= 8;
        }
        while (carry > 0) {
            bytes.push(carry & 0xff);
            carry >>= 8;
        }
    }
    for (let i = 0; i < str.length && str[i] === '1'; i++) {
        bytes.push(0);
    }
    return bytes.reverse();
}
console.log(JSON.stringify(b58decode('$SOLANA_DEPLOYER_PRIVATE_KEY')));
" > "$DEPLOYER_KEYPAIR"

if [ ! -s "$DEPLOYER_KEYPAIR" ]; then
    echo "ERROR: Failed to convert private key"
    rm -rf "$TEMP_KEYPAIR_DIR"
    exit 1
fi

# Verify keypair
DERIVED_ADDR=$(solana-keygen pubkey "$DEPLOYER_KEYPAIR" 2>&1) || {
    echo "ERROR: solana-keygen pubkey failed: $DERIVED_ADDR"
    rm -rf "$TEMP_KEYPAIR_DIR"
    exit 1
}

if [ -n "$SOLANA_DEPLOYER_ADDR" ] && [ "$DERIVED_ADDR" != "$SOLANA_DEPLOYER_ADDR" ]; then
    echo "ERROR: Derived address does not match SOLANA_DEPLOYER_ADDR"
    echo "   Derived:  $DERIVED_ADDR"
    echo "   Expected: $SOLANA_DEPLOYER_ADDR"
    rm -rf "$TEMP_KEYPAIR_DIR"
    exit 1
fi

echo "   Deployer verified: $DERIVED_ADDR"
echo ""

# Build CLI if needed
CLI_BIN="$PROJECT_ROOT/intent-frameworks/svm/target/debug/intent_escrow_cli"
if [ ! -x "$CLI_BIN" ]; then
    echo " Building CLI tool..."
    cd "$PROJECT_ROOT/intent-frameworks/svm"
    cargo build --bin intent_escrow_cli 2>/dev/null
fi

if [ ! -x "$CLI_BIN" ]; then
    echo "ERROR: CLI tool not built at $CLI_BIN"
    rm -rf "$TEMP_KEYPAIR_DIR"
    exit 1
fi

# Pad hub address to 64 hex characters (32 bytes)
HUB_ADDR_PADDED=$(pad_address_32 "$MOVEMENT_INTENT_MODULE_ADDR")

echo " Cross-chain configuration..."
echo ""

echo " 1. Setting hub as trusted remote..."
if ! "$CLI_BIN" gmp-set-trusted-remote \
    --gmp-program-id "$SOLANA_GMP_ID" \
    --payer "$DEPLOYER_KEYPAIR" \
    --src-chain-id "$HUB_CHAIN_ID" \
    --trusted-addr "$HUB_ADDR_PADDED" \
    --rpc "$SOLANA_RPC_URL"; then
    echo "   Command failed, verifying on-chain state..."
fi
# TrustedRemoteAccount: disc=3 base64=Aw==, size=38
verify_solana_has_account "$SOLANA_GMP_ID" "$SOLANA_RPC_URL" "Aw==" 38 \
    "GMP trusted remote for hub (chain $HUB_CHAIN_ID)"

echo " 2. Initializing outflow validator..."
if ! "$CLI_BIN" outflow-init \
    --outflow-program-id "$SOLANA_OUTFLOW_ID" \
    --payer "$DEPLOYER_KEYPAIR" \
    --gmp-endpoint "$SOLANA_GMP_ID" \
    --hub-chain-id "$HUB_CHAIN_ID" \
    --hub-address "$HUB_ADDR_PADDED" \
    --rpc "$SOLANA_RPC_URL"; then
    echo "   Command failed, verifying on-chain state..."
fi
# ConfigAccount: disc=2 base64=Ag==, size=102
verify_solana_has_account "$SOLANA_OUTFLOW_ID" "$SOLANA_RPC_URL" "Ag==" 102 \
    "Outflow validator config"

echo " 3. Configuring escrow GMP..."
if ! "$CLI_BIN" escrow-set-gmp-config \
    --program-id "$SOLANA_PROGRAM_ID" \
    --payer "$DEPLOYER_KEYPAIR" \
    --hub-chain-id "$HUB_CHAIN_ID" \
    --hub-address "$HUB_ADDR_PADDED" \
    --gmp-endpoint "$SOLANA_GMP_ID" \
    --rpc "$SOLANA_RPC_URL"; then
    echo "   Command failed, verifying on-chain state..."
fi
# GmpConfig: disc="GMPCONFG" base64=R01QQ09ORkc=, size=109
verify_solana_has_account "$SOLANA_PROGRAM_ID" "$SOLANA_RPC_URL" "R01QQ09ORkc=" 109 \
    "Escrow GMP config"

echo " 4. Setting GMP routing..."
if ! "$CLI_BIN" gmp-set-routing \
    --gmp-program-id "$SOLANA_GMP_ID" \
    --payer "$DEPLOYER_KEYPAIR" \
    --outflow-validator "$SOLANA_OUTFLOW_ID" \
    --intent-escrow "$SOLANA_PROGRAM_ID" \
    --rpc "$SOLANA_RPC_URL"; then
    echo "   Command failed, verifying on-chain state..."
fi
# RoutingConfig: disc=6 base64=Bg==, size=66
verify_solana_has_account "$SOLANA_GMP_ID" "$SOLANA_RPC_URL" "Bg==" 66 \
    "GMP routing config"

# 5. Add GMP relay authorization (optional)
if [ -n "$INTEGRATED_GMP_SVM_ADDR" ]; then
    echo " 5. Adding GMP relay: $INTEGRATED_GMP_SVM_ADDR"
    if ! "$CLI_BIN" gmp-add-relay \
        --gmp-program-id "$SOLANA_GMP_ID" \
        --payer "$DEPLOYER_KEYPAIR" \
        --relay "$INTEGRATED_GMP_SVM_ADDR" \
        --rpc "$SOLANA_RPC_URL"; then
        echo "   Command failed, verifying on-chain state..."
    fi
    # RelayAccount: disc=2 base64=Ag==, size=35
    verify_solana_has_account "$SOLANA_GMP_ID" "$SOLANA_RPC_URL" "Ag==" 35 \
        "GMP relay authorization for $INTEGRATED_GMP_SVM_ADDR"
fi

# Clean up
rm -rf "$TEMP_KEYPAIR_DIR"

echo ""
echo " Solana configuration verified."
