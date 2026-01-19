#!/bin/bash

# Setup Hub Chain Test Requester/Solver Accounts
# This script:
# 1. Creates and funds Requester and Solver accounts on Hub
# Run this from the host machine (not inside Docker)

set -e

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"

# Setup project root and logging
setup_project_root
setup_logging "setup-requester-solver-hub"
cd "$PROJECT_ROOT"

# Expected funding amount in octas
# Note: aptos init funds accounts with 100_000_000, then we fund again with 100_000_000 = 200_000_000 total
EXPECTED_FUNDING_AMOUNT=200000000

log " Requester and Solver Account Setup - HUB"
log "====================================================="
log_and_echo " All output logged to: $LOG_FILE"

log ""
log "% - - - - - - - - - - - SETUP - - - - - - - - - - - -"
log "% - - - - - - - - - - - - - - - - - - - - - - - - - - - - -"

# Create test accounts for Hub
log ""
log " Creating test accounts for Hub..."

# Create requester account for Hub
log "Creating requester-chain1 account for Hub..."
init_aptos_profile "requester-chain1" "1" "$LOG_FILE"

# Create solver account for Hub
log "Creating solver-chain1 account for Hub..."
init_aptos_profile "solver-chain1" "1" "$LOG_FILE"

# Create test-tokens account for Hub (for USDhub deployment)
log "Creating test-tokens-chain1 account for Hub..."
init_aptos_profile "test-tokens-chain1" "1" "$LOG_FILE"

log ""
log "% - - - - - - - - - - - FUNDING - - - - - - - - - - - -"
log "% - - - - - - - - - - - - - - - - - - - - - - - - - - - - -"

# Fund Hub accounts using common function
fund_and_verify_account "requester-chain1" "1" "Requester Hub" "$EXPECTED_FUNDING_AMOUNT" "REQUESTER_BALANCE"
fund_and_verify_account "solver-chain1" "1" "Solver Hub" "$EXPECTED_FUNDING_AMOUNT" "SOLVER_BALANCE"

log_and_echo "✅ Hub accounts funded"

log ""
log " HUB REQUESTER AND SOLVER SETUP COMPLETE!"
log "=========================================="
log " Hub chain accounts ready!"

