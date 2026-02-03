#!/bin/bash

# Configure Trusted-GMP for Connected SVM Chain
#
# This script adds the [connected_chain_svm] section to trusted-gmp-e2e-ci-testing.toml.
# Must be called AFTER chain-hub/configure-trusted-gmp.sh which creates the base config.

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"

setup_project_root
setup_logging "configure-trusted-gmp-connected-svm"
cd "$PROJECT_ROOT"

log_and_echo "   Configuring trusted-gmp for Connected SVM Chain..."
log_and_echo ""

CHAIN_INFO="$PROJECT_ROOT/.tmp/chain-info.env"
if [ -f "$CHAIN_INFO" ]; then
    source "$CHAIN_INFO"
fi

if [ -z "$SVM_PROGRAM_ID" ]; then
    log_and_echo "   ERROR: SVM_PROGRAM_ID not found. Run chain-connected-svm/deploy-contract.sh first."
    exit 1
fi

# Use GMP endpoint ID if available, otherwise fall back to escrow program ID
# The native GMP relay uses this to know which program to monitor for MessageSent events
GMP_PROGRAM_ID="${SVM_GMP_ENDPOINT_ID:-$SVM_PROGRAM_ID}"

TRUSTED_GMP_E2E_CI_TESTING_CONFIG="$PROJECT_ROOT/trusted-gmp/config/trusted-gmp-e2e-ci-testing.toml"
if [ ! -f "$TRUSTED_GMP_E2E_CI_TESTING_CONFIG" ]; then
    log_and_echo "   ERROR: Config file not found. Run chain-hub/configure-trusted-gmp.sh first."
    exit 1
fi

TEMP_FILE=$(mktemp)
cat > "$TEMP_FILE" << EOF

[connected_chain_svm]
name = "Connected SVM Chain"
rpc_url = "http://127.0.0.1:8899"
escrow_program_id = "$GMP_PROGRAM_ID"
chain_id = 4
EOF

awk -v svm_section="$(cat $TEMP_FILE)" '
/^\[trusted_gmp\]/ { print svm_section; print ""; }
{ print }
' "$TRUSTED_GMP_E2E_CI_TESTING_CONFIG" > "${TRUSTED_GMP_E2E_CI_TESTING_CONFIG}.tmp"
mv "${TRUSTED_GMP_E2E_CI_TESTING_CONFIG}.tmp" "$TRUSTED_GMP_E2E_CI_TESTING_CONFIG"

rm -f "$TEMP_FILE"

export TRUSTED_GMP_CONFIG_PATH="$TRUSTED_GMP_E2E_CI_TESTING_CONFIG"

log_and_echo "   Added Connected SVM Chain section to trusted-gmp config"
log_and_echo "   GMP program ID: $GMP_PROGRAM_ID"
log_and_echo ""
