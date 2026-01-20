#!/bin/bash

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/../util_evm.sh"
source "$SCRIPT_DIR/../chain-connected-evm/utils.sh"

# Setup project root and logging
setup_project_root
setup_logging "inflow-submit-escrow"
cd "$PROJECT_ROOT"


# Load INTENT_ID and SOLVER_EVM_ADDR from info file if not provided
if ! load_intent_info "INTENT_ID,SOLVER_EVM_ADDR"; then
    exit 1
fi

# Get EVM escrow contract address from deployment logs
cd intent-frameworks/evm
ESCROW_ADDR=$(grep -i "IntentEscrow deployed to" "$PROJECT_ROOT/.tmp/e2e-tests/deploy-contract"*.log 2>/dev/null | tail -1 | awk '{print $NF}' | tr -d '\n')
if [ -z "$ESCROW_ADDR" ]; then
    # Try to get from hardhat config or last deployment
    ESCROW_ADDR=$(nix develop "$PROJECT_ROOT/nix" -c bash -c "npx hardhat run scripts/deploy.js --network localhost --dry-run 2>&1 | grep 'IntentEscrow deployed to' | awk '{print \$NF}'" 2>/dev/null | tail -1 | tr -d '\n')
fi
cd "$PROJECT_ROOT"

if [ -z "$ESCROW_ADDR" ]; then
    log_and_echo "❌ ERROR: Could not find escrow contract address. Please ensure IntentEscrow is deployed."
    log_and_echo "   Run: ./testing-infra/ci-e2e/chain-connected-evm/deploy-contract.sh"
    exit 1
fi

log ""
log " Chain Information:"
log "   EVM Chain (Chain 3):    $ESCROW_ADDR"
log "   Intent ID:              $INTENT_ID"

EXPIRY_TIME=$(date -d "+1 hour" +%s)

# Get USDcon token address from chain-info.env
if [ -f "$PROJECT_ROOT/.tmp/chain-info.env" ]; then
    source "$PROJECT_ROOT/.tmp/chain-info.env"
    USD_EVM_ADDR="$USD_EVM_ADDR"
fi
if [ -z "$USD_EVM_ADDR" ]; then
    log_and_echo "❌ ERROR: Could not find USDcon token address. Please ensure USDcon is deployed."
    exit 1
fi

# Get test tokens address for balance display
TEST_TOKENS_HUB=$(get_profile_address "test-tokens-chain1")

log ""
log " Configuration:"
log "   Expiry time: $EXPIRY_TIME"
log "   Intent ID (for escrow): $INTENT_ID"
log "   USDcon token address: $USD_EVM_ADDR"
log "   Escrow amount: 1 USDcon (matches intent offered_amount)"

# Check and display initial balances using common function
log ""
display_balances_hub "0x$TEST_TOKENS_HUB"
display_balances_connected_evm "$USD_EVM_ADDR"
log_and_echo ""

log ""
log "   Creating escrow on EVM chain..."
log "   - Requester locks 1 USDcon in escrow on Chain 3 (EVM)"
log "   - Requester provides hub chain intent_id when creating escrow"
log "   - Using intent_id from hub chain: $INTENT_ID"
log "   - Amount matches intent offered_amount"

cd intent-frameworks/evm

# Convert intent_id from Move VM format to EVM uint256
INTENT_ID_EVM=$(convert_intent_id_to_evm "$INTENT_ID")
log "     Intent ID (EVM): $INTENT_ID_EVM"

# Create escrow for this intent with USDcon ERC20 token
log "   - Creating escrow for intent (USDcon ERC20 escrow) with funds..."
# Reserved solver: Must come from verifier (via exported SOLVER_EVM_ADDR)
if [ -z "$SOLVER_EVM_ADDR" ]; then
    log_and_echo "❌ ERROR: SOLVER_EVM_ADDR not set"
    log_and_echo "   The solver must register with an EVM address"
    exit 1
fi
SOLVER_ADDR="$SOLVER_EVM_ADDR"
log "     Using solver EVM address from verifier: $SOLVER_ADDR"
# Escrow amount must match the intent's offered_amount (1 USDcon)
USDCON_AMOUNT="1000000"  # 1 USDcon = 1_000_000 (6 decimals)
CREATE_OUTPUT=$(nix develop "$PROJECT_ROOT/nix" -c bash -c "cd '$PROJECT_ROOT/intent-frameworks/evm' && ESCROW_ADDR='$ESCROW_ADDR' TOKEN_ADDR='$USD_EVM_ADDR' INTENT_ID_EVM='$INTENT_ID_EVM' AMOUNT='$USDCON_AMOUNT' RESERVED_SOLVER='$SOLVER_ADDR' npx hardhat run scripts/create-escrow-e2e-tests.js --network localhost" 2>&1 | tee -a "$LOG_FILE")
CREATE_EXIT_CODE=$?

# Check if creation was successful
if [ $CREATE_EXIT_CODE -ne 0 ]; then
    log_and_echo "     ❌ ERROR: Escrow creation failed!"
    log_and_echo "   Creation output: $CREATE_OUTPUT"
    log_and_echo "   Log file contents:"
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    cat "$LOG_FILE"
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    exit 1
fi

# Verify creation succeeded by checking for success message
if ! echo "$CREATE_OUTPUT" | grep -qi "Escrow created for intent"; then
    log_and_echo "     ❌ ERROR: Escrow creation did not complete successfully"
    log_and_echo "   Creation output: $CREATE_OUTPUT"
    log_and_echo "   Expected to see 'Escrow created for intent (ERC20)' in output"
    exit 1
fi

log "     ✅ Escrow created on Chain 3 (EVM)!"
log_and_echo "✅ Escrow created"

cd "$PROJECT_ROOT"

log ""
log " ESCROW CREATION COMPLETE!"
log "============================"
log ""
log "✅ Step completed successfully:"
log "   1. Escrow created on Chain 3 (EVM) with locked USDcon"
log ""
log " Escrow Details:"
log "   Intent ID: $INTENT_ID"
log "   Escrow Address: $ESCROW_ADDR"
log "   Locked Amount: 1 USDcon (matches intent offered_amount)"

# Check final balances using common function
display_balances_hub "0x$TEST_TOKENS_HUB"
display_balances_connected_evm "$USD_EVM_ADDR"
log_and_echo ""


