#!/bin/bash

# Deploy Move Intent Framework to Movement Mainnet
#
# This script generates a FRESH address for each deployment to avoid
# backward-incompatible module update errors. Funds are transferred from
# the deployer account in .env.mainnet to the new module address.
#
# REQUIRES: Movement CLI (not aptos CLI)
# Reference: https://docs.movementnetwork.xyz/devs/movementcli

set -e

CALLER_SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SCRIPT_DIR="$CALLER_SCRIPT_DIR"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../../.." && pwd )"
export PROJECT_ROOT

source "$SCRIPT_DIR/../lib/env-utils.sh"

# Load .env.mainnet
load_env_file "$SCRIPT_DIR/../.env.mainnet"

MVM_RPC_URL="https://mainnet.movementnetwork.xyz/v1"
MVM_DISPLAY_NAME="Movement Mainnet"
MVM_NETWORK_LABEL="mainnet"
MVM_LOG_PREFIX="movement-mainnet"
MVM_PUBLISH_FLAGS=""
MVM_NEXT_STEPS="Deploy Base and HyperEVM contracts"
MVM_FRONTEND_INTENT_CONTRACT_ADDR_ENV_VAR="NEXT_PUBLIC_MOVEMENT_MAINNET_INTENT_CONTRACT_ADDRESS"

source "$SCRIPT_DIR/../../common/scripts/deploy-to-movement.sh"
