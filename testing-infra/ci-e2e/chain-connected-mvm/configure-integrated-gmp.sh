#!/bin/bash

# Configure Integrated-GMP for Connected Move VM Chain
#
# This script adds a [[connected_chain_mvm]] entry to integrated-gmp-e2e-ci-testing.toml.
# Must be called AFTER chain-hub/configure-integrated-gmp.sh which creates the base config.
# Accepts instance number as argument (default: 2).

set -e

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/utils.sh"

# Setup project root and logging
setup_project_root

# Accept instance number as argument (default: 2)
mvm_instance_vars "${1:-2}"

setup_logging "configure-integrated-gmp-connected-mvm${MVM_INSTANCE}"
cd "$PROJECT_ROOT"

log_and_echo "   Configuring integrated-gmp for Connected Move VM Chain (instance $MVM_INSTANCE)..."
log_and_echo ""

# Extract deployed address from aptos profile
CHAIN_ADDR=$(get_profile_address "intent-account-chain${MVM_INSTANCE}")

if [ -z "$CHAIN_ADDR" ]; then
    log_and_echo "   ERROR: Could not extract Chain $MVM_INSTANCE deployed module address"
    exit 1
fi

log_and_echo "   Chain $MVM_INSTANCE deployer: $CHAIN_ADDR"

# Config file path (created by chain-hub/configure-integrated-gmp.sh)
INTEGRATED_GMP_E2E_CI_TESTING_CONFIG="$PROJECT_ROOT/integrated-gmp/config/integrated-gmp-e2e-ci-testing.toml"

if [ ! -f "$INTEGRATED_GMP_E2E_CI_TESTING_CONFIG" ]; then
    log_and_echo "   ERROR: Config file not found. Run chain-hub/configure-integrated-gmp.sh first."
    exit 1
fi

# Append connected_chain_mvm entry to config (insert before [integrated_gmp] section)
TEMP_FILE=$(mktemp)
cat > "$TEMP_FILE" << EOF

[[connected_chain_mvm]]
name = "Connected Move VM Chain $MVM_INSTANCE"
rpc_url = "http://127.0.0.1:$MVM_REST_PORT"
chain_id = $MVM_CHAIN_ID
intent_module_addr = "0x$CHAIN_ADDR"
escrow_module_addr = "0x$CHAIN_ADDR"
EOF

# Insert the MVM section before [integrated_gmp] section
awk -v mvm_section="$(cat $TEMP_FILE)" '
/^\[integrated_gmp\]/ { print mvm_section; print ""; }
{ print }
' "$INTEGRATED_GMP_E2E_CI_TESTING_CONFIG" > "${INTEGRATED_GMP_E2E_CI_TESTING_CONFIG}.tmp"
mv "${INTEGRATED_GMP_E2E_CI_TESTING_CONFIG}.tmp" "$INTEGRATED_GMP_E2E_CI_TESTING_CONFIG"

rm -f "$TEMP_FILE"

export INTEGRATED_GMP_CONFIG_PATH="$INTEGRATED_GMP_E2E_CI_TESTING_CONFIG"

log_and_echo "   Added Connected Move VM Chain $MVM_INSTANCE section to integrated-gmp config"
log_and_echo ""
