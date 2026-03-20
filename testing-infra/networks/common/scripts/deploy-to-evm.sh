#!/bin/bash
# Common EVM deployment script.
# Called by network-specific wrappers that set the required variables:
#
#   EVM_CHAIN_PREFIX      - Variable prefix (BASE, HYPERLIQUID)
#   EVM_RPC_URL           - RPC endpoint URL
#   EVM_DEPLOYER_ADDR     - Deployer address (for RPC check + logs)
#   EVM_HARDHAT_NETWORK   - Hardhat network name (baseSepolia, baseMainnet, hyperliquidMainnet)
#   EVM_DISPLAY_NAME      - Human-readable name ("Base Sepolia Testnet", "HyperEVM Mainnet")
#   EVM_HUB_CHAIN_ID      - Hub chain ID (from get_chain_id)
#   EVM_NETWORK_LABEL     - testnet/mainnet (for config file paths)
#   EVM_CHAIN_LABEL       - Base/HyperEVM (for config section descriptions)
#   EVM_FRONTEND_ESCROW_CONTRACT_ADDR_ENV_VAR  - .env.local key for the escrow contract addr (e.g. NEXT_PUBLIC_BASE_TESTNET_ESCROW_CONTRACT_ADDR)
#   EVM_LOG_PREFIX        - Log file prefix (base-sepolia, base-mainnet, hyperliquid-mainnet)
#   EVM_CHECK_SCRIPT      - Preparedness check script path

set -e

# Resolve indirect variable names from CHAIN_PREFIX
PRIVATE_KEY_VAR="${EVM_CHAIN_PREFIX}_DEPLOYER_PRIVATE_KEY"

echo " Deploying EVM Contracts to ${EVM_DISPLAY_NAME}"
echo "================================================="
echo "   IntentGmp, IntentInflowEscrow, IntentOutflowValidator"
echo ""

# Check required variables
require_var "$PRIVATE_KEY_VAR" "${!PRIVATE_KEY_VAR}"
require_var "INTEGRATED_GMP_EVM_PUBKEY_HASH" "$INTEGRATED_GMP_EVM_PUBKEY_HASH" \
    "Run: nix develop ./nix -c bash -c 'cd integrated-gmp && INTEGRATED_GMP_CONFIG_PATH=config/integrated-gmp_${EVM_NETWORK_LABEL}.toml cargo run --bin get_approver_eth_address'"
require_var "MOVEMENT_INTENT_MODULE_ADDR" "$MOVEMENT_INTENT_MODULE_ADDR" \
    "This should be set to the deployed MVM intent module address"

echo " Configuration:"
echo "   Deployer Address: $EVM_DEPLOYER_ADDR"
echo "   Integrated-GMP EVM Pubkey Hash: $INTEGRATED_GMP_EVM_PUBKEY_HASH"
echo "   Network: ${EVM_DISPLAY_NAME}"
echo "   RPC URL: $EVM_RPC_URL"
echo ""

# Check if Hardhat config exists
if [ ! -f "$PROJECT_ROOT/intent-frameworks/evm/hardhat.config.js" ]; then
    echo "ERROR: hardhat.config.js not found"
    echo "   Make sure intent-frameworks/evm directory exists"
    exit 1
fi

# Change to intent-frameworks/evm directory
cd "$PROJECT_ROOT/intent-frameworks/evm"

# Export environment variables for Hardhat
export DEPLOYER_PRIVATE_KEY="${!PRIVATE_KEY_VAR}"
export APPROVER_ADDR="$INTEGRATED_GMP_EVM_PUBKEY_HASH"
export MOVEMENT_INTENT_MODULE_ADDR
export HUB_CHAIN_ID="$EVM_HUB_CHAIN_ID"
# Relay address for integrated-gmp (derived from ECDSA key, different from deployer)
export RELAY_ADDRESS="${INTEGRATED_GMP_EVM_PUBKEY_HASH}"

echo " Environment configured for Hardhat"
echo ""

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo " Installing dependencies..."
    npm install
    echo "Dependencies installed"
    echo ""
fi

