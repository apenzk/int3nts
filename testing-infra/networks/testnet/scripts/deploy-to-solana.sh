#!/bin/bash

# Deploy SVM GMP Contracts to Solana Devnet
# Deploys all 3 programs: intent_inflow_escrow, intent_gmp, intent_outflow_validator
# Reads keys from .env.testnet and deploys the programs

set -e

CALLER_SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SCRIPT_DIR="$CALLER_SCRIPT_DIR"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../../.." && pwd )"
export PROJECT_ROOT

# Re-exec inside nix develop if not already in a nix shell
if [ -z "${IN_NIX_SHELL:-}" ]; then
    exec nix develop "$PROJECT_ROOT/nix" --command bash "$SCRIPT_DIR/deploy-to-solana.sh" "$@"
fi

source "$SCRIPT_DIR/../lib/env-utils.sh"
source "$SCRIPT_DIR/../../common/lib/solana-utils.sh"

# Load .env.testnet
load_env_file "$SCRIPT_DIR/../.env.testnet"

SVM_RPC_URL="${SOLANA_RPC_URL:-https://api.devnet.solana.com}"
SVM_DISPLAY_NAME="Solana Devnet"
SVM_NETWORK_LABEL="testnet"
SVM_LOG_PREFIX="solana-devnet"
SVM_HUB_CHAIN_ID=$(get_chain_id "movement_bardock_testnet" "$TESTNET_ASSETS_CONFIG")
SVM_CHAIN_ID=$(get_chain_id "solana_devnet" "$TESTNET_ASSETS_CONFIG")
SVM_CHECK_SCRIPT="./testing-infra/networks/testnet/check-preparedness.sh"
SVM_CONFIGURE_SCRIPT="configure-solana.sh"

source "$SCRIPT_DIR/../../common/scripts/deploy-to-solana.sh"
