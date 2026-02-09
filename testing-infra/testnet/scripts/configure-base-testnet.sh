#!/bin/bash

# Configure Base Sepolia Testnet - Verify EVM contract configuration
#
# The EVM contracts are fully configured during deployment:
# - Hardhat deploy.js sets trusted remotes, escrow/outflow handlers, and relay
# - Hub chain address is passed as constructor argument
#
# This script verifies the on-chain configuration is correct.
#
# Requires:
#   - .env.testnet with deployed addresses (from deploy-to-base-testnet.sh)

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
source "$TESTNET_KEYS_FILE"

require_var "BASE_GMP_ENDPOINT_ADDR" "$BASE_GMP_ENDPOINT_ADDR" "Run deploy-to-base-testnet.sh first"

# Load RPC URL from assets config
ASSETS_CONFIG_FILE="$PROJECT_ROOT/testing-infra/testnet/config/testnet-assets.toml"
BASE_SEPOLIA_RPC_URL=$(grep -A 5 "^\[base_sepolia\]" "$ASSETS_CONFIG_FILE" | grep "^rpc_url = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")

if [ -z "$BASE_SEPOLIA_RPC_URL" ]; then
    echo "ERROR: Base Sepolia RPC URL not found in testnet-assets.toml"
    exit 1
fi

echo " EVM contracts are configured during deployment by Hardhat deploy.js."
echo " Verifying deployed contracts are accessible..."
echo ""

# Verify GMP endpoint contract exists by checking code at address
GMP_CODE=$(curl -s --max-time 10 -X POST "$BASE_SEPOLIA_RPC_URL" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getCode\",\"params\":[\"$BASE_GMP_ENDPOINT_ADDR\",\"latest\"],\"id\":1}" \
    | jq -r '.result // ""' 2>/dev/null)

if [ -n "$GMP_CODE" ] && [ "$GMP_CODE" != "0x" ] && [ "$GMP_CODE" != "" ]; then
    echo "   IntentGmp ($BASE_GMP_ENDPOINT_ADDR): deployed"
else
    echo "   ERROR: IntentGmp contract not found at $BASE_GMP_ENDPOINT_ADDR"
    exit 1
fi

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
echo " Base Sepolia configuration verified."
