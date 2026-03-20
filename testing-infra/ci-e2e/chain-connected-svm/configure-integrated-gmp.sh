#!/bin/bash

# Configure Integrated-GMP for Connected SVM Chain
#
# This script adds a [[connected_chain_svm]] entry to integrated-gmp-e2e-ci-testing.toml.
# Must be called AFTER chain-hub/configure-integrated-gmp.sh which creates the base config.
# Accepts instance number as argument (default: 2).

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/utils.sh"

setup_project_root

# Accept instance number as argument
svm_instance_vars "${1:-2}"
source "$SVM_CHAIN_INFO_FILE" 2>/dev/null || true

setup_logging "configure-integrated-gmp-connected-svm${SVM_INSTANCE}"
cd "$PROJECT_ROOT"

log_and_echo "   Configuring integrated-gmp for Connected SVM Chain (instance $SVM_INSTANCE)..."
log_and_echo ""

if [ -z "$SVM_PROGRAM_ID" ]; then
    log_and_echo "   ERROR: SVM_PROGRAM_ID not found. Run chain-connected-svm/deploy-contracts.sh $SVM_INSTANCE first."
    exit 1
fi

if [ -z "$SVM_OUTFLOW_VALIDATOR_ID" ]; then
    log_and_echo "   ERROR: SVM_OUTFLOW_VALIDATOR_ID not found. Run chain-connected-svm/deploy-contracts.sh $SVM_INSTANCE first."
    exit 1
fi

INTEGRATED_GMP_E2E_CI_TESTING_CONFIG="$PROJECT_ROOT/integrated-gmp/config/integrated-gmp-e2e-ci-testing.toml"
if [ ! -f "$INTEGRATED_GMP_E2E_CI_TESTING_CONFIG" ]; then
    log_and_echo "   ERROR: Config file not found. Run chain-hub/configure-integrated-gmp.sh first."
    exit 1
fi

TEMP_FILE=$(mktemp)
cat > "$TEMP_FILE" << EOF

[[connected_chain_svm]]
name = "Connected SVM Chain $SVM_INSTANCE"
rpc_url = "$SVM_RPC_URL"
escrow_program_id = "$SVM_PROGRAM_ID"
outflow_program_id = "$SVM_OUTFLOW_VALIDATOR_ID"
chain_id = $SVM_CHAIN_ID
gmp_endpoint_program_id = "$SVM_GMP_ENDPOINT_ID"
EOF

awk -v svm_section="$(cat $TEMP_FILE)" '
/^\[integrated_gmp\]/ { print svm_section; print ""; }
{ print }
' "$INTEGRATED_GMP_E2E_CI_TESTING_CONFIG" > "${INTEGRATED_GMP_E2E_CI_TESTING_CONFIG}.tmp"
mv "${INTEGRATED_GMP_E2E_CI_TESTING_CONFIG}.tmp" "$INTEGRATED_GMP_E2E_CI_TESTING_CONFIG"

rm -f "$TEMP_FILE"

export INTEGRATED_GMP_CONFIG_PATH="$INTEGRATED_GMP_E2E_CI_TESTING_CONFIG"

log_and_echo "   Added Connected SVM Chain $SVM_INSTANCE section to integrated-gmp config"
log_and_echo "   Escrow program ID: $SVM_PROGRAM_ID"
log_and_echo "   Outflow validator ID: $SVM_OUTFLOW_VALIDATOR_ID"
log_and_echo "   GMP endpoint program ID: $SVM_GMP_ENDPOINT_ID"
log_and_echo ""
