#!/bin/bash

# Deploy Intent Framework to Testnets
#
# Deploys contracts on each chain:
#   1. Movement Bardock Testnet (hub chain)
#   2. Base Sepolia (connected EVM chain)
#   3. Solana Devnet (connected SVM chain)
#
# After deployment, prints a summary of addresses to update in
# .env.testnet and service config files. Run configure.sh after updating.
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

# Source .env.testnet once and export all vars. Child scripts skip their
# own sourcing when DEPLOY_ENV_SOURCED=1, so we control the env centrally.
set -a
source "$SCRIPT_DIR/.env.testnet"
set +a
export DEPLOY_ENV_SOURCED=1

LOG_DIR="$SCRIPT_DIR/logs"

echo "=========================================="
echo " Testnet Deploy"
echo "=========================================="
echo ""

echo "--------------------------------------------"
echo " Step 1: Deploy to Movement Testnet"
echo "--------------------------------------------"
"$SCRIPT_DIR/scripts/deploy-to-movement-testnet.sh"
echo ""

# Propagate MVM addresses for subsequent deploys (EVM/SVM need MOVEMENT_INTENT_MODULE_ADDR)
MVM_LOG=$(ls -t "$LOG_DIR"/deploy-movement-testnet-*.log 2>/dev/null | head -1)
if [ -n "$MVM_LOG" ]; then
    export MOVEMENT_INTENT_MODULE_ADDR=$(grep "^Module address:" "$MVM_LOG" | awk '{print $NF}')
    export MOVEMENT_MODULE_PRIVATE_KEY=$(grep "^Module private key:" "$MVM_LOG" | awk '{print $NF}')
    echo " Propagated MOVEMENT_INTENT_MODULE_ADDR=$MOVEMENT_INTENT_MODULE_ADDR"
fi

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

# ============================================================================
# Summary: read deployment logs and print addresses to update
# ============================================================================

echo "=========================================="
echo " Deployment Complete!"
echo "=========================================="
echo ""

MVM_LOG=$(ls -t "$LOG_DIR"/deploy-movement-testnet-*.log 2>/dev/null | head -1)
EVM_LOG=$(ls -t "$LOG_DIR"/deploy-base-sepolia-*.log 2>/dev/null | head -1)
SVM_LOG=$(ls -t "$LOG_DIR"/deploy-solana-devnet-*.log 2>/dev/null | head -1)

MVM_MODULE_ADDR=""
MVM_MODULE_PRIVATE_KEY=""
EVM_GMP_ADDR=""
EVM_ESCROW_ADDR=""
EVM_OUTFLOW_ADDR=""
SVM_ESCROW_ID=""
SVM_GMP_ID=""
SVM_OUTFLOW_ID=""

if [ -n "$MVM_LOG" ]; then
    MVM_MODULE_ADDR=$(grep "^Module address:" "$MVM_LOG" | awk '{print $NF}')
    MVM_MODULE_PRIVATE_KEY=$(grep "^Module private key:" "$MVM_LOG" | awk '{print $NF}')
fi
if [ -n "$EVM_LOG" ]; then
    EVM_GMP_ADDR=$(grep "^IntentGmp:" "$EVM_LOG" | awk '{print $NF}')
    EVM_ESCROW_ADDR=$(grep "^IntentInflowEscrow:" "$EVM_LOG" | awk '{print $NF}')
    EVM_OUTFLOW_ADDR=$(grep "^IntentOutflowValidator:" "$EVM_LOG" | awk '{print $NF}')
fi
if [ -n "$SVM_LOG" ]; then
    SVM_ESCROW_ID=$(grep "^Escrow" "$SVM_LOG" | awk '{print $NF}')
    SVM_GMP_ID=$(grep "^GMP Endpoint" "$SVM_LOG" | awk '{print $NF}')
    SVM_OUTFLOW_ID=$(grep "^Outflow" "$SVM_LOG" | awk '{print $NF}')
fi

SUMMARY_LOG="$LOG_DIR/deploy-summary-$(date +%Y%m%d-%H%M%S).log"
mkdir -p "$LOG_DIR"

