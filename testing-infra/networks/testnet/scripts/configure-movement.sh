#!/bin/bash

# Configure Movement Bardock Testnet - Set remote GMP endpoints for connected chains
#
# This script sets up cross-chain remote GMP endpoints on the Movement hub.
# Must be run AFTER all chain deployments are complete, because it needs
# the GMP endpoint addresses from connected chains.
#
# Requires:
#   - Movement CLI
#   - .env.testnet with:
#     - MOVEMENT_INTENT_MODULE_ADDR (from deploy-to-movement.sh)
#     - MOVEMENT_MODULE_PRIVATE_KEY (from deploy-to-movement.sh)
#     - BASE_GMP_ENDPOINT_ADDR + BASE_CHAIN_ID (from deploy-to-base.sh)
#     - SOLANA_GMP_ID (from deploy-to-solana.sh) [optional]
#     - INTEGRATED_GMP_MVM_ADDR (from get_relay_addresses) [optional, for relay auth]

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../../.." && pwd )"

source "$SCRIPT_DIR/../lib/env-utils.sh"

echo " Configuring Movement Bardock Testnet (Remote GMP Endpoints)"
echo "============================================================"
echo ""

# Check for movement CLI
if ! command -v movement &> /dev/null; then
    echo "ERROR: movement CLI not found"
    echo "   See deploy-to-movement.sh header for install instructions"
    exit 1
fi

# Load .env.testnet
load_env_file "$SCRIPT_DIR/../.env.testnet"

require_var "MOVEMENT_INTENT_MODULE_ADDR" "$MOVEMENT_INTENT_MODULE_ADDR" "Run deploy-to-movement.sh first"
require_var "MOVEMENT_MODULE_PRIVATE_KEY" "$MOVEMENT_MODULE_PRIVATE_KEY" "Should have been saved by deploy-to-movement.sh"
require_var "BASE_GMP_ENDPOINT_ADDR" "$BASE_GMP_ENDPOINT_ADDR" "Run deploy-to-base.sh first"
require_var "BASE_CHAIN_ID" "$BASE_CHAIN_ID" "Run deploy-to-base.sh first"
require_var "SOLANA_GMP_ID" "$SOLANA_GMP_ID" "Run deploy-to-solana.sh first"

MODULE_ADDR="$MOVEMENT_INTENT_MODULE_ADDR"
MOVEMENT_RPC_URL="https://testnet.movementnetwork.xyz/v1"
SVM_CHAIN_ID=$(get_chain_id "solana_devnet" "$TESTNET_ASSETS_CONFIG")

# Create temporary Movement CLI profile with module admin key
TEMP_PROFILE="movement-configure-$$"
echo " Setting up admin profile..."
movement init --profile "$TEMP_PROFILE" \
  --network custom \
  --rest-url "$MOVEMENT_RPC_URL" \
  --private-key "$MOVEMENT_MODULE_PRIVATE_KEY" \
  --skip-faucet \
  --assume-yes 2>/dev/null

echo "   Module address: $MODULE_ADDR"
echo ""

# --- Base Sepolia (EVM) ---
echo " Setting remote GMP endpoint: Base Sepolia (chain $BASE_CHAIN_ID)..."

ADDR_PADDED=$(pad_address_32 "$BASE_GMP_ENDPOINT_ADDR")
echo "   Remote address: 0x$ADDR_PADDED"

movement move run \
  --profile "$TEMP_PROFILE" \
  --function-id "${MODULE_ADDR}::intent_gmp::set_remote_gmp_endpoint_addr" \
  --args "u32:$BASE_CHAIN_ID" "hex:${ADDR_PADDED}" \
  --assume-yes

verify_movement_view "$MOVEMENT_RPC_URL" \
    "${MODULE_ADDR}::intent_gmp::get_remote_gmp_endpoint_addrs" \
    "[$BASE_CHAIN_ID]" \
    "intent_gmp remote GMP endpoint for Base (chain $BASE_CHAIN_ID)"

movement move run \
  --profile "$TEMP_PROFILE" \
  --function-id "${MODULE_ADDR}::intent_gmp_hub::set_remote_gmp_endpoint_addr" \
  --args "u32:$BASE_CHAIN_ID" "hex:${ADDR_PADDED}" \
  --assume-yes

verify_movement_view "$MOVEMENT_RPC_URL" \
    "${MODULE_ADDR}::intent_gmp_hub::get_remote_gmp_endpoint_addr" \
    "[$BASE_CHAIN_ID]" \
    "intent_gmp_hub remote GMP endpoint for Base (chain $BASE_CHAIN_ID)"

echo ""

# --- Solana Devnet (SVM) ---
echo " Setting remote GMP endpoint: Solana Devnet (chain $SVM_CHAIN_ID)..."

# Convert base58 Solana program ID to 32-byte hex
SOLANA_GMP_HEX=$(node -e "
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
const bytes = b58decode('$SOLANA_GMP_ID');
console.log(Buffer.from(bytes).toString('hex').padStart(64, '0'));
")

if [ -z "$SOLANA_GMP_HEX" ]; then
    echo "ERROR: Failed to convert Solana GMP program ID to hex"
    exit 1
fi

echo "   Remote address: 0x$SOLANA_GMP_HEX"

movement move run \
  --profile "$TEMP_PROFILE" \
  --function-id "${MODULE_ADDR}::intent_gmp::set_remote_gmp_endpoint_addr" \
  --args "u32:$SVM_CHAIN_ID" "hex:${SOLANA_GMP_HEX}" \
  --assume-yes

verify_movement_view "$MOVEMENT_RPC_URL" \
    "${MODULE_ADDR}::intent_gmp::get_remote_gmp_endpoint_addrs" \
    "[$SVM_CHAIN_ID]" \
    "intent_gmp remote GMP endpoint for Solana (chain $SVM_CHAIN_ID)"

movement move run \
  --profile "$TEMP_PROFILE" \
  --function-id "${MODULE_ADDR}::intent_gmp_hub::set_remote_gmp_endpoint_addr" \
  --args "u32:$SVM_CHAIN_ID" "hex:${SOLANA_GMP_HEX}" \
  --assume-yes

verify_movement_view "$MOVEMENT_RPC_URL" \
    "${MODULE_ADDR}::intent_gmp_hub::get_remote_gmp_endpoint_addr" \
    "[$SVM_CHAIN_ID]" \
    "intent_gmp_hub remote GMP endpoint for Solana (chain $SVM_CHAIN_ID)"

echo ""

# --- Add GMP relay as authorized relay ---
if [ -n "$INTEGRATED_GMP_MVM_ADDR" ]; then
    echo " Adding GMP relay as authorized relay: $INTEGRATED_GMP_MVM_ADDR"
    movement move run \
      --profile "$TEMP_PROFILE" \
      --function-id "${MODULE_ADDR}::intent_gmp::add_relay" \
      --args "address:${INTEGRATED_GMP_MVM_ADDR}" \
      --assume-yes || echo "   (may already be added)"

    verify_movement_view "$MOVEMENT_RPC_URL" \
        "${MODULE_ADDR}::intent_gmp::is_relay_authorized" \
        "[\"${INTEGRATED_GMP_MVM_ADDR}\"]" \
        "intent_gmp relay authorization for GMP relay"
else
    echo " WARN: INTEGRATED_GMP_MVM_ADDR not set, skipping relay authorization"
    echo "   Set it in .env.testnet (derive with: cd integrated-gmp && cargo run --bin get_relay_addresses)"
fi

echo ""
echo " Movement configuration verified."
