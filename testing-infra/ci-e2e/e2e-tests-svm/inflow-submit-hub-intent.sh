#!/bin/bash

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/../util_svm.sh"

# Setup project root and logging
setup_project_root
setup_logging "submit-hub-intent-svm-inflow"
cd "$PROJECT_ROOT"

verify_verifier_running
verify_solver_running
verify_solver_registered

INTENT_ID="0x$(openssl rand -hex 32)"

CONNECTED_CHAIN_ID=4
HUB_CHAIN_ID=1

HUB_MODULE_ADDR=$(get_profile_address "intent-account-chain1")
TEST_TOKENS_HUB=$(get_profile_address "test-tokens-chain1")
REQUESTER_HUB_ADDR=$(get_profile_address "requester-chain1")
SOLVER_HUB_ADDR=$(get_profile_address "solver-chain1")

source "$PROJECT_ROOT/.tmp/chain-info.env" 2>/dev/null || true

if [ -z "$REQUESTER_SVM_PUBKEY" ] || [ -z "$SOLVER_SVM_PUBKEY" ] || [ -z "$USD_SVM_MINT_ADDR" ]; then
    log_and_echo "❌ ERROR: Missing SVM chain info. Run chain-connected-svm/setup-requester-solver.sh first."
    exit 1
fi

REQUESTER_SVM_ADDR=$(svm_pubkey_to_hex "$REQUESTER_SVM_PUBKEY")
SOLVER_SVM_ADDR=$(svm_pubkey_to_hex "$SOLVER_SVM_PUBKEY")

log ""
log " Chain Information:"
log "   Hub Module Address:                    $HUB_MODULE_ADDR"
log "   Requester Hub:                         $REQUESTER_HUB_ADDR"
log "   Solver Hub:                            $SOLVER_HUB_ADDR"
log "   Requester SVM (hex):                   $REQUESTER_SVM_ADDR"
log "   Solver SVM (hex):                      $SOLVER_SVM_ADDR"

EXPIRY_TIME=$(date -d "+1 hour" +%s)
OFFERED_AMOUNT="1000000"
DESIRED_AMOUNT="1000000"

log ""
log " Configuration:"
log "   Intent ID: $INTENT_ID"
log "   Expiry time: $EXPIRY_TIME"
log "   Offered amount: $OFFERED_AMOUNT (1 USDcon on connected SVM)"
log "   Desired amount: $DESIRED_AMOUNT (1 USDhub on hub)"

log ""
log "   - Getting USD token metadata addresses..."
USDHUB_METADATA_HUB=$(get_usdxyz_metadata_addr "0x$TEST_TOKENS_HUB" "1")
if [ -z "$USDHUB_METADATA_HUB" ]; then
    log_and_echo "❌ Failed to get USDhub metadata on Hub"
    exit 1
fi
log "     ✅ Got USDhub metadata on Hub: $USDHUB_METADATA_HUB"

SVM_TOKEN_HEX=$(svm_pubkey_to_hex "$USD_SVM_MINT_ADDR")
OFFERED_METADATA_SVM="$SVM_TOKEN_HEX"
DESIRED_METADATA_HUB="$USDHUB_METADATA_HUB"

log "     Inflow configuration:"
log "       Offered metadata (connected SVM): $OFFERED_METADATA_SVM"
log "       Desired metadata (hub):          $DESIRED_METADATA_HUB"

log ""
display_balances_hub "0x$TEST_TOKENS_HUB"
log_and_echo ""

log ""
log " Starting verifier-based negotiation routing..."
log "   Flow: Requester → Verifier → Solver → Verifier → Requester"

log ""
log "   Step 1: Requester submits draft intent to verifier..."
DRAFT_DATA=$(build_draft_data \
    "$OFFERED_METADATA_SVM" \
    "$OFFERED_AMOUNT" \
    "$CONNECTED_CHAIN_ID" \
    "$DESIRED_METADATA_HUB" \
    "$DESIRED_AMOUNT" \
    "$HUB_CHAIN_ID" \
    "$EXPIRY_TIME" \
    "$INTENT_ID" \
    "$REQUESTER_HUB_ADDR" \
    "{\"chain_addr\": \"$HUB_MODULE_ADDR\", \"flow_type\": \"inflow\", \"connected_chain_type\": \"svm\"}")

