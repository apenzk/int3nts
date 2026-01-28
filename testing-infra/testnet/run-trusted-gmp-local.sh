#!/bin/bash

# Run Trusted GMP Locally (Against Testnets)
#
# This script runs the trusted-gmp service locally, connecting to:
#   - Movement Bardock Testnet (hub chain)
#   - Base Sepolia (connected chain)
#
# Use this to test before deploying to EC2.
#
# Prerequisites:
#   - trusted-gmp/config/trusted-gmp_testnet.toml configured with actual deployed addresses
#   - .env.testnet with TRUSTED_GMP_PRIVATE_KEY and TRUSTED_GMP_PUBLIC_KEY
#   - Rust toolchain installed
#
# Usage:
#   ./run-trusted-gmp-local.sh
#   ./run-trusted-gmp-local.sh --release  # Run release build (faster)

set -e

# Get the script directory and project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"

echo " Running Trusted GMP Locally (Testnet Mode)"
echo "================================================"
echo ""

# Check config exists
TRUSTED_GMP_CONFIG="$PROJECT_ROOT/trusted-gmp/config/trusted-gmp_testnet.toml"

if [ ! -f "$TRUSTED_GMP_CONFIG" ]; then
    echo "❌ ERROR: trusted-gmp_testnet.toml not found at $TRUSTED_GMP_CONFIG"
    echo ""
    echo "   Create it from the template:"
    echo "   cp trusted-gmp/config/trusted-gmp.template.toml trusted-gmp/config/trusted-gmp_testnet.toml"
    echo ""
    echo "   Then populate with actual deployed contract addresses:"
    echo "   - intent_module_addr (hub_chain section)"
    echo "   - escrow_contract_addr (connected_chain_evm section)"
    echo "   - approver_evm_pubkey_hash (connected_chain_evm section)"
    exit 1
fi

# Load .env.testnet for environment variables
TESTNET_KEYS_FILE="$SCRIPT_DIR/.env.testnet"

if [ ! -f "$TESTNET_KEYS_FILE" ]; then
    echo "❌ ERROR: .env.testnet not found at $TESTNET_KEYS_FILE"
    echo ""
    echo "   Create it from the template in this directory:"
    echo "   cp env.testnet.example .env.testnet"
    echo ""
    echo "   Then populate with your testnet keys."
    exit 1
fi

# Source keys file to export environment variables
source "$TESTNET_KEYS_FILE"

# Check required environment variables (keys only)
REQUIRED_VARS=(
    "TRUSTED_GMP_PRIVATE_KEY"
    "TRUSTED_GMP_PUBLIC_KEY"
)

MISSING_VARS=()
for var in "${REQUIRED_VARS[@]}"; do
    if [ -z "${!var}" ]; then
        MISSING_VARS+=("$var")
    fi
done

if [ ${#MISSING_VARS[@]} -ne 0 ]; then
    echo "❌ ERROR: Missing required environment variables in .env.testnet:"
    for var in "${MISSING_VARS[@]}"; do
        echo "   - $var"
    done
    echo ""
    echo "   These keys are required for the trusted-gmp service to sign approvals."
    exit 1
fi

# Validate config has actual addresses (not placeholders)
# Check for common placeholder patterns
if grep -qE "(0x123|0x\.\.\.|0xalice|0xbob)" "$TRUSTED_GMP_CONFIG"; then
    echo "❌ ERROR: trusted-gmp_testnet.toml still has placeholder addresses"
    echo ""
    echo "   Update the config file with actual deployed addresses:"
    echo "   - intent_module_addr (hub_chain section)"
    echo "   - escrow_contract_addr (connected_chain_evm section)"
    echo "   - approver_evm_pubkey_hash (connected_chain_evm section)"
    echo ""
    echo "   Contract addresses should be read from your deployment logs."
    exit 1
fi

# Extract config values for display
HUB_RPC=$(grep -A5 "\[hub_chain\]" "$TRUSTED_GMP_CONFIG" | grep "rpc_url" | head -1 | sed 's/.*= *"\(.*\)".*/\1/')
EVM_RPC=$(grep -A5 "\[connected_chain_evm\]" "$TRUSTED_GMP_CONFIG" | grep "rpc_url" | head -1 | sed 's/.*= *"\(.*\)".*/\1/')
API_PORT=$(grep -A5 "\[api\]" "$TRUSTED_GMP_CONFIG" | grep "port" | head -1 | sed 's/.*= *\([0-9]*\).*/\1/')
INTENT_MODULE=$(grep -A5 "\[hub_chain\]" "$TRUSTED_GMP_CONFIG" | grep "intent_module_addr" | head -1 | sed 's/.*= *"\(.*\)".*/\1/')
ESCROW_CONTRACT=$(grep -A5 "\[connected_chain_evm\]" "$TRUSTED_GMP_CONFIG" | grep "escrow_contract_addr" | head -1 | sed 's/.*= *"\(.*\)".*/\1/')

# Check for API key placeholders in RPC URLs
if [[ "$HUB_RPC" == *"ALCHEMY_API_KEY"* ]] || [[ "$EVM_RPC" == *"ALCHEMY_API_KEY"* ]]; then
    echo "️  WARNING: RPC URLs contain API key placeholders (ALCHEMY_API_KEY)"
    echo "   The trusted-gmp service does not substitute placeholders - use full URLs in config"
    echo "   Or use the public RPC URLs from testnet-assets.toml"
    echo ""
fi

echo " Configuration:"
echo "   Config file: $TRUSTED_GMP_CONFIG"
echo "   Keys file:   $TESTNET_KEYS_FILE"
echo ""
echo "   Hub Chain:"
echo "     RPC:              $HUB_RPC"
echo "     Intent Module:     $INTENT_MODULE"
echo ""
echo "   EVM Chain:"
echo "     RPC:              $EVM_RPC"
echo "     Escrow Contract:  $ESCROW_CONTRACT"
echo ""
echo "   API Server:"
echo "     Port:             ${API_PORT:-3334}"
echo ""

cd "$PROJECT_ROOT/trusted-gmp"

# Export environment variables for trusted-gmp keys
export TRUSTED_GMP_PRIVATE_KEY
export TRUSTED_GMP_PUBLIC_KEY

# Check if --release flag is passed
if [ "$1" = "--release" ]; then
    echo " Building release binary..."
    nix develop "$PROJECT_ROOT/nix" --command bash -c "cd '$PROJECT_ROOT/trusted-gmp' && cargo build --release"
    echo ""
    echo " Starting trusted-gmp (release mode)..."
    echo "   Press Ctrl+C to stop"
    echo ""
    RUST_LOG=info ./target/release/trusted-gmp --testnet
else
    echo " Starting trusted-gmp (debug mode)..."
    echo "   Press Ctrl+C to stop"
    echo "   (Use --release for faster performance)"
    echo ""
    nix develop "$PROJECT_ROOT/nix" --command bash -c "cd '$PROJECT_ROOT/trusted-gmp' && RUST_LOG=info cargo run --bin trusted-gmp -- --testnet"
fi
