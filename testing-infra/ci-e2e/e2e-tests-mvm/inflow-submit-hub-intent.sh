#!/bin/bash

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"

# Setup project root and logging
setup_project_root
setup_logging "submit-hub-intent"
cd "$PROJECT_ROOT"

# Verify services are running before proceeding
verify_coordinator_running
verify_integrated_gmp_running
verify_solver_running
verify_solver_registered

# ============================================================================
# SECTION 1: LOAD DEPENDENCIES
# ============================================================================
# Generate a random intent_id that will be used for both hub and escrow
INTENT_ID="0x$(openssl rand -hex 32)"

# ============================================================================
# SECTION 2: GET ADDRES AND CONFIGURATION
# ============================================================================
CONNECTED_CHAIN_ID=2
HUB_MODULE_ADDR=$(get_profile_address "intent-account-chain1")
MVMCON_MODULE_ADDR=$(get_profile_address "intent-account-chain2")
TEST_TOKENS_HUB=$(get_profile_address "test-tokens-chain1")
USD_MVMCON_MODULE_ADDR=$(get_profile_address "test-tokens-chain2")
REQUESTER_HUB_ADDR=$(get_profile_address "requester-chain1")
SOLVER_HUB_ADDR=$(get_profile_address "solver-chain1")
REQUESTER_MVMCON_ADDR=$(get_profile_address "requester-chain2")
SOLVER_MVMCON_ADDR=$(get_profile_address "solver-chain2")

log ""
log " Chain Information:"
log "   Hub Module Address:            $HUB_MODULE_ADDR"
log "   Connected MVM Module Address:          $MVMCON_MODULE_ADDR"
log "   Requester Hub:               $REQUESTER_HUB_ADDR"
log "   Solver Hub:                  $SOLVER_HUB_ADDR"
log "   Requester MVM (connected):             $REQUESTER_MVMCON_ADDR"
log "   Solver MVM (connected):                $SOLVER_MVMCON_ADDR"

EXPIRY_TIME=$(date -d "+1 hour" +%s)
# Requester and Solver get funded with 1 USDhub / 1 USDcon each (6 decimals = 1_000_000)
OFFERED_AMOUNT="1000000"  # 1 USDcon (connected chain, 6 decimals = 1_000_000)
DESIRED_AMOUNT="1000000"  # 1 USDhub (hub chain, 6 decimals = 1_000_000)
HUB_CHAIN_ID=1
EVM_ADDR="0x0000000000000000000000000000000000000001"

log ""
log " Configuration:"
log "   Intent ID: $INTENT_ID"
log "   Expiry time: $EXPIRY_TIME"
log "   Offered amount: $OFFERED_AMOUNT (1 USDcon on connected MVM)"
log "   Desired amount: $DESIRED_AMOUNT (1 USDhub on hub)"

log ""
log "   - Getting USD token metadata addresses..."
log "     Getting USDhub metadata on Hub..."
USDHUB_METADATA_HUB=$(get_usdxyz_metadata_addr "0x$TEST_TOKENS_HUB" "1")
if [ -z "$USDHUB_METADATA_HUB" ]; then
    log_and_echo "❌ Failed to get USDhub metadata on Hub"
    exit 1
fi
log "     ✅ Got USDhub metadata on Hub: $USDHUB_METADATA_HUB"

log "     Getting USDcon metadata on connected MVM..."
USD_MVMCON_ADDR=$(get_usdxyz_metadata_addr "0x$USD_MVMCON_MODULE_ADDR" "2")
if [ -z "$USD_MVMCON_ADDR" ]; then
    log_and_echo "❌ Failed to get USDcon metadata on connected MVM"
    exit 1
fi
log "     ✅ Got USDcon metadata on connected MVM: $USD_MVMCON_ADDR"

# For INFLOW: offered tokens are on connected MVM, desired tokens are on hub
OFFERED_METADATA_MVMCON="$USD_MVMCON_ADDR"
DESIRED_METADATA_HUB="$USDHUB_METADATA_HUB"
log "     Inflow configuration:"
log "       Offered metadata (connected MVM): $OFFERED_METADATA_MVMCON"
log "       Desired metadata (hub): $DESIRED_METADATA_HUB"