# Verify RPC is responsive before deploying
echo " Checking RPC endpoint: $EVM_RPC_URL"
RPC_RESPONSE=$(curl -s -m 10 -X POST "$EVM_RPC_URL" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_getBalance","params":["'"$EVM_DEPLOYER_ADDR"'","latest"],"id":1}' 2>&1)

if ! echo "$RPC_RESPONSE" | grep -q '"result"'; then
    echo "   RPC endpoint not responding or returned error:"
    echo "   $RPC_RESPONSE"
    exit 1
fi
echo "   RPC OK"
echo ""

# Deploy contracts
echo " Deploying all 3 contracts..."
echo "   (Run this script from within 'nix develop ./nix' shell)"
echo ""
set +e
DEPLOY_OUTPUT=$(npx hardhat run scripts/deploy.js --network "$EVM_HARDHAT_NETWORK" 2>&1)
DEPLOY_EXIT_CODE=$?
set -e

# Show deployment output
echo "$DEPLOY_OUTPUT"

if [ $DEPLOY_EXIT_CODE -ne 0 ]; then
    echo "Deployment failed with exit code $DEPLOY_EXIT_CODE"
    exit 1
fi

echo ""
echo " Deployment Complete!"
echo "======================"
echo ""

# Extract contract addresses from deployment output
GMP_ENDPOINT_ADDR=$(echo "$DEPLOY_OUTPUT" | grep "IntentGmp:" | tail -1 | awk '{print $NF}' | tr -d '\n' || echo "")
ESCROW_ADDR=$(echo "$DEPLOY_OUTPUT" | grep "IntentInflowEscrow:" | tail -1 | awk '{print $NF}' | tr -d '\n' || echo "")
OUTFLOW_ADDR=$(echo "$DEPLOY_OUTPUT" | grep "IntentOutflowValidator:" | tail -1 | awk '{print $NF}' | tr -d '\n' || echo "")

if [ -n "$GMP_ENDPOINT_ADDR" ] && [ -n "$ESCROW_ADDR" ]; then
    echo " Add these to ${ENV_FILE_NAME}:"
    echo ""
    echo "   ${EVM_CHAIN_PREFIX}_GMP_ENDPOINT_ADDR=$GMP_ENDPOINT_ADDR"
    echo "   ${EVM_CHAIN_PREFIX}_INFLOW_ESCROW_ADDR=$ESCROW_ADDR"
    if [ -n "$OUTFLOW_ADDR" ]; then
        echo "   ${EVM_CHAIN_PREFIX}_OUTFLOW_VALIDATOR_ADDR=$OUTFLOW_ADDR"
    fi
    echo ""

    echo " Deployed contract addresses:"
    echo "   IntentGmp (GMP Endpoint):       $GMP_ENDPOINT_ADDR"
    echo "   IntentInflowEscrow:             $ESCROW_ADDR"
    echo "   IntentOutflowValidator:         $OUTFLOW_ADDR"
    echo ""
    echo " Update the following files:"
    echo ""
    echo "   1. coordinator/config/coordinator_${EVM_NETWORK_LABEL}.toml"
    echo "      escrow_contract_addr = \"$ESCROW_ADDR\""
    echo "      (in the [[connected_chain_evm]] ${EVM_CHAIN_LABEL} section)"
    echo ""
    echo "   2. integrated-gmp/config/integrated-gmp_${EVM_NETWORK_LABEL}.toml"
    echo "      escrow_contract_addr = \"$ESCROW_ADDR\""
    echo "      gmp_endpoint_addr = \"$GMP_ENDPOINT_ADDR\""
    echo "      (in the [[connected_chain_evm]] ${EVM_CHAIN_LABEL} section)"
    echo ""
    echo "   3. solver/config/solver_${EVM_NETWORK_LABEL}.toml"
    echo "      escrow_contract_addr = \"$ESCROW_ADDR\""
    echo "      (in the [[connected_chain]] EVM ${EVM_CHAIN_LABEL} section)"
    echo ""
    echo "   4. frontend/.env.local"
    echo "      ${EVM_FRONTEND_ESCROW_CONTRACT_ADDR_ENV_VAR}=$ESCROW_ADDR"
    echo ""
    echo "   5. Run ${EVM_CHECK_SCRIPT} to verify"

    # Save deployment log
    LOG_DIR="$CALLER_SCRIPT_DIR/../logs"
    mkdir -p "$LOG_DIR"
    LOG_FILE="$LOG_DIR/deploy-${EVM_LOG_PREFIX}-$(date +%Y%m%d-%H%M%S).log"
    {
        echo "${EVM_DISPLAY_NAME} Deployment — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
        echo ""
        echo "Deployer:                  $EVM_DEPLOYER_ADDR"
        echo "Relay:                     $INTEGRATED_GMP_EVM_PUBKEY_HASH"
        echo "Hub chain ID:              $HUB_CHAIN_ID"
        echo "Hub module addr:           $MOVEMENT_INTENT_MODULE_ADDR"
        echo ""
        echo "IntentGmp:                 $GMP_ENDPOINT_ADDR"
        echo "IntentInflowEscrow:        $ESCROW_ADDR"
        echo "IntentOutflowValidator:    $OUTFLOW_ADDR"
    } > "$LOG_FILE"
    echo ""
    echo " Deployment log saved to: $LOG_FILE"
else
    echo "Could not extract contract addresses from output"
    echo "   Please copy them manually from the deployment output above"
    echo ""
    echo " Update the following files:"
    echo "   - coordinator/config/coordinator_${EVM_NETWORK_LABEL}.toml (escrow_contract_addr in [[connected_chain_evm]] ${EVM_CHAIN_LABEL} section)"
    echo "   - integrated-gmp/config/integrated-gmp_${EVM_NETWORK_LABEL}.toml (escrow_contract_addr + gmp_endpoint_addr in [[connected_chain_evm]] ${EVM_CHAIN_LABEL} section)"
    echo "   - solver/config/solver_${EVM_NETWORK_LABEL}.toml (escrow_contract_addr in [[connected_chain]] EVM ${EVM_CHAIN_LABEL} section)"
    echo "   - frontend/.env.local (${EVM_FRONTEND_ESCROW_CONTRACT_ADDR_ENV_VAR})"
fi
echo ""
