#!/bin/bash

# Cleanup for E2E Tests
#
# This script stops all chains and coordinator/integrated-gmp processes.
# Profile cleanup is handled by individual stop-chain.sh scripts.
# Used by both Aptos and EVM e2e tests.

set -e

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"

# Setup project root and logging
setup_project_root
setup_logging "cleanup"
cd "$PROJECT_ROOT"

log_and_echo " Cleaning up chains and processes..."

# Delete logs folder for fresh start
rm -rf "$PROJECT_ROOT/.tmp/e2e-tests"

# Stop both EVM instances
./testing-infra/ci-e2e/chain-connected-evm/stop-chain.sh 1 || true
./testing-infra/ci-e2e/chain-connected-evm/stop-chain.sh 2 || true
./testing-infra/ci-e2e/chain-hub/stop-chain.sh
# Stop both MVM instances
./testing-infra/ci-e2e/chain-connected-mvm/stop-chain.sh 2 || true
./testing-infra/ci-e2e/chain-connected-mvm/stop-chain.sh 3 || true
stop_coordinator
stop_integrated_gmp
stop_solver

# Delete target folders to ensure fresh binaries are built (skip with --no-build)
if [ "$SKIP_BUILD" = "true" ]; then
    log_and_echo "   Keeping target folders (--no-build)"
else
    log_and_echo "   Deleting target folders for fresh builds..."
    rm -rf "$PROJECT_ROOT/coordinator/target"
    rm -rf "$PROJECT_ROOT/integrated-gmp/target"
    rm -rf "$PROJECT_ROOT/solver/target"
fi

# Clean up ephemeral test config to leave clean state
rm -f "$PROJECT_ROOT/testing-infra/ci-e2e/.integrated-gmp-keys.env"
rm -f "$PROJECT_ROOT/.tmp/intent-info.env"
rm -f "$PROJECT_ROOT/.tmp/chain-info.env"
rm -f "$PROJECT_ROOT/.tmp/chain-info-mvm2.env"
rm -f "$PROJECT_ROOT/.tmp/chain-info-mvm3.env"
rm -f "$PROJECT_ROOT/.tmp/chain-info-evm1.env"
rm -f "$PROJECT_ROOT/.tmp/chain-info-evm2.env"
rm -f "$PROJECT_ROOT/.tmp/solver-e2e.toml"
rm -f "$PROJECT_ROOT/.tmp/solver-mvm-shared-key.hex"
rm -f "$PROJECT_ROOT/.tmp/solver-e2e-evm.toml"
rm -f "$PROJECT_ROOT/.tmp/docker-compose-mvm2.yml"
rm -f "$PROJECT_ROOT/.tmp/docker-compose-mvm3.yml"
rm -f "$PROJECT_ROOT/coordinator/config/coordinator-e2e-ci-testing.toml"
rm -f "$PROJECT_ROOT/integrated-gmp/config/integrated-gmp-e2e-ci-testing.toml"
rm -f "$PROJECT_ROOT/solver/config/solver-e2e-ci-testing.toml"

log_and_echo "✅ Cleanup complete"
