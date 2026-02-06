#!/bin/bash

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_evm.sh"
source "$SCRIPT_DIR/utils.sh"

# Setup project root and logging
setup_project_root
setup_logging "deploy-contract"
cd "$PROJECT_ROOT"

log " EVM CHAIN - DEPLOY GMP CONTRACTS"
log "==================================="
log_and_echo " All output logged to: $LOG_FILE"

log ""
log " Deploying GMP contracts to EVM chain..."
log "============================================="

# Check if Hardhat node is running
if ! check_evm_chain_running; then
    log_and_echo "❌ Hardhat node is not running. Please run testing-infra/ci-e2e/chain-connected-evm/setup-chain.sh first"
    exit 1
fi

log ""
log " Configuration:"

# Load hub module address for trusted remote configuration
source "$PROJECT_ROOT/.tmp/chain-info.env" 2>/dev/null || true

if [ -z "$HUB_MODULE_ADDR" ]; then
    log_and_echo "❌ ERROR: HUB_MODULE_ADDR not found in chain-info.env"
    log_and_echo "   Please deploy hub chain first: ./testing-infra/ci-e2e/chain-hub/deploy-contracts.sh"
    exit 1
fi

# Convert hub address to 32-byte hex for GMP (pad with leading zeros if needed)
HUB_ADDR_CLEAN=$(echo "$HUB_MODULE_ADDR" | sed 's/^0x//')
# Pad to 64 hex characters (32 bytes)
TRUSTED_HUB_ADDR=$(printf "0x%064s" "$HUB_ADDR_CLEAN" | tr ' ' '0')

log "   Hub Module Address: $HUB_MODULE_ADDR"
log "   Trusted Hub Address (32 bytes): $TRUSTED_HUB_ADDR"

# Load trusted-gmp keys for relay authorization
load_trusted_gmp_keys

# Get trusted-gmp Ethereum address (relay address)
TEMP_CONFIG="$PROJECT_ROOT/.tmp/trusted-gmp-minimal.toml"
mkdir -p "$(dirname "$TEMP_CONFIG")"
cat > "$TEMP_CONFIG" << 'TMPEOF'
[hub_chain]
name = "placeholder"
rpc_url = "http://127.0.0.1:8080"
chain_id = 1
intent_module_addr = "0x1"

[trusted_gmp]
private_key_env = "E2E_TRUSTED_GMP_PRIVATE_KEY"
public_key_env = "E2E_TRUSTED_GMP_PUBLIC_KEY"
polling_interval_ms = 2000
validation_timeout_ms = 30000

[api]
host = "127.0.0.1"
port = 3334
cors_origins = []
TMPEOF

export TRUSTED_GMP_CONFIG_PATH="$TEMP_CONFIG"
CONFIG_PATH="$TRUSTED_GMP_CONFIG_PATH"

# Use pre-built binary (must be built in Step 1)
GET_APPROVER_ETH_BIN="$PROJECT_ROOT/trusted-gmp/target/debug/get_approver_eth_address"
if [ ! -x "$GET_APPROVER_ETH_BIN" ]; then
    log_and_echo "❌ PANIC: get_approver_eth_address not built. Step 1 (build binaries) failed."
    exit 1
fi

APPROVER_ETH_OUTPUT=$(cd "$PROJECT_ROOT" && env HOME="${HOME}" TRUSTED_GMP_CONFIG_PATH="$CONFIG_PATH" "$GET_APPROVER_ETH_BIN" 2>&1 | tee -a "$LOG_FILE")
RELAY_ETH_ADDRESS=$(echo "$APPROVER_ETH_OUTPUT" | grep -E '^0x[a-fA-F0-9]{40}$' | head -1 | tr -d '\n')

if [ -z "$RELAY_ETH_ADDRESS" ]; then
    log_and_echo "❌ ERROR: Could not compute trusted-gmp EVM address from config"
    log_and_echo "   Command output:"
    echo "$APPROVER_ETH_OUTPUT"
    log_and_echo "   Check that E2E_TRUSTED_GMP_PRIVATE_KEY and E2E_TRUSTED_GMP_PUBLIC_KEY env vars are set"
    exit 1
fi

