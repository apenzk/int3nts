#!/bin/bash

# Deploy EVM Intent Contracts to Base Sepolia Testnet
# Deploys all 3 contracts: IntentGmp, IntentInflowEscrow, IntentOutflowValidator
# Reads keys from .env.testnet and deploys/configures all contracts

set -e

# Get the script directory and project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../.." && pwd )"
export PROJECT_ROOT

# Source utilities from testing-infra (for CI testing infrastructure)
source "$PROJECT_ROOT/testing-infra/ci-e2e/util.sh" 2>/dev/null || true

echo " Deploying EVM Contracts to Base Sepolia Testnet"
echo "================================================="
echo "   IntentGmp, IntentInflowEscrow, IntentOutflowValidator"
echo ""

# Load .env.testnet
TESTNET_KEYS_FILE="$SCRIPT_DIR/../.env.testnet"

if [ ! -f "$TESTNET_KEYS_FILE" ]; then
    echo "❌ ERROR: .env.testnet not found at $TESTNET_KEYS_FILE"
    echo "   Create it from env.testnet.example in this directory"
    exit 1
fi

# Source the keys file (skip if parent already exported env)
if [ "${DEPLOY_ENV_SOURCED:-}" != "1" ]; then
    source "$TESTNET_KEYS_FILE"
fi

# Check required variables
if [ -z "$BASE_DEPLOYER_PRIVATE_KEY" ]; then
    echo "❌ ERROR: BASE_DEPLOYER_PRIVATE_KEY not set in .env.testnet"
    exit 1
fi

if [ -z "$INTEGRATED_GMP_EVM_PUBKEY_HASH" ]; then
    echo "❌ ERROR: INTEGRATED_GMP_EVM_PUBKEY_HASH not set in .env.testnet"
    echo "   Run: nix develop ./nix -c bash -c 'cd integrated-gmp && INTEGRATED_GMP_CONFIG_PATH=config/integrated-gmp_testnet.toml cargo run --bin get_approver_eth_address'"
    exit 1
fi

# Load assets configuration
ASSETS_CONFIG_FILE="$PROJECT_ROOT/testing-infra/testnet/config/testnet-assets.toml"

if [ ! -f "$ASSETS_CONFIG_FILE" ]; then
    echo "❌ ERROR: testnet-assets.toml not found at $ASSETS_CONFIG_FILE"
    exit 1
fi

if [ -z "$ALCHEMY_BASE_SEPOLIA_API_KEY" ]; then
    echo "❌ ERROR: ALCHEMY_BASE_SEPOLIA_API_KEY not set in .env.testnet"
    echo "   Get your free API key at: https://www.alchemy.com/"
    exit 1
fi
BASE_SEPOLIA_RPC_URL="https://base-sepolia.g.alchemy.com/v2/${ALCHEMY_BASE_SEPOLIA_API_KEY}"

echo " Configuration:"
echo "   Deployer Address: $BASE_DEPLOYER_ADDR"
echo "   Integrated-GMP EVM Pubkey Hash: $INTEGRATED_GMP_EVM_PUBKEY_HASH"
echo "   Network: Base Sepolia"
echo "   RPC URL: $BASE_SEPOLIA_RPC_URL"
echo ""

# Check if Hardhat config exists
if [ ! -f "$PROJECT_ROOT/intent-frameworks/evm/hardhat.config.js" ]; then
    echo "❌ ERROR: hardhat.config.js not found"
    echo "   Make sure intent-frameworks/evm directory exists"
    exit 1
fi

# Change to intent-frameworks/evm directory
cd "$PROJECT_ROOT/intent-frameworks/evm"

# Check for Movement hub module address
if [ -z "$MOVEMENT_INTENT_MODULE_ADDR" ]; then
    echo "❌ ERROR: MOVEMENT_INTENT_MODULE_ADDR not set in .env.testnet"
    echo "   This should be set to the deployed MVM intent module address"
    echo "   Example: MOVEMENT_INTENT_MODULE_ADDR=0x1b7c806f87339383d29b94fa481a2ea2ef50ac518f66cff419453c9a1154c8da"
    exit 1
fi

# Export environment variables for Hardhat
export DEPLOYER_PRIVATE_KEY="$BASE_DEPLOYER_PRIVATE_KEY"
export APPROVER_ADDR="$INTEGRATED_GMP_EVM_PUBKEY_HASH"
export MOVEMENT_INTENT_MODULE_ADDR
export HUB_CHAIN_ID="${HUB_CHAIN_ID:-250}"  # Movement Bardock testnet chain ID
export BASE_SEPOLIA_RPC_URL
# Relay address for integrated-gmp (derived from ECDSA key, different from deployer)
export RELAY_ADDRESS="${INTEGRATED_GMP_EVM_PUBKEY_HASH}"

