#!/bin/bash

# Deploy Intent Framework to Mainnets
#
# Deploys contracts on each chain:
#   1. Movement Mainnet (hub chain)
#   2. Base Mainnet (connected EVM chain)
#   3. HyperEVM Mainnet (connected EVM chain)
#
# After deployment, prints a summary of addresses to update in
# .env.mainnet and service config files. Run configure.sh after updating.
#
# Requires:
#   - .env.mainnet with deployer keys for all chains
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

LOG_DIR="$SCRIPT_DIR/logs"

echo "=========================================="
echo " Mainnet Deploy"
echo "=========================================="
echo ""

echo "--------------------------------------------"
echo " Step 1: Deploy to Movement Mainnet"
echo "--------------------------------------------"
"$SCRIPT_DIR/scripts/deploy-to-movement.sh"
echo ""

# Propagate MVM addresses for subsequent deploys (EVM needs MOVEMENT_INTENT_MODULE_ADDR)
MVM_LOG=$(ls -t "$LOG_DIR"/deploy-movement-mainnet-*.log 2>/dev/null | head -1)
if [ -n "$MVM_LOG" ]; then
    export MOVEMENT_INTENT_MODULE_ADDR=$(grep "^Module address:" "$MVM_LOG" | awk '{print $NF}')
    export MOVEMENT_MODULE_PRIVATE_KEY=$(grep "^Module private key:" "$MVM_LOG" | awk '{print $NF}')
    echo " Propagated MOVEMENT_INTENT_MODULE_ADDR=$MOVEMENT_INTENT_MODULE_ADDR"
fi

echo "--------------------------------------------"
echo " Step 2: Deploy to Base Mainnet"
echo "--------------------------------------------"
"$SCRIPT_DIR/scripts/deploy-to-base.sh"
echo ""

echo "--------------------------------------------"
echo " Step 3: Deploy to HyperEVM Mainnet"
echo "--------------------------------------------"
"$SCRIPT_DIR/scripts/deploy-to-hyperliquid.sh"
echo ""

# ============================================================================
# Summary: read deployment logs and print addresses to update
# ============================================================================

echo "=========================================="
echo " Deployment Complete!"
echo "=========================================="
echo ""

MVM_LOG=$(ls -t "$LOG_DIR"/deploy-movement-mainnet-*.log 2>/dev/null | head -1)
BASE_LOG=$(ls -t "$LOG_DIR"/deploy-base-mainnet-*.log 2>/dev/null | head -1)
HYPER_LOG=$(ls -t "$LOG_DIR"/deploy-hyperliquid-mainnet-*.log 2>/dev/null | head -1)

MVM_MODULE_ADDR=""
MVM_MODULE_PRIVATE_KEY=""
BASE_GMP_ADDR=""
BASE_ESCROW_ADDR=""
BASE_OUTFLOW_ADDR=""
HYPER_GMP_ADDR=""
HYPER_ESCROW_ADDR=""
HYPER_OUTFLOW_ADDR=""

if [ -n "$MVM_LOG" ]; then
    MVM_MODULE_ADDR=$(grep "^Module address:" "$MVM_LOG" | awk '{print $NF}')
    MVM_MODULE_PRIVATE_KEY=$(grep "^Module private key:" "$MVM_LOG" | awk '{print $NF}')
fi
if [ -n "$BASE_LOG" ]; then
    BASE_GMP_ADDR=$(grep "^IntentGmp:" "$BASE_LOG" | awk '{print $NF}')
    BASE_ESCROW_ADDR=$(grep "^IntentInflowEscrow:" "$BASE_LOG" | awk '{print $NF}')
    BASE_OUTFLOW_ADDR=$(grep "^IntentOutflowValidator:" "$BASE_LOG" | awk '{print $NF}')
fi
if [ -n "$HYPER_LOG" ]; then
    HYPER_GMP_ADDR=$(grep "^IntentGmp:" "$HYPER_LOG" | awk '{print $NF}')
    HYPER_ESCROW_ADDR=$(grep "^IntentInflowEscrow:" "$HYPER_LOG" | awk '{print $NF}')
    HYPER_OUTFLOW_ADDR=$(grep "^IntentOutflowValidator:" "$HYPER_LOG" | awk '{print $NF}')
fi

SUMMARY_LOG="$LOG_DIR/deploy-summary-$(date +%Y%m%d-%H%M%S).log"
mkdir -p "$LOG_DIR"

