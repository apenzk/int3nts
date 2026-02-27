#!/bin/bash
set -e

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

# Verify chain is ready before deployment (exits on failure)
log ""
log "⏳ Verifying Chain 2 is ready..."
wait_for_mvm_chain_ready "2"

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

# Deploy intent-gmp package first (base layer)
log "   - Deploying intent-gmp to Chain 2 with address: $CHAIN2_ADDR"
cd intent-frameworks/mvm/intent-gmp
if aptos move publish --dev --profile intent-account-chain2 --named-addresses mvmt_intent=$CHAIN2_ADDR --assume-yes --included-artifacts none --max-gas 500000 --gas-unit-price 100 >> "$LOG_FILE" 2>&1; then
    log "   ✅ intent-gmp deployment successful!"
else
    log_and_echo "   ❌ intent-gmp deployment failed!"
    log_and_echo "   Log file contents:"
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    cat "$LOG_FILE"
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    exit 1
fi

# Deploy intent-connected package (depends on intent-gmp)
log "   - Deploying intent-connected to Chain 2 with address: $CHAIN2_ADDR"
cd ../intent-connected
if aptos move publish --dev --profile intent-account-chain2 --named-addresses mvmt_intent=$CHAIN2_ADDR --assume-yes --included-artifacts none --max-gas 500000 --gas-unit-price 100 >> "$LOG_FILE" 2>&1; then
    log "   ✅ intent-connected deployment successful!"
    log_and_echo "✅ Connected chain contracts deployed"
else
    log_and_echo "   ❌ intent-connected deployment failed!"
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
if aptos move run --profile intent-account-chain2 --assume-yes \
    --function-id ${CHAIN2_ADDR}::fa_intent::initialize \
    --args u64:2 >> "$LOG_FILE" 2>&1; then
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

# Initialize integrated GMP endpoint for cross-chain messaging
log ""
log " Initializing integrated GMP endpoint..."
if aptos move run --profile intent-account-chain2 --assume-yes \
    --function-id ${CHAIN2_ADDR}::intent_gmp::initialize >> "$LOG_FILE" 2>&1; then
    log "   ✅ Integrated GMP endpoint initialized"
else
    log "   ️ Integrated GMP endpoint may already be initialized (ignoring)"
fi

# Initialize GMP intent state for cross-chain intent tracking
log ""
log " Initializing GMP intent state..."
if aptos move run --profile intent-account-chain2 --assume-yes \
    --function-id ${CHAIN2_ADDR}::gmp_intent_state::initialize >> "$LOG_FILE" 2>&1; then
    log "   ✅ GMP intent state initialized"
else
    log "   ️ GMP intent state may already be initialized (ignoring)"
fi

# Initialize GMP sender for outbound cross-chain messaging
log ""
log " Initializing GMP sender..."
if aptos move run --profile intent-account-chain2 --assume-yes \
    --function-id ${CHAIN2_ADDR}::gmp_sender::initialize >> "$LOG_FILE" 2>&1; then
    log "   ✅ GMP sender initialized"
else
    log "   ️ GMP sender may already be initialized (ignoring)"
fi

# Load hub module address for remote GMP endpoint configuration
source "$PROJECT_ROOT/.tmp/chain-info.env" 2>/dev/null || true

if [ -n "$HUB_MODULE_ADDR" ]; then
    # Convert hub address to 32-byte hex for GMP (pad with leading zeros if needed)
    HUB_ADDR_CLEAN=$(echo "$HUB_MODULE_ADDR" | sed 's/^0x//')
    # Pad to 64 hex characters (32 bytes)
    HUB_ADDR_PADDED=$(printf "%064s" "$HUB_ADDR_CLEAN" | tr ' ' '0')

    # Initialize outflow_validator with hub chain config
    log ""
    log " Initializing outflow validator with hub config..."
    if aptos move run --profile intent-account-chain2 --assume-yes \
        --function-id ${CHAIN2_ADDR}::intent_outflow_validator_impl::initialize \
        --args u32:1 "hex:${HUB_ADDR_PADDED}" >> "$LOG_FILE" 2>&1; then
        log "   ✅ Outflow validator initialized (hub_chain_id=1)"
    else
        log "   ️ Outflow validator may already be initialized (ignoring)"
    fi

    # Initialize inflow_escrow with hub chain config
    log ""
    log " Initializing inflow escrow GMP with hub config..."
    if aptos move run --profile intent-account-chain2 --assume-yes \
        --function-id ${CHAIN2_ADDR}::intent_inflow_escrow::initialize \
        --args u32:1 "hex:${HUB_ADDR_PADDED}" >> "$LOG_FILE" 2>&1; then
        log "   ✅ Inflow escrow GMP initialized (hub_chain_id=1)"
    else
        log "   ️ Inflow escrow GMP may already be initialized (ignoring)"
    fi

    # Set remote GMP endpoint in intent_gmp (trust hub chain)
    log ""
    log " Setting remote GMP endpoint for hub chain..."
    if aptos move run --profile intent-account-chain2 --assume-yes \
        --function-id ${CHAIN2_ADDR}::intent_gmp::set_remote_gmp_endpoint_addr \
        --args u32:1 "hex:${HUB_ADDR_PADDED}" >> "$LOG_FILE" 2>&1; then
        log "   ✅ Remote GMP endpoint set for hub chain"
    else
        log "   ️ Could not set remote GMP endpoint (ignoring)"
    fi
else
    log "   ️ WARNING: HUB_MODULE_ADDR not found, skipping GMP hub config"
fi

# Fund the relay address and add as authorized relay
log ""
log " Setting up integrated GMP relay authorization..."

# Get the relay's Move address from integrated-gmp keys
load_integrated_gmp_keys

