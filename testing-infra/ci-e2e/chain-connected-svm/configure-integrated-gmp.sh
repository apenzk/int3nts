#!/bin/bash

# Configure Integrated-GMP for Connected SVM Chain
#
# This script adds the [connected_chain_svm] section to integrated-gmp-e2e-ci-testing.toml.
# Must be called AFTER chain-hub/configure-integrated-gmp.sh which creates the base config.

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"

setup_project_root
setup_logging "configure-integrated-gmp-connected-svm"
cd "$PROJECT_ROOT"

log_and_echo "   Configuring integrated-gmp for Connected SVM Chain..."
log_and_echo ""

CHAIN_INFO="$PROJECT_ROOT/.tmp/chain-info.env"
if [ -f "$CHAIN_INFO" ]; then
    source "$CHAIN_INFO"
fi

if [ -z "$SVM_PROGRAM_ID" ]; then
    log_and_echo "   ERROR: SVM_PROGRAM_ID not found. Run chain-connected-svm/deploy-contracts.sh first."
    exit 1
fi

if [ -z "$SVM_OUTFLOW_VALIDATOR_ID" ]; then
    log_and_echo "   ERROR: SVM_OUTFLOW_VALIDATOR_ID not found. Run chain-connected-svm/deploy-contracts.sh first."
    exit 1
fi

INTEGRATED_GMP_E2E_CI_TESTING_CONFIG="$PROJECT_ROOT/integrated-gmp/config/integrated-gmp-e2e-ci-testing.toml"
if [ ! -f "$INTEGRATED_GMP_E2E_CI_TESTING_CONFIG" ]; then
    log_and_echo "   ERROR: Config file not found. Run chain-hub/configure-integrated-gmp.sh first."
    exit 1
fi

TEMP_FILE=$(mktemp)
cat > "$TEMP_FILE" << EOF

[connected_chain_svm]
name = "Connected SVM Chain"
rpc_url = "http://127.0.0.1:8899"
escrow_program_id = "$SVM_PROGRAM_ID"
outflow_program_id = "$SVM_OUTFLOW_VALIDATOR_ID"
chain_id = 901
gmp_endpoint_program_id = "$SVM_GMP_ENDPOINT_ID"
EOF

awk -v svm_section="$(cat $TEMP_FILE)" '
/^\[integrated_gmp\]/ { print svm_section; print ""; }
{ print }
' "$INTEGRATED_GMP_E2E_CI_TESTING_CONFIG" > "${INTEGRATED_GMP_E2E_CI_TESTING_CONFIG}.tmp"
mv "${INTEGRATED_GMP_E2E_CI_TESTING_CONFIG}.tmp" "$INTEGRATED_GMP_E2E_CI_TESTING_CONFIG"

rm -f "$TEMP_FILE"

export INTEGRATED_GMP_CONFIG_PATH="$INTEGRATED_GMP_E2E_CI_TESTING_CONFIG"

log_and_echo "   Added Connected SVM Chain section to integrated-gmp config"
log_and_echo "   Escrow program ID: $SVM_PROGRAM_ID"
log_and_echo "   Outflow validator ID: $SVM_OUTFLOW_VALIDATOR_ID"
log_and_echo "   GMP endpoint program ID: $SVM_GMP_ENDPOINT_ID"
log_and_echo ""
