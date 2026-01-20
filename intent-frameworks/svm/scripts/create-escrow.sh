#!/usr/bin/env bash
# SVM Intent Framework Create Escrow Script
#
# Creates a new escrow and deposits tokens atomically.

set -e


SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(dirname "$PROJECT_DIR")"

# If not in nix shell, re-exec inside nix develop ./nix
if [ -z "$IN_NIX_SHELL" ]; then
    echo "[create-escrow.sh] Entering nix develop ./nix..."
    exec env NIX_CONFIG="warn-dirty = false" nix develop "$REPO_ROOT/nix" -c bash "$0" "$@"
fi

SVM_RPC_URL="${SVM_RPC_URL:-http://localhost:8899}"
SVM_PAYER_KEYPAIR="${SVM_PAYER_KEYPAIR:-$HOME/.config/solana/id.json}"
SVM_REQUESTER_KEYPAIR="${SVM_REQUESTER_KEYPAIR:-$SVM_PAYER_KEYPAIR}"

if [ -z "$USD_SVM_MINT_ADDR" ]; then
    echo "[create-escrow.sh] Missing USD_SVM_MINT_ADDR"
    exit 1
fi
if [ -z "$SVM_REQUESTER_TOKEN" ]; then
    echo "[create-escrow.sh] Missing SVM_REQUESTER_TOKEN"
    exit 1
fi
if [ -z "$SVM_SOLVER_PUBKEY" ]; then
    echo "[create-escrow.sh] Missing SVM_SOLVER_PUBKEY"
    exit 1
fi
if [ -z "$SVM_INTENT_ID" ]; then
    echo "[create-escrow.sh] Missing SVM_INTENT_ID"
    exit 1
fi
if [ -z "$SVM_AMOUNT" ]; then
    echo "[create-escrow.sh] Missing SVM_AMOUNT"
    exit 1
fi

ARGS=(create-escrow \
    --payer "$SVM_PAYER_KEYPAIR" \
    --requester "$SVM_REQUESTER_KEYPAIR" \
    --token-mint "$USD_SVM_MINT_ADDR" \
    --requester-token "$SVM_REQUESTER_TOKEN" \
    --solver "$SVM_SOLVER_PUBKEY" \
    --intent-id "$SVM_INTENT_ID" \
    --amount "$SVM_AMOUNT" \
    --rpc "$SVM_RPC_URL")

if [ -n "$SVM_EXPIRY" ]; then
    ARGS+=(--expiry "$SVM_EXPIRY")
fi
if [ -n "$SVM_PROGRAM_ID" ]; then
    ARGS+=(--program-id "$SVM_PROGRAM_ID")
fi

cd "$PROJECT_DIR"

CLI_BIN="$PROJECT_DIR/target/debug/intent_escrow_cli"
if [ ! -x "$CLI_BIN" ]; then
    echo "❌ PANIC: intent_escrow_cli not built. Step 1 (build binaries) failed."
    exit 1
fi
"$CLI_BIN" "${ARGS[@]}"
