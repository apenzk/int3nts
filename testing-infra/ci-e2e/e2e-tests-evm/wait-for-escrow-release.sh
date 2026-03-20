#!/bin/bash

# Wait for escrow auto-release for EVM E2E tests
# Polls the IntentInflowEscrow::isReleased view function on connected EVM chain
# to verify the escrow was auto-released to the solver when FulfillmentProof arrived.

set -e

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_evm.sh"
source "$SCRIPT_DIR/../chain-connected-evm/utils.sh"

# Setup project root
setup_project_root

# Load EVM instance vars and chain info
evm_instance_vars "${EVM_INSTANCE:-2}"
source "$EVM_CHAIN_INFO_FILE" 2>/dev/null || true

# Load INTENT_ID - this will exit if missing
log "   Loading intent info..."
load_intent_info "INTENT_ID"
log "   ✅ load_intent_info completed, INTENT_ID=${INTENT_ID:0:20}..."

# Verify INTENT_ID was actually loaded (defensive check)
if [ -z "$INTENT_ID" ]; then
    log_and_echo "❌ PANIC: INTENT_ID is empty after load_intent_info"
    log_and_echo "   This should not happen - load_intent_info should have exited"
    log_and_echo "   Checking intent-info.env file..."
    if [ -f "$PROJECT_ROOT/.tmp/intent-info.env" ]; then
        log_and_echo "   File exists, contents:"
        cat "$PROJECT_ROOT/.tmp/intent-info.env" | sed 's/^/      /'
    else
        log_and_echo "   File does not exist: $PROJECT_ROOT/.tmp/intent-info.env"
    fi
    display_service_logs "INTENT_ID empty after load"
    exit 1
fi

# Use GMP contract address (IntentInflowEscrow)
ESCROW_GMP_ADDR="${ESCROW_GMP_ADDR:-}"

# Convert INTENT_ID to EVM format
INTENT_ID_EVM=$(convert_intent_id_to_evm "$INTENT_ID")

if [ -z "$ESCROW_GMP_ADDR" ]; then
    log_and_echo "❌ PANIC: Missing required variable ESCROW_GMP_ADDR"
    log_and_echo "   ESCROW_GMP_ADDR: ${ESCROW_GMP_ADDR:-not set}"
    log_and_echo "   INTENT_ID: ${INTENT_ID:-not set}"
    display_service_logs "Missing ESCROW_GMP_ADDR for escrow release check"
    exit 1
fi

log_and_echo "⏳ Waiting for escrow auto-release..."
log "   Intent ID: $INTENT_ID"
log "   IntentInflowEscrow: $ESCROW_GMP_ADDR"
log "   Intent ID (EVM): $INTENT_ID_EVM"

# Poll for escrow release (max 30 seconds, every 2 seconds)
MAX_ATTEMPTS=15
ATTEMPT=1
ESCROW_RELEASED=false

while [ $ATTEMPT -le $MAX_ATTEMPTS ]; do
    IS_RELEASED=$(is_released_evm "$ESCROW_GMP_ADDR" "$INTENT_ID_EVM" 2>/dev/null || echo "false")
    if [ "$IS_RELEASED" = "true" ]; then
        log_and_echo "   ✅ Escrow auto-released to solver! (isReleased=true)"
        ESCROW_RELEASED=true
        break
    fi
    log "   Attempt $ATTEMPT/$MAX_ATTEMPTS: Escrow not yet auto-released, waiting..."
    if [ $ATTEMPT -lt $MAX_ATTEMPTS ]; then
        sleep 2
    fi
    ATTEMPT=$((ATTEMPT + 1))
done

if [ "$ESCROW_RELEASED" = "false" ]; then
    log_and_echo "❌ PANIC: Escrow not auto-released after ${MAX_ATTEMPTS} attempts ($((MAX_ATTEMPTS * 2))s)"
    log_and_echo "   Intent ID: $INTENT_ID"
    display_service_logs "Escrow release timeout"
    exit 1
fi

log_and_echo "✅ Escrow auto-release verified!"
