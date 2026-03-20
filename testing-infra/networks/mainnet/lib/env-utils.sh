#!/bin/bash
# Utilities for mainnet deployment and configuration scripts.
# Source this file: source "$(dirname "$0")/lib/env-utils.sh"

# Shared functions (update_env_var, pad_address_32, require_var, get_chain_id, verify_movement_view)
SHARED_LIB_DIR="$(dirname "${BASH_SOURCE[0]}")/../../common/lib"
ENV_FILE_NAME=".env.mainnet"
source "$SHARED_LIB_DIR/env-utils.sh"

# Default assets config for mainnet — callers can override by passing $2 to get_chain_id
MAINNET_ASSETS_CONFIG="$(dirname "${BASH_SOURCE[0]}")/../config/mainnet-assets.toml"
