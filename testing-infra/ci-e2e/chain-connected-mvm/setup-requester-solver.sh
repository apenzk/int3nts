#!/bin/bash

# Setup Connected Chain Test Requester/Solver Accounts
# Accepts instance number as argument (default: 2)

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

setup_logging "setup-requester-solver-connected-mvm${MVM_INSTANCE}"
cd "$PROJECT_ROOT"

# Expected funding amount in octas
# Note: aptos init funds accounts with 100_000_000, then we fund again with 100_000_000 = 200_000_000 total
EXPECTED_FUNDING_AMOUNT=200000000

log " Requester and Solver Account Setup - CONNECTED CHAIN (Chain $MVM_INSTANCE)"
log "==========================================================="
log_and_echo " All output logged to: $LOG_FILE"

log ""
log "% - - - - - - - - - - - SETUP - - - - - - - - - - - -"
log "% - - - - - - - - - - - - - - - - - - - - - - - - - - - - -"

# Create test accounts
log ""
log " Creating test accounts for Chain $MVM_INSTANCE..."

log "Creating requester-chain${MVM_INSTANCE} account for Chain $MVM_INSTANCE..."
init_aptos_profile "requester-chain${MVM_INSTANCE}" "$MVM_INSTANCE" "$LOG_FILE"

# Solver uses a shared key across all MVM connected chain instances so the solver
# address is the same on every chain (mirrors EVM where Hardhat's deterministic
# mnemonic gives the same account on all instances).
SOLVER_MVM_KEY_FILE="$PROJECT_ROOT/.tmp/solver-mvm-shared-key.hex"
SOLVER_KEY=""
if [ -f "$SOLVER_MVM_KEY_FILE" ]; then
    SOLVER_KEY=$(cat "$SOLVER_MVM_KEY_FILE")
    log "   Using shared solver key from $SOLVER_MVM_KEY_FILE"
fi
log "Creating solver-chain${MVM_INSTANCE} account for Chain $MVM_INSTANCE..."
init_aptos_profile "solver-chain${MVM_INSTANCE}" "$MVM_INSTANCE" "$LOG_FILE" "$SOLVER_KEY"

log "Creating test-tokens-chain${MVM_INSTANCE} account for Chain $MVM_INSTANCE..."
init_aptos_profile "test-tokens-chain${MVM_INSTANCE}" "$MVM_INSTANCE" "$LOG_FILE"

log ""
log "% - - - - - - - - - - - FUNDING - - - - - - - - - - - -"
log "% - - - - - - - - - - - - - - - - - - - - - - - - - - - - -"

# Fund accounts using common function
fund_and_verify_account "requester-chain${MVM_INSTANCE}" "$MVM_INSTANCE" "Requester Chain $MVM_INSTANCE" "$EXPECTED_FUNDING_AMOUNT" "REQUESTER_BALANCE"
fund_and_verify_account "solver-chain${MVM_INSTANCE}" "$MVM_INSTANCE" "Solver Chain $MVM_INSTANCE" "$EXPECTED_FUNDING_AMOUNT" "SOLVER_BALANCE"

log_and_echo "✅ Connected chain $MVM_INSTANCE accounts funded"

log ""
log " CONNECTED CHAIN REQUESTER AND SOLVER SETUP COMPLETE!"
log "=================================================="
log " Connected chain $MVM_INSTANCE accounts ready!"