log "   Relay ETH Address: $RELAY_ETH_ADDRESS"
log "   Hub Chain ID: 1"
log "   RPC URL: http://127.0.0.1:8545"

# Deploy GMP contracts
log ""
log " Deploying GMP contracts..."
DEPLOY_OUTPUT=$(run_hardhat_command "npx hardhat run scripts/deploy-gmp.js --network localhost" "HUB_CHAIN_ID='1' TRUSTED_HUB_ADDR='$TRUSTED_HUB_ADDR' RELAY_ADDRESS='$RELAY_ETH_ADDRESS'" 2>&1 | tee -a "$LOG_FILE")

# Extract contract addresses from output
GMP_ENDPOINT_ADDR=$(echo "$DEPLOY_OUTPUT" | grep "IntentGmp:" | awk '{print $NF}' | tr -d '\n')
ESCROW_GMP_ADDR=$(echo "$DEPLOY_OUTPUT" | grep "IntentInflowEscrow:" | awk '{print $NF}' | tr -d '\n')
OUTFLOW_VALIDATOR_ADDR=$(echo "$DEPLOY_OUTPUT" | grep "IntentOutflowValidator:" | awk '{print $NF}' | tr -d '\n')

if [ -z "$GMP_ENDPOINT_ADDR" ] || [ -z "$ESCROW_GMP_ADDR" ] || [ -z "$OUTFLOW_VALIDATOR_ADDR" ]; then
    log_and_echo "❌ GMP contract deployment failed!"
    log_and_echo "   Deployment output:"
    echo "$DEPLOY_OUTPUT"
    exit 1
fi

log ""
log "✅ GMP contracts deployed successfully!"
log "   IntentGmp: $GMP_ENDPOINT_ADDR"
log "   IntentInflowEscrow: $ESCROW_GMP_ADDR"
log "   IntentOutflowValidator: $OUTFLOW_VALIDATOR_ADDR"

# Deploy USDcon token
log ""
log " Deploying USDcon token to EVM chain..."

USDCON_OUTPUT=$(run_hardhat_command "npx hardhat run test-scripts/deploy-usdcon.js --network localhost" 2>&1 | tee -a "$LOG_FILE")
# Extract token address from Hardhat output (line containing 'deployed to:')
USD_EVM_ADDR=$(echo "$USDCON_OUTPUT" | grep "deployed to:" | awk '{print $NF}' | tr -d '\n')

if [ -z "$USD_EVM_ADDR" ]; then
    log_and_echo "❌ USDcon deployment failed!"
    exit 1
fi

log "   ✅ USDcon deployed to: $USD_EVM_ADDR"

# Save contract addresses for other scripts
echo "GMP_ENDPOINT_ADDR=$GMP_ENDPOINT_ADDR" >> "$PROJECT_ROOT/.tmp/chain-info.env"
echo "ESCROW_GMP_ADDR=$ESCROW_GMP_ADDR" >> "$PROJECT_ROOT/.tmp/chain-info.env"
echo "OUTFLOW_VALIDATOR_ADDR=$OUTFLOW_VALIDATOR_ADDR" >> "$PROJECT_ROOT/.tmp/chain-info.env"
echo "USD_EVM_ADDR=$USD_EVM_ADDR" >> "$PROJECT_ROOT/.tmp/chain-info.env"

# Mint USDcon to Requester and Solver (accounts 1 and 2)
log ""
log " Minting USDcon to Requester and Solver on EVM chain..."

REQUESTER_EVM_ADDR=$(get_hardhat_account_address "1")
SOLVER_EVM_ADDR=$(get_hardhat_account_address "2")
USDCON_MINT_AMOUNT="1000000"  # 1 USDcon (6 decimals = 1_000_000)

log "   - Minting $USDCON_MINT_AMOUNT 10e-6.USDcon to Requester ($REQUESTER_EVM_ADDR)..."
MINT_OUTPUT=$(run_hardhat_command "npx hardhat run scripts/mint-token.js --network localhost" "TOKEN_ADDR='$USD_EVM_ADDR' RECIPIENT='$REQUESTER_EVM_ADDR' AMOUNT='$USDCON_MINT_AMOUNT'" 2>&1 | tee -a "$LOG_FILE")
if echo "$MINT_OUTPUT" | grep -q "SUCCESS"; then
    log "   ✅ Minted USDcon to Requester"
