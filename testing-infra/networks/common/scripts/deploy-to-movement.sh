#!/bin/bash
# Common Movement deployment script.
# Called by network-specific wrappers that set the required variables:
#
#   MVM_RPC_URL            - Movement RPC endpoint
#   MVM_DISPLAY_NAME       - Human-readable network name ("Movement Bardock Testnet")
#   MVM_NETWORK_LABEL      - testnet/mainnet (for config file paths)
#   MVM_LOG_PREFIX         - Log file prefix (movement-testnet, movement-mainnet)
#   MVM_PUBLISH_FLAGS      - Extra flags for `movement move publish` (e.g. "--dev")
#   MVM_NEXT_STEPS         - Next steps text (chain names to deploy next)
#   MVM_FRONTEND_INTENT_CONTRACT_ADDR_ENV_VAR   - .env.local key for the intent contract addr (e.g. NEXT_PUBLIC_MOVEMENT_TESTNET_INTENT_CONTRACT_ADDR)
#   CALLER_SCRIPT_DIR      - The calling script's directory (for log/env paths)
#
# Also expects env-utils.sh to be sourced (for load_env_file, require_var).

set -e

# Check for movement CLI
if ! command -v movement &> /dev/null; then
    echo "ERROR: movement CLI not found"
    echo ""
    echo "   Install the Movement CLI (Move 2 support):"
    echo ""
    echo "   # For Mac ARM64 (M-series):"
    echo "   curl -LO https://github.com/movementlabsxyz/homebrew-movement-cli/releases/download/bypass-homebrew/movement-move2-testnet-macos-arm64.tar.gz && mkdir -p temp_extract && tar -xzf movement-move2-testnet-macos-arm64.tar.gz -C temp_extract && chmod +x temp_extract/movement && sudo mv temp_extract/movement /usr/local/bin/movement && rm -rf temp_extract"
    echo ""
    echo "   # For Mac Intel (x86_64):"
    echo "   curl -LO https://github.com/movementlabsxyz/homebrew-movement-cli/releases/download/bypass-homebrew/movement-move2-testnet-macos-x86_64.tar.gz && mkdir -p temp_extract && tar -xzf movement-move2-testnet-macos-x86_64.tar.gz -C temp_extract && chmod +x temp_extract/movement && sudo mv temp_extract/movement /usr/local/bin/movement && rm -rf temp_extract"
    echo ""
    echo "   Reference: https://docs.movementnetwork.xyz/devs/movementcli"
    exit 1
fi

echo " Deploying Move Intent Framework to ${MVM_DISPLAY_NAME}"
echo "=============================================================="
echo ""
echo "Movement CLI found: $(movement --version)"
echo ""

require_var "MOVEMENT_DEPLOYER_PRIVATE_KEY" "$MOVEMENT_DEPLOYER_PRIVATE_KEY"
require_var "MOVEMENT_DEPLOYER_ADDR" "$MOVEMENT_DEPLOYER_ADDR"

FUNDER_ADDR="${MOVEMENT_DEPLOYER_ADDR#0x}"
FUNDER_ADDR_FULL="0x${FUNDER_ADDR}"

# Step 1: Setup funding account profile
echo " Step 1: Setting up funding account..."
movement init --profile movement-funder \
  --network custom \
  --rest-url "$MVM_RPC_URL" \
  --private-key "$MOVEMENT_DEPLOYER_PRIVATE_KEY" \
  --skip-faucet \
  --assume-yes 2>/dev/null

echo "   Funder address: $FUNDER_ADDR_FULL"
echo ""

# Step 2: Generate a fresh key pair for module deployment
echo " Step 2: Generating fresh module address..."

TEMP_DIR=$(mktemp -d)
KEY_FILE="$TEMP_DIR/deploy_key"

movement key generate --key-type ed25519 --output-file "$KEY_FILE" --assume-yes 2>/dev/null

DEPLOY_PRIVATE_KEY=$(cat "${KEY_FILE}.key" 2>/dev/null || cat "$KEY_FILE" 2>/dev/null)

TEMP_PROFILE="movement-deploy-temp-$$"
movement init --profile "$TEMP_PROFILE" \
  --network custom \
  --rest-url "$MVM_RPC_URL" \
  --private-key "$DEPLOY_PRIVATE_KEY" \
  --skip-faucet \
  --assume-yes 2>/dev/null

