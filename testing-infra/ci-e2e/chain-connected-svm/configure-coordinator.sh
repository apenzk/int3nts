#!/bin/bash

# Configure Coordinator for Connected SVM Chain
#
# This script adds the [connected_chain_svm] section to coordinator-e2e-ci-testing.toml.
# Must be called AFTER chain-hub/configure-coordinator.sh which creates the base config.

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"

setup_project_root
setup_logging "configure-coordinator-connected-svm"
cd "$PROJECT_ROOT"

log_and_echo "   Configuring coordinator for Connected SVM Chain..."
log_and_echo ""

CHAIN_INFO="$PROJECT_ROOT/.tmp/chain-info.env"
if [ -f "$CHAIN_INFO" ]; then
    source "$CHAIN_INFO"
fi

if [ -z "$SVM_PROGRAM_ID" ]; then
    log_and_echo "   ERROR: SVM_PROGRAM_ID not found. Run chain-connected-svm/deploy-contract.sh first."
    exit 1
fi

COORDINATOR_E2E_CI_TESTING_CONFIG="$PROJECT_ROOT/coordinator/config/coordinator-e2e-ci-testing.toml"
if [ ! -f "$COORDINATOR_E2E_CI_TESTING_CONFIG" ]; then
    log_and_echo "   ERROR: Config file not found. Run chain-hub/configure-coordinator.sh first."
    exit 1
fi

TEMP_FILE=$(mktemp)
cat > "$TEMP_FILE" << EOF

[connected_chain_svm]
name = "Connected SVM Chain"
rpc_url = "http://127.0.0.1:8899"
escrow_program_id = "$SVM_PROGRAM_ID"
chain_id = 4
EOF

awk -v svm_section="$(cat $TEMP_FILE)" '
/^\[api\]/ { print svm_section; print ""; }
{ print }
' "$COORDINATOR_E2E_CI_TESTING_CONFIG" > "${COORDINATOR_E2E_CI_TESTING_CONFIG}.tmp"
mv "${COORDINATOR_E2E_CI_TESTING_CONFIG}.tmp" "$COORDINATOR_E2E_CI_TESTING_CONFIG"

rm -f "$TEMP_FILE"

export COORDINATOR_CONFIG_PATH="$COORDINATOR_E2E_CI_TESTING_CONFIG"

log_and_echo "   Added Connected SVM Chain section to coordinator config"
log_and_echo ""
