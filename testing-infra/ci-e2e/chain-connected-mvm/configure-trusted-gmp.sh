#!/bin/bash

# Configure Trusted-GMP for Connected Move VM Chain
#
# This script adds the [connected_chain_mvm] section to trusted-gmp-e2e-ci-testing.toml.
# Must be called AFTER chain-hub/configure-trusted-gmp.sh which creates the base config.

set -e

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"

# Setup project root and logging
setup_project_root
setup_logging "configure-trusted-gmp-connected-mvm"
cd "$PROJECT_ROOT"

log_and_echo "   Configuring trusted-gmp for Connected Move VM Chain..."
log_and_echo ""

# Extract deployed address from aptos profile
CHAIN2_ADDR=$(get_profile_address "intent-account-chain2")

if [ -z "$CHAIN2_ADDR" ]; then
    log_and_echo "   ERROR: Could not extract Chain 2 deployed module address"
    exit 1
fi

log_and_echo "   Chain 2 deployer: $CHAIN2_ADDR"

# Config file path (created by chain-hub/configure-trusted-gmp.sh)
TRUSTED_GMP_E2E_CI_TESTING_CONFIG="$PROJECT_ROOT/trusted-gmp/config/trusted-gmp-e2e-ci-testing.toml"

if [ ! -f "$TRUSTED_GMP_E2E_CI_TESTING_CONFIG" ]; then
    log_and_echo "   ERROR: Config file not found. Run chain-hub/configure-trusted-gmp.sh first."
    exit 1
fi

# Append connected_chain_mvm section to config (insert before [trusted_gmp] section)
TEMP_FILE=$(mktemp)
cat > "$TEMP_FILE" << EOF

[connected_chain_mvm]
name = "Connected Move VM Chain"
rpc_url = "http://127.0.0.1:8082"
chain_id = 2
intent_module_addr = "0x$CHAIN2_ADDR"
escrow_module_addr = "0x$CHAIN2_ADDR"
EOF

# Insert the MVM section before [trusted_gmp] section
awk -v mvm_section="$(cat $TEMP_FILE)" '
/^\[trusted_gmp\]/ { print mvm_section; print ""; }
{ print }
' "$TRUSTED_GMP_E2E_CI_TESTING_CONFIG" > "${TRUSTED_GMP_E2E_CI_TESTING_CONFIG}.tmp"
mv "${TRUSTED_GMP_E2E_CI_TESTING_CONFIG}.tmp" "$TRUSTED_GMP_E2E_CI_TESTING_CONFIG"

rm -f "$TEMP_FILE"

export TRUSTED_GMP_CONFIG_PATH="$TRUSTED_GMP_E2E_CI_TESTING_CONFIG"

log_and_echo "   Added Connected Move VM Chain section to trusted-gmp config"
log_and_echo ""
