#!/bin/bash

# Verify solver rejects a second draft intent due to insufficient liquidity.
# Requires E2E_FLOW to be set (inflow | outflow) by the caller (e2e_init).

set -eo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/../util_evm.sh"
source "$SCRIPT_DIR/../chain-connected-evm/utils.sh"

setup_project_root
cd "$PROJECT_ROOT"

# Resolve chain addresses for the second draft
CONNECTED_CHAIN_ID=31337
HUB_CHAIN_ID=1
HUB_MODULE_ADDR=$(get_profile_address "intent-account-chain1")
TEST_TOKENS_HUB=$(get_profile_address "test-tokens-chain1")
REQUESTER_HUB_ADDR=$(get_profile_address "requester-chain1")
USDHUB_METADATA_HUB=$(get_usdxyz_metadata_addr "0x$TEST_TOKENS_HUB" "1")

source "$PROJECT_ROOT/.tmp/chain-info.env" 2>/dev/null || true
EVM_TOKEN_ADDR_NO_PREFIX="${USD_EVM_ADDR#0x}"
EVM_TOKEN_ADDR_LOWER=$(echo "$EVM_TOKEN_ADDR_NO_PREFIX" | tr '[:upper:]' '[:lower:]')
EVM_TOKEN_METADATA="0x000000000000000000000000${EVM_TOKEN_ADDR_LOWER}"

EXPIRY_TIME=$(date -d "+1 hour" +%s)
SECOND_INTENT_ID="0x$(openssl rand -hex 32)"

if [ "$E2E_FLOW" = "inflow" ]; then
    DRAFT_DATA=$(build_draft_data \
        "$EVM_TOKEN_METADATA" \
        "1030000" \
        "$CONNECTED_CHAIN_ID" \
        "$USDHUB_METADATA_HUB" \
        "1015000" \
        "$HUB_CHAIN_ID" \
        "$EXPIRY_TIME" \
        "$SECOND_INTENT_ID" \
        "$REQUESTER_HUB_ADDR" \
        "15150" \
        "{\"chain_addr\": \"$HUB_MODULE_ADDR\", \"flow_type\": \"inflow\", \"connected_chain_type\": \"evm\"}")
else
    REQUESTER_EVM_ADDR=$(get_hardhat_account_address "1")
    DRAFT_DATA=$(build_draft_data \
        "$USDHUB_METADATA_HUB" \
        "1030000" \
        "$HUB_CHAIN_ID" \
        "$EVM_TOKEN_METADATA" \
        "1015000" \
        "$CONNECTED_CHAIN_ID" \
        "$EXPIRY_TIME" \
        "$SECOND_INTENT_ID" \
        "$REQUESTER_HUB_ADDR" \
        "15150" \
        "{\"chain_addr\": \"$HUB_MODULE_ADDR\", \"flow_type\": \"outflow\", \"connected_chain_type\": \"evm\", \"requester_addr_connected_chain\": \"$REQUESTER_EVM_ADDR\"}")
fi

assert_solver_rejects_draft "$REQUESTER_HUB_ADDR" "$DRAFT_DATA" "$EXPIRY_TIME"
log_and_echo "✅ Solver correctly rejected second intent due to insufficient liquidity!"