echo " Environment configured for Hardhat"
echo ""

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo " Installing dependencies..."
    npm install
    echo "✅ Dependencies installed"
    echo ""
fi

# Verify RPC is responsive before deploying
echo " Checking RPC endpoint: $BASE_SEPOLIA_RPC_URL"
RPC_RESPONSE=$(curl -s -m 10 -X POST "$BASE_SEPOLIA_RPC_URL" \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_getBalance","params":["'"$BASE_DEPLOYER_ADDR"'","latest"],"id":1}' 2>&1)

if ! echo "$RPC_RESPONSE" | grep -q '"result"'; then
    echo "   RPC endpoint not responding or returned error:"
    echo "   $RPC_RESPONSE"
    exit 1
fi
echo "   RPC OK"
echo ""

# Deploy contracts (run from within nix develop ./nix shell)
echo " Deploying all 3 contracts..."
echo "   (Run this script from within 'nix develop ./nix' shell)"
echo ""
set +e
DEPLOY_OUTPUT=$(npx hardhat run scripts/deploy.js --network baseSepolia 2>&1)
DEPLOY_EXIT_CODE=$?
set -e

# Show deployment output
echo "$DEPLOY_OUTPUT"

if [ $DEPLOY_EXIT_CODE -ne 0 ]; then
    echo "❌ Deployment failed with exit code $DEPLOY_EXIT_CODE"
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
    echo " Add these to .env.testnet:"
    echo ""
    echo "   BASE_GMP_ENDPOINT_ADDR=$GMP_ENDPOINT_ADDR"
    echo "   BASE_INFLOW_ESCROW_ADDR=$ESCROW_ADDR"
    if [ -n "$OUTFLOW_ADDR" ]; then
        echo "   BASE_OUTFLOW_VALIDATOR_ADDR=$OUTFLOW_ADDR"
    fi
    echo ""

    echo " Deployed contract addresses:"
    echo "   IntentGmp (GMP Endpoint):       $GMP_ENDPOINT_ADDR"
    echo "   IntentInflowEscrow:             $ESCROW_ADDR"
    echo "   IntentOutflowValidator:         $OUTFLOW_ADDR"
    echo ""
    echo " Update the following files:"
    echo ""
    echo "   1. coordinator/config/coordinator_testnet.toml"
    echo "      escrow_contract_addr = \"$ESCROW_ADDR\""
    echo "      (in the [connected_chain_evm] section)"
    echo ""
    echo "   2. integrated-gmp/config/integrated-gmp_testnet.toml"
    echo "      escrow_contract_addr = \"$ESCROW_ADDR\""
    echo "      gmp_endpoint_addr = \"$GMP_ENDPOINT_ADDR\""
    echo "      (in the [connected_chain_evm] section)"
    echo ""
    echo "   3. solver/config/solver_testnet.toml"
    echo "      escrow_contract_addr = \"$ESCROW_ADDR\""
    echo "      (in the [[connected_chain]] EVM section)"
    echo ""
    echo "   4. frontend/.env.local"
    echo "      NEXT_PUBLIC_BASE_ESCROW_CONTRACT_ADDRESS=$ESCROW_ADDR"
    echo ""
    echo "   5. Run ./testing-infra/testnet/check-testnet-preparedness.sh to verify"

    # Save deployment log
    LOG_DIR="$SCRIPT_DIR/../logs"
    mkdir -p "$LOG_DIR"
    LOG_FILE="$LOG_DIR/deploy-base-sepolia-$(date +%Y%m%d-%H%M%S).log"
    {
        echo "Base Sepolia Deployment — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
        echo ""
        echo "Deployer:                  $BASE_DEPLOYER_ADDR"
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
    echo "️  Could not extract contract addresses from output"
    echo "   Please copy them manually from the deployment output above"
    echo ""
    echo " Update the following files:"
    echo "   - coordinator/config/coordinator_testnet.toml (escrow_contract_addr in [connected_chain_evm] section)"
    echo "   - integrated-gmp/config/integrated-gmp_testnet.toml (escrow_contract_addr + gmp_endpoint_addr in [connected_chain_evm] section)"
    echo "   - solver/config/solver_testnet.toml (escrow_contract_addr in [[connected_chain]] EVM section)"
    echo "   - frontend/.env.local (NEXT_PUBLIC_BASE_ESCROW_CONTRACT_ADDRESS)"
fi
echo ""

