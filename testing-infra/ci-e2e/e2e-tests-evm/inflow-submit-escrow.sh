#!/bin/bash

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/../util_evm.sh"
source "$SCRIPT_DIR/../chain-connected-evm/utils.sh"

# Setup project root and logging
setup_project_root
cd "$PROJECT_ROOT"

# Load EVM instance vars
evm_instance_vars "${EVM_INSTANCE:-2}"
source "$EVM_CHAIN_INFO_FILE" 2>/dev/null || true

setup_logging "inflow-submit-escrow-evm${EVM_INSTANCE}"

# ============================================================================
# SECTION 1: LOAD DEPENDENCIES
# ============================================================================
if ! load_intent_info "INTENT_ID"; then
    exit 1
fi

# ============================================================================
# SECTION 2: GET ADDRESSES AND CONFIGURATION
# ============================================================================

# Contract addresses loaded from chain-info-evm${EVM_INSTANCE}.env above
ESCROW_GMP_ADDR="${ESCROW_GMP_ADDR:-}"
USD_EVM_ADDR="${USD_EVM_ADDR:-}"
TEST_TOKENS_HUB=$(get_profile_address "test-tokens-chain1")
REQUESTER_EVM_ADDR=$(get_hardhat_account_address "1")
SOLVER_EVM_ADDR=$(get_hardhat_account_address "2")

if [ -z "$ESCROW_GMP_ADDR" ]; then
    log_and_echo "❌ ERROR: ESCROW_GMP_ADDR not found. Please ensure GMP contracts are deployed."
    log_and_echo "   Run: ./testing-infra/ci-e2e/chain-connected-evm/deploy-contracts.sh"
    exit 1
fi

if [ -z "$USD_EVM_ADDR" ]; then
    log_and_echo "❌ ERROR: USD_EVM_ADDR not found. Please ensure USDcon is deployed."
    exit 1
fi

log ""
log " Chain Information:"
log "   EVM Chain (Chain 3):    IntentInflowEscrow at $ESCROW_GMP_ADDR"
log "   Intent ID:              $INTENT_ID"
log "   Requester EVM:          $REQUESTER_EVM_ADDR"
log "   Solver EVM:             $SOLVER_EVM_ADDR"

EXPIRY_TIME=$(date -d "+180 seconds" +%s)

log ""
log " Configuration:"
log "   Expiry time: $EXPIRY_TIME"
log "   Intent ID (for escrow): $INTENT_ID"
log "   USDcon token address: $USD_EVM_ADDR"
log "   Escrow amount: 1 USDcon (matches intent offered_amount)"

# ============================================================================
# SECTION 3: DISPLAY INITIAL STATE
# ============================================================================
log ""
display_balances_hub "0x$TEST_TOKENS_HUB"
display_balances_connected_evm "$USD_EVM_ADDR"
log_and_echo ""

# ============================================================================
# SECTION 4: WAIT FOR GMP DELIVERY OF INTENT REQUIREMENTS
# ============================================================================
log ""
log "   Waiting for GMP relay to deliver IntentRequirements to EVM chain..."
log "   (Hub intent creation sends requirements via GMP - relay must deliver them first)"

# Convert intent ID to EVM format
INTENT_ID_EVM=$(convert_intent_id_to_evm "$INTENT_ID")
log "   Intent ID (EVM): $INTENT_ID_EVM"

GMP_DELIVERED=0
for attempt in $(seq 1 30); do
    HAS_REQ=$(has_requirements "$ESCROW_GMP_ADDR" "$INTENT_ID_EVM" 2>/dev/null || echo "false")
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
    log_and_echo "   The GMP relay failed to deliver requirements from hub to connected EVM chain."
    display_service_logs "GMP delivery timeout"
    exit 1
fi

# ============================================================================
# SECTION 5: EXECUTE MAIN OPERATION
# ============================================================================
log ""
log "   Creating escrow on EVM connected chain..."
log "   - Requester locks 1 USDcon in escrow on EVM"
log "   - Using intent_id from hub chain: $INTENT_ID"

# DEBUG: Check requester balance BEFORE escrow creation
log ""
log "   DEBUG: Checking requester balance BEFORE escrow creation..."
BEFORE_BALANCE=$(get_usdcon_balance_evm "$REQUESTER_EVM_ADDR" "$USD_EVM_ADDR")
log_and_echo "   DEBUG: Requester USDcon balance BEFORE escrow: $BEFORE_BALANCE"

log "   - Creating escrow intent on connected EVM..."
log "     IntentInflowEscrow: $ESCROW_GMP_ADDR"
log "     Token: $USD_EVM_ADDR"
log "     Amount: 1000000 (1 USDcon)"

# Use GMP-based escrow: validates against IntentRequirements received from hub
USDCON_AMOUNT="1000000"  # 1 USDcon = 1_000_000 (6 decimals)
ESCROW_OUTPUT=$(run_hardhat_command "npx hardhat run scripts/create-escrow-gmp.js --network $EVM_NETWORK" "ESCROW_GMP_ADDR='$ESCROW_GMP_ADDR' TOKEN_ADDR='$USD_EVM_ADDR' INTENT_ID_EVM='$INTENT_ID_EVM' AMOUNT='$USDCON_AMOUNT'" 2>&1 | tee -a "$LOG_FILE")
ESCROW_EXIT_CODE=$?

log "   DEBUG: Escrow transaction output:"
log "$ESCROW_OUTPUT"

# ============================================================================
# SECTION 6: VERIFY RESULTS
# ============================================================================
if [ $ESCROW_EXIT_CODE -eq 0 ]; then
    log "     ✅ Escrow intent created on connected EVM!"

    # DEBUG: Check requester balance AFTER escrow creation
    log ""
    log "   DEBUG: Checking requester balance AFTER escrow creation..."
    AFTER_BALANCE=$(get_usdcon_balance_evm "$REQUESTER_EVM_ADDR" "$USD_EVM_ADDR")
    log_and_echo "   DEBUG: Requester USDcon balance AFTER escrow: $AFTER_BALANCE"

    if [ "$BEFORE_BALANCE" = "$AFTER_BALANCE" ]; then
        log_and_echo "   WARNING: Requester balance did NOT change after escrow creation!"
        log_and_echo "      Before: $BEFORE_BALANCE, After: $AFTER_BALANCE"
    else
        DIFF=$((BEFORE_BALANCE - AFTER_BALANCE))
        log_and_echo "   ✅ Requester balance decreased by: $DIFF (locked in escrow)"
    fi

    # Verify escrow created by checking for success message or event
    if echo "$ESCROW_OUTPUT" | grep -qi "Escrow created"; then
        log "     ✅ EscrowCreated event confirmed"
    else
        log "     ️ Could not verify EscrowCreated event in output"
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
display_balances_connected_evm "$USD_EVM_ADDR"
log_and_echo ""

log ""
log " INFLOW - ESCROW CREATION COMPLETE!"
log "======================================"
log ""
log "✅ Step completed successfully:"
log "   1. Escrow created on connected EVM with locked tokens (via GMP validation)"
log ""
log " Escrow Details:"
log "   Intent ID: $INTENT_ID"
log "   IntentInflowEscrow: $ESCROW_GMP_ADDR"
log "   Token: $USD_EVM_ADDR"
log "   Amount: 1 USDcon (1000000)"
