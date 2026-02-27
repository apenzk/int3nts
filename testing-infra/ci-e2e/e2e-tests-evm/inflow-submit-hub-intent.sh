#!/bin/bash

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/../util_evm.sh"
source "$SCRIPT_DIR/../chain-connected-evm/utils.sh"

# Setup project root and logging
setup_project_root
setup_logging "inflow-submit-hub-intent-evm"
cd "$PROJECT_ROOT"

# Verify services are running before proceeding
verify_coordinator_running
verify_integrated_gmp_running
verify_solver_running
verify_solver_registered

# Generate a random intent_id that will be used for both hub and escrow
INTENT_ID="0x$(openssl rand -hex 32)"

# EVM mode: CONNECTED_CHAIN_ID=31337 (matches Hardhat default network chain ID)
CONNECTED_CHAIN_ID=31337

# Get addresses
HUB_MODULE_ADDR=$(get_profile_address "intent-account-chain1")
TEST_TOKENS_HUB=$(get_profile_address "test-tokens-chain1")

# Get Requester and Solver addresses on hub
REQUESTER_HUB_ADDR=$(get_profile_address "requester-chain1")
SOLVER_HUB_ADDR=$(get_profile_address "solver-chain1")

# Get Requester address on connected EVM chain (Account 1)
REQUESTER_EVM_ADDR=$(get_hardhat_account_address "1")
if [ -z "$REQUESTER_EVM_ADDR" ]; then
    log_and_echo "❌ ERROR: Failed to get Requester EVM address (Hardhat account 1)"
    log_and_echo "   Make sure Hardhat node is running and chain-connected-evm/utils.sh is available"
    display_service_logs "Missing Requester EVM address for inflow hub intent"
    exit 1
fi

# Get USDcon EVM address
source "$PROJECT_ROOT/.tmp/chain-info.env" 2>/dev/null || true
USD_EVM_ADDR="${USD_EVM_ADDR:-}"

log ""
log " Chain Information:"
log "   Hub Module Address:             $HUB_MODULE_ADDR"
log "   Requester Hub:                  $REQUESTER_HUB_ADDR"
log "   Solver Hub:                     $SOLVER_HUB_ADDR"
log "   Requester EVM (connected):      $REQUESTER_EVM_ADDR"

EXPIRY_TIME=$(date -d "+1 hour" +%s)

# Generate solver signature using helper function
# For cross-chain intents: offered tokens are on connected chain, desired tokens are on hub
OFFERED_AMOUNT="1000000"  # 1 USDcon = 1_000_000 (6 decimals, on EVM connected chain)
DESIRED_AMOUNT="1000000"  # 1 USDhub = 1_000_000 (6 decimals, on hub)
HUB_CHAIN_ID=1
EVM_ADDR="0x0000000000000000000000000000000000000001"

log ""
log " Configuration:"
log "   Intent ID: $INTENT_ID"
log "   Expiry time: $EXPIRY_TIME"
log "   Offered amount: $OFFERED_AMOUNT (1 USDcon on connected EVM chain, Chain 3)"
log "   Desired amount: $DESIRED_AMOUNT (1 USDhub on hub)"

# Check and display initial balances using common function
log ""
display_balances_hub "0x$TEST_TOKENS_HUB"
display_balances_connected_evm "$USD_EVM_ADDR"
log_and_echo ""

# Get USDhub metadata addresses (hub) and USDcon metadata (connected) as needed
log ""
log "   - Getting USD token metadata addresses..."
log "     Getting USDhub metadata on Hub..."
USDHUB_METADATA_HUB=$(get_usdxyz_metadata_addr "0x$TEST_TOKENS_HUB" "1")
log "     ✅ Got USDhub metadata on Hub: $USDHUB_METADATA_HUB"

# For EVM inflow: offered token is on EVM chain (connected), desired token is on hub
# Convert 20-byte Ethereum address to 32-byte Move address by padding with zeros
# Lowercase for consistent matching with solver acceptance config
EVM_TOKEN_ADDR_NO_PREFIX="${USD_EVM_ADDR#0x}"
EVM_TOKEN_ADDR_LOWER=$(echo "$EVM_TOKEN_ADDR_NO_PREFIX" | tr '[:upper:]' '[:lower:]')
OFFERED_METADATA_EVM="0x000000000000000000000000${EVM_TOKEN_ADDR_LOWER}"
DESIRED_METADATA_HUB="$USDHUB_METADATA_HUB"
log "     EVM USDcon token address: $USD_EVM_ADDR"
log "     Padded to 32-byte format: $OFFERED_METADATA_EVM"
log "     Inflow configuration:"
log "       Offered metadata (EVM connected chain): $OFFERED_METADATA_EVM"
log "       Desired metadata (hub): $DESIRED_METADATA_HUB"

# ============================================================================
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
    "$OFFERED_METADATA_EVM" \
    "$OFFERED_AMOUNT" \
    "$CONNECTED_CHAIN_ID" \
    "$DESIRED_METADATA_HUB" \
    "$DESIRED_AMOUNT" \
    "$HUB_CHAIN_ID" \
    "$EXPIRY_TIME" \
    "$INTENT_ID" \
    "$REQUESTER_HUB_ADDR" \
    "{\"chain_addr\": \"$HUB_MODULE_ADDR\", \"flow_type\": \"inflow\", \"connected_chain_type\": \"evm\"}")

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
RETRIEVED_SOLVER_EVM=$(echo "$SIGNATURE_DATA" | jq -r '.solver_evm_addr // empty')

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