if [ -n "$E2E_INTEGRATED_GMP_MOVE_ADDRESS" ]; then
    RELAY_ADDRESS="$E2E_INTEGRATED_GMP_MOVE_ADDRESS"
    log "   Relay address: $RELAY_ADDRESS"

    # Fund the relay address (transfer APT from deployer)
    log "   - Funding relay address with APT..."
    if aptos account fund-with-faucet --profile intent-account-chain2 --account "$RELAY_ADDRESS" >> "$LOG_FILE" 2>&1; then
        log "   ✅ Relay address funded"
    else
        log "   ️ Could not fund relay (may need manual funding)"
    fi

    # Add relay as authorized relay in intent_gmp
    log "   - Adding relay as authorized in intent_gmp..."
    if aptos move run --profile intent-account-chain2 --assume-yes \
        --function-id ${CHAIN2_ADDR}::intent_gmp::add_relay \
        --args address:${RELAY_ADDRESS} >> "$LOG_FILE" 2>&1; then
        log "   ✅ Relay added as authorized"
    else
        log "   ️ Could not add relay (may already be authorized)"
    fi
else
    log_and_echo "   ❌ ERROR: E2E_INTEGRATED_GMP_MOVE_ADDRESS not set after loading keys"
    exit 1
fi

# Deploy USDcon test token
log ""
log " Deploying USDcon test token to Chain 2..."

USD_MVMCON_MODULE_ADDR=$(get_profile_address "test-tokens-chain2")

log "   - Deploying USDcon with address: $USD_MVMCON_MODULE_ADDR"
cd "$PROJECT_ROOT/testing-infra/ci-e2e/test-tokens"
if aptos move publish --profile test-tokens-chain2 --named-addresses test_tokens=$USD_MVMCON_MODULE_ADDR --assume-yes >> "$LOG_FILE" 2>&1; then
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
USDCON_MINT_AMOUNT="2000000"  # 2 USDcon (6 decimals = 2_000_000)

log "   - Minting $USDCON_MINT_AMOUNT 10e-6.USDcon to Requester ($REQUESTER_MVMCON_ADDR)..."
if aptos move run --profile test-tokens-chain2 --assume-yes \
    --function-id ${USD_MVMCON_MODULE_ADDR}::usdxyz::mint \
    --args address:$REQUESTER_MVMCON_ADDR u64:$USDCON_MINT_AMOUNT >> "$LOG_FILE" 2>&1; then
    log "   ✅ Minted USDcon to Requester"
else
    log_and_echo "   ❌ Failed to mint USDcon to Requester"
    exit 1
fi

log "   - Minting $USDCON_MINT_AMOUNT 10e-6.USDcon to Solver ($SOLVER_MVMCON_ADDR)..."
if aptos move run --profile test-tokens-chain2 --assume-yes \
    --function-id ${USD_MVMCON_MODULE_ADDR}::usdxyz::mint \
    --args address:$SOLVER_MVMCON_ADDR u64:$USDCON_MINT_AMOUNT >> "$LOG_FILE" 2>&1; then
    log "   ✅ Minted USDcon to Solver"
else
    log_and_echo "   ❌ Failed to mint USDcon to Solver"
    exit 1
fi

log_and_echo "✅ USDcon minted to Requester and Solver on connected chain (2 USDcon each)"

# Assert balances are correct after minting
assert_usdxyz_balance "requester-chain2" "2" "$USD_MVMCON_MODULE_ADDR" "2000000" "post-mint-requester"
assert_usdxyz_balance "solver-chain2" "2" "$USD_MVMCON_MODULE_ADDR" "2000000" "post-mint-solver"

# Display balances (APT + USDcon)
display_balances_connected_mvm "$USD_MVMCON_MODULE_ADDR"

# Configure hub chain to trust connected chain (for receiving fulfillment proofs)
log ""
log " Configuring hub chain to trust connected chain..."

if [ -n "$HUB_MODULE_ADDR" ]; then
    # Convert connected chain address to 32-byte hex (pad with leading zeros if needed)
    CHAIN2_ADDR_CLEAN=$(echo "$CHAIN2_ADDR" | sed 's/^0x//')
    # Pad to 64 hex characters (32 bytes)
    CHAIN2_ADDR_PADDED=$(printf "%064s" "$CHAIN2_ADDR_CLEAN" | tr ' ' '0')

    # Set remote GMP endpoint on hub for connected chain (chain_id=2)
    if aptos move run --profile intent-account-chain1 --assume-yes \
        --function-id ${HUB_MODULE_ADDR}::intent_gmp::set_remote_gmp_endpoint_addr \
        --args u32:2 "hex:${CHAIN2_ADDR_PADDED}" >> "$LOG_FILE" 2>&1; then
        log "   ✅ Hub now trusts connected chain (chain_id=2)"
    else
        log "   ️ Could not set remote GMP endpoint on hub (ignoring)"
    fi

    # Also set remote GMP endpoint in intent_gmp_hub
    if aptos move run --profile intent-account-chain1 --assume-yes \
        --function-id ${HUB_MODULE_ADDR}::intent_gmp_hub::set_remote_gmp_endpoint_addr \
        --args u32:2 "hex:${CHAIN2_ADDR_PADDED}" >> "$LOG_FILE" 2>&1; then
        log "   ✅ Hub intent_gmp_hub now trusts connected chain"
    else
        log "   ️ Could not set remote GMP endpoint in intent_gmp_hub (ignoring)"
    fi
else
    log "   ️ WARNING: HUB_MODULE_ADDR not found, skipping hub trust config"
fi

log ""
log " CONNECTED CHAIN DEPLOYMENT COMPLETE!"
log "========================================"
log " Deployment script completed!"

