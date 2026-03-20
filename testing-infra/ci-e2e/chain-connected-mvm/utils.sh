#!/bin/bash

# MVM-specific utilities for multi-instance connected chain testing
# This file MUST be sourced AFTER util.sh and util_mvm.sh
# Usage:
#   source "$(dirname "$0")/../util.sh"
#   source "$(dirname "$0")/../util_mvm.sh"
#   source "$(dirname "$0")/utils.sh"

# Derive all instance-specific values from instance number.
# Sets: MVM_INSTANCE, MVM_REST_PORT, MVM_FAUCET_PORT, MVM_CHAIN_ID,
#       MVM_DOCKER_PROJECT, MVM_RPC_URL, MVM_CHAIN_INFO_FILE
# Usage: mvm_instance_vars <instance_number>
mvm_instance_vars() {
    local n="${1:-2}"
    export MVM_INSTANCE="$n"
    case "$n" in
        2) export MVM_REST_PORT=2000; export MVM_FAUCET_PORT=2010; export MVM_CHAIN_ID=2; export MVM_DOCKER_PROJECT=aptos-chain2 ;;
        3) export MVM_REST_PORT=3000; export MVM_FAUCET_PORT=3010; export MVM_CHAIN_ID=3; export MVM_DOCKER_PROJECT=aptos-chain3 ;;
        *) echo "Unknown MVM instance: $n" >&2; exit 1 ;;
    esac
    export MVM_RPC_URL="http://127.0.0.1:$MVM_REST_PORT/v1"
    if [ -z "$PROJECT_ROOT" ]; then
        setup_project_root
    fi
    export MVM_CHAIN_INFO_FILE="$PROJECT_ROOT/.tmp/chain-info-mvm${n}.env"
}

# Load chain info for a specific MVM instance.
# Sources the instance-specific chain-info file and sets MVM_INSTANCE vars.
# Usage: load_mvm_chain_info [instance_number]
load_mvm_chain_info() {
    local n="${1:-${MVM_INSTANCE:-2}}"
    mvm_instance_vars "$n"
    if [ -f "$MVM_CHAIN_INFO_FILE" ]; then
        source "$MVM_CHAIN_INFO_FILE"
    fi
}
