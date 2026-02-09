#!/bin/bash

# Configure Movement Bardock Testnet - Set GMP trusted remotes for connected chains
#
# This script sets up cross-chain GMP trusted remotes on the Movement hub.
# Must be run AFTER all chain deployments are complete, because it needs
# the GMP endpoint addresses from connected chains.
#
# Requires:
#   - Movement CLI
#   - .env.testnet with:
#     - MOVEMENT_INTENT_MODULE_ADDR (from deploy-to-movement-testnet.sh)
#     - MOVEMENT_MODULE_PRIVATE_KEY (from deploy-to-movement-testnet.sh)
#     - BASE_GMP_ENDPOINT_ADDR + BASE_CHAIN_ID (from deploy-to-base-testnet.sh)
#     - SOLANA_GMP_ID + SVM_CHAIN_ID (from deploy-to-solana-devnet.sh) [optional]

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../.." && pwd )"

source "$SCRIPT_DIR/../lib/env-utils.sh"

echo " Configuring Movement Bardock Testnet (GMP Trusted Remotes)"
echo "============================================================"
echo ""

# Check for movement CLI
if ! command -v movement &> /dev/null; then
    echo "ERROR: movement CLI not found"
    echo "   See deploy-to-movement-testnet.sh header for install instructions"
    exit 1
fi

# Load .env.testnet
TESTNET_KEYS_FILE="$SCRIPT_DIR/../.env.testnet"
if [ ! -f "$TESTNET_KEYS_FILE" ]; then
    echo "ERROR: .env.testnet not found at $TESTNET_KEYS_FILE"
    exit 1
fi
source "$TESTNET_KEYS_FILE"

require_var "MOVEMENT_INTENT_MODULE_ADDR" "$MOVEMENT_INTENT_MODULE_ADDR" "Run deploy-to-movement-testnet.sh first"
require_var "MOVEMENT_MODULE_PRIVATE_KEY" "$MOVEMENT_MODULE_PRIVATE_KEY" "Should have been saved by deploy-to-movement-testnet.sh"
require_var "BASE_GMP_ENDPOINT_ADDR" "$BASE_GMP_ENDPOINT_ADDR" "Run deploy-to-base-testnet.sh first"
require_var "BASE_CHAIN_ID" "$BASE_CHAIN_ID" "Run deploy-to-base-testnet.sh first"
require_var "SOLANA_GMP_ID" "$SOLANA_GMP_ID" "Run deploy-to-solana-devnet.sh first"

MODULE_ADDR="$MOVEMENT_INTENT_MODULE_ADDR"
MOVEMENT_RPC_URL="https://testnet.movementnetwork.xyz/v1"
SVM_CHAIN_ID="${SVM_CHAIN_ID:-4}"  # Solana devnet chain ID

# Create temporary Movement CLI profile with module admin key
TEMP_PROFILE="movement-configure-$$"
echo " Setting up admin profile..."
movement init --profile "$TEMP_PROFILE" \
  --network custom \
  --rest-url https://testnet.movementnetwork.xyz/v1 \
  --faucet-url https://faucet.movementnetwork.xyz/ \
  --private-key "$MOVEMENT_MODULE_PRIVATE_KEY" \
  --skip-faucet \
  --assume-yes 2>/dev/null

echo "   Module address: $MODULE_ADDR"
echo ""

# --- Base Sepolia (EVM) ---
echo " Setting trusted remote: Base Sepolia (chain $BASE_CHAIN_ID)..."

ADDR_PADDED=$(pad_address_32 "$BASE_GMP_ENDPOINT_ADDR")
echo "   Remote address: 0x$ADDR_PADDED"

movement move run \
  --profile "$TEMP_PROFILE" \
  --function-id "${MODULE_ADDR}::intent_gmp::set_trusted_remote" \
  --args "u32:$BASE_CHAIN_ID" "hex:${ADDR_PADDED}" \
  --assume-yes

verify_movement_view "$MOVEMENT_RPC_URL" \
    "${MODULE_ADDR}::intent_gmp::get_trusted_remote" \
    "[$BASE_CHAIN_ID]" \
    "intent_gmp trusted remote for Base (chain $BASE_CHAIN_ID)"

movement move run \
  --profile "$TEMP_PROFILE" \
  --function-id "${MODULE_ADDR}::intent_gmp_hub::set_trusted_remote" \
  --args "u32:$BASE_CHAIN_ID" "hex:${ADDR_PADDED}" \
  --assume-yes

verify_movement_view "$MOVEMENT_RPC_URL" \
    "${MODULE_ADDR}::intent_gmp_hub::get_trusted_remote" \
    "[$BASE_CHAIN_ID]" \
    "intent_gmp_hub trusted remote for Base (chain $BASE_CHAIN_ID)"

echo ""

# --- Solana Devnet (SVM) ---
echo " Setting trusted remote: Solana Devnet (chain $SVM_CHAIN_ID)..."

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
  --function-id "${MODULE_ADDR}::intent_gmp::set_trusted_remote" \
  --args "u32:$SVM_CHAIN_ID" "hex:${SOLANA_GMP_HEX}" \
  --assume-yes

verify_movement_view "$MOVEMENT_RPC_URL" \
    "${MODULE_ADDR}::intent_gmp::get_trusted_remote" \
    "[$SVM_CHAIN_ID]" \
    "intent_gmp trusted remote for Solana (chain $SVM_CHAIN_ID)"

movement move run \
  --profile "$TEMP_PROFILE" \
  --function-id "${MODULE_ADDR}::intent_gmp_hub::set_trusted_remote" \
  --args "u32:$SVM_CHAIN_ID" "hex:${SOLANA_GMP_HEX}" \
  --assume-yes

verify_movement_view "$MOVEMENT_RPC_URL" \
    "${MODULE_ADDR}::intent_gmp_hub::get_trusted_remote" \
    "[$SVM_CHAIN_ID]" \
    "intent_gmp_hub trusted remote for Solana (chain $SVM_CHAIN_ID)"

echo ""
echo " Movement configuration verified."
