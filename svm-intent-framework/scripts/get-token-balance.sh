#!/usr/bin/env bash
# SVM Intent Framework Token Balance Script
#
# Reads the SPL token account balance.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(dirname "$PROJECT_DIR")"

# If not in nix shell, re-exec inside nix develop
if [ -z "$IN_NIX_SHELL" ]; then
    echo "[get-token-balance.sh] Entering nix develop..."
    exec env NIX_CONFIG="warn-dirty = false" nix develop "$REPO_ROOT" -c bash "$0" "$@"
fi

SVM_RPC_URL="${SVM_RPC_URL:-http://localhost:8899}"

if [ -z "$SVM_TOKEN_ACCOUNT" ]; then
    echo "[get-token-balance.sh] Missing SVM_TOKEN_ACCOUNT"
    exit 1
fi

cd "$PROJECT_DIR"

CLI_BIN="$PROJECT_DIR/target/debug/intent_escrow_cli"
if [ ! -x "$CLI_BIN" ]; then
    echo "❌ PANIC: intent_escrow_cli not built. Step 1 (build binaries) failed."
    exit 1
fi
"$CLI_BIN" get-token-balance --token-account "$SVM_TOKEN_ACCOUNT" --rpc "$SVM_RPC_URL"
