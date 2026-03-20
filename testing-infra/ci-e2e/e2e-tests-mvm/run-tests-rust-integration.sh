#!/bin/bash

# E2E Integration Test Runner - Rust Integration Tests
#
# This script runs the Rust integration tests for coordinator and solver.
# It sets up chains, deploys contracts, starts services, then runs Rust tests.

# -e: exit on error; -o pipefail: fail pipeline if ANY command fails (not just the last).
# Without pipefail, `grep ... | sed ...` silently succeeds even when grep finds no match.
set -eo pipefail

# Parse flags
SKIP_BUILD=false
for arg in "$@"; do
    case "$arg" in
        --no-build) SKIP_BUILD=true ;;
    esac
done
export SKIP_BUILD

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"

# Setup project root
setup_project_root
cd "$PROJECT_ROOT"

echo " Rust Integration Tests"
echo "========================="
echo ""

echo " Step 0: Cleaning up any existing chains, accounts and processes..."
echo "================================================================"
./testing-infra/ci-e2e/chain-connected-mvm/cleanup.sh

echo ""
if [ "$SKIP_BUILD" = "true" ]; then
    echo " Step 1: Build if missing (--no-build)"
    echo "========================================"
    build_common_bins_if_missing
    build_if_missing "$PROJECT_ROOT/solver" "cargo build --bin sign_intent" \
        "Solver: sign_intent" \
        "$PROJECT_ROOT/solver/target/debug/sign_intent"
else
    echo " Step 1: Build bins and pre-pull docker images"
    echo "========================================"
    # Delete existing binaries to ensure fresh build
    rm -f "$PROJECT_ROOT/target/debug/integrated-gmp" "$PROJECT_ROOT/target/debug/solver" "$PROJECT_ROOT/target/debug/coordinator"
    rm -f "$PROJECT_ROOT/target/release/integrated-gmp" "$PROJECT_ROOT/target/release/solver" "$PROJECT_ROOT/target/release/coordinator"

    pushd "$PROJECT_ROOT/coordinator" > /dev/null
    cargo build --bin coordinator 2>&1 | tail -5
    popd > /dev/null
    echo "   ✅ Coordinator: coordinator"

    pushd "$PROJECT_ROOT/integrated-gmp" > /dev/null
    cargo build --bin integrated-gmp --bin generate_keys 2>&1 | tail -5
    popd > /dev/null
    echo "   ✅ Integrated-GMP: integrated-gmp, generate_keys"

    pushd "$PROJECT_ROOT/solver" > /dev/null
    cargo build --bin solver --bin sign_intent 2>&1 | tail -5
    popd > /dev/null
    echo "   ✅ Solver: solver, sign_intent"
fi

echo ""
docker pull "$APTOS_DOCKER_IMAGE"

echo " Step 2: Generating integrated-gmp keys..."
echo "======================================="
generate_integrated_gmp_keys
echo ""

echo " Step 3: Setting up chains, deploying contracts, funding accounts"
echo "===================================================================="
./testing-infra/ci-e2e/chain-hub/setup-chain.sh
./testing-infra/ci-e2e/chain-hub/setup-requester-solver.sh

# Generate shared solver key so both MVM instances use the same solver address
mkdir -p "$PROJECT_ROOT/.tmp"
openssl rand -hex 32 | sed 's/^/0x/' > "$PROJECT_ROOT/.tmp/solver-mvm-shared-key.hex"

./testing-infra/ci-e2e/chain-connected-mvm/setup-chain.sh 2
./testing-infra/ci-e2e/chain-connected-mvm/setup-requester-solver.sh 2
./testing-infra/ci-e2e/chain-hub/deploy-contracts.sh
./testing-infra/ci-e2e/chain-connected-mvm/deploy-contracts.sh 2

./testing-infra/ci-e2e/chain-connected-mvm/setup-chain.sh 3
./testing-infra/ci-e2e/chain-connected-mvm/setup-requester-solver.sh 3
./testing-infra/ci-e2e/chain-connected-mvm/deploy-contracts.sh 3

echo ""
echo " Step 4: Configuring and starting services..."
echo "================================================"
./testing-infra/ci-e2e/e2e-tests-mvm/start-coordinator.sh
./testing-infra/ci-e2e/e2e-tests-mvm/start-integrated-gmp.sh

echo ""
echo " Step 5: Running Rust integration tests..."
echo "============================================="
./testing-infra/ci-e2e/e2e-tests-mvm/coordinator-rust-integration-tests.sh

echo ""
echo "✅ Rust integration tests completed!"
echo ""

echo " Step 6: Cleaning up chains, accounts and processes..."
echo "========================================================"
./testing-infra/ci-e2e/chain-connected-mvm/cleanup.sh

