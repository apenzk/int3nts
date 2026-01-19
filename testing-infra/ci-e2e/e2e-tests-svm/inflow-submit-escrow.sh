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

USD_SVM_MINT_ADDR="$USD_SVM_MINT_ADDR" \
SVM_REQUESTER_TOKEN="$REQUESTER_SVM_TOKEN_ACCOUNT" \
SVM_SOLVER_PUBKEY="$SOLVER_SVM_PUBKEY" \
SVM_INTENT_ID="$INTENT_ID" \
SVM_AMOUNT="$SVM_AMOUNT" \
SVM_EXPIRY="$SVM_EXPIRY" \
SVM_PROGRAM_ID="$SVM_PROGRAM_ID" \
SVM_RPC_URL="${SVM_RPC_URL:-http://127.0.0.1:8899}" \
SVM_PAYER_KEYPAIR="$SVM_PAYER_KEYPAIR" \
SVM_REQUESTER_KEYPAIR="$SVM_REQUESTER_KEYPAIR" \
"$PROJECT_ROOT/svm-intent-framework/scripts/create-escrow.sh" >> "$LOG_FILE" 2>&1

log "✅ SVM escrow created"
