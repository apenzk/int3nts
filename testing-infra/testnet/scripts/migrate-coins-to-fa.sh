#!/bin/bash

# Migrate CoinStore balances to Fungible Asset (FA) stores on Movement Testnet
#
# On Movement, tokens can exist as legacy CoinStore resources or as Fungible
# Assets (FA). The solver's liquidity monitor queries FA balances only
# (primary_fungible_store::balance). If tokens are held in CoinStore, the
# solver sees 0 balance and rejects all drafts.
#
# This script calls 0x1::coin::migrate_to_fungible_store for each token type
# to move balances from CoinStore into the primary fungible store.
#
# Usage:
#   nix develop ./nix -c bash -c "./testing-infra/testnet/scripts/migrate-coins-to-fa.sh"
#
# Reads MOVEMENT_SOLVER_PRIVATE_KEY from testing-infra/testnet/.env.testnet.

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../../.." && pwd )"

# Load .env.testnet
TESTNET_KEYS_FILE="$SCRIPT_DIR/../.env.testnet"
if [ ! -f "$TESTNET_KEYS_FILE" ]; then
    echo "ERROR: .env.testnet not found at $TESTNET_KEYS_FILE"
    exit 1
fi
source "$TESTNET_KEYS_FILE"

if [ -z "${MOVEMENT_SOLVER_PRIVATE_KEY:-}" ]; then
    echo "ERROR: MOVEMENT_SOLVER_PRIVATE_KEY not set in .env.testnet"
    exit 1
fi

RPC_URL="https://testnet.movementnetwork.xyz/v1"

# Token coin types to migrate
COIN_TYPES=(
    "0x1::aptos_coin::AptosCoin"
)

LABELS=(
    "MOVE"
)

echo "Migrating CoinStore -> Fungible Asset"
echo "======================================="
echo "  RPC: $RPC_URL"
echo ""

SUCCESS=0
SKIPPED=0
FAILED=0

for i in "${!COIN_TYPES[@]}"; do
    coin_type="${COIN_TYPES[$i]}"
    label="${LABELS[$i]}"

    echo -n "  $label ($coin_type) ... "

    output=$(movement move run \
        --private-key "$MOVEMENT_SOLVER_PRIVATE_KEY" \
        --url "$RPC_URL" \
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
