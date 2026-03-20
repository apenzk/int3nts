#!/bin/bash
# Common script to migrate CoinStore balances to Fungible Asset (FA) stores.
# Called by network-specific wrappers that set the required variables:
#
#   MVM_RPC_URL                    - Movement RPC endpoint
#   MVM_SOLVER_PRIVATE_KEY         - Solver private key
#
# Also expects env-utils.sh to be sourced.

set -euo pipefail

require_var "MVM_SOLVER_PRIVATE_KEY" "$MVM_SOLVER_PRIVATE_KEY"

# Token coin types to migrate
COIN_TYPES=(
    "0x1::aptos_coin::AptosCoin"
)

LABELS=(
    "MOVE"
)

echo "Migrating CoinStore -> Fungible Asset"
echo "======================================="
echo "  RPC: $MVM_RPC_URL"
echo ""

SUCCESS=0
SKIPPED=0
FAILED=0

for i in "${!COIN_TYPES[@]}"; do
    coin_type="${COIN_TYPES[$i]}"
    label="${LABELS[$i]}"

    echo -n "  $label ($coin_type) ... "

    output=$(movement move run \
        --private-key "$MVM_SOLVER_PRIVATE_KEY" \
        --url "$MVM_RPC_URL" \
        --function-id "0x1::coin::migrate_to_fungible_store" \
        --type-args "$coin_type" \
        --assume-yes 2>&1) && {
        echo "migrated"
        ((SUCCESS++))
    } || {
        if echo "$output" | grep -qi "ECOIN_STORE_NOT_PUBLISHED"; then
            echo "already FA (skipped)"
            ((SKIPPED++))
        else
            echo "FAILED"
            echo "    $output" | head -5
            ((FAILED++))
        fi
    }
done

echo ""
echo "Done: $SUCCESS migrated, $SKIPPED skipped, $FAILED failed"

if [ "$FAILED" -gt 0 ]; then
    exit 1
fi
