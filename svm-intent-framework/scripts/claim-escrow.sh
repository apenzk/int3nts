#!/usr/bin/env bash
# SVM Intent Framework Claim Escrow Script
#
# Claims escrow funds using a verifier signature.

set -e


SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(dirname "$PROJECT_DIR")"

# If not in nix shell, re-exec inside nix develop
if [ -z "$IN_NIX_SHELL" ]; then
    echo "[claim-escrow.sh] Entering nix develop..."
    exec env NIX_CONFIG="warn-dirty = false" nix develop "$REPO_ROOT" -c bash "$0" "$@"
fi

SVM_RPC_URL="${SVM_RPC_URL:-http://localhost:8899}"
SVM_PAYER_KEYPAIR="${SVM_PAYER_KEYPAIR:-$HOME/.config/solana/id.json}"

if [ -z "$SVM_SOLVER_TOKEN" ]; then
    echo "[claim-escrow.sh] Missing SVM_SOLVER_TOKEN"
    exit 1
fi
if [ -z "$SVM_INTENT_ID" ]; then
    echo "[claim-escrow.sh] Missing SVM_INTENT_ID"
    exit 1
fi
if [ -z "$SVM_SIGNATURE_HEX" ]; then
    echo "[claim-escrow.sh] Missing SVM_SIGNATURE_HEX"
    exit 1
fi

ARGS=(claim \
    --payer "$SVM_PAYER_KEYPAIR" \
    --solver-token "$SVM_SOLVER_TOKEN" \
    --intent-id "$SVM_INTENT_ID" \
    --signature "$SVM_SIGNATURE_HEX" \
    --rpc "$SVM_RPC_URL")

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