# ============================================================================
# SECTION 3: DISPLAY INITIAL STATE
# ============================================================================
# Check and display initial balances using common function
log ""
display_balances_hub "0x$TEST_TOKENS_HUB"
display_balances_connected_mvm "0x$USD_MVMCON_MODULE_ADDR"
log_and_echo ""

# ============================================================================
# SECTION 4: COORDINATOR-BASED NEGOTIATION ROUTING
# ============================================================================
log ""
log " Starting coordinator-based negotiation routing..."
log "   Flow: Requester → Coordinator → Solver → Coordinator → Requester"

# Step 1: Requester submits draft intent to coordinator
log ""
log "   Step 1: Requester submits draft intent to coordinator..."
DRAFT_DATA=$(build_draft_data \
    "$OFFERED_METADATA_MVMCON" \
    "$OFFERED_AMOUNT" \
    "$CONNECTED_CHAIN_ID" \
    "$DESIRED_METADATA_HUB" \
    "$DESIRED_AMOUNT" \
    "$HUB_CHAIN_ID" \
    "$EXPIRY_TIME" \
    "$INTENT_ID" \
    "$REQUESTER_HUB_ADDR" \
    "{\"chain_addr\": \"$HUB_MODULE_ADDR\", \"flow_type\": \"inflow\"}")

DRAFT_ID=$(submit_draft_intent "$REQUESTER_HUB_ADDR" "$DRAFT_DATA" "$EXPIRY_TIME")
log "     Draft ID: $DRAFT_ID"

# Step 2: Wait for solver service to sign the draft (polls automatically)
# The solver service running in the background will:
# - Poll for pending drafts
# - Evaluate acceptance criteria
# - Generate signature
# - Submit signature to coordinator (FCFS)
log ""
log "   Step 2: Waiting for solver service to sign draft..."
log "     (Solver service polls coordinator automatically)"

# Poll for signature with retry logic (solver service needs time to process)
SIGNATURE_DATA=$(poll_for_signature "$DRAFT_ID" 10 2)
RETRIEVED_SIGNATURE=$(echo "$SIGNATURE_DATA" | jq -r '.signature')
RETRIEVED_SOLVER=$(echo "$SIGNATURE_DATA" | jq -r '.solver_hub_addr')

if [ -z "$RETRIEVED_SIGNATURE" ] || [ "$RETRIEVED_SIGNATURE" = "null" ]; then
    log_and_echo "❌ ERROR: Failed to retrieve signature from coordinator/integrated-gmp"
    log_and_echo ""
    log_and_echo " Diagnostics:"
    
    # Check if solver is running
    SOLVER_LOG_FILE="$PROJECT_ROOT/.tmp/e2e-tests/solver.log"
    if [ -f "$PROJECT_ROOT/.tmp/e2e-tests/solver.pid" ]; then
        SOLVER_PID=$(cat "$PROJECT_ROOT/.tmp/e2e-tests/solver.pid")
        if ps -p "$SOLVER_PID" > /dev/null 2>&1; then
            log_and_echo "   ✅ Solver process is running (PID: $SOLVER_PID)"
        else
            log_and_echo "   ❌ Solver process is NOT running (PID: $SOLVER_PID)"
        fi
    else
        log_and_echo "   ❌ Solver PID file not found"
    fi
    
    # Show solver log - surface WARN/ERROR lines first
    if [ -f "$SOLVER_LOG_FILE" ]; then
        SOLVER_WARNINGS=$(grep -E 'WARN|ERROR' "$SOLVER_LOG_FILE" 2>/dev/null || true)
        if [ -n "$SOLVER_WARNINGS" ]; then
            log_and_echo ""
            log_and_echo "   ⚠ Solver WARN/ERROR lines:"
            log_and_echo "   ----------------------------------------"
            echo "$SOLVER_WARNINGS" | while IFS= read -r line; do log_and_echo "   $line"; done
            log_and_echo "   ----------------------------------------"
        fi
        log_and_echo ""
        log_and_echo "    Solver log (last 100 lines):"
        log_and_echo "   ----------------------------------------"
        tail -100 "$SOLVER_LOG_FILE" | while IFS= read -r line; do log_and_echo "   $line"; done
        log_and_echo "   ----------------------------------------"
    else
        log_and_echo "   ️  Solver log file not found: $SOLVER_LOG_FILE"
    fi

    # Show coordinator and integrated-gmp logs
    for f in "$PROJECT_ROOT/.tmp/e2e-tests/coordinator.log" "$PROJECT_ROOT/.tmp/e2e-tests/integrated-gmp.log"; do
        if [ -f "$f" ]; then
            log_and_echo ""
            log_and_echo "    $(basename "$f") (last 30 lines):"
            log_and_echo "   ----------------------------------------"
            tail -30 "$f" | while IFS= read -r line; do log_and_echo "   $line"; done
            log_and_echo "   ----------------------------------------"
        fi
    done
    
    exit 1
