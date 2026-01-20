#!/bin/bash

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"

# Setup project root and logging
setup_project_root
setup_logging "deploy-contracts-connected"
cd "$PROJECT_ROOT"

log " DEPLOY CONTRACTS - CONNECTED CHAIN (Chain 2)"
log "================================================"
log_and_echo " All output logged to: $LOG_FILE"

log ""
log "️  Configuring Aptos CLI for Chain 2..."

# Clean up any existing profile to ensure fresh address each run
log " Cleaning up existing CLI profile..."
cleanup_aptos_profile "intent-account-chain2" "$LOG_FILE"

# Configure Chain 2 (port 8082)
log "   - Configuring Chain 2 (port 8082)..."
init_aptos_profile "intent-account-chain2" "2" "$LOG_FILE"

log ""
log " Deploying contracts to Chain 2..."
log "   - Getting account address for Chain 2..."
CHAIN2_ADDR=$(get_profile_address "intent-account-chain2")

log "   - Deploying to Chain 2 with address: $CHAIN2_ADDR"
cd intent-frameworks/mvm
aptos move publish --dev --profile intent-account-chain2 --named-addresses mvmt_intent=$CHAIN2_ADDR --assume-yes >> "$LOG_FILE" 2>&1

if [ $? -eq 0 ]; then
    log "   ✅ Chain 2 deployment successful!"
    log_and_echo "✅ Connected chain contracts deployed"
else
    log_and_echo "   ❌ Chain 2 deployment failed!"
    log_and_echo "   Log file contents:"
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    cat "$LOG_FILE"
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    exit 1
fi

cd "$PROJECT_ROOT"

# Initialize fa_intent chain info (required for cross-chain intent detection)
log ""
log " Initializing fa_intent chain info (chain_id=2)..."
aptos move run --profile intent-account-chain2 --assume-yes \
    --function-id ${CHAIN2_ADDR}::fa_intent::initialize \
    --args u64:2 >> "$LOG_FILE" 2>&1

if [ $? -eq 0 ]; then
    log "   ✅ fa_intent chain info initialized (chain_id=2)"
else
    log "   ️  fa_intent chain info may already be initialized (ignoring)"
fi

# Initialize solver registry (idempotent - will fail silently if already initialized)
log ""
log " Initializing solver registry..."
initialize_solver_registry "intent-account-chain2" "$CHAIN2_ADDR" "$LOG_FILE"

# Initialize intent registry (idempotent - will fail silently if already initialized)
log ""
log " Initializing intent registry..."
initialize_intent_registry "intent-account-chain2" "$CHAIN2_ADDR" "$LOG_FILE"

# Deploy USDcon test token
log ""
log " Deploying USDcon test token to Chain 2..."

USD_MVMCON_MODULE_ADDR=$(get_profile_address "test-tokens-chain2")

log "   - Deploying USDcon with address: $USD_MVMCON_MODULE_ADDR"
cd "$PROJECT_ROOT/testing-infra/ci-e2e/test-tokens"
aptos move publish --profile test-tokens-chain2 --named-addresses test_tokens=$USD_MVMCON_MODULE_ADDR --assume-yes >> "$LOG_FILE" 2>&1

if [ $? -eq 0 ]; then
    log "   ✅ USDcon deployment successful on Chain 2!"
    log_and_echo "✅ USDcon test token deployed on connected chain"
else
    log_and_echo "   ❌ USDcon deployment failed on Chain 2!"
    exit 1
fi

cd "$PROJECT_ROOT"

# Export USDcon address for other scripts
echo "USD_MVMCON_MODULE_ADDR=$USD_MVMCON_MODULE_ADDR" >> "$PROJECT_ROOT/.tmp/chain-info.env"
log "   ✅ USDcon address saved: $USD_MVMCON_MODULE_ADDR"

# Mint USDcon to Requester and Solver
log ""
log " Minting USDcon to Requester and Solver on Chain 2..."

REQUESTER_MVMCON_ADDR=$(get_profile_address "requester-chain2")
SOLVER_MVMCON_ADDR=$(get_profile_address "solver-chain2")
USDCON_MINT_AMOUNT="1000000"  # 1 USDcon (6 decimals = 1_000_000)

log "   - Minting $USDCON_MINT_AMOUNT 10e-6.USDcon to Requester ($REQUESTER_MVMCON_ADDR)..."
aptos move run --profile test-tokens-chain2 --assume-yes \
    --function-id ${USD_MVMCON_MODULE_ADDR}::usdxyz::mint \
    --args address:$REQUESTER_MVMCON_ADDR u64:$USDCON_MINT_AMOUNT >> "$LOG_FILE" 2>&1

if [ $? -eq 0 ]; then
    log "   ✅ Minted USDcon to Requester"
else
    log_and_echo "   ❌ Failed to mint USDcon to Requester"
    exit 1
fi

log "   - Minting $USDCON_MINT_AMOUNT 10e-6.USDcon to Solver ($SOLVER_MVMCON_ADDR)..."
aptos move run --profile test-tokens-chain2 --assume-yes \
    --function-id ${USD_MVMCON_MODULE_ADDR}::usdxyz::mint \
    --args address:$SOLVER_MVMCON_ADDR u64:$USDCON_MINT_AMOUNT >> "$LOG_FILE" 2>&1

if [ $? -eq 0 ]; then
    log "   ✅ Minted USDcon to Solver"
else
    log_and_echo "   ❌ Failed to mint USDcon to Solver"
    exit 1
fi

log_and_echo "✅ USDcon minted to Requester and Solver on connected chain (1 USDcon each)"

# Assert balances are correct after minting
assert_usdxyz_balance "requester-chain2" "2" "$USD_MVMCON_MODULE_ADDR" "1000000" "post-mint-requester"
assert_usdxyz_balance "solver-chain2" "2" "$USD_MVMCON_MODULE_ADDR" "1000000" "post-mint-solver"

# Display balances (APT + USDcon)
display_balances_connected_mvm "$USD_MVMCON_MODULE_ADDR"

log ""
log " CONNECTED CHAIN DEPLOYMENT COMPLETE!"
log "========================================"
log " Deployment script completed!"

