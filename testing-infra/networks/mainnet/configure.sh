#!/bin/bash

# Configure Intent Framework Cross-Chain Links on Mainnets
#
# Sets up cross-chain GMP routing between deployed contracts:
#   1. Movement Mainnet: set remote GMP endpoints for Base and HyperEVM
#   2. Base Mainnet: set remote GMP endpoint for hub
#   3. HyperEVM Mainnet: set remote GMP endpoint for hub
#
# Requires:
#   - .env.mainnet with deployed contract addresses (run deploy.sh first)
#   - Movement CLI

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../.." && pwd )"

# Re-exec inside nix develop if not already in a nix shell
if [ -z "$IN_NIX_SHELL" ]; then
    echo " Entering nix develop shell..."
    exec nix develop "$PROJECT_ROOT/nix" -c bash "$0" "$@"
fi

# Check .env.mainnet exists
if [ ! -f "$SCRIPT_DIR/.env.mainnet" ]; then
    echo "ERROR: .env.mainnet not found"
    echo "   Create it from env.mainnet.example:"
    echo "   cp $SCRIPT_DIR/env.mainnet.example $SCRIPT_DIR/.env.mainnet"
    exit 1
fi

# Source .env.mainnet once and export all vars. Child scripts skip their
# own sourcing when DEPLOY_ENV_SOURCED=1, so we control the env centrally.
set -a
source "$SCRIPT_DIR/.env.mainnet"
set +a
export DEPLOY_ENV_SOURCED=1

echo "=========================================="
echo " Mainnet Configure Cross-Chain"
echo "=========================================="
echo ""

echo "--------------------------------------------"
echo " Step 1: Configure Movement"
echo "--------------------------------------------"
"$SCRIPT_DIR/scripts/configure-movement.sh"
echo ""

echo "--------------------------------------------"
echo " Step 2: Configure Base Mainnet"
echo "--------------------------------------------"
"$SCRIPT_DIR/scripts/configure-base.sh"
echo ""

echo "--------------------------------------------"
echo " Step 3: Configure HyperEVM Mainnet"
echo "--------------------------------------------"
"$SCRIPT_DIR/scripts/configure-hyperliquid.sh"
echo ""

echo "=========================================="
echo " Configuration Complete!"
echo "=========================================="
echo ""
