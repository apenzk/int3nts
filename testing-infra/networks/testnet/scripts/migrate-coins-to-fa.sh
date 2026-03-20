#!/bin/bash

# Migrate CoinStore balances to Fungible Asset (FA) stores on Movement Testnet
#
# On Movement, tokens can exist as legacy CoinStore resources or as Fungible
# Assets (FA). The solver's liquidity monitor queries FA balances only
# (primary_fungible_store::balance). If tokens are held in CoinStore, the
# solver sees 0 balance and rejects all drafts.
#
# Usage:
#   nix develop ./nix -c bash -c "./testing-infra/networks/testnet/scripts/migrate-coins-to-fa.sh"

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

source "$SCRIPT_DIR/../lib/env-utils.sh"

# Load .env.testnet
load_env_file "$SCRIPT_DIR/../.env.testnet"

MVM_RPC_URL="https://testnet.movementnetwork.xyz/v1"
MVM_SOLVER_PRIVATE_KEY="${MOVEMENT_SOLVER_PRIVATE_KEY:-}"

source "$SCRIPT_DIR/../../common/scripts/migrate-coins-to-fa.sh"
