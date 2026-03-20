#!/bin/bash

# Verify solver rejects a second draft intent due to insufficient liquidity.
# Requires E2E_FLOW to be set (inflow | outflow) by the caller (e2e_init).
# Respects MVM_INSTANCE env var for multi-instance testing.

set -eo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/../chain-connected-mvm/utils.sh"

setup_project_root
cd "$PROJECT_ROOT"

# Load MVM instance vars
mvm_instance_vars "${MVM_INSTANCE:-2}"

# Resolve chain addresses for the second draft
CONNECTED_CHAIN_ID=$MVM_CHAIN_ID
HUB_CHAIN_ID=1
HUB_MODULE_ADDR=$(get_profile_address "intent-account-chain1")
TEST_TOKENS_HUB=$(get_profile_address "test-tokens-chain1")
USD_MVMCON_MODULE_ADDR=$(get_profile_address "test-tokens-chain${MVM_INSTANCE}")
REQUESTER_HUB_ADDR=$(get_profile_address "requester-chain1")
USDHUB_METADATA_HUB=$(get_usdxyz_metadata_addr "0x$TEST_TOKENS_HUB" "1")
USD_MVMCON_ADDR=$(get_usdxyz_metadata_addr "0x$USD_MVMCON_MODULE_ADDR" "$MVM_INSTANCE")
EXPIRY_TIME=$(date -d "+180 seconds" +%s)

SECOND_INTENT_ID="0x$(openssl rand -hex 32)"

if [ "$E2E_FLOW" = "inflow" ]; then
    DRAFT_DATA=$(build_draft_data \
        "$USD_MVMCON_ADDR" \
        "1030000" \
        "$CONNECTED_CHAIN_ID" \
        "$USDHUB_METADATA_HUB" \
        "1015000" \
        "$HUB_CHAIN_ID" \
        "$EXPIRY_TIME" \
        "$SECOND_INTENT_ID" \
        "$REQUESTER_HUB_ADDR" \
        "15150" \
        "{\"chain_addr\": \"$HUB_MODULE_ADDR\", \"flow_type\": \"inflow\"}")
else
    REQUESTER_MVMCON_ADDR=$(get_profile_address "requester-chain${MVM_INSTANCE}")
    DRAFT_DATA=$(build_draft_data \
        "$USDHUB_METADATA_HUB" \
        "1030000" \
        "$HUB_CHAIN_ID" \
        "$USD_MVMCON_ADDR" \
        "1015000" \
        "$CONNECTED_CHAIN_ID" \
        "$EXPIRY_TIME" \
        "$SECOND_INTENT_ID" \
        "$REQUESTER_HUB_ADDR" \
        "15150" \
        "{\"chain_addr\": \"$HUB_MODULE_ADDR\", \"flow_type\": \"outflow\", \"requester_addr_connected_chain\": \"$REQUESTER_MVMCON_ADDR\"}")
fi

assert_solver_rejects_draft "$REQUESTER_HUB_ADDR" "$DRAFT_DATA" "$EXPIRY_TIME"
log_and_echo "✅ Solver correctly rejected second intent due to insufficient liquidity!"
