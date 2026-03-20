#!/bin/bash

# Configure Base Mainnet - Set remote GMP endpoint and update hub config
#
# Requires:
#   - .env.mainnet with:
#     - BASE_DEPLOYER_PRIVATE_KEY
#     - BASE_GMP_ENDPOINT_ADDR, BASE_INFLOW_ESCROW_ADDR, BASE_OUTFLOW_VALIDATOR_ADDR
#     - MOVEMENT_INTENT_MODULE_ADDR
#   - Node.js + Hardhat (for contract interaction)

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../../.." && pwd )"

source "$SCRIPT_DIR/../lib/env-utils.sh"

# Load .env.mainnet
load_env_file "$SCRIPT_DIR/../.env.mainnet"

require_var "BASE_RPC_URL" "$BASE_RPC_URL"

EVM_CHAIN_PREFIX="BASE"
EVM_RPC_URL="$BASE_RPC_URL"
export BASE_RPC_URL
EVM_HARDHAT_NETWORK="baseMainnet"
EVM_DISPLAY_NAME="Base Mainnet"
EVM_HUB_CHAIN_ID=$(get_chain_id "movement_mainnet" "$MAINNET_ASSETS_CONFIG")
EVM_DEPLOY_SCRIPT="deploy-to-base.sh"

source "$SCRIPT_DIR/../../common/scripts/configure-evm.sh"
