#!/usr/bin/env bash
# Run all chain-clients unit tests
# Usage: ./chain-clients/scripts/test.sh
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CHAIN_CLIENTS_DIR="$(dirname "$SCRIPT_DIR")"

echo "Running chain-clients/common tests..."
cd "$CHAIN_CLIENTS_DIR/common" && RUST_LOG=off cargo test --quiet

echo "Running chain-clients/mvm tests..."
cd "$CHAIN_CLIENTS_DIR/mvm" && RUST_LOG=off cargo test --quiet

echo "Running chain-clients/evm tests..."
cd "$CHAIN_CLIENTS_DIR/evm" && RUST_LOG=off cargo test --quiet

echo "Running chain-clients/svm tests..."
cd "$CHAIN_CLIENTS_DIR/svm" && RUST_LOG=off cargo test --quiet

echo "All chain-clients tests passed."