# Validate solver EVM address for inflow escrow creation
if [ -z "$RETRIEVED_SOLVER_EVM" ] || [ "$RETRIEVED_SOLVER_EVM" = "null" ]; then
    log_and_echo "❌ ERROR: Solver EVM address not in signature response"
    log_and_echo "   The solver must register with an EVM address for inflow intents"
    exit 1
fi
log "     Solver EVM address: $RETRIEVED_SOLVER_EVM"

# Export for escrow creation script
export SOLVER_EVM_ADDR="$RETRIEVED_SOLVER_EVM"

# ============================================================================
# SECTION 5: CREATE INTENT ON-CHAIN WITH RETRIEVED SIGNATURE
# ============================================================================
log ""
log "   Creating cross-chain intent on Hub..."
log "     Offered metadata: $OFFERED_METADATA_EVM"
log "     Desired metadata: $DESIRED_METADATA_HUB"
log "     Solver address: $RETRIEVED_SOLVER"

SOLVER_SIGNATURE_HEX="${RETRIEVED_SIGNATURE#0x}"
# Zero-pad 20-byte EVM address to 32-byte Move address
SOLVER_EVM_RAW="${SOLVER_EVM_ADDR#0x}"
SOLVER_EVM_PADDED="0x000000000000000000000000${SOLVER_EVM_RAW}"
aptos move run --profile requester-chain1 --assume-yes \
    --function-id "0x${HUB_MODULE_ADDR}::fa_intent_inflow::create_inflow_intent_entry" \
    --args "address:${OFFERED_METADATA_EVM}" "u64:${OFFERED_AMOUNT}" "u64:${CONNECTED_CHAIN_ID}" "address:${DESIRED_METADATA_HUB}" "u64:${DESIRED_AMOUNT}" "u64:${HUB_CHAIN_ID}" "u64:${EXPIRY_TIME}" "address:${INTENT_ID}" "address:${RETRIEVED_SOLVER}" "address:${SOLVER_EVM_PADDED}" "hex:${SOLVER_SIGNATURE_HEX}" "address:${REQUESTER_EVM_ADDR}" >> "$LOG_FILE" 2>&1

# ============================================================================
# SECTION 6: VERIFY RESULTS
# ============================================================================
if [ $? -eq 0 ]; then
    log "     ✅ Intent created on Hub!"
    
    # Verify intent was stored on-chain by checking Requester's latest transaction
    sleep 2
    log "     - Verifying intent stored on-chain..."
    HUB_INTENT_ADDR=$(curl -s "http://127.0.0.1:8080/v1/accounts/${REQUESTER_HUB_ADDR}/transactions?limit=1" | \
        jq -r '.[0].events[] | select(.type | contains("LimitOrderEvent")) | .data.intent_addr' | head -n 1)
    
    if [ -n "$HUB_INTENT_ADDR" ] && [ "$HUB_INTENT_ADDR" != "null" ]; then
        log "     ✅ Hub intent stored at: $HUB_INTENT_ADDR"
        log_and_echo "✅ Intent created (via coordinator/integrated-gmp negotiation)"
    else
        log_and_echo "     ❌ ERROR: Could not verify hub intent address"
        exit 1
    fi
else
    log_and_echo "     ❌ Intent creation failed on Hub!"
    log_and_echo "   Log file contents:"
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    cat "$LOG_FILE"
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    # Include service logs (coordinator, integrated-gmp, solver) for easier debugging
    display_service_logs "EVM inflow hub intent creation failed"
    exit 1
fi

# ============================================================================
# SECTION 7: FINAL SUMMARY
# ============================================================================
log ""
log " HUB CHAIN INTENT CREATION COMPLETE!"
log "======================================="
log ""
log "✅ Steps completed successfully (via coordinator-based negotiation):"
log "   1. Solver registered on-chain"
log "   2. Requester submitted draft intent to coordinator"
log "   3. Solver service signed draft automatically (FCFS)"
log "   4. Requester polled coordinator and retrieved signature"
log "   5. Requester created intent on-chain with retrieved signature"
log ""
log " Intent Details:"
log "   Intent ID: $INTENT_ID"
log "   Draft ID: $DRAFT_ID"
log "   Solver: $RETRIEVED_SOLVER"
if [ -n "$HUB_INTENT_ADDR" ] && [ "$HUB_INTENT_ADDR" != "null" ]; then
    log "   Hub Intent: $HUB_INTENT_ADDR"
fi

# Export values for use by other scripts
# Ensure SOLVER_EVM_ADDR is set before saving (re-export to be safe)
if [ -z "$SOLVER_EVM_ADDR" ] && [ -n "$RETRIEVED_SOLVER_EVM" ]; then
    export SOLVER_EVM_ADDR="$RETRIEVED_SOLVER_EVM"
    log "   Re-exported SOLVER_EVM_ADDR: $SOLVER_EVM_ADDR"
fi
save_intent_info "$INTENT_ID" "$HUB_INTENT_ADDR"

# Check final balances using common function
display_balances_hub "0x$TEST_TOKENS_HUB"
display_balances_connected_evm "$USD_EVM_ADDR"
log_and_echo ""

