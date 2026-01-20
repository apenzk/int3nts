#!/usr/bin/env bash
# SVM Intent Framework Build Script
#
# Native Solana build with edition2024 compatibility workarounds.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

echo "[build.sh] Building native Solana program..."

# Add Solana CLI and rustup to PATH
export PATH="$HOME/.local/share/solana/install/active_release/bin:$HOME/.cargo/bin:$PATH"

# Step 1: Generate lockfile with pinned dependencies
if [ ! -f "Cargo.lock" ]; then
    echo "[build.sh] Generating Cargo.lock..."
    cargo generate-lockfile
    
    # Pin blake3 and constant_time_eq to avoid edition2024
    # blake3 1.8.3+ uses edition2024, which Cargo <1.85 can't parse
    echo "[build.sh] Pinning dependencies to avoid edition2024..."
    cargo update -p blake3 --precise 1.8.2
    cargo update -p constant_time_eq --precise 0.3.1
fi

# Step 2: Downgrade Cargo.lock to version 3 (older platform-tools can't read v4)
LOCK_VERSION=$(head -5 "$PROJECT_DIR/Cargo.lock" | grep "^version" || echo "")
if echo "$LOCK_VERSION" | grep -q "version = 4"; then
    echo "[build.sh] Downgrading Cargo.lock from v4 to v3..."
    sed -i 's/^version = 4$/version = 3/' "$PROJECT_DIR/Cargo.lock"
fi

# Step 3: Build with Solana toolchain
echo "[build.sh] Environment:"
echo "  cargo-build-sbf: $(which cargo-build-sbf 2>/dev/null || echo 'not found')"
echo "  solana: $(solana --version 2>/dev/null || echo 'not found')"

echo "[build.sh] Running cargo build-sbf..."
cargo build-sbf --manifest-path programs/intent_escrow/Cargo.toml -- --locked

echo "[build.sh] Build complete!"
echo "[build.sh] Output: target/deploy/intent_escrow.so"
