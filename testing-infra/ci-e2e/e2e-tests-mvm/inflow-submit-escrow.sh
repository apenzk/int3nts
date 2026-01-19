#!/bin/bash

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"

# Setup project root and logging
setup_project_root
setup_logging "submit-escrow"
cd "$PROJECT_ROOT"

# ============================================================================
# SECTION 1: LOAD DEPENDENCIES
# ============================================================================
if ! load_intent_info "INTENT_ID"; then
    exit 1
fi

# ============================================================================
# SECTION 2: GET ADDRES AND CONFIGURATION
# ============================================================================
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

# Load verifier keys (generated during deployment)
load_verifier_keys

# Get public key from environment variable
VERIFIER_PUBLIC_KEY_B64="${E2E_VERIFIER_PUBLIC_KEY}"

if [ -z "$VERIFIER_PUBLIC_KEY_B64" ]; then
    log_and_echo "❌ ERROR: E2E_VERIFIER_PUBLIC_KEY environment variable not set"
    log_and_echo "   The verifier public key is required for escrow creation."
    log_and_echo "   Please ensure E2E_VERIFIER_PUBLIC_KEY is set (generate_verifier_keys should do this)."
    exit 1
fi

ORACLE_PUBLIC_KEY_HEX=$(echo "$VERIFIER_PUBLIC_KEY_B64" | base64 -d 2>/dev/null | xxd -p -c 1000 | tr -d '\n')

