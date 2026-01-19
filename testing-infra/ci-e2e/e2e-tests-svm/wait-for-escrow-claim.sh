#!/bin/bash

# Wait for escrow claim script for SVM E2E tests
# Polls escrow state until claimed or times out

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"

setup_project_root

source "$PROJECT_ROOT/.tmp/chain-info.env" 2>/dev/null || true

log "   Loading intent info..."
load_intent_info "INTENT_ID"

if [ -z "$INTENT_ID" ]; then
    log_and_echo "❌ PANIC: INTENT_ID not set after load_intent_info"
    display_service_logs "Missing INTENT_ID"
    exit 1
fi

if [ -z "$SVM_PROGRAM_ID" ]; then
    log_and_echo "❌ PANIC: SVM_PROGRAM_ID not set in chain-info.env"
    display_service_logs "Missing SVM_PROGRAM_ID"
    exit 1
fi

SVM_RPC_URL="${SVM_RPC_URL:-http://127.0.0.1:8899}"

log_and_echo "⏳ Waiting for solver to claim escrow..."
log "   Intent ID: $INTENT_ID"
log "   Program ID: $SVM_PROGRAM_ID"

MAX_ATTEMPTS=15
ATTEMPT=1
ESCROW_CLAIMED=false

while [ $ATTEMPT -le $MAX_ATTEMPTS ]; do
    CLAIM_STATUS=$(SVM_RPC_URL="$SVM_RPC_URL" SVM_PROGRAM_ID="$SVM_PROGRAM_ID" SVM_INTENT_ID="$INTENT_ID" \
        "$PROJECT_ROOT/svm-intent-framework/scripts/get-escrow.sh" 2>/dev/null \
        | grep -Eo 'Claimed: (true|false)' | awk '{print $2}' | tail -1 | tr -d '\n')

    if [ "$CLAIM_STATUS" = "true" ]; then
        log_and_echo "   ✅ Escrow claimed!"
        ESCROW_CLAIMED=true
        break
    fi

    if [ $ATTEMPT -lt $MAX_ATTEMPTS ]; then
        sleep 2
    fi
    ATTEMPT=$((ATTEMPT + 1))
done

if [ "$ESCROW_CLAIMED" = "false" ]; then
    log_and_echo "❌ PANIC: Escrow not claimed after ${MAX_ATTEMPTS} attempts ($((MAX_ATTEMPTS * 2))s)"
    display_service_logs "Escrow claim timeout"
    exit 1
fi

log_and_echo "✅ Escrow claim verified!"
