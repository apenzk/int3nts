#!/usr/bin/env bash
# SVM Intent Framework Test Script
#
# Runs Rust tests using solana-program-test.

set -e


SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(dirname "$(dirname "$PROJECT_DIR")")"

# If not in nix shell, re-exec inside nix develop ./nix
if [ -z "$IN_NIX_SHELL" ]; then
    echo "[test.sh] Entering nix develop ./nix..."
    exec env NIX_CONFIG="warn-dirty = false" nix develop "$REPO_ROOT/nix" -c bash "$0" "$@"
fi

cd "$PROJECT_DIR"

# Run Rust tests (native Solana, no validator required)
# Suppress verbose logs: set tarpc to error level (suppresses OpenTelemetry warnings)
# and solana_runtime to warn (suppresses DEBUG messages)
echo "[test.sh] Running Rust tests..."
RUST_LOG=tarpc=error,solana_runtime=warn cargo test -p intent_escrow --tests -- --nocapture "$@"

echo "[test.sh] Running gmp-common tests..."
RUST_LOG=tarpc=error,solana_runtime=warn cargo test -p gmp-common --tests -- --nocapture "$@"

echo "[test.sh] Running outflow-validator tests..."
RUST_LOG=tarpc=error,solana_runtime=warn cargo test -p outflow_validator --tests -- --nocapture "$@"
