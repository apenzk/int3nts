#!/bin/bash

# Wait for escrow auto-release for MVM E2E tests
# Polls the intent_inflow_escrow::is_released view function on connected MVM chain
# to verify the escrow was auto-released to the solver when FulfillmentProof arrived.
# Respects MVM_INSTANCE env var for multi-instance testing.

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/../chain-connected-mvm/utils.sh"

setup_project_root

# Load MVM instance vars
mvm_instance_vars "${MVM_INSTANCE:-2}"

# Load intent info
if ! load_intent_info "INTENT_ID"; then
    exit 1
fi

MVMCON_MODULE_ADDR=$(get_profile_address "intent-account-chain${MVM_INSTANCE}")

# Format intent_id for view function call: strip 0x prefix, zero-pad to 64 hex chars
INTENT_ID_HEX=$(echo "$INTENT_ID" | sed 's/^0x//')
INTENT_ID_HEX=$(printf "%064s" "$INTENT_ID_HEX" | tr ' ' '0')

log_and_echo "⏳ Waiting for escrow auto-release (instance $MVM_INSTANCE)..."
log "   Intent ID: $INTENT_ID"
log "   Module: 0x${MVMCON_MODULE_ADDR}::intent_inflow_escrow::is_released"

# Poll for escrow release (max 30 seconds, every 2 seconds)
MAX_ATTEMPTS=15
ATTEMPT=1
ESCROW_CLAIMED=false

while [ $ATTEMPT -le $MAX_ATTEMPTS ]; do
    IS_RELEASED=$(curl -s "http://127.0.0.1:${MVM_REST_PORT}/v1/view" \
        -H 'Content-Type: application/json' \
        -d "{
            \"function\": \"0x${MVMCON_MODULE_ADDR}::intent_inflow_escrow::is_released\",
            \"type_arguments\": [],
            \"arguments\": [\"0x${INTENT_ID_HEX}\"]
        }" 2>/dev/null | jq -r '.[0]' 2>/dev/null)

    if [ "$IS_RELEASED" = "true" ]; then
        log_and_echo "   ✅ Escrow auto-released to solver! (is_released=true)"
        ESCROW_CLAIMED=true
        break
    fi

    log "   Attempt $ATTEMPT/$MAX_ATTEMPTS: Escrow not yet auto-released, waiting..."
    if [ $ATTEMPT -lt $MAX_ATTEMPTS ]; then
        sleep 2
    fi
    ATTEMPT=$((ATTEMPT + 1))
done

if [ "$ESCROW_CLAIMED" = "false" ]; then
    log_and_echo "❌ PANIC: Escrow not auto-released after ${MAX_ATTEMPTS} attempts ($((MAX_ATTEMPTS * 2))s)"
    log_and_echo "   Intent ID: $INTENT_ID"
    display_service_logs "Escrow release timeout"
    exit 1
fi

log_and_echo "✅ Escrow auto-release verified!"
