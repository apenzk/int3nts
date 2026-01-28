#!/usr/bin/env bash
# Build SVM intent escrow program inside a Docker container.
#
# This avoids local toolchain issues by running in a clean rust:1.84
# environment with a fresh Solana CLI install — same as CI.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "[build-with-docker.sh] Building SVM program in Docker container..."

docker run --rm \
  -v "$PROJECT_DIR:/workspace" \
  -w /workspace \
  rust:1.84 \
  bash -c 'sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)" && ./scripts/build.sh'

echo "[build-with-docker.sh] Done. Output: target/deploy/intent_escrow.so"
