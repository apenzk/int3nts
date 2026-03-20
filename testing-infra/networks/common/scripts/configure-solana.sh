#!/bin/bash
# Common Solana configuration script.
# Called by network-specific wrappers that set the required variables:
#
#   SVM_RPC_URL            - Solana RPC endpoint
#   SVM_DISPLAY_NAME       - Human-readable name ("Solana Devnet")
#   SVM_HUB_CHAIN_ID       - Hub chain ID
#   SVM_DEPLOY_SCRIPT      - Deploy script name for error messages
#   SVM_MVM_DEPLOY_SCRIPT  - Movement deploy script name for error messages
#   CALLER_SCRIPT_DIR      - The calling script's directory
#
# Also expects env-utils.sh and solana-utils.sh to be sourced.

set -e

echo " Configuring ${SVM_DISPLAY_NAME} (Cross-Chain GMP)"
echo "=============================================="
echo ""

require_var "SOLANA_DEPLOYER_PRIVATE_KEY" "$SOLANA_DEPLOYER_PRIVATE_KEY"
require_var "MOVEMENT_INTENT_MODULE_ADDR" "$MOVEMENT_INTENT_MODULE_ADDR" "Run ${SVM_MVM_DEPLOY_SCRIPT} first"
require_var "SOLANA_GMP_ID" "$SOLANA_GMP_ID" "Run ${SVM_DEPLOY_SCRIPT} first"
require_var "SOLANA_PROGRAM_ID" "$SOLANA_PROGRAM_ID" "Run ${SVM_DEPLOY_SCRIPT} first"
require_var "SOLANA_OUTFLOW_ID" "$SOLANA_OUTFLOW_ID" "Run ${SVM_DEPLOY_SCRIPT} first"

echo " Configuration:"
echo "   Hub Chain ID: $SVM_HUB_CHAIN_ID"
echo "   Movement Module: $MOVEMENT_INTENT_MODULE_ADDR"
echo "   Solana GMP:      $SOLANA_GMP_ID"
echo "   Solana Escrow:   $SOLANA_PROGRAM_ID"
echo "   Solana Outflow:  $SOLANA_OUTFLOW_ID"
echo ""

# Create deployer keypair from base58 private key
solana_create_keypair "$SOLANA_DEPLOYER_PRIVATE_KEY" "${SOLANA_DEPLOYER_ADDR:-}"

# Build CLI
build_solana_cli

# Pad hub address to 64 hex characters (32 bytes)
HUB_ADDR_PADDED=$(pad_address_32 "$MOVEMENT_INTENT_MODULE_ADDR")

echo " Cross-chain configuration..."
echo ""

echo " 1. Setting hub as remote GMP endpoint..."
run_solana_idempotent "Set remote GMP endpoint" \
    "$CLI_BIN" gmp-set-remote-gmp-endpoint-addr \
    --gmp-program-id "$SOLANA_GMP_ID" \
    --payer "$DEPLOYER_KEYPAIR" \
    --src-chain-id "$SVM_HUB_CHAIN_ID" \
    --addr "$HUB_ADDR_PADDED" \
    --rpc "$SVM_RPC_URL"

echo " 2. Initializing outflow validator..."
run_solana_idempotent "Initialize outflow validator" \
    "$CLI_BIN" outflow-init \
    --outflow-program-id "$SOLANA_OUTFLOW_ID" \
    --payer "$DEPLOYER_KEYPAIR" \
    --gmp-endpoint "$SOLANA_GMP_ID" \
    --hub-chain-id "$SVM_HUB_CHAIN_ID" \
    --hub-address "$HUB_ADDR_PADDED" \
    --rpc "$SVM_RPC_URL"

echo " 2b. Updating outflow validator hub config..."
"$CLI_BIN" outflow-update-hub-config \
    --outflow-program-id "$SOLANA_OUTFLOW_ID" \
    --payer "$DEPLOYER_KEYPAIR" \
    --hub-chain-id "$SVM_HUB_CHAIN_ID" \
    --hub-address "$HUB_ADDR_PADDED" \
    --rpc "$SVM_RPC_URL"

echo " 3. Configuring escrow GMP..."
run_solana_idempotent "Configure escrow GMP" \
    "$CLI_BIN" escrow-set-gmp-config \
    --program-id "$SOLANA_PROGRAM_ID" \
    --payer "$DEPLOYER_KEYPAIR" \
    --hub-chain-id "$SVM_HUB_CHAIN_ID" \
    --hub-address "$HUB_ADDR_PADDED" \
    --gmp-endpoint "$SOLANA_GMP_ID" \
    --rpc "$SVM_RPC_URL"

echo " 4. Setting GMP routing..."
run_solana_idempotent "Set GMP routing" \
    "$CLI_BIN" gmp-set-routing \
    --gmp-program-id "$SOLANA_GMP_ID" \
    --payer "$DEPLOYER_KEYPAIR" \
    --outflow-validator "$SOLANA_OUTFLOW_ID" \
    --intent-escrow "$SOLANA_PROGRAM_ID" \
    --rpc "$SVM_RPC_URL"

# 5. Add GMP relay authorization (required)
require_var "INTEGRATED_GMP_SVM_ADDR" "$INTEGRATED_GMP_SVM_ADDR" \
    "Set INTEGRATED_GMP_SVM_ADDR in ${ENV_FILE_NAME} (derive with: cd integrated-gmp && cargo run --bin get_relay_addresses)"
echo " 5. Adding GMP relay: $INTEGRATED_GMP_SVM_ADDR"
run_solana_idempotent "Add GMP relay" \
    "$CLI_BIN" gmp-add-relay \
    --gmp-program-id "$SOLANA_GMP_ID" \
    --payer "$DEPLOYER_KEYPAIR" \
    --relay "$INTEGRATED_GMP_SVM_ADDR" \
    --rpc "$SVM_RPC_URL"
# RelayAccount layout: disc(1) + relay_pubkey(32) + is_authorized(1) + bump(1) = 35 bytes
verify_solana_has_account "$SOLANA_GMP_ID" "$SVM_RPC_URL" "Ag==" 35 \
    "GMP relay authorization for $INTEGRATED_GMP_SVM_ADDR" \
    1 "$INTEGRATED_GMP_SVM_ADDR"

# Clean up
rm -rf "$TEMP_KEYPAIR_DIR"

echo ""
echo " ${SVM_DISPLAY_NAME} configuration verified."
