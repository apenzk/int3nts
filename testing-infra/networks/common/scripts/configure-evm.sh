#!/bin/bash
# Common EVM chain configuration script.
# Called by network-specific wrappers that set the required variables:
#
#   EVM_CHAIN_PREFIX      - Variable prefix (BASE, HYPERLIQUID)
#   EVM_RPC_URL           - RPC endpoint URL
#   EVM_HARDHAT_NETWORK   - Hardhat network name (baseSepolia, baseMainnet, hyperliquidMainnet)
#   EVM_DISPLAY_NAME      - Human-readable name ("Base Sepolia Testnet", "HyperEVM Mainnet")
#   EVM_HUB_CHAIN_ID      - Hub chain ID (from get_chain_id)
#   EVM_DEPLOY_SCRIPT     - Deploy script name for error messages

set -e

# Resolve indirect variable names from CHAIN_PREFIX
PRIVATE_KEY_VAR="${EVM_CHAIN_PREFIX}_DEPLOYER_PRIVATE_KEY"
GMP_ADDR_VAR="${EVM_CHAIN_PREFIX}_GMP_ENDPOINT_ADDR"
ESCROW_ADDR_VAR="${EVM_CHAIN_PREFIX}_INFLOW_ESCROW_ADDR"
OUTFLOW_ADDR_VAR="${EVM_CHAIN_PREFIX}_OUTFLOW_VALIDATOR_ADDR"

echo " Configuring ${EVM_DISPLAY_NAME}"
echo "=================================="
echo ""

require_var "$PRIVATE_KEY_VAR" "${!PRIVATE_KEY_VAR}"
require_var "$GMP_ADDR_VAR" "${!GMP_ADDR_VAR}" "Run ${EVM_DEPLOY_SCRIPT} first"
require_var "MOVEMENT_INTENT_MODULE_ADDR" "$MOVEMENT_INTENT_MODULE_ADDR" "Run deploy-to-movement first"

echo " Configuration:"
echo "   GMP Endpoint:  ${!GMP_ADDR_VAR}"
echo "   Hub Chain ID:  $EVM_HUB_CHAIN_ID"
echo "   Hub Module:    $MOVEMENT_INTENT_MODULE_ADDR"
echo ""

# 1. Verify contracts are deployed
echo " 1. Verifying deployed contracts..."

verify_evm_contract "$EVM_RPC_URL" "${!GMP_ADDR_VAR}" "IntentGmp"

require_var "$ESCROW_ADDR_VAR" "${!ESCROW_ADDR_VAR}" "Run ${EVM_DEPLOY_SCRIPT} first"
verify_evm_contract "$EVM_RPC_URL" "${!ESCROW_ADDR_VAR}" "IntentInflowEscrow"

require_var "$OUTFLOW_ADDR_VAR" "${!OUTFLOW_ADDR_VAR}" "Run ${EVM_DEPLOY_SCRIPT} first"
verify_evm_contract "$EVM_RPC_URL" "${!OUTFLOW_ADDR_VAR}" "IntentOutflowValidator"
echo ""

# 2. Set remote GMP endpoint on GMP endpoint
echo " 2. Setting remote GMP endpoint for hub chain $EVM_HUB_CHAIN_ID..."

cd "$PROJECT_ROOT/intent-frameworks/evm"

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo "   Installing dependencies..."
    npm install
fi

export DEPLOYER_PRIVATE_KEY="${!PRIVATE_KEY_VAR}"
export GMP_ENDPOINT_ADDR="${!GMP_ADDR_VAR}"
export HUB_CHAIN_ID="$EVM_HUB_CHAIN_ID"
export MOVEMENT_INTENT_MODULE_ADDR

set +e
CONFIGURE_OUTPUT=$(npx hardhat run scripts/configure-gmp.js --network "$EVM_HARDHAT_NETWORK" 2>&1)
CONFIGURE_EXIT=$?
set -e

echo "$CONFIGURE_OUTPUT"

if [ $CONFIGURE_EXIT -ne 0 ]; then
    echo "FATAL: Failed to set remote GMP endpoint on IntentGmp"
    exit 1
fi

# 3. Update hub config on escrow and outflow validator
echo " 3. Updating hub config on IntentInflowEscrow and IntentOutflowValidator..."

export INFLOW_ESCROW_ADDR="${!ESCROW_ADDR_VAR}"
export OUTFLOW_VALIDATOR_ADDR="${!OUTFLOW_ADDR_VAR}"

set +e
HUB_CONFIG_OUTPUT=$(npx hardhat run scripts/configure-hub-config.js --network "$EVM_HARDHAT_NETWORK" 2>&1)
HUB_CONFIG_EXIT=$?
set -e

echo "$HUB_CONFIG_OUTPUT"

if [ $HUB_CONFIG_EXIT -ne 0 ]; then
    echo "FATAL: Failed to update hub config on escrow/outflow contracts"
    exit 1
fi

echo ""
echo " ${EVM_DISPLAY_NAME} configuration verified."
