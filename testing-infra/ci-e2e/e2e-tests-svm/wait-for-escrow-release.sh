#!/bin/bash

# Wait for escrow auto-release for SVM E2E tests
# Polls escrow state until released (auto-released when FulfillmentProof arrives)
# Respects SVM_INSTANCE env var for multi-instance testing.

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../chain-connected-svm/utils.sh"

setup_project_root

# Load SVM instance vars
svm_instance_vars "${SVM_INSTANCE:-2}"
source "$SVM_CHAIN_INFO_FILE" 2>/dev/null || true

log "   Loading intent info..."
load_intent_info "INTENT_ID"

if [ -z "$INTENT_ID" ]; then
    log_and_echo "❌ PANIC: INTENT_ID not set after load_intent_info"
    display_service_logs "Missing INTENT_ID"
    exit 1
fi

if [ -z "$SVM_PROGRAM_ID" ]; then
    log_and_echo "❌ PANIC: SVM_PROGRAM_ID not set in chain-info-svm${SVM_INSTANCE}.env"
    display_service_logs "Missing SVM_PROGRAM_ID"
    exit 1
fi

log_and_echo "⏳ Waiting for escrow auto-release (instance $SVM_INSTANCE)..."
log "   Intent ID: $INTENT_ID"
log "   Program ID: $SVM_PROGRAM_ID"

MAX_ATTEMPTS=15
ATTEMPT=1
ESCROW_CLAIMED=false

while [ $ATTEMPT -le $MAX_ATTEMPTS ]; do
    CLAIM_STATUS=$(SVM_RPC_URL="$SVM_RPC_URL" SVM_PROGRAM_ID="$SVM_PROGRAM_ID" SVM_INTENT_ID="$INTENT_ID" \
        "$PROJECT_ROOT/intent-frameworks/svm/scripts/get-escrow.sh" 2>/dev/null \
        | grep -Eo 'Claimed: (true|false)' | awk '{print $2}' | tail -1 | tr -d '\n')

    if [ "$CLAIM_STATUS" = "true" ]; then
        log_and_echo "   ✅ Escrow auto-released to solver!"
        ESCROW_CLAIMED=true
        break
    fi

    if [ $ATTEMPT -lt $MAX_ATTEMPTS ]; then
        sleep 2
    fi
    ATTEMPT=$((ATTEMPT + 1))
done

if [ "$ESCROW_CLAIMED" = "false" ]; then
    log_and_echo "❌ PANIC: Escrow not auto-released after ${MAX_ATTEMPTS} attempts ($((MAX_ATTEMPTS * 2))s)"
    display_service_logs "Escrow release timeout"
    exit 1
fi

log_and_echo "✅ Escrow auto-release verified!"
