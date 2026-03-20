#!/bin/bash
# Common Solana deployment script.
# Called by network-specific wrappers that set the required variables:
#
#   SVM_RPC_URL            - Solana RPC endpoint
#   SVM_DISPLAY_NAME       - Human-readable name ("Solana Devnet")
#   SVM_NETWORK_LABEL      - testnet/mainnet (for config file paths)
#   SVM_LOG_PREFIX         - Log file prefix (solana-devnet)
#   SVM_HUB_CHAIN_ID       - Hub chain ID
#   SVM_CHAIN_ID           - SVM chain ID
#   SVM_CHECK_SCRIPT       - Preparedness check script path
#   SVM_CONFIGURE_SCRIPT   - Configure script name for next-steps
#   CALLER_SCRIPT_DIR      - The calling script's directory
#
# Also expects env-utils.sh and solana-utils.sh to be sourced.

set -e

echo " Deploying GMP Contracts to ${SVM_DISPLAY_NAME}"
echo "=========================================="
echo ""

require_var "SOLANA_DEPLOYER_PRIVATE_KEY" "$SOLANA_DEPLOYER_PRIVATE_KEY"
require_var "SOLANA_DEPLOYER_ADDR" "$SOLANA_DEPLOYER_ADDR"
require_var "MOVEMENT_INTENT_MODULE_ADDR" "$MOVEMENT_INTENT_MODULE_ADDR" \
    "This should be set to the deployed MVM hub intent module address"

echo " Configuration:"
echo "   Deployer Address: $SOLANA_DEPLOYER_ADDR"
echo "   Network: ${SVM_DISPLAY_NAME}"
echo "   RPC URL: $SVM_RPC_URL"
echo "   Hub Chain ID: $SVM_HUB_CHAIN_ID"
echo "   SVM Chain ID: $SVM_CHAIN_ID"
echo "   Movement Intent Module: $MOVEMENT_INTENT_MODULE_ADDR"
echo ""

# Change to intent-frameworks/svm directory
cd "$PROJECT_ROOT/intent-frameworks/svm"

# Create deployer keypair from base58 private key
solana_create_keypair "$SOLANA_DEPLOYER_PRIVATE_KEY" "$SOLANA_DEPLOYER_ADDR"

# Check deployer balance
echo " Checking deployer balance..."
BALANCE=$(solana balance "$SOLANA_DEPLOYER_ADDR" --url "$SVM_RPC_URL" 2>/dev/null | awk '{print $1}' || echo "0")
echo "   Balance: $BALANCE SOL"

if (( $(echo "$BALANCE < 2" | bc -l) )); then
    echo "ERROR: Insufficient balance for deployment"
    echo "   Current balance: $BALANCE SOL"
    echo "   Required: at least 2 SOL (recommended 3+ SOL)"
    echo ""
    echo "   Fund this wallet: $SOLANA_DEPLOYER_ADDR"
    rm -rf "$TEMP_KEYPAIR_DIR"
    exit 1
fi
echo ""

# Build all programs
echo " Building all programs..."
./scripts/build.sh

# Program paths
ESCROW_SO="$PROJECT_ROOT/intent-frameworks/svm/target/deploy/intent_inflow_escrow.so"
ESCROW_KEYPAIR="$PROJECT_ROOT/intent-frameworks/svm/target/deploy/intent_inflow_escrow-keypair.json"
GMP_SO="$PROJECT_ROOT/intent-frameworks/svm/target/deploy/intent_gmp.so"
GMP_KEYPAIR="$PROJECT_ROOT/intent-frameworks/svm/target/deploy/intent_gmp-keypair.json"
OUTFLOW_SO="$PROJECT_ROOT/intent-frameworks/svm/target/deploy/intent_outflow_validator.so"
OUTFLOW_KEYPAIR="$PROJECT_ROOT/intent-frameworks/svm/target/deploy/intent_outflow_validator-keypair.json"

# Verify all binaries exist
for SO_FILE in "$ESCROW_SO" "$GMP_SO" "$OUTFLOW_SO"; do
    if [ ! -f "$SO_FILE" ]; then
        echo "ERROR: Program binary not found at $SO_FILE"
        rm -rf "$TEMP_KEYPAIR_DIR"
        exit 1
    fi
done

# Helper: deploy a single program
deploy_program() {
    local name="$1"
    local keypair="$2"
    local so="$3"

    echo " Deploying $name..."
    solana program deploy \
        --url "$SVM_RPC_URL" \
        --keypair "$DEPLOYER_KEYPAIR" \
        "$so" \
        --program-id "$keypair"

    local exit_code=$?
    if [ $exit_code -ne 0 ]; then
        echo "ERROR: Failed to deploy $name (exit code: $exit_code)"
        rm -rf "$TEMP_KEYPAIR_DIR"
        exit 1
    fi
    echo "$name deployed"
}

# Deploy all 3 programs
echo ""
echo " Deploying programs to ${SVM_DISPLAY_NAME}..."
echo "======================================="
deploy_program "intent_inflow_escrow" "$ESCROW_KEYPAIR" "$ESCROW_SO"
deploy_program "intent_gmp" "$GMP_KEYPAIR" "$GMP_SO"
deploy_program "intent_outflow_validator" "$OUTFLOW_KEYPAIR" "$OUTFLOW_SO"

