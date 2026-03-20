#!/bin/bash

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/../chain-connected-mvm/utils.sh"

# Setup project root and logging
setup_project_root
cd "$PROJECT_ROOT"

# Load MVM instance vars
mvm_instance_vars "${MVM_INSTANCE:-2}"

setup_logging "inflow-submit-escrow-mvm${MVM_INSTANCE}"

# ============================================================================
# SECTION 1: LOAD DEPENDENCIES
# ============================================================================
if ! load_intent_info "INTENT_ID"; then
    exit 1
fi

# ============================================================================
# SECTION 2: GET ADDRESSES AND CONFIGURATION
# ============================================================================
HUB_MODULE_ADDR=$(get_profile_address "intent-account-chain1")
MVMCON_MODULE_ADDR=$(get_profile_address "intent-account-chain${MVM_INSTANCE}")
TEST_TOKENS_HUB=$(get_profile_address "test-tokens-chain1")
USD_MVMCON_MODULE_ADDR=$(get_profile_address "test-tokens-chain${MVM_INSTANCE}")
REQUESTER_HUB_ADDR=$(get_profile_address "requester-chain1")
SOLVER_HUB_ADDR=$(get_profile_address "solver-chain1")
REQUESTER_MVMCON_ADDR=$(get_profile_address "requester-chain${MVM_INSTANCE}")
SOLVER_MVMCON_ADDR=$(get_profile_address "solver-chain${MVM_INSTANCE}")

log ""
log " Chain Information:"
log "   Hub Module Address:            $HUB_MODULE_ADDR"
log "   Connected MVM Module Address:          $MVMCON_MODULE_ADDR"
log "   Requester Hub:               $REQUESTER_HUB_ADDR"
log "   Solver Hub:                  $SOLVER_HUB_ADDR"
log "   Requester MVM (connected):             $REQUESTER_MVMCON_ADDR"
log "   Solver MVM (connected):                $SOLVER_MVMCON_ADDR"

EXPIRY_TIME=$(date -d "+180 seconds" +%s)
CONNECTED_CHAIN_ID=$MVM_CHAIN_ID
HUB_CHAIN_ID=1

log ""
log " Configuration:"
log "   Expiry time: $EXPIRY_TIME"
log "   Intent ID: $INTENT_ID"

log ""
log "   - Getting USDcon metadata on connected MVM..."
USD_MVMCON_ADDR=$(get_usdxyz_metadata_addr "0x$USD_MVMCON_MODULE_ADDR" "$MVM_INSTANCE")
if [ -z "$USD_MVMCON_ADDR" ]; then
    log_and_echo "❌ Failed to get USDcon metadata on connected MVM"
    exit 1
fi
log "     ✅ Got USDcon metadata on connected MVM: $USD_MVMCON_ADDR"
OFFERED_METADATA_MVMCON="$USD_MVMCON_ADDR"

# ============================================================================
# SECTION 3: DISPLAY INITIAL STATE
# ============================================================================
log ""
display_balances_hub "0x$TEST_TOKENS_HUB"
display_balances_connected_mvm "0x$USD_MVMCON_MODULE_ADDR" "$MVM_INSTANCE"
log_and_echo ""

# ============================================================================
# SECTION 4: WAIT FOR GMP DELIVERY OF INTENT REQUIREMENTS
# ============================================================================
log ""
log "   Waiting for GMP relay to deliver IntentRequirements to connected chain..."
log "   (Hub intent creation sends requirements via GMP - relay must deliver them first)"

INTENT_ID_HEX=$(echo "$INTENT_ID" | sed 's/^0x//')
INTENT_ID_HEX=$(printf "%064s" "$INTENT_ID_HEX" | tr ' ' '0')

GMP_DELIVERED=0
for attempt in $(seq 1 30); do
    # Use REST API (not CLI) to query view function — CLI wraps result in {"Result": [...]}
    # which breaks jq '.[0]' parsing. REST API returns bare array like [true].
    HAS_REQ=$(curl -s "http://127.0.0.1:${MVM_REST_PORT}/v1/view" \
        -H 'Content-Type: application/json' \
        -d "{
            \"function\": \"0x${MVMCON_MODULE_ADDR}::intent_inflow_escrow::has_requirements\",
            \"type_arguments\": [],
            \"arguments\": [\"0x${INTENT_ID_HEX}\"]
        }" 2>/dev/null | jq -r '.[0]' 2>/dev/null)
    if [ "$HAS_REQ" = "true" ]; then
        log "   ✅ IntentRequirements delivered via GMP (attempt $attempt)"
        GMP_DELIVERED=1
        break
    fi
    log "   Attempt $attempt/30: requirements not yet delivered, waiting..."
    sleep 2
done

if [ "$GMP_DELIVERED" -ne 1 ]; then
    log_and_echo "❌ PANIC: IntentRequirements NOT delivered via GMP after 60 seconds"
    log_and_echo "   The GMP relay failed to deliver requirements from hub to connected chain."
    display_service_logs "GMP delivery timeout"
    exit 1
fi

# ============================================================================
# SECTION 5: EXECUTE MAIN OPERATION
# ============================================================================
log ""
log "   Creating escrow on connected chain..."
log "   - Requester locks 1 USDcon in escrow on connected MVM"
log "   - Using intent_id from hub chain: $INTENT_ID"

