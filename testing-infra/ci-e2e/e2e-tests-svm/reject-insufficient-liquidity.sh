#!/bin/bash

# Verify solver rejects a second draft intent due to insufficient liquidity.
# Requires E2E_FLOW to be set (inflow | outflow) by the caller (e2e_init).

set -eo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/../util_svm.sh"

setup_project_root
cd "$PROJECT_ROOT"

# Resolve chain addresses for the second draft
HUB_CHAIN_ID=1
CONNECTED_CHAIN_ID=901
HUB_MODULE_ADDR=$(get_profile_address "intent-account-chain1")
TEST_TOKENS_HUB=$(get_profile_address "test-tokens-chain1")
REQUESTER_HUB_ADDR=$(get_profile_address "requester-chain1")
USDHUB_METADATA_HUB=$(get_usdxyz_metadata_addr "0x$TEST_TOKENS_HUB" "1")

source "$PROJECT_ROOT/.tmp/chain-info.env" 2>/dev/null || true
SVM_TOKEN_HEX=$(svm_pubkey_to_hex "$USD_SVM_MINT_ADDR")

EXPIRY_TIME=$(date -d "+1 hour" +%s)
SECOND_INTENT_ID="0x$(openssl rand -hex 32)"

if [ "$E2E_FLOW" = "inflow" ]; then
    DRAFT_DATA=$(build_draft_data \
        "$SVM_TOKEN_HEX" \
        "1030000" \
        "$CONNECTED_CHAIN_ID" \
        "$USDHUB_METADATA_HUB" \
        "1015000" \
        "$HUB_CHAIN_ID" \
        "$EXPIRY_TIME" \
        "$SECOND_INTENT_ID" \
        "$REQUESTER_HUB_ADDR" \
        "15150" \
        "{\"chain_addr\": \"$HUB_MODULE_ADDR\", \"flow_type\": \"inflow\", \"connected_chain_type\": \"svm\"}")
else
    REQUESTER_SVM_ADDR=$(svm_pubkey_to_hex "$REQUESTER_SVM_PUBKEY")
    DRAFT_DATA=$(build_draft_data \
        "$USDHUB_METADATA_HUB" \
        "1030000" \
        "$HUB_CHAIN_ID" \
        "$SVM_TOKEN_HEX" \
        "1015000" \
        "$CONNECTED_CHAIN_ID" \
        "$EXPIRY_TIME" \
        "$SECOND_INTENT_ID" \
        "$REQUESTER_HUB_ADDR" \
        "15150" \
        "{\"chain_addr\": \"$HUB_MODULE_ADDR\", \"flow_type\": \"outflow\", \"connected_chain_type\": \"svm\", \"requester_addr_connected_chain\": \"$REQUESTER_SVM_ADDR\"}")
fi

assert_solver_rejects_draft "$REQUESTER_HUB_ADDR" "$DRAFT_DATA" "$EXPIRY_TIME"
log_and_echo "✅ Solver correctly rejected second intent due to insufficient liquidity!"
