#!/bin/bash

# SVM-specific utilities for testing infrastructure scripts
# This file MUST be sourced AFTER util.sh
# Usage:
#   source "$(dirname "$0")/../util.sh"
#   source "$(dirname "$0")/utils.sh"
#
# Note: This file depends on functions from util.sh (log, log_and_echo, setup_project_root, etc.)

SCRIPT_DIR_SVM_UTILS="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR_SVM_UTILS/../util.sh"
source "$SCRIPT_DIR_SVM_UTILS/../util_svm.sh"

# Derive all instance-specific values from instance number.
# Sets: SVM_INSTANCE, SVM_PORT, SVM_FAUCET_PORT, SVM_GOSSIP_PORT,
#       SVM_DYNAMIC_PORT_RANGE, SVM_CHAIN_ID, SVM_RPC_URL, SVM_PID_FILE,
#       SVM_CHAIN_INFO_FILE, SVM_LEDGER_DIR, SVM_E2E_DIR
# Usage: svm_instance_vars <instance_number>
svm_instance_vars() {
    local n="${1:-1}"
    export SVM_INSTANCE="$n"
    # dynamic-port-range split so instances don't contend for gossip/TPU ports
    case "$n" in
        2) export SVM_PORT=2000; export SVM_FAUCET_PORT=2010; export SVM_GOSSIP_PORT=8000; export SVM_DYNAMIC_PORT_RANGE="8001-8499"; export SVM_CHAIN_ID=2 ;;
        3) export SVM_PORT=3000; export SVM_FAUCET_PORT=3010; export SVM_GOSSIP_PORT=8500; export SVM_DYNAMIC_PORT_RANGE="8501-8999"; export SVM_CHAIN_ID=3 ;;
        *) echo "Unknown SVM instance: $n" >&2; exit 1 ;;
    esac
    export SVM_RPC_URL="http://127.0.0.1:$SVM_PORT"
    if [ -z "$PROJECT_ROOT" ]; then
        setup_project_root
    fi
    export SVM_PID_FILE="$PROJECT_ROOT/.tmp/solana-test-validator-${n}.pid"
    export SVM_CHAIN_INFO_FILE="$PROJECT_ROOT/.tmp/chain-info-svm${n}.env"
    export SVM_LEDGER_DIR="$PROJECT_ROOT/.tmp/solana-test-validator-${n}"
    export SVM_E2E_DIR="$PROJECT_ROOT/.tmp/svm-e2e-${n}"
}

# Load chain info for a specific SVM instance.
# Sources the instance-specific chain-info file and sets SVM_INSTANCE vars.
# Usage: load_svm_chain_info [instance_number]
load_svm_chain_info() {
    local n="${1:-${SVM_INSTANCE:-2}}"
    svm_instance_vars "$n"
    if [ -f "$SVM_CHAIN_INFO_FILE" ]; then
        source "$SVM_CHAIN_INFO_FILE"
    fi
}