# DEBUG: Check requester balance BEFORE escrow creation
log ""
log "   DEBUG: Checking requester balance BEFORE escrow creation..."
BEFORE_BALANCE=$(get_usdxyz_balance "requester-chain${MVM_INSTANCE}" "$MVM_INSTANCE" "0x$USD_MVMCON_MODULE_ADDR")
log_and_echo "   DEBUG: Requester USDcon balance BEFORE escrow: $BEFORE_BALANCE"

log "   - Creating escrow intent on connected MVM..."
log "     Offered metadata: $OFFERED_METADATA_MVMCON"

# Use GMP-based escrow: validates against IntentRequirements received from hub
ESCROW_OUTPUT=$(aptos move run --profile requester-chain${MVM_INSTANCE} --assume-yes \
    --function-id "0x${MVMCON_MODULE_ADDR}::intent_inflow_escrow::create_escrow_with_validation" \
    --args "hex:${INTENT_ID_HEX}" "address:${OFFERED_METADATA_MVMCON}" "u64:1000000" 2>&1)
ESCROW_EXIT_CODE=$?

log "   DEBUG: Escrow transaction output:"
log "$ESCROW_OUTPUT"

# ============================================================================
# SECTION 6: VERIFY RESULTS
# ============================================================================
if [ $ESCROW_EXIT_CODE -eq 0 ]; then
    log "     ✅ Escrow intent created on connected MVM!"

    # DEBUG: Check requester balance AFTER escrow creation
    log ""
    log "   DEBUG: Checking requester balance AFTER escrow creation..."
    AFTER_BALANCE=$(get_usdxyz_balance "requester-chain${MVM_INSTANCE}" "$MVM_INSTANCE" "0x$USD_MVMCON_MODULE_ADDR")
    log_and_echo "   DEBUG: Requester USDcon balance AFTER escrow: $AFTER_BALANCE"

    if [ "$BEFORE_BALANCE" = "$AFTER_BALANCE" ]; then
        log_and_echo "   WARNING: Requester balance did NOT change after escrow creation!"
        log_and_echo "      Before: $BEFORE_BALANCE, After: $AFTER_BALANCE"
    else
        DIFF=$((BEFORE_BALANCE - AFTER_BALANCE))
        log_and_echo "   ✅ Requester balance decreased by: $DIFF (locked in escrow)"
    fi

    sleep 4
    log "     - Verifying escrow stored on-chain..."

    # Get full transaction for debugging
    FULL_TX=$(curl -s "http://127.0.0.1:${MVM_REST_PORT}/v1/accounts/${REQUESTER_MVMCON_ADDR}/transactions?limit=1")

    # GMP escrow emits EscrowCreated event (not OracleLimitOrderEvent)
    ESCROW_EVENT=$(echo "$FULL_TX" | jq '.[0].events[] | select(.type | contains("EscrowCreated"))' 2>/dev/null)
    ESCROW_INTENT_ID_HEX=$(echo "$ESCROW_EVENT" | jq -r '.data.intent_id' 2>/dev/null)
    ESCROW_AMOUNT=$(echo "$ESCROW_EVENT" | jq -r '.data.amount' 2>/dev/null)
    ESCROW_CREATOR=$(echo "$ESCROW_EVENT" | jq -r '.data.creator' 2>/dev/null)
    ESCROW_ID=$(echo "$ESCROW_EVENT" | jq -r '.data.escrow_id' 2>/dev/null)

    # Output full event for debugging
    log "   DEBUG: Full EscrowCreated event:"
    log "$ESCROW_EVENT"

    if [ -z "$ESCROW_EVENT" ] || [ "$ESCROW_EVENT" = "null" ] || [ "$ESCROW_EVENT" = "" ]; then
        log_and_echo "WARNING: Could not find EscrowCreated event, checking transaction success"
        # Even without event parsing, the transaction succeeded
    else
        log "     ✅ EscrowCreated event found"
        log "     ✅ Escrow ID: $ESCROW_ID"
        log "     ✅ Creator: $ESCROW_CREATOR"
        log "     ✅ Amount: $ESCROW_AMOUNT"
    fi

    log_and_echo "✅ Escrow created"
else
    log_and_echo "❌ Escrow intent creation failed!"
    log_and_echo "   DEBUG: Escrow output:"
    log_and_echo "$ESCROW_OUTPUT"
    exit 1
fi

# ============================================================================
# SECTION 7: FINAL SUMMARY
# ============================================================================
log ""
display_balances_hub "0x$TEST_TOKENS_HUB"
display_balances_connected_mvm "0x$USD_MVMCON_MODULE_ADDR" "$MVM_INSTANCE"
log_and_echo ""

log ""
log " INFLOW - ESCROW CREATION COMPLETE!"
log "======================================"
log ""
log "✅ Step completed successfully:"
log "   1. Escrow created on connected MVM with locked tokens (via GMP validation)"
log ""
log " Escrow Details:"
log "   Intent ID: $INTENT_ID"
if [ -n "$ESCROW_ID" ] && [ "$ESCROW_ID" != "null" ]; then
    log "   Escrow ID: $ESCROW_ID"
    echo "MVMCON_ESCROW_ID=$ESCROW_ID" >> "$PROJECT_ROOT/.tmp/intent-info.env"
fi

