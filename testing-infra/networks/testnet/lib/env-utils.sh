#!/bin/bash
# Utilities for testnet deployment and configuration scripts.
# Source this file: source "$(dirname "$0")/lib/env-utils.sh"

# Shared functions (update_env_var, pad_address_32, require_var, get_chain_id,
# verify_movement_view, run_solana_idempotent, verify_solana_has_account)
SHARED_LIB_DIR="$(dirname "${BASH_SOURCE[0]}")/../../common/lib"
ENV_FILE_NAME=".env.testnet"
source "$SHARED_LIB_DIR/env-utils.sh"

# Default assets config for testnet — callers can override by passing $2 to get_chain_id
TESTNET_ASSETS_CONFIG="$(dirname "${BASH_SOURCE[0]}")/../config/testnet-assets.toml"
