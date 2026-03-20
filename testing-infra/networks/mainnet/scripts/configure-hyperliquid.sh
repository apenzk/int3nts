#!/bin/bash

# Configure HyperEVM Mainnet - Set remote GMP endpoint and update hub config
#
# Requires:
#   - .env.mainnet with:
#     - HYPERLIQUID_DEPLOYER_PRIVATE_KEY
#     - HYPERLIQUID_GMP_ENDPOINT_ADDR, HYPERLIQUID_INFLOW_ESCROW_ADDR, HYPERLIQUID_OUTFLOW_VALIDATOR_ADDR
#     - MOVEMENT_INTENT_MODULE_ADDR
#   - Node.js + Hardhat (for contract interaction)

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../../.." && pwd )"

source "$SCRIPT_DIR/../lib/env-utils.sh"

# Load .env.mainnet
load_env_file "$SCRIPT_DIR/../.env.mainnet"

require_var "HYPERLIQUID_RPC_URL" "$HYPERLIQUID_RPC_URL"

EVM_CHAIN_PREFIX="HYPERLIQUID"
EVM_RPC_URL="$HYPERLIQUID_RPC_URL"
export HYPERLIQUID_RPC_URL
EVM_HARDHAT_NETWORK="hyperliquidMainnet"
EVM_DISPLAY_NAME="HyperEVM Mainnet"
EVM_HUB_CHAIN_ID=$(get_chain_id "movement_mainnet" "$MAINNET_ASSETS_CONFIG")
EVM_DEPLOY_SCRIPT="deploy-to-hyperliquid.sh"

source "$SCRIPT_DIR/../../common/scripts/configure-evm.sh"
