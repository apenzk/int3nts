#!/bin/bash

# Configure Coordinator for Connected SVM Chain
#
# This script adds a [[connected_chain_svm]] entry to coordinator-e2e-ci-testing.toml.
# Must be called AFTER chain-hub/configure-coordinator.sh which creates the base config.
# Accepts instance number as argument (default: 2).

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/utils.sh"

setup_project_root

# Accept instance number as argument
svm_instance_vars "${1:-2}"
source "$SVM_CHAIN_INFO_FILE" 2>/dev/null || true

setup_logging "configure-coordinator-connected-svm${SVM_INSTANCE}"
cd "$PROJECT_ROOT"

log_and_echo "   Configuring coordinator for Connected SVM Chain (instance $SVM_INSTANCE)..."
log_and_echo ""

if [ -z "$SVM_PROGRAM_ID" ]; then
    log_and_echo "   ERROR: SVM_PROGRAM_ID not found. Run chain-connected-svm/deploy-contracts.sh $SVM_INSTANCE first."
    exit 1
fi

COORDINATOR_E2E_CI_TESTING_CONFIG="$PROJECT_ROOT/coordinator/config/coordinator-e2e-ci-testing.toml"
if [ ! -f "$COORDINATOR_E2E_CI_TESTING_CONFIG" ]; then
    log_and_echo "   ERROR: Config file not found. Run chain-hub/configure-coordinator.sh first."
    exit 1
fi

TEMP_FILE=$(mktemp)
cat > "$TEMP_FILE" << EOF

[[connected_chain_svm]]
name = "Connected SVM Chain $SVM_INSTANCE"
rpc_url = "$SVM_RPC_URL"
escrow_program_id = "$SVM_PROGRAM_ID"
chain_id = $SVM_CHAIN_ID
EOF

awk -v svm_section="$(cat $TEMP_FILE)" '
/^\[api\]/ { print svm_section; print ""; }
{ print }
' "$COORDINATOR_E2E_CI_TESTING_CONFIG" > "${COORDINATOR_E2E_CI_TESTING_CONFIG}.tmp"
mv "${COORDINATOR_E2E_CI_TESTING_CONFIG}.tmp" "$COORDINATOR_E2E_CI_TESTING_CONFIG"

rm -f "$TEMP_FILE"

export COORDINATOR_CONFIG_PATH="$COORDINATOR_E2E_CI_TESTING_CONFIG"

log_and_echo "   Added Connected SVM Chain $SVM_INSTANCE section to coordinator config"
log_and_echo ""
