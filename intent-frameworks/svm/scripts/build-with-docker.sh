#!/usr/bin/env bash
# Build SVM intent escrow program inside a Docker container.
#
# This avoids local toolchain issues by running in a clean rust:1.84
# environment with a fresh Solana CLI install — same as CI.
#
# Uses a Docker volume to cache the Solana installation between runs.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Create a named volume for Solana cache if it doesn't exist
SOLANA_CACHE_VOLUME="int3nts-solana-cache"

echo "[build-with-docker.sh] Building SVM program in Docker container..."

HOST_UID=$(id -u)
HOST_GID=$(id -g)

docker run --rm \
  -v "$PROJECT_DIR:/workspace" \
  -v "$SOLANA_CACHE_VOLUME:/solana-cache" \
  -w /workspace \
  -e HOME=/solana-cache \
  -e HOST_UID="$HOST_UID" \
  -e HOST_GID="$HOST_GID" \
  rust:1.84 \
  bash -c '
    set -e
    SOLANA_BIN="/solana-cache/.local/share/solana/install/active_release/bin"
    if [ ! -f "$SOLANA_BIN/cargo-build-sbf" ]; then
      echo "[build-with-docker.sh] Installing Solana CLI (first run or cache miss)..."
      sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)" || {
        echo "[build-with-docker.sh] ERROR: Failed to install Solana CLI"
        exit 1
      }
    else
      echo "[build-with-docker.sh] Using cached Solana CLI..."
    fi
    export PATH="$SOLANA_BIN:$PATH"
    ./scripts/build.sh
    # Fix ownership of entire target dir for host user (not just deploy/)
    # so subsequent host-side cargo builds can write to target/
    chown -R "$HOST_UID:$HOST_GID" /workspace/target 2>/dev/null || true
  '

echo "[build-with-docker.sh] Done. Output: target/deploy/intent_escrow.so"