DEPLOY_ADDR=$(movement config show-profiles --profile "$TEMP_PROFILE" 2>/dev/null | jq -r ".Result.\"$TEMP_PROFILE\".account // empty" || echo "")

if [ -z "$DEPLOY_ADDR" ]; then
    echo "ERROR: Failed to extract address from generated key"
    rm -rf "$TEMP_DIR"
    exit 1
fi

DEPLOY_ADDR_FULL="0x${DEPLOY_ADDR}"
echo "   Module address: $DEPLOY_ADDR_FULL"
echo ""

# Step 3: Fund the new address — transfer from deployer
echo " Step 3: Funding module address..."

FUND_AMOUNT=100000000  # 1 MOVE in octas

echo "   Transferring from deployer account..."
movement move run \
  --profile movement-funder \
  --function-id "0x1::aptos_account::transfer" \
  --args "address:$DEPLOY_ADDR_FULL" "u64:$FUND_AMOUNT" \
  --assume-yes
echo "   Transferred $FUND_AMOUNT octas (1 MOVE) from deployer"

# Wait for transaction to propagate
sleep 3

# Verify balance with retry option
while true; do
    echo "   Verifying balance..."
    BALANCE=$(curl -s "$MVM_RPC_URL/accounts/$DEPLOY_ADDR_FULL/resources" 2>/dev/null | jq -r '.[] | select(.type == "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>") | .data.coin.value // "0"' || echo "0")
    if [ -z "$BALANCE" ]; then BALANCE="0"; fi
    echo "   Module address balance: $BALANCE octas"

    if [ "$BALANCE" != "0" ] && [ -n "$BALANCE" ]; then
        echo "   Module address funded"
        break
    fi

    echo ""
    echo "   Balance is still 0."
    echo "   [r] Retry balance check"
    echo "   [y] Continue anyway (deployment may fail)"
    echo "   [n] Cancel deployment"
    read -p "   Choice (r/y/n): " -n 1 -r
    echo

    if [[ $REPLY =~ ^[Rr]$ ]]; then
        echo "   Waiting 3 seconds before retry..."
        sleep 3
        continue
    elif [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "   Continuing with 0 balance..."
        break
    else
        echo "   Deployment cancelled"
        rm -rf "$TEMP_DIR"
        exit 1
    fi
done
echo ""

echo " Configuration:"
echo "   Funder Address: $FUNDER_ADDR_FULL"
echo "   Module Address: $DEPLOY_ADDR_FULL"
echo "   Network: ${MVM_DISPLAY_NAME}"
echo "   RPC URL: $MVM_RPC_URL"
echo ""

# Step 4: Deploy intent-gmp package
echo " Step 4: Deploying intent-gmp package..."
cd "$PROJECT_ROOT/intent-frameworks/mvm/intent-gmp"

# shellcheck disable=SC2086
movement move publish \
  $MVM_PUBLISH_FLAGS \
  --profile "$TEMP_PROFILE" \
  --named-addresses mvmt_intent="$DEPLOY_ADDR_FULL" \
  --assume-yes \
  --included-artifacts none \
  --max-gas 500000 \
  --gas-unit-price 100

echo "intent-gmp deployed"
echo ""

# Wait for intent-gmp to be fully indexed before deploying intent-hub
echo " Waiting for intent-gmp to be indexed..."
sleep 10

# Step 5: Deploy intent-hub package
echo " Step 5: Deploying intent-hub package..."
cd "$PROJECT_ROOT/intent-frameworks/mvm/intent-hub"

# shellcheck disable=SC2086
movement move publish \
  $MVM_PUBLISH_FLAGS \
  --profile "$TEMP_PROFILE" \
  --named-addresses mvmt_intent="$DEPLOY_ADDR_FULL" \
  --assume-yes \
  --included-artifacts none \
  --override-size-check \
  --max-gas 500000 \
  --gas-unit-price 100

echo "intent-hub deployed"
echo ""

# Step 6: Verify deployment
echo " Step 6: Verifying deployment..."

movement move view \
  --profile "$TEMP_PROFILE" \
  --function-id "${DEPLOY_ADDR_FULL}::solver_registry::is_registered" \
  --args "address:$DEPLOY_ADDR_FULL" && {
    echo "   View function works - module deployed correctly with #[view] attribute"
  } || {
    echo "   Warning: View function verification failed"
    echo "   This may indicate the module wasn't deployed correctly"
  }

echo ""

# Steps 7-13: Initialize modules
MODULES=(
    "fa_intent::initialize --args u64:250"
    "solver_registry::initialize"
    "intent_registry::initialize"
    "intent_gmp::initialize"
    "intent_gmp_hub::initialize"
    "gmp_intent_state::initialize"
    "gmp_sender::initialize"
)

STEP=7
for module_call in "${MODULES[@]}"; do
    # Split into function and optional args
    func="${module_call%% --args*}"
    args=""
    if [[ "$module_call" == *"--args"* ]]; then
        args="${module_call#*--args }"
    fi

    module_name="${func%%::*}"
    echo " Step $STEP: Initializing ${module_name}..."

    if [ -n "$args" ]; then
        movement move run \
          --profile "$TEMP_PROFILE" \
          --function-id "${DEPLOY_ADDR_FULL}::${func}" \
          --args $args \
          --assume-yes 2>/dev/null && {
            echo "   ${module_name} initialized"
          } || {
            echo "   ${module_name} may already be initialized (this is OK)"
          }
    else
        movement move run \
          --profile "$TEMP_PROFILE" \
          --function-id "${DEPLOY_ADDR_FULL}::${func}" \
          --assume-yes 2>/dev/null && {
            echo "   ${module_name} initialized"
          } || {
            echo "   ${module_name} may already be initialized (this is OK)"
          }
    fi

    echo ""
    STEP=$((STEP + 1))
done

# Step 14: Output module private key and address
echo " Step 14: Add these to ${ENV_FILE_NAME}:"
echo ""
echo "   MOVEMENT_MODULE_PRIVATE_KEY=$DEPLOY_PRIVATE_KEY"
echo "   MOVEMENT_INTENT_MODULE_ADDR=$DEPLOY_ADDR_FULL"

echo ""

# Save deployment log
LOG_DIR="$CALLER_SCRIPT_DIR/../logs"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/deploy-${MVM_LOG_PREFIX}-$(date +%Y%m%d-%H%M%S).log"
{
    echo "${MVM_DISPLAY_NAME} Deployment — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo ""
    echo "Funder:                    $FUNDER_ADDR_FULL"
    echo "Module address:            $DEPLOY_ADDR_FULL"
    echo "Module private key:        $DEPLOY_PRIVATE_KEY"
    echo "Network:                   ${MVM_DISPLAY_NAME}"
    echo "RPC URL:                   $MVM_RPC_URL"
} > "$LOG_FILE"
echo " Deployment log saved to: $LOG_FILE"

# Cleanup temp profile
echo " Cleaning up..."
rm -rf "$TEMP_DIR"

echo ""
echo " Deployment Complete!"
echo "======================"
echo ""
echo " NEW Module Address:     $DEPLOY_ADDR_FULL"
echo " NEW Module Private Key: $DEPLOY_PRIVATE_KEY"
echo ""
echo " IMPORTANT: Update these files with the new module address and private key:"
echo ""
echo "   1. coordinator/config/coordinator_${MVM_NETWORK_LABEL}.toml:"
echo "      intent_module_addr = \"$DEPLOY_ADDR_FULL\""
echo "      (in the [hub_chain] section)"
echo ""
echo "   2. integrated-gmp/config/integrated-gmp_${MVM_NETWORK_LABEL}.toml:"
echo "      intent_module_addr = \"$DEPLOY_ADDR_FULL\""
echo "      (in the [hub_chain] section)"
echo ""
echo "   3. solver/config/solver_${MVM_NETWORK_LABEL}.toml:"
echo "      module_addr = \"$DEPLOY_ADDR_FULL\""
echo "      (in the [hub_chain] section)"
echo ""
echo "   4. frontend/.env.local:"
echo "      ${MVM_FRONTEND_INTENT_CONTRACT_ADDR_ENV_VAR}=$DEPLOY_ADDR_FULL"
echo ""
echo " Next steps:"
echo "   1. Update the config files above with the new module address"
echo "   2. ${MVM_NEXT_STEPS}"
echo "   3. Run configure-movement-${MVM_NETWORK_LABEL}.sh to set remote GMP endpoints"
echo "   (Or use deploy.sh to run the full pipeline)"
echo ""
