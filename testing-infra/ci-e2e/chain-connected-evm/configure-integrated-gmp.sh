#!/bin/bash

# Configure Integrated-GMP for Connected EVM Chain
#
# This script adds a [[connected_chain_evm]] entry to integrated-gmp-e2e-ci-testing.toml.
# Must be called AFTER chain-hub/configure-integrated-gmp.sh which creates the base config.
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

setup_logging "configure-integrated-gmp-connected-evm${EVM_INSTANCE}"
cd "$PROJECT_ROOT"

log_and_echo "   Configuring integrated-gmp for Connected EVM Chain (instance $EVM_INSTANCE)..."
log_and_echo ""

# Get EVM contract addresses
CONTRACT_ADDR=$(extract_escrow_contract_address)
log_and_echo "   EVM Escrow Contract: $CONTRACT_ADDR"

GMP_ENDPOINT="${GMP_ENDPOINT_ADDR:-}"
OUTFLOW_VALIDATOR="${OUTFLOW_VALIDATOR_ADDR:-}"
log_and_echo "   EVM GMP Endpoint: $GMP_ENDPOINT"
log_and_echo "   EVM Outflow Validator: $OUTFLOW_VALIDATOR"

# Use the relay's actual ECDSA-derived EVM address (saved by deploy-contracts.sh)
APPROVER_ADDR="${RELAY_ETH_ADDRESS:-}"
if [ -z "$APPROVER_ADDR" ]; then
    log_and_echo "   ERROR: RELAY_ETH_ADDRESS not found in chain-info-evm${EVM_INSTANCE}.env. Run deploy-contracts.sh $EVM_INSTANCE first."
    exit 1
fi
log_and_echo "   EVM Approver (relay): $APPROVER_ADDR"

# Config file path (created by chain-hub/configure-integrated-gmp.sh)
INTEGRATED_GMP_E2E_CI_TESTING_CONFIG="$PROJECT_ROOT/integrated-gmp/config/integrated-gmp-e2e-ci-testing.toml"

if [ ! -f "$INTEGRATED_GMP_E2E_CI_TESTING_CONFIG" ]; then
    log_and_echo "   ERROR: Config file not found. Run chain-hub/configure-integrated-gmp.sh first."
    exit 1
fi

# Append connected_chain_evm entry to config (insert before [integrated_gmp] section)
TEMP_FILE=$(mktemp)
cat > "$TEMP_FILE" << EOF

[[connected_chain_evm]]
name = "Connected EVM Chain $EVM_INSTANCE"
rpc_url = "$EVM_RPC_URL"
escrow_contract_addr = "$CONTRACT_ADDR"
chain_id = $EVM_CHAIN_ID
approver_evm_pubkey_hash = "$APPROVER_ADDR"
gmp_endpoint_addr = "$GMP_ENDPOINT"
outflow_validator_addr = "$OUTFLOW_VALIDATOR"
EOF

# Insert the EVM section before [integrated_gmp] section
awk -v evm_section="$(cat $TEMP_FILE)" '
/^\[integrated_gmp\]/ { print evm_section; print ""; }
{ print }
' "$INTEGRATED_GMP_E2E_CI_TESTING_CONFIG" > "${INTEGRATED_GMP_E2E_CI_TESTING_CONFIG}.tmp"
mv "${INTEGRATED_GMP_E2E_CI_TESTING_CONFIG}.tmp" "$INTEGRATED_GMP_E2E_CI_TESTING_CONFIG"

rm -f "$TEMP_FILE"

export INTEGRATED_GMP_CONFIG_PATH="$INTEGRATED_GMP_E2E_CI_TESTING_CONFIG"

log_and_echo "   Added Connected EVM Chain $EVM_INSTANCE section to integrated-gmp config"
log_and_echo ""