# Get program IDs
ESCROW_ID=$(solana-keygen pubkey "$ESCROW_KEYPAIR")
GMP_ID=$(solana-keygen pubkey "$GMP_KEYPAIR")
OUTFLOW_ID=$(solana-keygen pubkey "$OUTFLOW_KEYPAIR")

echo ""
echo " All Programs Deployed!"
echo "========================"
echo "  Escrow (SOLANA_PROGRAM_ID):          $ESCROW_ID"
echo "  GMP Endpoint (SOLANA_GMP_ID):        $GMP_ID"
echo "  Outflow Validator (SVM_OUTFLOW_ID):  $OUTFLOW_ID"
echo ""

# Initialize self-contained components (no cross-chain dependencies)
echo ""
echo " Initializing self-contained components..."
echo "============================================"
echo ""

# Check for integrated-gmp public key (used as on-chain approver)
if [ -z "$INTEGRATED_GMP_PUBLIC_KEY" ]; then
    echo "WARNING: INTEGRATED_GMP_PUBLIC_KEY not set in ${ENV_FILE_NAME}"
    echo "   Skipping initialization - you'll need to run it manually later"
    echo ""
else
    INTEGRATED_GMP_PUBKEY_BASE58=$(base64_to_base58 "$INTEGRATED_GMP_PUBLIC_KEY")

    if [ -z "$INTEGRATED_GMP_PUBKEY_BASE58" ]; then
        echo "ERROR: Failed to convert integrated-gmp public key to base58"
        echo "   Skipping initialization - you'll need to run it manually"
    else
        echo " Integrated-GMP public key (base58): $INTEGRATED_GMP_PUBKEY_BASE58"

        build_solana_cli

        echo " 1. Initializing escrow with approver..."
        "$CLI_BIN" initialize \
            --program-id "$ESCROW_ID" \
            --payer "$DEPLOYER_KEYPAIR" \
            --approver "$INTEGRATED_GMP_PUBKEY_BASE58" \
            --rpc "$SVM_RPC_URL" && echo "Escrow initialized" || echo "Escrow init may have failed (OK if already initialized)"

        echo " 2. Initializing GMP endpoint..."
        "$CLI_BIN" gmp-init \
            --gmp-program-id "$GMP_ID" \
            --payer "$DEPLOYER_KEYPAIR" \
            --chain-id "$SVM_CHAIN_ID" \
            --rpc "$SVM_RPC_URL" && echo "GMP endpoint initialized" || echo "GMP endpoint init may have failed (OK if already initialized)"
    fi
fi

# Output deployed program IDs
echo " Add these to ${ENV_FILE_NAME}:"
echo ""
echo "   SOLANA_PROGRAM_ID=$ESCROW_ID"
echo "   SOLANA_GMP_ID=$GMP_ID"
echo "   SOLANA_OUTFLOW_ID=$OUTFLOW_ID"
echo ""

# Save deployment log
LOG_DIR="$CALLER_SCRIPT_DIR/../logs"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/deploy-${SVM_LOG_PREFIX}-$(date +%Y%m%d-%H%M%S).log"
{
    echo "${SVM_DISPLAY_NAME} Deployment — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo ""
    echo "Deployer:                  $SOLANA_DEPLOYER_ADDR"
    echo "Hub chain ID:              $SVM_HUB_CHAIN_ID"
    echo "SVM chain ID:              $SVM_CHAIN_ID"
    echo "Hub module addr:           $MOVEMENT_INTENT_MODULE_ADDR"
    echo ""
    echo "Escrow (SOLANA_PROGRAM_ID):          $ESCROW_ID"
    echo "GMP Endpoint (SOLANA_GMP_ID):        $GMP_ID"
    echo "Outflow (SOLANA_OUTFLOW_ID):         $OUTFLOW_ID"
} > "$LOG_FILE"
echo " Deployment log saved to: $LOG_FILE"

# Clean up temporary keypair
rm -rf "$TEMP_KEYPAIR_DIR"

echo ""
echo "========================================="
echo " Deployment Complete!"
echo "========================================="
echo ""
echo " Deployed Program IDs:"
echo "   SOLANA_PROGRAM_ID=$ESCROW_ID"
echo "   SOLANA_GMP_ID=$GMP_ID"
echo "   SOLANA_OUTFLOW_ID=$OUTFLOW_ID"
echo ""
echo " Update the following files:"
echo ""
echo "   1. ${ENV_FILE_NAME}"
echo "      SOLANA_PROGRAM_ID=$ESCROW_ID"
echo ""
echo "   2. coordinator/config/coordinator_${SVM_NETWORK_LABEL}.toml"
echo "      escrow_program_id = \"$ESCROW_ID\""
echo "      (in the [connected_chain_svm] section)"
echo ""
echo "   3. integrated-gmp/config/integrated-gmp_${SVM_NETWORK_LABEL}.toml"
echo "      escrow_program_id = \"$ESCROW_ID\""
echo "      gmp_endpoint_program_id = \"$GMP_ID\""
echo "      (in the [connected_chain_svm] section)"
echo ""
echo "   4. solver/config/solver_${SVM_NETWORK_LABEL}.toml"
echo "      escrow_program_id = \"$ESCROW_ID\""
echo "      (in the [[connected_chain]] SVM section)"
echo ""
echo "   5. Run ${SVM_CONFIGURE_SCRIPT} to set up cross-chain config"
echo "   6. Run ${SVM_CHECK_SCRIPT} to verify"
echo "   (Or use deploy.sh to run the full pipeline)"
echo ""
