#!/bin/bash

# Configure Base Sepolia Testnet - Set remote GMP endpoint and update hub config
#
# Requires:
#   - .env.testnet with:
#     - BASE_DEPLOYER_PRIVATE_KEY
#     - BASE_GMP_ENDPOINT_ADDR, BASE_INFLOW_ESCROW_ADDR, BASE_OUTFLOW_VALIDATOR_ADDR
#     - MOVEMENT_INTENT_MODULE_ADDR
#   - Node.js + Hardhat (for contract interaction)

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../../.." && pwd )"

source "$SCRIPT_DIR/../lib/env-utils.sh"

# Load .env.testnet
load_env_file "$SCRIPT_DIR/../.env.testnet"

require_var "ALCHEMY_BASE_SEPOLIA_API_KEY" "$ALCHEMY_BASE_SEPOLIA_API_KEY" "Get your free API key at: https://www.alchemy.com/"

EVM_CHAIN_PREFIX="BASE"
EVM_RPC_URL="https://base-sepolia.g.alchemy.com/v2/${ALCHEMY_BASE_SEPOLIA_API_KEY}"
export BASE_SEPOLIA_RPC_URL="$EVM_RPC_URL"
EVM_HARDHAT_NETWORK="baseSepolia"
EVM_DISPLAY_NAME="Base Sepolia Testnet"
EVM_HUB_CHAIN_ID=$(get_chain_id "movement_bardock_testnet" "$TESTNET_ASSETS_CONFIG")
EVM_DEPLOY_SCRIPT="deploy-to-base.sh"

source "$SCRIPT_DIR/../../common/scripts/configure-evm.sh"