{
echo " UPDATE THESE FILES WITH THE ADDRESSES BELOW"
echo "=========================================="
echo ""
echo " .env.testnet:"
[ -n "$MVM_MODULE_ADDR" ] && echo "   MOVEMENT_INTENT_MODULE_ADDR=$MVM_MODULE_ADDR"
[ -n "$MVM_MODULE_PRIVATE_KEY" ] && echo "   MOVEMENT_MODULE_PRIVATE_KEY=$MVM_MODULE_PRIVATE_KEY"
[ -n "$EVM_GMP_ADDR" ] && echo "   BASE_GMP_ENDPOINT_ADDR=$EVM_GMP_ADDR"
[ -n "$EVM_ESCROW_ADDR" ] && echo "   BASE_INFLOW_ESCROW_ADDR=$EVM_ESCROW_ADDR"
[ -n "$EVM_OUTFLOW_ADDR" ] && echo "   BASE_OUTFLOW_VALIDATOR_ADDR=$EVM_OUTFLOW_ADDR"
[ -n "$SVM_ESCROW_ID" ] && echo "   SOLANA_PROGRAM_ID=$SVM_ESCROW_ID"
[ -n "$SVM_GMP_ID" ] && echo "   SOLANA_GMP_ID=$SVM_GMP_ID"
[ -n "$SVM_OUTFLOW_ID" ] && echo "   SOLANA_OUTFLOW_ID=$SVM_OUTFLOW_ID"
echo ""
echo " coordinator/config/coordinator_testnet.toml:"
[ -n "$MVM_MODULE_ADDR" ] && echo "   [hub_chain] intent_module_addr = \"$MVM_MODULE_ADDR\""
[ -n "$EVM_ESCROW_ADDR" ] && echo "   [connected_chain_evm] escrow_contract_addr = \"$EVM_ESCROW_ADDR\""
[ -n "$SVM_ESCROW_ID" ] && echo "   [connected_chain_svm] escrow_program_id = \"$SVM_ESCROW_ID\""
echo ""
echo " integrated-gmp/config/integrated-gmp_testnet.toml:"
[ -n "$MVM_MODULE_ADDR" ] && echo "   [hub_chain] intent_module_addr = \"$MVM_MODULE_ADDR\""
[ -n "$EVM_ESCROW_ADDR" ] && echo "   [connected_chain_evm] escrow_contract_addr = \"$EVM_ESCROW_ADDR\""
[ -n "$EVM_GMP_ADDR" ] && echo "   [connected_chain_evm] gmp_endpoint_addr = \"$EVM_GMP_ADDR\""
[ -n "$SVM_ESCROW_ID" ] && echo "   [connected_chain_svm] escrow_program_id = \"$SVM_ESCROW_ID\""
[ -n "$SVM_GMP_ID" ] && echo "   [connected_chain_svm] gmp_endpoint_program_id = \"$SVM_GMP_ID\""
echo ""
echo " solver/config/solver_testnet.toml:"
[ -n "$MVM_MODULE_ADDR" ] && echo "   [hub_chain] module_addr = \"$MVM_MODULE_ADDR\""
[ -n "$EVM_ESCROW_ADDR" ] && echo "   [[connected_chain]] EVM escrow_contract_addr = \"$EVM_ESCROW_ADDR\""
[ -n "$EVM_OUTFLOW_ADDR" ] && echo "   [[connected_chain]] EVM outflow_validator_addr = \"$EVM_OUTFLOW_ADDR\""
[ -n "$EVM_GMP_ADDR" ] && echo "   [[connected_chain]] EVM gmp_endpoint_addr = \"$EVM_GMP_ADDR\""
[ -n "$SVM_ESCROW_ID" ] && echo "   [[connected_chain]] SVM escrow_program_id = \"$SVM_ESCROW_ID\""
[ -n "$SVM_OUTFLOW_ID" ] && echo "   [[connected_chain]] SVM outflow_validator_program_id = \"$SVM_OUTFLOW_ID\""
[ -n "$SVM_GMP_ID" ] && echo "   [[connected_chain]] SVM gmp_endpoint_program_id = \"$SVM_GMP_ID\""
echo ""
echo " frontend/.env.local:"
[ -n "$MVM_MODULE_ADDR" ] && echo "   NEXT_PUBLIC_INTENT_CONTRACT_ADDRESS=$MVM_MODULE_ADDR"
[ -n "$EVM_ESCROW_ADDR" ] && echo "   NEXT_PUBLIC_BASE_ESCROW_CONTRACT_ADDRESS=$EVM_ESCROW_ADDR"
[ -n "$EVM_OUTFLOW_ADDR" ] && echo "   NEXT_PUBLIC_BASE_OUTFLOW_VALIDATOR_ADDRESS=$EVM_OUTFLOW_ADDR"
[ -n "$SVM_ESCROW_ID" ] && echo "   NEXT_PUBLIC_SVM_PROGRAM_ID=$SVM_ESCROW_ID"
[ -n "$SVM_OUTFLOW_ID" ] && echo "   NEXT_PUBLIC_SVM_OUTFLOW_PROGRAM_ID=$SVM_OUTFLOW_ID"
echo ""
} | tee "$SUMMARY_LOG"

echo " Summary saved to: $SUMMARY_LOG"
echo ""
