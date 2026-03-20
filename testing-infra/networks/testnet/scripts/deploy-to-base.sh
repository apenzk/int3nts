#!/bin/bash

# Deploy EVM Intent Contracts to Base Sepolia Testnet
# Deploys all 3 contracts: IntentGmp, IntentInflowEscrow, IntentOutflowValidator
# Reads keys from .env.testnet and deploys/configures all contracts

set -e

# Get the script directory and project root
CALLER_SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SCRIPT_DIR="$CALLER_SCRIPT_DIR"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../../.." && pwd )"
export PROJECT_ROOT

# Source utilities from testing-infra (for CI testing infrastructure)
source "$PROJECT_ROOT/testing-infra/ci-e2e/util.sh" 2>/dev/null || true
source "$SCRIPT_DIR/../lib/env-utils.sh"

# Load .env.testnet
load_env_file "$SCRIPT_DIR/../.env.testnet"

# Load assets configuration
ASSETS_CONFIG_FILE="$PROJECT_ROOT/testing-infra/networks/testnet/config/testnet-assets.toml"
if [ ! -f "$ASSETS_CONFIG_FILE" ]; then
    echo "ERROR: testnet-assets.toml not found at $ASSETS_CONFIG_FILE"
    exit 1
fi

require_var "ALCHEMY_BASE_SEPOLIA_API_KEY" "$ALCHEMY_BASE_SEPOLIA_API_KEY" "Get your free API key at: https://www.alchemy.com/"

EVM_CHAIN_PREFIX="BASE"
EVM_RPC_URL="https://base-sepolia.g.alchemy.com/v2/${ALCHEMY_BASE_SEPOLIA_API_KEY}"
export BASE_SEPOLIA_RPC_URL="$EVM_RPC_URL"
EVM_DEPLOYER_ADDR="$BASE_DEPLOYER_ADDR"
EVM_HARDHAT_NETWORK="baseSepolia"
EVM_DISPLAY_NAME="Base Sepolia Testnet"
EVM_HUB_CHAIN_ID=$(get_chain_id "movement_bardock_testnet" "$ASSETS_CONFIG_FILE")
EVM_NETWORK_LABEL="testnet"
EVM_CHAIN_LABEL="Base"
EVM_FRONTEND_ESCROW_CONTRACT_ADDR_ENV_VAR="NEXT_PUBLIC_BASE_TESTNET_ESCROW_CONTRACT_ADDRESS"
EVM_LOG_PREFIX="base-sepolia"
EVM_CHECK_SCRIPT="./testing-infra/networks/testnet/check-preparedness.sh"

source "$SCRIPT_DIR/../../common/scripts/deploy-to-evm.sh"