DRAFT_ID=$(submit_draft_intent "$REQUESTER_HUB_ADDR" "$DRAFT_DATA" "$EXPIRY_TIME")
log "     Draft ID: $DRAFT_ID"

log ""
log "   Step 2: Waiting for solver service to sign draft..."
SIGNATURE_DATA=$(poll_for_signature "$DRAFT_ID" 10 2)
RETRIEVED_SIGNATURE=$(echo "$SIGNATURE_DATA" | jq -r '.signature')
RETRIEVED_SOLVER=$(echo "$SIGNATURE_DATA" | jq -r '.solver_hub_addr')

if [ -z "$RETRIEVED_SIGNATURE" ] || [ "$RETRIEVED_SIGNATURE" = "null" ]; then
    log_and_echo "❌ ERROR: Failed to retrieve signature from verifier"
    display_service_logs "SVM inflow draft signature missing"
    exit 1
fi

log "     ✅ Retrieved signature from solver: $RETRIEVED_SOLVER"
log "     Signature: ${RETRIEVED_SIGNATURE:0:20}..."

log ""
log "   Creating cross-chain intent on Hub..."
log "     Offered metadata (connected SVM): $OFFERED_METADATA_SVM"
log "     Desired metadata (hub): $DESIRED_METADATA_HUB"
log "     Solver address: $RETRIEVED_SOLVER"

SOLVER_SIGNATURE_HEX="${RETRIEVED_SIGNATURE#0x}"
aptos move run --profile requester-chain1 --assume-yes \
    --function-id "0x${HUB_MODULE_ADDR}::fa_intent_inflow::create_inflow_intent_entry" \
    --args "address:${OFFERED_METADATA_SVM}" "u64:${OFFERED_AMOUNT}" "u64:${CONNECTED_CHAIN_ID}" "address:${DESIRED_METADATA_HUB}" "u64:${DESIRED_AMOUNT}" "u64:${HUB_CHAIN_ID}" "u64:${EXPIRY_TIME}" "address:${INTENT_ID}" "address:${RETRIEVED_SOLVER}" "hex:${SOLVER_SIGNATURE_HEX}" "address:${REQUESTER_SVM_ADDR}" >> "$LOG_FILE" 2>&1

if [ $? -eq 0 ]; then
    log "     ✅ Request-intent created on Hub!"
    sleep 2
    HUB_INTENT_ADDR=$(curl -s "http://127.0.0.1:8080/v1/accounts/${REQUESTER_HUB_ADDR}/transactions?limit=1" | \
        jq -r '.[0].events[] | select(.type | contains("LimitOrderEvent")) | .data.intent_addr' | head -n 1)
    if [ -n "$HUB_INTENT_ADDR" ] && [ "$HUB_INTENT_ADDR" != "null" ]; then
        log "     ✅ Hub intent stored at: $HUB_INTENT_ADDR"
        log_and_echo "✅ Request-intent created (via verifier negotiation)"
    else
        log_and_echo "❌ ERROR: Could not verify hub intent address"
        exit 1
    fi
else
    log_and_echo "❌ Request-intent creation failed on Hub!"
    log_and_echo "   Log file contents:"
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    cat "$LOG_FILE"
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    exit 1
fi

log ""
log " INFLOW - HUB CHAIN INTENT CREATION COMPLETE!"
log "================================================"
log ""
log "✅ Steps completed successfully (via verifier-based negotiation):"
log "   1. Solver registered on-chain"
log "   2. Requester submitted draft intent to verifier"
log "   3. Solver service signed draft automatically (FCFS)"
log "   4. Requester polled verifier and retrieved signature"
log "   5. Requester created intent on-chain with retrieved signature"
log ""
log " Request-intent Details:"
log "   Intent ID: $INTENT_ID"
log "   Draft ID: $DRAFT_ID"
log "   Solver: $RETRIEVED_SOLVER"
if [ -n "$HUB_INTENT_ADDR" ] && [ "$HUB_INTENT_ADDR" != "null" ]; then
    log "   Hub Request-intent: $HUB_INTENT_ADDR"
fi

save_intent_info "$INTENT_ID" "$HUB_INTENT_ADDR"
