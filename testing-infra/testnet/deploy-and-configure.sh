#!/bin/bash

# Deploy and Configure Intent Framework on Testnets
#
# Phase 1 - Deploy contracts on each chain:
#   1. Movement Bardock Testnet (hub chain)
#   2. Base Sepolia (connected EVM chain)
#   3. Solana Devnet (connected SVM chain)
#
# Phase 2 - Configure cross-chain links:
#   4. Movement: set trusted remotes for Base and Solana
#   5. Base Sepolia: configure contracts
#   6. Solana: set trusted remotes + routing
#
# Each deploy script saves its output addresses to .env.testnet so
# subsequent scripts can read them.
#
# Requires:
#   - .env.testnet with deployer keys for all chains
#   - Movement CLI (see deploy-to-movement-testnet.sh for install)

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"

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

echo "=========================================="
echo " Testnet Deploy & Configure"
echo "=========================================="
echo ""
echo " [1] Deploy + Configure (full fresh deploy)"
echo " [2] Configure only (contracts already deployed)"
echo " [0] Exit"
echo ""
read -p " Choice (1/2/0): " -n 1 -r
echo
echo ""

if [[ $REPLY == "0" ]]; then
    echo "Aborted."
    exit 0
fi

RUN_DEPLOY=false
if [[ $REPLY == "1" ]]; then
    RUN_DEPLOY=true
elif [[ $REPLY != "2" ]]; then
    echo "Invalid choice. Aborted."
    exit 1
fi

# ============================================================================
# Phase 1: Deploy (optional)
# ============================================================================

if [ "$RUN_DEPLOY" = true ]; then
    echo "=========================================="
    echo " PHASE 1: DEPLOY"
    echo "=========================================="
    echo ""

    echo "--------------------------------------------"
    echo " Step 1: Deploy to Movement Testnet"
    echo "--------------------------------------------"
    "$SCRIPT_DIR/scripts/deploy-to-movement-testnet.sh"
    echo ""

    # Re-source .env.testnet to pick up new MOVEMENT_INTENT_MODULE_ADDR
    source "$SCRIPT_DIR/.env.testnet"

    if [ -z "$MOVEMENT_INTENT_MODULE_ADDR" ]; then
        echo "ERROR: MOVEMENT_INTENT_MODULE_ADDR not set after MVM deployment"
        exit 1
    fi
    echo " Movement module deployed: $MOVEMENT_INTENT_MODULE_ADDR"
    echo ""

    echo "--------------------------------------------"
    echo " Step 2: Deploy to Base Sepolia"
    echo "--------------------------------------------"
    "$SCRIPT_DIR/scripts/deploy-to-base-testnet.sh"
    echo ""

    echo "--------------------------------------------"
    echo " Step 3: Deploy to Solana Devnet"
    echo "--------------------------------------------"
    "$SCRIPT_DIR/scripts/deploy-to-solana-devnet.sh"
    echo ""
fi

# ============================================================================
# Phase 2: Configure cross-chain
# ============================================================================

echo "=========================================="
echo " PHASE 2: CONFIGURE CROSS-CHAIN"
echo "=========================================="
echo ""

# Re-source to pick up all deployed addresses
source "$SCRIPT_DIR/.env.testnet"

echo "--------------------------------------------"
echo " Step 4: Configure Movement"
echo "--------------------------------------------"
"$SCRIPT_DIR/scripts/configure-movement-testnet.sh"
echo ""

echo "--------------------------------------------"
echo " Step 5: Configure Base Sepolia"
echo "--------------------------------------------"
"$SCRIPT_DIR/scripts/configure-base-testnet.sh"
echo ""

echo "--------------------------------------------"
echo " Step 6: Configure Solana"
echo "--------------------------------------------"
"$SCRIPT_DIR/scripts/configure-solana-devnet.sh"
echo ""

# ============================================================================
# Done
# ============================================================================

# Run preparedness check to verify everything
echo "=========================================="
echo " VERIFYING DEPLOYMENT"
echo "=========================================="
echo ""
"$SCRIPT_DIR/check-testnet-preparedness.sh"