if [ -z "$ORACLE_PUBLIC_KEY_HEX" ] || [ ${#ORACLE_PUBLIC_KEY_HEX} -ne 64 ]; then
    log_and_echo "❌ ERROR: Invalid public key format"
    log_and_echo "   Expected: base64-encoded 32-byte Ed25519 public key"
    log_and_echo "   Got: $VERIFIER_PUBLIC_KEY_B64"
    log_and_echo "   Please ensure E2E_VERIFIER_PUBLIC_KEY is valid base64 and decodes to 32 bytes (64 hex chars)."
    exit 1
fi

ORACLE_PUBLIC_KEY="0x${ORACLE_PUBLIC_KEY_HEX}"
EXPIRY_TIME=$(date -d "+1 hour" +%s)
CONNECTED_CHAIN_ID=2
HUB_CHAIN_ID=1

log ""
log " Configuration:"
log "   Verifier public key: $ORACLE_PUBLIC_KEY"
log "   Expiry time: $EXPIRY_TIME"
log "   Intent ID: $INTENT_ID"

log ""
log "   - Getting USDcon metadata on connected MVM..."
USD_MVMCON_ADDR=$(get_usdxyz_metadata_addr "0x$USD_MVMCON_MODULE_ADDR" "2")
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
display_balances_connected_mvm "0x$USD_MVMCON_MODULE_ADDR"
log_and_echo ""

# ============================================================================
# SECTION 4: EXECUTE MAIN OPERATION
# ============================================================================
log ""
log "   Creating escrow on connected chain..."
log "   - Requester locks 1 USDcon in escrow on connected MVM"
log "   - Using intent_id from hub chain: $INTENT_ID"

# DEBUG: Check requester balance BEFORE escrow creation
log ""
log "   DEBUG: Checking requester balance BEFORE escrow creation..."
BEFORE_BALANCE=$(get_usdxyz_balance "requester-chain2" "2" "0x$USD_MVMCON_MODULE_ADDR")
log_and_echo "   DEBUG: Requester USDcon balance BEFORE escrow: $BEFORE_BALANCE"

log "   - Creating escrow intent on connected MVM..."
log "     Offered metadata: $OFFERED_METADATA_MVMCON"
log "     Reserved solver (Connected MVM Solver): $SOLVER_MVMCON_ADDR"

ESCROW_OUTPUT=$(aptos move run --profile requester-chain2 --assume-yes \
    --function-id "0x${MVMCON_MODULE_ADDR}::intent_as_escrow_entry::create_escrow_from_fa" \
    --args "address:${OFFERED_METADATA_MVMCON}" "u64:1000000" "u64:${CONNECTED_CHAIN_ID}" "hex:${ORACLE_PUBLIC_KEY}" "u64:${EXPIRY_TIME}" "address:${INTENT_ID}" "address:${SOLVER_MVMCON_ADDR}" "u64:${HUB_CHAIN_ID}" 2>&1)
ESCROW_EXIT_CODE=$?

log "   DEBUG: Escrow transaction output:"
log "$ESCROW_OUTPUT"

# ============================================================================
# SECTION 5: VERIFY RESULTS
# ============================================================================
if [ $ESCROW_EXIT_CODE -eq 0 ]; then
log "     ✅ Escrow intent created on connected MVM!"

    # DEBUG: Check requester balance AFTER escrow creation
    log ""
    log "   DEBUG: Checking requester balance AFTER escrow creation..."
    AFTER_BALANCE=$(get_usdxyz_balance "requester-chain2" "2" "0x$USD_MVMCON_MODULE_ADDR")
    log_and_echo "   DEBUG: Requester USDcon balance AFTER escrow: $AFTER_BALANCE"
    
    if [ "$BEFORE_BALANCE" = "$AFTER_BALANCE" ]; then
        log_and_echo "   ️  WARNING: Requester balance did NOT change after escrow creation!"
        log_and_echo "      Before: $BEFORE_BALANCE, After: $AFTER_BALANCE"
    else
        DIFF=$((BEFORE_BALANCE - AFTER_BALANCE))
        log_and_echo "   ✅ Requester balance decreased by: $DIFF (locked in escrow)"
    fi

    sleep 4
    log "     - Verifying escrow stored on-chain with locked tokens..."

    # Get full transaction for debugging
    FULL_TX=$(curl -s "http://127.0.0.1:8082/v1/accounts/${REQUESTER_MVMCON_ADDR}/transactions?limit=1")
    
    ESCROW_ADDR=$(echo "$FULL_TX" | jq -r '.[0].events[] | select(.type | contains("OracleLimitOrderEvent")) | .data.intent_addr' | head -n 1)
    ESCROW_INTENT_ID=$(echo "$FULL_TX" | jq -r '.[0].events[] | select(.type | contains("OracleLimitOrderEvent")) | .data.intent_id' | head -n 1)
    LOCKED_AMOUNT=$(echo "$FULL_TX" | jq -r '.[0].events[] | select(.type | contains("OracleLimitOrderEvent")) | .data.offered_amount' | head -n 1)
    DESIRED_AMOUNT=$(echo "$FULL_TX" | jq -r '.[0].events[] | select(.type | contains("OracleLimitOrderEvent")) | .data.desired_amount' | head -n 1)
    
    # Output full event for debugging
    FULL_EVENT=$(echo "$FULL_TX" | jq '.[0].events[] | select(.type | contains("OracleLimitOrderEvent"))')
    log "   DEBUG: Full OracleLimitOrderEvent:"
    log "$FULL_EVENT"

    if [ -z "$ESCROW_ADDR" ] || [ "$ESCROW_ADDR" = "null" ]; then
        log_and_echo "❌ ERROR: Could not verify escrow from events"
        exit 1
    fi

    log "     ✅ Escrow stored at: $ESCROW_ADDR"
    log "     ✅ Intent ID link: $ESCROW_INTENT_ID (should match: $INTENT_ID)"
    log "     ✅ Locked amount: $LOCKED_AMOUNT tokens"
    log "     ✅ Desired amount: $DESIRED_AMOUNT tokens"

    NORMALIZED_INTENT_ID=$(echo "$INTENT_ID" | tr '[:upper:]' '[:lower:]' | sed 's/^0x//' | sed 's/^0*//')
    NORMALIZED_ESCROW_INTENT_ID=$(echo "$ESCROW_INTENT_ID" | tr '[:upper:]' '[:lower:]' | sed 's/^0x//' | sed 's/^0*//')

    [ -z "$NORMALIZED_INTENT_ID" ] && NORMALIZED_INTENT_ID="0"
    [ -z "$NORMALIZED_ESCROW_INTENT_ID" ] && NORMALIZED_ESCROW_INTENT_ID="0"

    if [ "$NORMALIZED_INTENT_ID" = "$NORMALIZED_ESCROW_INTENT_ID" ]; then
        log "     ✅ Intent IDs match - correct cross-chain link!"
    else
        log_and_echo "❌ ERROR: Intent IDs don't match!"
        log_and_echo "   Expected: $INTENT_ID"
        log_and_echo "   Got: $ESCROW_INTENT_ID"
        exit 1
    fi

    if [ "$LOCKED_AMOUNT" = "1000000" ]; then
        log "     ✅ Escrow has correct locked amount (1 USDcon)"
    else
        log_and_echo "❌ ERROR: Escrow has unexpected locked amount: $LOCKED_AMOUNT"
        log_and_echo "   Expected: 100_000_000 (1 USDcon)"
        exit 1
    fi

    log_and_echo "✅ Escrow created"
else
    log_and_echo "❌ Escrow intent creation failed!"
    log_and_echo "   Log file contents:"
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    cat "$LOG_FILE"
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    exit 1
fi

# ============================================================================
# SECTION 6: FINAL SUMMARY
# ============================================================================
log ""
display_balances_hub "0x$TEST_TOKENS_HUB"
display_balances_connected_mvm "0x$USD_MVMCON_MODULE_ADDR"
log_and_echo ""

log ""
log " INFLOW - ESCROW CREATION COMPLETE!"
log "======================================"
log ""
log "✅ Step completed successfully:"
log "   1. Escrow created on connected MVM with locked tokens"
log ""
log " Escrow Details:"
log "   Intent ID: $INTENT_ID"
if [ -n "$ESCROW_ADDR" ] && [ "$ESCROW_ADDR" != "null" ]; then
    log "   Connected MVM Escrow: $ESCROW_ADDR"
    # Save ESCROW_ADDR to intent-info.env for escrow claim verification
    echo "MVMCON_ESCROW_ADDR=$ESCROW_ADDR" >> "$PROJECT_ROOT/.tmp/intent-info.env"
fi


