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

log " EVM CHAIN - DEPLOY"
log "===================="
log_and_echo " All output logged to: $LOG_FILE"

log ""
log " Deploying IntentEscrow to EVM chain..."
log "============================================="

# Check if Hardhat node is running
if ! check_evm_chain_running; then
    log_and_echo "❌ Hardhat node is not running. Please run testing-infra/ci-e2e/chain-connected-evm/setup-chain.sh first"
    exit 1
fi

log ""
log " Configuration:"
log "   Computing verifier Ethereum address from config..."

# Generate fresh ephemeral keys for CI/E2E testing
load_verifier_keys

# Get verifier Ethereum address (derived from ECDSA public key)
VERIFIER_DIR="$PROJECT_ROOT/trusted-verifier"
export VERIFIER_CONFIG_PATH="$PROJECT_ROOT/trusted-verifier/config/verifier-e2e-ci-testing.toml"
CONFIG_PATH="$VERIFIER_CONFIG_PATH"

# Use pre-built binary (must be built in Step 1)
GET_VERIFIER_ETH_BIN="$PROJECT_ROOT/trusted-verifier/target/debug/get_verifier_eth_address"
if [ ! -x "$GET_VERIFIER_ETH_BIN" ]; then
    log_and_echo "❌ PANIC: get_verifier_eth_address not built. Step 1 (build binaries) failed."
    exit 1
fi

VERIFIER_ETH_OUTPUT=$(cd "$PROJECT_ROOT" && env HOME="${HOME}" VERIFIER_CONFIG_PATH="$CONFIG_PATH" "$GET_VERIFIER_ETH_BIN" 2>&1 | tee -a "$LOG_FILE")
VERIFIER_EVM_PUBKEY_HASH=$(echo "$VERIFIER_ETH_OUTPUT" | grep -E '^0x[a-fA-F0-9]{40}$' | head -1 | tr -d '\n')

if [ -z "$VERIFIER_EVM_PUBKEY_HASH" ]; then
    log_and_echo "❌ ERROR: Could not compute verifier EVM pubkey hash from config"
    log_and_echo "   Command output:"
    echo "$VERIFIER_ETH_OUTPUT"
    log_and_echo "   Check that trusted-verifier/config/verifier-e2e-ci-testing.toml has valid keys"
    exit 1
fi

log "   ✅ Verifier EVM pubkey hash: $VERIFIER_EVM_PUBKEY_HASH"
log "   RPC URL: http://127.0.0.1:8545"

# Deploy escrow contract (run in nix develop)
log ""
log " Deploying IntentEscrow..."
DEPLOY_OUTPUT=$(run_hardhat_command "npx hardhat run scripts/deploy.js --network localhost" "VERIFIER_ADDR='$VERIFIER_EVM_PUBKEY_HASH'" 2>&1 | tee -a "$LOG_FILE")

# Extract contract address from output
CONTRACT_ADDR=$(extract_escrow_contract_address "$DEPLOY_OUTPUT")

log ""
log "✅ IntentEscrow deployed successfully!"
log "   Contract Address: $CONTRACT_ADDR"
log ""
log " Contract Details:"
log "   Network:      localhost"
log "   RPC URL:      http://127.0.0.1:8545"
log "   Chain ID:     31337 (Hardhat default)"
log ""
log " Verify deployment:"
log "   npx hardhat verify --network localhost $CONTRACT_ADDR <verifier_address>"

log ""
log "✅ IntentEscrow deployed"

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

# Save escrow and USDcon addresses for other scripts
echo "ESCROW_CONTRACT_ADDR=$CONTRACT_ADDR" >> "$PROJECT_ROOT/.tmp/chain-info.env"
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

# Display balances (ETH + USDcon)
display_balances_connected_evm "$USD_EVM_ADDR"

log ""
log " EVM DEPLOYMENT COMPLETE!"
log "==========================="
log "EVM Chain:"
log "   RPC URL:  http://127.0.0.1:8545"
log "   Chain ID: 31337"
log "   IntentEscrow: $CONTRACT_ADDR"
log "   USDcon Token: $USD_EVM_ADDR"
log "   Verifier EVM Pubkey Hash: $VERIFIER_EVM_PUBKEY_HASH"
log ""
log " API Examples:"
log "   Check EVM Chain:    curl -X POST http://127.0.0.1:8545 -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"method\":\"eth_blockNumber\",\"params\":[],\"id\":1}'"
log ""
log " Useful commands:"
log "   Stop EVM chain:  ./testing-infra/ci-e2e/chain-connected-evm/stop-chain.sh"
log ""
log " EVM deployment script completed!"

