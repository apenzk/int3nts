#!/bin/bash

# Configure Solana Devnet - Set up cross-chain GMP routing
#
# Must be run AFTER all deployments are complete.
#
# Requires:
#   - .env.testnet with:
#     - SOLANA_DEPLOYER_PRIVATE_KEY
#     - MOVEMENT_INTENT_MODULE_ADDR (from deploy-to-movement.sh)
#     - SOLANA_GMP_ID, SOLANA_PROGRAM_ID, SOLANA_OUTFLOW_ID (from deploy-to-solana.sh)
#     - INTEGRATED_GMP_SVM_ADDR (for relay authorization)
#   - Node.js, Solana CLI

set -e

CALLER_SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SCRIPT_DIR="$CALLER_SCRIPT_DIR"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../../.." && pwd )"

# Re-exec inside nix develop if not already in a nix shell
if [ -z "${IN_NIX_SHELL:-}" ]; then
    exec nix develop "$PROJECT_ROOT/nix" --command bash "$SCRIPT_DIR/configure-solana.sh" "$@"
fi

source "$SCRIPT_DIR/../lib/env-utils.sh"
source "$SCRIPT_DIR/../../common/lib/solana-utils.sh"

# Load .env.testnet
load_env_file "$SCRIPT_DIR/../.env.testnet"

SVM_RPC_URL="${SOLANA_RPC_URL:-https://api.devnet.solana.com}"
SVM_DISPLAY_NAME="Solana Devnet"
SVM_HUB_CHAIN_ID=$(get_chain_id "movement_bardock_testnet" "$TESTNET_ASSETS_CONFIG")
SVM_DEPLOY_SCRIPT="deploy-to-solana.sh"
SVM_MVM_DEPLOY_SCRIPT="deploy-to-movement.sh"

source "$SCRIPT_DIR/../../common/scripts/configure-solana.sh"
