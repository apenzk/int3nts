#!/bin/bash

# Deploy EVM IntentEscrow to Base Sepolia Testnet
# Reads keys from .env.testnet and deploys the contract

set -e

# Get the script directory and project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"
export PROJECT_ROOT

# Source utilities from testing-infra (for CI testing infrastructure)
source "$PROJECT_ROOT/testing-infra/ci-e2e/util.sh" 2>/dev/null || true

echo " Deploying IntentEscrow to Base Sepolia Testnet"
echo "=================================================="
echo ""

# Load .env.testnet
TESTNET_KEYS_FILE="$SCRIPT_DIR/.env.testnet"

if [ ! -f "$TESTNET_KEYS_FILE" ]; then
    echo "❌ ERROR: .env.testnet not found at $TESTNET_KEYS_FILE"
    echo "   Create it from env.testnet.example in this directory"
    exit 1
fi

# Source the keys file
source "$TESTNET_KEYS_FILE"

# Check required variables
if [ -z "$BASE_DEPLOYER_PRIVATE_KEY" ]; then
    echo "❌ ERROR: BASE_DEPLOYER_PRIVATE_KEY not set in .env.testnet"
    exit 1
fi

if [ -z "$TRUSTED_GMP_EVM_PUBKEY_HASH" ]; then
    echo "❌ ERROR: TRUSTED_GMP_EVM_PUBKEY_HASH not set in .env.testnet"
    echo "   Run: nix develop ./nix -c bash -c 'cd trusted-gmp && TRUSTED_GMP_CONFIG_PATH=config/trusted-gmp_testnet.toml cargo run --bin get_approver_eth_address'"
    exit 1
fi

# Load assets configuration
ASSETS_CONFIG_FILE="$PROJECT_ROOT/testing-infra/testnet/config/testnet-assets.toml"

if [ ! -f "$ASSETS_CONFIG_FILE" ]; then
    echo "❌ ERROR: testnet-assets.toml not found at $ASSETS_CONFIG_FILE"
    exit 1
fi

# Read Base Sepolia RPC URL from config
BASE_SEPOLIA_RPC_URL=$(grep -A 5 "^\[base_sepolia\]" "$ASSETS_CONFIG_FILE" | grep "^rpc_url = " | sed 's/.*= "\(.*\)".*/\1/' | tr -d '"' || echo "")

if [ -z "$BASE_SEPOLIA_RPC_URL" ]; then
    echo "❌ ERROR: Base Sepolia RPC URL not found in testnet-assets.toml"
    exit 1
fi

echo " Configuration:"
echo "   Deployer Address: $BASE_DEPLOYER_ADDR"
echo "   Trusted-GMP EVM Pubkey Hash: $TRUSTED_GMP_EVM_PUBKEY_HASH"
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

# Export environment variables for Hardhat
export DEPLOYER_PRIVATE_KEY="$BASE_DEPLOYER_PRIVATE_KEY"
export APPROVER_ADDR="$TRUSTED_GMP_EVM_PUBKEY_HASH"
export BASE_SEPOLIA_RPC_URL

echo " Environment configured for Hardhat"
echo ""

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo " Installing dependencies..."
    npm install
    echo "✅ Dependencies installed"
    echo ""
fi

# Deploy contract (run from within nix develop ./nix shell)
echo " Deploying IntentEscrow contract..."
echo "   (Run this script from within 'nix develop ./nix' shell)"
echo ""
DEPLOY_OUTPUT=$(npx hardhat run scripts/deploy.js --network baseSepolia 2>&1)
DEPLOY_EXIT_CODE=$?

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
CONTRACT_ADDR=$(echo "$DEPLOY_OUTPUT" | grep "Contract address:" | tail -1 | awk '{print $NF}' | tr -d '\n' || echo "")

if [ -n "$CONTRACT_ADDR" ]; then
    echo " Deployed contract address: $CONTRACT_ADDR"
    echo ""
    echo " Update the following files with this address:"
    echo ""
    echo "   1. coordinator/config/coordinator_testnet.toml"
    echo "      escrow_contract_addr = \"$CONTRACT_ADDR\""
    echo "      (in the [connected_chain_evm] section)"
    echo ""
    echo "   2. trusted-gmp/config/trusted-gmp_testnet.toml"
    echo "      escrow_contract_addr = \"$CONTRACT_ADDR\""
    echo "      (in the [connected_chain_evm] section)"
    echo ""
    echo "   3. solver/config/solver_testnet.toml"
    echo "      escrow_contract_addr = \"$CONTRACT_ADDR\""
    echo "      (in the [[connected_chain]] EVM section)"
    echo ""
    echo "   4. frontend/.env.local"
    echo "      NEXT_PUBLIC_BASE_ESCROW_CONTRACT_ADDRESS=$CONTRACT_ADDR"
    echo ""
    echo "   5. Run ./testing-infra/testnet/check-testnet-preparedness.sh to verify"
else
    echo "️  Could not extract contract address from output"
    echo "   Please copy it manually from the deployment output above"
    echo ""
    echo " Update the following files:"
    echo "   - coordinator/config/coordinator_testnet.toml (escrow_contract_addr in [connected_chain_evm] section)"
    echo "   - trusted-gmp/config/trusted-gmp_testnet.toml (escrow_contract_addr in [connected_chain_evm] section)"
    echo "   - solver/config/solver_testnet.toml (escrow_contract_addr in [[connected_chain]] EVM section)"
    echo "   - frontend/.env.local (NEXT_PUBLIC_BASE_ESCROW_CONTRACT_ADDRESS)"
fi
echo ""

