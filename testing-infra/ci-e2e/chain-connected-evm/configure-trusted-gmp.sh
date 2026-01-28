#!/bin/bash

# Configure Trusted-GMP for Connected EVM Chain
#
# This script adds the [connected_chain_evm] section to trusted-gmp-e2e-ci-testing.toml.
# Must be called AFTER chain-hub/configure-trusted-gmp.sh which creates the base config.

set -e

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/utils.sh"

# Setup project root and logging
setup_project_root
setup_logging "configure-trusted-gmp-connected-evm"
cd "$PROJECT_ROOT"

log_and_echo "   Configuring trusted-gmp for Connected EVM Chain..."
log_and_echo ""

# Get EVM escrow contract address (single contract, one escrow per intentId)
CONTRACT_ADDR=$(extract_escrow_contract_address)
log_and_echo "   EVM Escrow Contract: $CONTRACT_ADDR"

# Get trusted-gmp Ethereum address (Hardhat account 0; on-chain approver)
log "   - Getting trusted-gmp Ethereum address (Hardhat account 0)..."
APPROVER_ADDR=$(get_hardhat_account_address "0")
log_and_echo "   EVM Approver: $APPROVER_ADDR"

# Config file path (created by chain-hub/configure-trusted-gmp.sh)
TRUSTED_GMP_E2E_CI_TESTING_CONFIG="$PROJECT_ROOT/trusted-gmp/config/trusted-gmp-e2e-ci-testing.toml"

if [ ! -f "$TRUSTED_GMP_E2E_CI_TESTING_CONFIG" ]; then
    log_and_echo "   ERROR: Config file not found. Run chain-hub/configure-trusted-gmp.sh first."
    exit 1
fi

# Append connected_chain_evm section to config (insert before [trusted_gmp] section)
TEMP_FILE=$(mktemp)
cat > "$TEMP_FILE" << EOF

[connected_chain_evm]
name = "Connected EVM Chain"
rpc_url = "http://127.0.0.1:8545"
escrow_contract_addr = "$CONTRACT_ADDR"
chain_id = 3
approver_evm_pubkey_hash = "$APPROVER_ADDR"
EOF

# Insert the EVM section before [trusted_gmp] section
awk -v evm_section="$(cat $TEMP_FILE)" '
/^\[trusted_gmp\]/ { print evm_section; print ""; }
{ print }
' "$TRUSTED_GMP_E2E_CI_TESTING_CONFIG" > "${TRUSTED_GMP_E2E_CI_TESTING_CONFIG}.tmp"
mv "${TRUSTED_GMP_E2E_CI_TESTING_CONFIG}.tmp" "$TRUSTED_GMP_E2E_CI_TESTING_CONFIG"

rm -f "$TEMP_FILE"

export TRUSTED_GMP_CONFIG_PATH="$TRUSTED_GMP_E2E_CI_TESTING_CONFIG"

log_and_echo "   Added Connected EVM Chain section to trusted-gmp config"
log_and_echo ""