{
echo " UPDATE THESE FILES WITH THE ADDRESSES BELOW"
echo "=========================================="
echo ""
echo " .env.mainnet:"
[ -n "$MVM_MODULE_ADDR" ] && echo "   MOVEMENT_INTENT_MODULE_ADDR=$MVM_MODULE_ADDR"
[ -n "$MVM_MODULE_PRIVATE_KEY" ] && echo "   MOVEMENT_MODULE_PRIVATE_KEY=$MVM_MODULE_PRIVATE_KEY"
[ -n "$BASE_GMP_ADDR" ] && echo "   BASE_GMP_ENDPOINT_ADDR=$BASE_GMP_ADDR"
[ -n "$BASE_ESCROW_ADDR" ] && echo "   BASE_INFLOW_ESCROW_ADDR=$BASE_ESCROW_ADDR"
[ -n "$BASE_OUTFLOW_ADDR" ] && echo "   BASE_OUTFLOW_VALIDATOR_ADDR=$BASE_OUTFLOW_ADDR"
[ -n "$HYPER_GMP_ADDR" ] && echo "   HYPERLIQUID_GMP_ENDPOINT_ADDR=$HYPER_GMP_ADDR"
[ -n "$HYPER_ESCROW_ADDR" ] && echo "   HYPERLIQUID_INFLOW_ESCROW_ADDR=$HYPER_ESCROW_ADDR"
[ -n "$HYPER_OUTFLOW_ADDR" ] && echo "   HYPERLIQUID_OUTFLOW_VALIDATOR_ADDR=$HYPER_OUTFLOW_ADDR"
echo ""
echo " coordinator/config/coordinator_mainnet.toml:"
[ -n "$MVM_MODULE_ADDR" ] && echo "   [hub_chain] intent_module_addr = \"$MVM_MODULE_ADDR\""
[ -n "$BASE_ESCROW_ADDR" ] && echo "   [[connected_chain_evm]] Base escrow_contract_addr = \"$BASE_ESCROW_ADDR\""
[ -n "$HYPER_ESCROW_ADDR" ] && echo "   [[connected_chain_evm]] HyperEVM escrow_contract_addr = \"$HYPER_ESCROW_ADDR\""
echo ""
echo " integrated-gmp/config/integrated-gmp_mainnet.toml:"
[ -n "$MVM_MODULE_ADDR" ] && echo "   [hub_chain] intent_module_addr = \"$MVM_MODULE_ADDR\""
[ -n "$BASE_ESCROW_ADDR" ] && echo "   [[connected_chain_evm]] Base escrow_contract_addr = \"$BASE_ESCROW_ADDR\""
[ -n "$BASE_GMP_ADDR" ] && echo "   [[connected_chain_evm]] Base gmp_endpoint_addr = \"$BASE_GMP_ADDR\""
[ -n "$HYPER_ESCROW_ADDR" ] && echo "   [[connected_chain_evm]] HyperEVM escrow_contract_addr = \"$HYPER_ESCROW_ADDR\""
[ -n "$HYPER_GMP_ADDR" ] && echo "   [[connected_chain_evm]] HyperEVM gmp_endpoint_addr = \"$HYPER_GMP_ADDR\""
echo ""
echo " solver/config/solver_mainnet.toml:"
[ -n "$MVM_MODULE_ADDR" ] && echo "   [hub_chain] module_addr = \"$MVM_MODULE_ADDR\""
[ -n "$BASE_ESCROW_ADDR" ] && echo "   [[connected_chain]] EVM (Base) escrow_contract_addr = \"$BASE_ESCROW_ADDR\""
[ -n "$BASE_OUTFLOW_ADDR" ] && echo "   [[connected_chain]] EVM (Base) outflow_validator_addr = \"$BASE_OUTFLOW_ADDR\""
[ -n "$BASE_GMP_ADDR" ] && echo "   [[connected_chain]] EVM (Base) gmp_endpoint_addr = \"$BASE_GMP_ADDR\""
[ -n "$HYPER_ESCROW_ADDR" ] && echo "   [[connected_chain]] EVM (HyperEVM) escrow_contract_addr = \"$HYPER_ESCROW_ADDR\""
[ -n "$HYPER_OUTFLOW_ADDR" ] && echo "   [[connected_chain]] EVM (HyperEVM) outflow_validator_addr = \"$HYPER_OUTFLOW_ADDR\""
[ -n "$HYPER_GMP_ADDR" ] && echo "   [[connected_chain]] EVM (HyperEVM) gmp_endpoint_addr = \"$HYPER_GMP_ADDR\""
echo ""
} | tee "$SUMMARY_LOG"

echo " Summary saved to: $SUMMARY_LOG"
echo ""
