#!/usr/bin/env bash
# SVM Intent Framework Deploy Script
#
# Builds and deploys the native Solana program.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(dirname "$PROJECT_DIR")"

# If not in nix shell, re-exec inside nix develop ./nix
if [ -z "$IN_NIX_SHELL" ]; then
    echo "[deploy.sh] Entering nix develop ./nix..."
    exec nix develop "$REPO_ROOT/nix" -c bash "$0" "$@"
fi

cd "$PROJECT_DIR"

SOLANA_URL="${SOLANA_URL:-http://localhost:8899}"
PROGRAM_KEYPAIR="${PROGRAM_KEYPAIR:-$PROJECT_DIR/target/deploy/intent_escrow-keypair.json}"
PROGRAM_SO="${PROGRAM_SO:-$PROJECT_DIR/target/deploy/intent_escrow.so}"

echo "[deploy.sh] Building program..."
./scripts/build.sh

if [ ! -f "$PROGRAM_KEYPAIR" ]; then
    echo "[deploy.sh] Missing program keypair: $PROGRAM_KEYPAIR"
    echo "[deploy.sh] Create one with: solana-keygen new -o \"$PROGRAM_KEYPAIR\""
    exit 1
fi

if [ ! -f "$PROGRAM_SO" ]; then
    echo "[deploy.sh] Missing program binary: $PROGRAM_SO"
    exit 1
fi

echo "[deploy.sh] Deploying to $SOLANA_URL..."
solana program deploy --url "$SOLANA_URL" "$PROGRAM_SO" --program-id "$PROGRAM_KEYPAIR"

PROGRAM_ID="$(solana address -k "$PROGRAM_KEYPAIR")"
echo "[deploy.sh] Program deployed: $PROGRAM_ID"
echo "[deploy.sh] Export for CLI: SVM_PROGRAM_ID=$PROGRAM_ID"
