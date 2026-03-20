#!/bin/bash

# Deploy EVM Intent Contracts to Base Mainnet
# Deploys all 3 contracts: IntentGmp, IntentInflowEscrow, IntentOutflowValidator
# Reads keys from .env.mainnet and deploys/configures all contracts

set -e

# Get the script directory and project root
CALLER_SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SCRIPT_DIR="$CALLER_SCRIPT_DIR"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../../.." && pwd )"
export PROJECT_ROOT

# Source utilities from testing-infra (for CI testing infrastructure)
source "$PROJECT_ROOT/testing-infra/ci-e2e/util.sh" 2>/dev/null || true
source "$SCRIPT_DIR/../lib/env-utils.sh"

ASSETS_CONFIG_FILE="$SCRIPT_DIR/../config/mainnet-assets.toml"

# Load .env.mainnet
load_env_file "$SCRIPT_DIR/../.env.mainnet"

# Load assets configuration
if [ ! -f "$ASSETS_CONFIG_FILE" ]; then
    echo "ERROR: mainnet-assets.toml not found at $ASSETS_CONFIG_FILE"
    exit 1
fi

require_var "BASE_RPC_URL" "$BASE_RPC_URL"

EVM_CHAIN_PREFIX="BASE"
EVM_RPC_URL="$BASE_RPC_URL"
export BASE_RPC_URL
EVM_DEPLOYER_ADDR="$BASE_DEPLOYER_ADDR"
EVM_HARDHAT_NETWORK="baseMainnet"
EVM_DISPLAY_NAME="Base Mainnet"
EVM_HUB_CHAIN_ID=$(get_chain_id "movement_mainnet" "$ASSETS_CONFIG_FILE")
EVM_NETWORK_LABEL="mainnet"
EVM_CHAIN_LABEL="Base"
EVM_FRONTEND_ESCROW_CONTRACT_ADDR_ENV_VAR="NEXT_PUBLIC_BASE_MAINNET_ESCROW_CONTRACT_ADDRESS"
EVM_LOG_PREFIX="base-mainnet"
EVM_CHECK_SCRIPT="./testing-infra/networks/mainnet/check-preparedness.sh"

source "$SCRIPT_DIR/../../common/scripts/deploy-to-evm.sh"
