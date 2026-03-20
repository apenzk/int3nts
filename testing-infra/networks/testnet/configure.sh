#!/bin/bash

# Configure Intent Framework Cross-Chain Links on Testnets
#
# Sets up cross-chain GMP routing between deployed contracts:
#   1. Movement: set remote GMP endpoints for Base and Solana
#   2. Base Sepolia: set remote GMP endpoint for hub
#   3. Solana: set remote GMP endpoints + routing + relay auth
#
# Requires:
#   - .env.testnet with deployed contract addresses (run deploy.sh first)
#   - Movement CLI

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../.." && pwd )"

# Re-exec inside nix develop if not already in a nix shell
if [ -z "$IN_NIX_SHELL" ]; then
    echo " Entering nix develop shell..."
    exec nix develop "$PROJECT_ROOT/nix" -c bash "$0" "$@"
fi

# Check .env.testnet exists
if [ ! -f "$SCRIPT_DIR/.env.testnet" ]; then
    echo "ERROR: .env.testnet not found"
    echo "   Create it from env.testnet.example:"
    echo "   cp $SCRIPT_DIR/env.testnet.example $SCRIPT_DIR/.env.testnet"
    exit 1
fi

# Source .env.testnet once and export all vars. Child scripts skip their
# own sourcing when DEPLOY_ENV_SOURCED=1, so we control the env centrally.
set -a
source "$SCRIPT_DIR/.env.testnet"
set +a
export DEPLOY_ENV_SOURCED=1

echo "=========================================="
echo " Testnet Configure Cross-Chain"
echo "=========================================="
echo ""

echo "--------------------------------------------"
echo " Step 1: Configure Movement"
echo "--------------------------------------------"
"$SCRIPT_DIR/scripts/configure-movement.sh"
echo ""

echo "--------------------------------------------"
echo " Step 2: Configure Base Sepolia"
echo "--------------------------------------------"
"$SCRIPT_DIR/scripts/configure-base.sh"
echo ""

echo "--------------------------------------------"
echo " Step 3: Configure Solana"
echo "--------------------------------------------"
"$SCRIPT_DIR/scripts/configure-solana.sh"
echo ""

echo "=========================================="
echo " Configuration Complete!"
echo "=========================================="
echo ""
