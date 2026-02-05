#!/bin/bash

# Create SVM escrow for inflow (connected chain)

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_svm.sh"

setup_project_root
setup_logging "inflow-submit-escrow-svm"
cd "$PROJECT_ROOT"

if ! load_intent_info "INTENT_ID"; then
    log_and_echo "❌ ERROR: Missing INTENT_ID. Run inflow-submit-hub-intent.sh first."
    exit 1
fi

source "$PROJECT_ROOT/.tmp/chain-info.env" 2>/dev/null || true

if [ -z "$USD_SVM_MINT_ADDR" ] || [ -z "$REQUESTER_SVM_TOKEN_ACCOUNT" ] || [ -z "$SOLVER_SVM_PUBKEY" ] || [ -z "$SVM_PROGRAM_ID" ]; then
    log_and_echo "❌ ERROR: Missing SVM chain info. Run chain-connected-svm/setup-requester-solver.sh and deploy-contract.sh first."
    exit 1
fi

SVM_AMOUNT="1000000"
SVM_EXPIRY="$(date -d "+10 minutes" +%s)"

log "Creating SVM escrow..."
log "   Intent ID: $INTENT_ID"
log "   Token mint: $USD_SVM_MINT_ADDR"
log "   Requester token account: $REQUESTER_SVM_TOKEN_ACCOUNT"
log "   Solver pubkey: $SOLVER_SVM_PUBKEY"
log "   Amount: $SVM_AMOUNT"

# Export all environment variables needed by create-escrow.sh
# (needed because the script re-executes itself via nix develop)

# SPL token mint address for the escrowed token (from chain-info.env)
export USD_SVM_MINT_ADDR="$USD_SVM_MINT_ADDR"
# Requester's associated token account holding the tokens to escrow
export SVM_REQUESTER_TOKEN="$REQUESTER_SVM_TOKEN_ACCOUNT"
# Solver's Solana pubkey who will receive tokens on fulfillment
export SVM_SOLVER_PUBKEY="$SOLVER_SVM_PUBKEY"
# 32-byte hex intent ID matching the hub-side intent
export SVM_INTENT_ID="$INTENT_ID"
# Amount of tokens to escrow (in smallest units)
export SVM_AMOUNT="$SVM_AMOUNT"
# Unix timestamp after which requester can cancel the escrow
export SVM_EXPIRY="$SVM_EXPIRY"
# Deployed intent_escrow program ID on SVM
export SVM_PROGRAM_ID="$SVM_PROGRAM_ID"
# Native GMP endpoint program ID for cross-chain messaging
export SVM_GMP_ENDPOINT_ID="$SVM_GMP_ENDPOINT_ID"
# Chain ID of the hub (Movement) for GMP routing
export HUB_CHAIN_ID="1"
# Solana RPC endpoint URL
export SVM_RPC_URL="${SVM_RPC_URL:-http://127.0.0.1:8899}"
# Path to keypair file for transaction fees
export SVM_PAYER_KEYPAIR="$SVM_PAYER_KEYPAIR"
# Path to requester's keypair file (signs the escrow creation)
export SVM_REQUESTER_KEYPAIR="$SVM_REQUESTER_KEYPAIR"

if ! "$PROJECT_ROOT/intent-frameworks/svm/scripts/create-escrow.sh" >> "$LOG_FILE" 2>&1; then
    log_and_echo "❌ ERROR: create-escrow.sh failed. Log contents:"
    cat "$LOG_FILE" >&2
    exit 1
fi

log "✅ SVM escrow created"
