#!/bin/bash

# Configure Coordinator for Connected EVM Chain
#
# This script adds a [[connected_chain_evm]] entry to coordinator-e2e-ci-testing.toml.
# Must be called AFTER chain-hub/configure-coordinator.sh which creates the base config.
# Accepts instance number as argument (default: 1).

set -e

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/utils.sh"

# Setup project root and logging
setup_project_root

# Accept instance number as argument (default: 1)
evm_instance_vars "${1:-1}"
source "$EVM_CHAIN_INFO_FILE" 2>/dev/null || true

setup_logging "configure-coordinator-connected-evm${EVM_INSTANCE}"
cd "$PROJECT_ROOT"

log_and_echo "   Configuring coordinator for Connected EVM Chain (instance $EVM_INSTANCE)..."
log_and_echo ""

# Get EVM escrow contract address (single contract, one escrow per intentId)
CONTRACT_ADDR=$(extract_escrow_contract_address)
log_and_echo "   EVM Escrow Contract: $CONTRACT_ADDR"

# Config file path (created by chain-hub/configure-coordinator.sh)
COORDINATOR_E2E_CI_TESTING_CONFIG="$PROJECT_ROOT/coordinator/config/coordinator-e2e-ci-testing.toml"

if [ ! -f "$COORDINATOR_E2E_CI_TESTING_CONFIG" ]; then
    log_and_echo "   ERROR: Config file not found. Run chain-hub/configure-coordinator.sh first."
    exit 1
fi

# Append connected_chain_evm entry to config (insert before [api] section)
TEMP_FILE=$(mktemp)
cat > "$TEMP_FILE" << EOF

[[connected_chain_evm]]
name = "Connected EVM Chain $EVM_INSTANCE"
rpc_url = "$EVM_RPC_URL"
escrow_contract_addr = "$CONTRACT_ADDR"
chain_id = $EVM_CHAIN_ID
EOF

# Insert the EVM section before [api] section
awk -v evm_section="$(cat $TEMP_FILE)" '
/^\[api\]/ { print evm_section; print ""; }
{ print }
' "$COORDINATOR_E2E_CI_TESTING_CONFIG" > "${COORDINATOR_E2E_CI_TESTING_CONFIG}.tmp"
mv "${COORDINATOR_E2E_CI_TESTING_CONFIG}.tmp" "$COORDINATOR_E2E_CI_TESTING_CONFIG"

rm -f "$TEMP_FILE"

export COORDINATOR_CONFIG_PATH="$COORDINATOR_E2E_CI_TESTING_CONFIG"

log_and_echo "   Added Connected EVM Chain $EVM_INSTANCE section to coordinator config"
log_and_echo ""
