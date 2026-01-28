#!/usr/bin/env bash
# SVM Intent Framework Initialize Script
#
# Initializes the program state with the approver pubkey.

set -e


SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(dirname "$PROJECT_DIR")"

# If not in nix shell, re-exec inside nix develop ./nix
if [ -z "$IN_NIX_SHELL" ]; then
    echo "[initialize.sh] Entering nix develop ./nix..."
    exec env NIX_CONFIG="warn-dirty = false" nix develop "$REPO_ROOT/nix" -c bash "$0" "$@"
fi

SVM_RPC_URL="${SVM_RPC_URL:-http://localhost:8899}"
SVM_PAYER_KEYPAIR="${SVM_PAYER_KEYPAIR:-$HOME/.config/solana/id.json}"

if [ -z "$SVM_APPROVER_PUBKEY" ]; then
    echo "[initialize.sh] Missing SVM_APPROVER_PUBKEY"
    exit 1
fi

ARGS=(initialize --payer "$SVM_PAYER_KEYPAIR" --approver "$SVM_APPROVER_PUBKEY" --rpc "$SVM_RPC_URL")
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
