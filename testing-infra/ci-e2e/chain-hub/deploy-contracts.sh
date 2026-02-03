#!/bin/bash

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"

# Setup project root and logging
setup_project_root
setup_logging "deploy-contracts-hub"
cd "$PROJECT_ROOT"

log " DEPLOY CONTRACTS - HUB"
log "=========================================="
log_and_echo " All output logged to: $LOG_FILE"

log ""
log "️  Configuring Aptos CLI for Hub..."

# Clean up any existing profile to ensure fresh address each run
log " Cleaning up existing CLI profile..."
cleanup_aptos_profile "intent-account-chain1" "$LOG_FILE"

# Configure Hub (port 8080)
log "   - Configuring Hub (port 8080)..."
init_aptos_profile "intent-account-chain1" "1" "$LOG_FILE"

log ""
log " Deploying contracts to Hub..."
log "   - Getting account address for Hub..."
HUB_MODULE_ADDR=$(get_profile_address "intent-account-chain1")

log "   - Deploying to Hub with address: $HUB_MODULE_ADDR"
cd intent-frameworks/mvm
aptos move publish --dev --profile intent-account-chain1 --named-addresses mvmt_intent=$HUB_MODULE_ADDR --assume-yes --chunked-publish >> "$LOG_FILE" 2>&1

if [ $? -eq 0 ]; then
    log "   ✅ Hub deployment successful!"
    log_and_echo "✅ Hub chain contracts deployed"
    # Save hub module address for connected chain to reference
    mkdir -p "$PROJECT_ROOT/.tmp"
    echo "HUB_MODULE_ADDR=$HUB_MODULE_ADDR" >> "$PROJECT_ROOT/.tmp/chain-info.env"
else
    log_and_echo "   ❌ Hub deployment failed!"
    log_and_echo "   Log file contents:"
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    cat "$LOG_FILE"
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    exit 1
fi

cd "$PROJECT_ROOT"

# Initialize fa_intent chain info (required for cross-chain intent detection)
log ""
log " Initializing fa_intent chain info (chain_id=1)..."
aptos move run --profile intent-account-chain1 --assume-yes \
    --function-id ${HUB_MODULE_ADDR}::fa_intent::initialize \
    --args u64:1 >> "$LOG_FILE" 2>&1

if [ $? -eq 0 ]; then
    log "   ✅ fa_intent chain info initialized (chain_id=1)"
else
    log "   ️  fa_intent chain info may already be initialized (ignoring)"
fi

# Initialize solver registry (idempotent - will fail silently if already initialized)
log ""
log " Initializing solver registry..."
initialize_solver_registry "intent-account-chain1" "$HUB_MODULE_ADDR" "$LOG_FILE"

# Initialize intent registry (idempotent - will fail silently if already initialized)
log ""
log " Initializing intent registry..."
initialize_intent_registry "intent-account-chain1" "$HUB_MODULE_ADDR" "$LOG_FILE"

# Initialize trusted-gmp config for outflow intents (uses trusted-gmp keys)
log ""
log " Initializing trusted-gmp config for outflow intents..."
load_trusted_gmp_keys
TRUSTED_GMP_PUBLIC_KEY_HEX=$(echo "$E2E_TRUSTED_GMP_PUBLIC_KEY" | base64 -d 2>/dev/null | xxd -p -c 1000 | tr -d '\n')
aptos move run --profile intent-account-chain1 --assume-yes \
    --function-id ${HUB_MODULE_ADDR}::fa_intent_outflow::initialize_approver \
    --args "hex:${TRUSTED_GMP_PUBLIC_KEY_HEX}" >> "$LOG_FILE" 2>&1

if [ $? -eq 0 ]; then
    log "   ✅ Trusted-gmp (approver) config initialized"
else
    log_and_echo "   ❌ Failed to initialize trusted-gmp config"
    exit 1
fi

# Initialize native GMP endpoint for cross-chain messaging
log ""
log " Initializing native GMP endpoint..."
aptos move run --profile intent-account-chain1 --assume-yes \
    --function-id ${HUB_MODULE_ADDR}::native_gmp_endpoint::initialize >> "$LOG_FILE" 2>&1

if [ $? -eq 0 ]; then
    log "   ✅ Native GMP endpoint initialized"
else
    log "   ️ Native GMP endpoint may already be initialized (ignoring)"
fi

# Initialize intent GMP hub for cross-chain intent messaging
log ""
log " Initializing intent GMP hub..."
aptos move run --profile intent-account-chain1 --assume-yes \
    --function-id ${HUB_MODULE_ADDR}::intent_gmp_hub::initialize >> "$LOG_FILE" 2>&1

if [ $? -eq 0 ]; then
    log "   ✅ Intent GMP hub initialized"
else
    log "   ️ Intent GMP hub may already be initialized (ignoring)"
fi

