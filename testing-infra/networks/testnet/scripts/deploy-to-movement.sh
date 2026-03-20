#!/bin/bash

# Deploy Move Intent Framework to Movement Bardock Testnet
#
# This script generates a FRESH address for each deployment to avoid
# backward-incompatible module update errors. Funds are transferred from
# the deployer account in .env.testnet to the new module address.
#
# REQUIRES: Movement CLI (not aptos CLI)
# Reference: https://docs.movementnetwork.xyz/devs/movementcli

set -e

CALLER_SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
SCRIPT_DIR="$CALLER_SCRIPT_DIR"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../../.." && pwd )"
export PROJECT_ROOT

source "$SCRIPT_DIR/../lib/env-utils.sh"

# Load .env.testnet
load_env_file "$SCRIPT_DIR/../.env.testnet"

MVM_RPC_URL="https://testnet.movementnetwork.xyz/v1"
MVM_DISPLAY_NAME="Movement Bardock Testnet"
MVM_NETWORK_LABEL="testnet"
MVM_LOG_PREFIX="movement-testnet"
MVM_PUBLISH_FLAGS="--dev"
MVM_NEXT_STEPS="Deploy Base and Solana contracts"
MVM_FRONTEND_INTENT_CONTRACT_ADDR_ENV_VAR="NEXT_PUBLIC_MOVEMENT_TESTNET_INTENT_CONTRACT_ADDRESS"

source "$SCRIPT_DIR/../../common/scripts/deploy-to-movement.sh"