fi
log "     ✅ Retrieved signature from solver: $RETRIEVED_SOLVER"
log "     Signature: ${RETRIEVED_SIGNATURE:0:20}..."

# ============================================================================
# SECTION 5: CREATE INTENT ON-CHAIN WITH RETRIEVED SIGNATURE
# ============================================================================
log ""
log "   Creating cross-chain intent on Hub..."
log "     Offered metadata (connected MVM): $OFFERED_METADATA_MVMCON"
log "     Desired metadata (hub): $DESIRED_METADATA_HUB"
log "     Solver address: $RETRIEVED_SOLVER"

SOLVER_SIGNATURE_HEX="${RETRIEVED_SIGNATURE#0x}"
aptos move run --profile requester-chain1 --assume-yes \
    --function-id "0x${HUB_MODULE_ADDR}::fa_intent_inflow::create_inflow_intent_entry" \
    --args "address:${OFFERED_METADATA_MVMCON}" "u64:${OFFERED_AMOUNT}" "u64:${CONNECTED_CHAIN_ID}" "address:${DESIRED_METADATA_HUB}" "u64:${DESIRED_AMOUNT}" "u64:${HUB_CHAIN_ID}" "u64:${EXPIRY_TIME}" "address:${INTENT_ID}" "address:${RETRIEVED_SOLVER}" "address:${SOLVER_MVMCON_ADDR}" "hex:${SOLVER_SIGNATURE_HEX}" "address:${REQUESTER_MVMCON_ADDR}" >> "$LOG_FILE" 2>&1

# ============================================================================
# SECTION 6: VERIFY RESULTS
# ============================================================================
if [ $? -eq 0 ]; then
    log "     ✅ Request-intent created on Hub!"

    sleep 2
    log "     - Verifying intent stored on-chain..."
    HUB_INTENT_ADDR=$(curl -s "http://127.0.0.1:8080/v1/accounts/${REQUESTER_HUB_ADDR}/transactions?limit=1" | \
        jq -r '.[0].events[] | select(.type | contains("LimitOrderEvent")) | .data.intent_addr' | head -n 1)

    if [ -n "$HUB_INTENT_ADDR" ] && [ "$HUB_INTENT_ADDR" != "null" ]; then
        log "     ✅ Hub intent stored at: $HUB_INTENT_ADDR"
        log_and_echo "✅ Request-intent created (via coordinator/integrated-gmp negotiation)"
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

# ============================================================================
# SECTION 7: FINAL SUMMARY
# ============================================================================
log ""
display_balances_hub "0x$TEST_TOKENS_HUB"
display_balances_connected_mvm "0x$USD_MVMCON_MODULE_ADDR"
log_and_echo ""

log ""
log " INFLOW - HUB CHAIN INTENT CREATION COMPLETE!"
log "================================================"
log ""
log "✅ Steps completed successfully (via coordinator-based negotiation):"
log "   1. Solver registered on-chain"
log "   2. Requester submitted draft intent to coordinator"
log "   3. Solver service signed draft automatically (FCFS)"
log "   4. Requester polled coordinator and retrieved signature"
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


