#!/bin/bash

# Configure Base Sepolia Testnet - Set GMP trusted remote and verify contracts
#
# Steps:
#   1. Verify all 3 contracts are deployed on-chain
#   2. Set trusted remote on IntentGmp for hub chain (Movement)
#
# Requires:
#   - .env.testnet with:
#     - BASE_DEPLOYER_PRIVATE_KEY
#     - BASE_GMP_ENDPOINT_ADDR, BASE_INFLOW_ESCROW_ADDR, BASE_OUTFLOW_VALIDATOR_ADDR
#     - MOVEMENT_INTENT_MODULE_ADDR
#   - Node.js + Hardhat (for contract interaction)

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../.." && pwd )"

source "$SCRIPT_DIR/../lib/env-utils.sh"

echo " Configuring Base Sepolia Testnet"
echo "=================================="
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

require_var "BASE_DEPLOYER_PRIVATE_KEY" "$BASE_DEPLOYER_PRIVATE_KEY"
require_var "BASE_GMP_ENDPOINT_ADDR" "$BASE_GMP_ENDPOINT_ADDR" "Run deploy-to-base-testnet.sh first"
require_var "MOVEMENT_INTENT_MODULE_ADDR" "$MOVEMENT_INTENT_MODULE_ADDR" "Run deploy-to-movement-testnet.sh first"

require_var "ALCHEMY_BASE_SEPOLIA_API_KEY" "$ALCHEMY_BASE_SEPOLIA_API_KEY" "Get your free API key at: https://www.alchemy.com/"
BASE_SEPOLIA_RPC_URL="https://base-sepolia.g.alchemy.com/v2/${ALCHEMY_BASE_SEPOLIA_API_KEY}"

HUB_CHAIN_ID="${HUB_CHAIN_ID:-250}"

echo " Configuration:"
echo "   GMP Endpoint:  $BASE_GMP_ENDPOINT_ADDR"
echo "   Hub Chain ID:  $HUB_CHAIN_ID"
echo "   Hub Module:    $MOVEMENT_INTENT_MODULE_ADDR"
echo ""

# 1. Verify contracts are deployed
echo " 1. Verifying deployed contracts..."

GMP_CODE=$(curl -s --max-time 10 -X POST "$BASE_SEPOLIA_RPC_URL" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getCode\",\"params\":[\"$BASE_GMP_ENDPOINT_ADDR\",\"latest\"],\"id\":1}" \
    | jq -r '.result // ""' 2>/dev/null)

if [ -z "$GMP_CODE" ] || [ "$GMP_CODE" = "0x" ] || [ "$GMP_CODE" = "" ]; then
    echo "FATAL: IntentGmp contract not found at $BASE_GMP_ENDPOINT_ADDR"
    exit 1
fi
echo "   IntentGmp ($BASE_GMP_ENDPOINT_ADDR): deployed"

require_var "BASE_INFLOW_ESCROW_ADDR" "$BASE_INFLOW_ESCROW_ADDR" "Run deploy-to-base-testnet.sh first"
ESCROW_CODE=$(curl -s --max-time 10 -X POST "$BASE_SEPOLIA_RPC_URL" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getCode\",\"params\":[\"$BASE_INFLOW_ESCROW_ADDR\",\"latest\"],\"id\":1}" \
    | jq -r '.result // ""' 2>/dev/null)

if [ -z "$ESCROW_CODE" ] || [ "$ESCROW_CODE" = "0x" ] || [ "$ESCROW_CODE" = "" ]; then
    echo "FATAL: IntentInflowEscrow contract not found at $BASE_INFLOW_ESCROW_ADDR"
    exit 1
fi
echo "   IntentInflowEscrow ($BASE_INFLOW_ESCROW_ADDR): deployed"

require_var "BASE_OUTFLOW_VALIDATOR_ADDR" "$BASE_OUTFLOW_VALIDATOR_ADDR" "Run deploy-to-base-testnet.sh first"
OUTFLOW_CODE=$(curl -s --max-time 10 -X POST "$BASE_SEPOLIA_RPC_URL" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getCode\",\"params\":[\"$BASE_OUTFLOW_VALIDATOR_ADDR\",\"latest\"],\"id\":1}" \
    | jq -r '.result // ""' 2>/dev/null)

if [ -z "$OUTFLOW_CODE" ] || [ "$OUTFLOW_CODE" = "0x" ] || [ "$OUTFLOW_CODE" = "" ]; then
    echo "FATAL: IntentOutflowValidator contract not found at $BASE_OUTFLOW_VALIDATOR_ADDR"
    exit 1
fi
echo "   IntentOutflowValidator ($BASE_OUTFLOW_VALIDATOR_ADDR): deployed"
echo ""

# 2. Set trusted remote on GMP endpoint
echo " 2. Setting trusted remote for hub chain $HUB_CHAIN_ID..."

cd "$PROJECT_ROOT/intent-frameworks/evm"

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo "   Installing dependencies..."
    npm install
fi

export DEPLOYER_PRIVATE_KEY="$BASE_DEPLOYER_PRIVATE_KEY"
export BASE_SEPOLIA_RPC_URL
export GMP_ENDPOINT_ADDR="$BASE_GMP_ENDPOINT_ADDR"
export HUB_CHAIN_ID
export MOVEMENT_INTENT_MODULE_ADDR

set +e
CONFIGURE_OUTPUT=$(npx hardhat run scripts/configure-gmp.js --network baseSepolia 2>&1)
CONFIGURE_EXIT=$?
set -e

echo "$CONFIGURE_OUTPUT"

if [ $CONFIGURE_EXIT -ne 0 ]; then
    echo "FATAL: Failed to set trusted remote on IntentGmp"
    exit 1
fi

echo ""
echo " Base Sepolia configuration verified."