# Fund the relay address and add as authorized relay
log ""
log " Setting up native GMP relay authorization..."

# Get the relay's Move address from trusted-gmp keys
if [ -z "$E2E_TRUSTED_GMP_MOVE_ADDRESS" ]; then
    load_trusted_gmp_keys
fi

if [ -n "$E2E_TRUSTED_GMP_MOVE_ADDRESS" ]; then
    RELAY_ADDRESS="$E2E_TRUSTED_GMP_MOVE_ADDRESS"
    log "   Relay address: $RELAY_ADDRESS"

    # Fund the relay address (transfer APT from deployer)
    log "   - Funding relay address with APT..."
    aptos account fund-with-faucet --profile intent-account-chain1 --account "$RELAY_ADDRESS" >> "$LOG_FILE" 2>&1

    if [ $? -eq 0 ]; then
        log "   ✅ Relay address funded"
    else
        log "   ️ Could not fund relay (may need manual funding)"
    fi

    # Add relay as authorized relay in native_gmp_endpoint
    log "   - Adding relay as authorized in native_gmp_endpoint..."
    aptos move run --profile intent-account-chain1 --assume-yes \
        --function-id ${HUB_MODULE_ADDR}::native_gmp_endpoint::add_authorized_relay \
        --args address:${RELAY_ADDRESS} >> "$LOG_FILE" 2>&1

    if [ $? -eq 0 ]; then
        log "   ✅ Relay added as authorized"
    else
        log "   ️ Could not add relay (may already be authorized)"
    fi
else
    log "   ️ WARNING: E2E_TRUSTED_GMP_MOVE_ADDRESS not set, skipping relay setup"
fi

# Deploy USDhub test token
log ""
log " Deploying USDhub test token to Hub..."

TEST_TOKENS_HUB_ADDR=$(get_profile_address "test-tokens-chain1")

log "   - Deploying USDhub with address: $TEST_TOKENS_HUB_ADDR"
cd "$PROJECT_ROOT/testing-infra/ci-e2e/test-tokens"
aptos move publish --profile test-tokens-chain1 --named-addresses test_tokens=$TEST_TOKENS_HUB_ADDR --assume-yes >> "$LOG_FILE" 2>&1

if [ $? -eq 0 ]; then
    log "   ✅ USDhub deployment successful on Hub!"
    log_and_echo "✅ USDhub test token deployed on hub chain"
else
    log_and_echo "   ❌ USDhub deployment failed on Hub!"
    exit 1
fi

cd "$PROJECT_ROOT"

# Export USDhub address for other scripts (cleanup deletes this file, so append is safe - creates file if it doesn't exist)
echo "TEST_TOKENS_HUB_ADDR=$TEST_TOKENS_HUB_ADDR" >> "$PROJECT_ROOT/.tmp/chain-info.env"
log "   ✅ USDhub address saved: $TEST_TOKENS_HUB_ADDR"

# Mint USDhub to Requester and Solver
log ""
log " Minting USDhub to Requester and Solver on Hub..."

REQUESTER_HUB_ADDR=$(get_profile_address "requester-chain1")
SOLVER_HUB_ADDR=$(get_profile_address "solver-chain1")
USDHUB_MINT_AMOUNT="1000000"  # 1 USDhub (6 decimals = 1_000_000)

log "   - Minting $USDHUB_MINT_AMOUNT 10e-6.USDhub to Requester ($REQUESTER_HUB_ADDR)..."
aptos move run --profile test-tokens-chain1 --assume-yes \
    --function-id ${TEST_TOKENS_HUB_ADDR}::usdxyz::mint \
    --args address:$REQUESTER_HUB_ADDR u64:$USDHUB_MINT_AMOUNT >> "$LOG_FILE" 2>&1

if [ $? -eq 0 ]; then
    log "   ✅ Minted USDhub to Requester"
else
    log_and_echo "   ❌ Failed to mint USDhub to Requester"
    exit 1
fi

log "   - Minting $USDHUB_MINT_AMOUNT 10e-6.USDhub to Solver ($SOLVER_HUB_ADDR)..."
aptos move run --profile test-tokens-chain1 --assume-yes \
    --function-id ${TEST_TOKENS_HUB_ADDR}::usdxyz::mint \
    --args address:$SOLVER_HUB_ADDR u64:$USDHUB_MINT_AMOUNT >> "$LOG_FILE" 2>&1

if [ $? -eq 0 ]; then
    log "   ✅ Minted USDhub to Solver"
else
    log_and_echo "   ❌ Failed to mint USDhub to Solver"
    exit 1
fi

log_and_echo "✅ USDhub minted to Requester and Solver on hub chain (1 USDhub each)"

# Display balances (APT + USDhub)
display_balances_hub "$TEST_TOKENS_HUB_ADDR"

log ""
log " HUB CHAIN DEPLOYMENT COMPLETE!"
log "=================================="
log " Deployment script completed!"

