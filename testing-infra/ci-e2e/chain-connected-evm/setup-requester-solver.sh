#!/bin/bash

# Setup EVM Chain and Test Requester/Solver Accounts
# Accepts instance number as argument (default: 1)

set -e

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/utils.sh"

# Setup project root and logging
setup_project_root

# Accept instance number as argument (default: 1)
evm_instance_vars "${1:-1}"

setup_logging "setup-evm${EVM_INSTANCE}-requester-solver"
cd "$PROJECT_ROOT"

log " Requester and Solver Account Testing - EVM CHAIN (instance $EVM_INSTANCE)"
log "=============================================="
log_and_echo " All output logged to: $LOG_FILE"

log ""
log "% - - - - - - - - - - - SETUP - - - - - - - - - - - -"
log "% - - - - - - - - - - - - - - - - - - - - - - - - - - - - -"

# Wait for node to be fully ready (assumes setup-chain.sh was already run)
log "⏳ Waiting for node to be fully ready..."
sleep 5

# Verify EVM chain is running
log " Verifying EVM chain is running..."
if ! check_evm_chain_running "$EVM_PORT"; then
    log_and_echo "❌ Error: EVM chain failed to start on port $EVM_PORT"
    exit 1
fi

log ""
log "% - - - - - - - - - - - ACCOUNTS - - - - - - - - - - - -"
log "% - - - - - - - - - - - - - - - - - - - - - - - - - - - - -"

log ""
log " Hardhat Default Accounts:"
log "   Deployer/Approver = Account 0 (signer index 0)"
log "   Requester         = Account 1 (signer index 1)"
log "   Solver            = Account 2 (signer index 2)"

# Get account addresses using Hardhat
log ""
log " Getting Requester and Solver addresses..."

REQUESTER_ADDR=$(get_hardhat_account_address "1" "$EVM_NETWORK")
SOLVER_ADDR=$(get_hardhat_account_address "2" "$EVM_NETWORK")

log "   ✅ Requester (Account 1): $REQUESTER_ADDR"
log "   ✅ Solver (Account 2):   $SOLVER_ADDR"

log ""
log "% - - - - - - - - - - - BALANCES - - - - - - - - - - - -"
log "% - - - - - - - - - - - - - - - - - - - - - - - - - - - - -"

# Check initial balances
log ""
log " Checking initial balances..."

cd intent-frameworks/evm
BALANCES_OUTPUT=$(nix develop "$PROJECT_ROOT/nix" -c bash -c "npx hardhat run scripts/get-accounts.js --network $EVM_NETWORK" 2>&1)

if [ $? -ne 0 ]; then
    log_and_echo "❌ Error: Failed to get account balances"
    echo "$BALANCES_OUTPUT" >> "$LOG_FILE"
    exit 1
fi

REQUESTER_BALANCE=$(echo "$BALANCES_OUTPUT" | grep "^REQUESTER_BALANCE=" | cut -d'=' -f2 | tr -d '\n')
SOLVER_BALANCE=$(echo "$BALANCES_OUTPUT" | grep "^SOLVER_BALANCE=" | cut -d'=' -f2 | tr -d '\n')

cd "$PROJECT_ROOT"

if [ -z "$REQUESTER_BALANCE" ] || [ -z "$SOLVER_BALANCE" ]; then
    log_and_echo "❌ Error: Failed to extract account balances from output"
    echo "$BALANCES_OUTPUT" >> "$LOG_FILE"
    exit 1
fi

log "   Requester balance: $REQUESTER_BALANCE wei (should be 1 ETH = 1_000_000_000_000_000_000 wei)"
log "   Solver balance:   $SOLVER_BALANCE wei (should be 1 ETH = 1_000_000_000_000_000_000 wei)"

log ""
log " All EVM chain setup and testing complete!"
log ""
log " Summary:"
log "   EVM Chain:     $EVM_RPC_URL"
log "   Chain ID:      $EVM_CHAIN_ID"
log "   Requester (Acc 1): $REQUESTER_ADDR"
log "   Solver (Acc 2):   $SOLVER_ADDR"
log ""
log " Script completed!"