else
    log_and_echo "   ❌ Failed to mint USDcon to Requester"
    exit 1
fi

log "   - Minting $USDCON_MINT_AMOUNT 10e-6.USDcon to Solver ($SOLVER_EVM_ADDR)..."
MINT_OUTPUT=$(run_hardhat_command "npx hardhat run scripts/mint-token.js --network localhost" "TOKEN_ADDR='$USD_EVM_ADDR' RECIPIENT='$SOLVER_EVM_ADDR' AMOUNT='$USDCON_MINT_AMOUNT'" 2>&1 | tee -a "$LOG_FILE")
if echo "$MINT_OUTPUT" | grep -q "SUCCESS"; then
    log "   ✅ Minted USDcon to Solver"
else
    log_and_echo "   ❌ Failed to mint USDcon to Solver"
    exit 1
fi

log_and_echo "✅ USDcon minted to Requester and Solver on EVM chain"

# Configure hub chain to trust EVM connected chain
log ""
log " Configuring hub chain to trust EVM connected chain..."

# Get the EVM chain's "address" for hub trust config
# For EVM, we use the IntentGmp contract address as the trusted remote
# (IntentGmp is the GMP endpoint that sends/receives cross-chain messages)
GMP_ENDPOINT_ADDR_CLEAN=$(echo "$GMP_ENDPOINT_ADDR" | sed 's/^0x//')
# Pad to 64 hex characters (32 bytes)
GMP_ENDPOINT_ADDR_PADDED=$(printf "%064s" "$GMP_ENDPOINT_ADDR_CLEAN" | tr ' ' '0')

# Set trusted remote on hub for connected EVM chain (chain_id=3)
if aptos move run --profile intent-account-chain1 --assume-yes \
    --function-id ${HUB_MODULE_ADDR}::intent_gmp::set_trusted_remote \
    --args u32:3 "hex:${GMP_ENDPOINT_ADDR_PADDED}" >> "$LOG_FILE" 2>&1; then
    log "   ✅ Hub now trusts EVM connected chain (chain_id=3, addr=$GMP_ENDPOINT_ADDR)"
else
    log "   ️ Could not set trusted remote on hub (ignoring)"
fi

# Also set trusted remote in intent_gmp_hub for EVM chain
if aptos move run --profile intent-account-chain1 --assume-yes \
    --function-id ${HUB_MODULE_ADDR}::intent_gmp_hub::set_trusted_remote \
    --args u32:3 "hex:${GMP_ENDPOINT_ADDR_PADDED}" >> "$LOG_FILE" 2>&1; then
    log "   ✅ Hub intent_gmp_hub now trusts EVM connected chain"
else
    log "   ️ Could not set trusted remote in intent_gmp_hub (ignoring)"
fi

# Display balances (ETH + USDcon)
display_balances_connected_evm "$USD_EVM_ADDR"

log ""
log " EVM GMP DEPLOYMENT COMPLETE!"
log "=============================="
log "GMP Contracts:"
log "   IntentGmp: $GMP_ENDPOINT_ADDR"
log "   IntentInflowEscrow: $ESCROW_GMP_ADDR"
log "   IntentOutflowValidator: $OUTFLOW_VALIDATOR_ADDR"
log "EVM Chain:"
log "   RPC URL:  http://127.0.0.1:8545"
log "   Chain ID: 31337"
log "   USDcon Token: $USD_EVM_ADDR"
log "   Relay Address: $RELAY_ETH_ADDRESS"
log ""
log "Configuration:"
log "   Hub Chain ID: 1"
log "   Trusted Hub Address: $TRUSTED_HUB_ADDR"
log ""
log " API Examples:"
log "   Check EVM Chain:    curl -X POST http://127.0.0.1:8545 -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"method\":\"eth_blockNumber\",\"params\":[],\"id\":1}'"
log ""
log " Useful commands:"
log "   Stop EVM chain:  ./testing-infra/ci-e2e/chain-connected-evm/stop-chain.sh"
log ""
log " EVM GMP deployment script completed!"

