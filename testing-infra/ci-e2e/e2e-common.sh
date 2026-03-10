#!/bin/bash
# ==============================================================================
# E2E Common Framework
#
# Shared functions for all E2E test scripts. Source this file from individual
# test scripts to avoid duplicating setup, build, service management, and
# test execution logic.
#
# Usage in test scripts:
#   SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
#   source "$SCRIPT_DIR/../e2e-common.sh"
#   e2e_init "evm" "inflow" "$@"
#   e2e_cleanup_pre
#   e2e_build
#   generate_integrated_gmp_keys
#   e2e_setup_chains
#   e2e_start_services
#   ... chain-specific test logic ...
#   e2e_wait_for_fulfillment "inflow" 20
#   ... more test logic ...
#   e2e_cleanup_post
# ==============================================================================

# Globals set by e2e_init:
#   E2E_CHAIN       - chain name (mvm, evm, svm)
#   E2E_FLOW        - flow type (inflow, outflow)
#   SKIP_BUILD      - whether to skip full builds
#   SCRIPT_DIR      - caller's script directory (must be set before sourcing)
#   PROJECT_ROOT    - project root (set by setup_project_root)

# ------------------------------------------------------------------------------
# e2e_init CHAIN FLOW "$@"
#
# Parse flags, source utilities, setup project root and logging.
# CHAIN: mvm | evm | svm
# FLOW:  inflow | outflow
# "$@":  pass through the script's CLI args (e.g. --no-build) for flag parsing
# ------------------------------------------------------------------------------
e2e_init() {
    local chain="$1"; shift
    local flow="$1"; shift

    export E2E_CHAIN="$chain"
    export E2E_FLOW="$flow"

    # Parse flags from remaining args
    SKIP_BUILD=false
    for arg in "$@"; do
        case "$arg" in
            --no-build) SKIP_BUILD=true ;;
        esac
    done
    export SKIP_BUILD

    # Source common utilities (SCRIPT_DIR must be set by caller)
    local ci_e2e_dir
    ci_e2e_dir="$( cd "$SCRIPT_DIR/.." && pwd )"
    source "$ci_e2e_dir/util.sh"
    source "$ci_e2e_dir/util_mvm.sh"

    # Source chain-specific utilities
    case "$chain" in
        evm) source "$ci_e2e_dir/util_evm.sh" ;;
        svm) source "$ci_e2e_dir/util_svm.sh" ;;
    esac

    # Setup project root and logging
    setup_project_root
    setup_logging "run-tests-${chain}-${flow}"
    cd "$PROJECT_ROOT"

    local flow_upper
    flow_upper=$(echo "$flow" | tr '[:lower:]' '[:upper:]')
    local chain_upper
    chain_upper=$(echo "$chain" | tr '[:lower:]' '[:upper:]')
    log_and_echo " E2E Test for Connected ${chain_upper} Chain - ${flow_upper}"
    log_and_echo "============================================="
    log_and_echo " All output logged to: $LOG_FILE"
    log_and_echo ""
}

# ------------------------------------------------------------------------------
# e2e_cleanup_pre
#
# Clean up any existing chains, accounts, and processes.
# ------------------------------------------------------------------------------
e2e_cleanup_pre() {
    log_and_echo " Cleaning up any existing chains, accounts and processes..."
    log_and_echo "=========================================================="
    ./testing-infra/ci-e2e/chain-connected-${E2E_CHAIN}/cleanup.sh
    # SVM needs extra hub cleanup to avoid stale state
    if [ "$E2E_CHAIN" = "svm" ]; then
        ./testing-infra/ci-e2e/chain-hub/stop-chain.sh || true
    fi
}

# ------------------------------------------------------------------------------
# e2e_build
#
# Build binaries (or build-if-missing with --no-build).
# Handles chain-specific build requirements.
# ------------------------------------------------------------------------------
e2e_build() {
    log_and_echo ""
    if [ "$SKIP_BUILD" = "true" ]; then
        log_and_echo " Build if missing (--no-build)"
        log_and_echo "========================================"
        _e2e_build_skip
    else
        log_and_echo " Build bins and pre-pull docker images"
        log_and_echo "========================================"
        _e2e_build_full
    fi

    log_and_echo ""
    docker pull "$APTOS_DOCKER_IMAGE"
}

# Build-if-missing logic (--no-build mode)
_e2e_build_skip() {
    # SVM on-chain programs
    if [ "$E2E_CHAIN" = "svm" ]; then
        if [ ! -f "$PROJECT_ROOT/intent-frameworks/svm/target/deploy/intent_inflow_escrow.so" ] || \
           [ ! -f "$PROJECT_ROOT/intent-frameworks/svm/target/deploy/intent_gmp.so" ] || \
           [ ! -f "$PROJECT_ROOT/intent-frameworks/svm/target/deploy/intent_outflow_validator.so" ]; then
            pushd "$PROJECT_ROOT/intent-frameworks/svm" > /dev/null
            ./scripts/build-with-docker.sh 2>&1 | tail -5
            popd > /dev/null
            log_and_echo "   ✅ SVM: on-chain programs (built)"
        else
            log_and_echo "   ✅ SVM: on-chain programs (exists)"
        fi
    fi

    build_common_bins_if_missing

    case "$E2E_CHAIN" in
        mvm)
            build_if_missing "$PROJECT_ROOT/solver" "cargo build --bin sign_intent" \
                "Solver: sign_intent" \
                "$PROJECT_ROOT/solver/target/debug/sign_intent"
            ;;
        evm)
            build_if_missing "$PROJECT_ROOT/integrated-gmp" "cargo build --bin get_approver_eth_address" \
                "Integrated-GMP: get_approver_eth_address" \
                "$PROJECT_ROOT/integrated-gmp/target/debug/get_approver_eth_address"
            build_if_missing "$PROJECT_ROOT/solver" "cargo build --bin sign_intent" \
                "Solver: sign_intent" \
                "$PROJECT_ROOT/solver/target/debug/sign_intent"
            ;;
        svm)
            build_if_missing "$PROJECT_ROOT/intent-frameworks/svm" "cargo build -p intent_escrow_cli" \
                "SVM: intent_escrow_cli" \
                "$PROJECT_ROOT/intent-frameworks/svm/target/debug/intent_escrow_cli"
            ;;
    esac
}

# Full build logic
_e2e_build_full() {
    # Delete existing binaries to ensure fresh build
    rm -f "$PROJECT_ROOT/target/debug/integrated-gmp" "$PROJECT_ROOT/target/debug/solver" "$PROJECT_ROOT/target/debug/coordinator"
    rm -f "$PROJECT_ROOT/target/release/integrated-gmp" "$PROJECT_ROOT/target/release/solver" "$PROJECT_ROOT/target/release/coordinator"

    # SVM on-chain programs (must come before common bins)
    if [ "$E2E_CHAIN" = "svm" ]; then
        pushd "$PROJECT_ROOT/intent-frameworks/svm" > /dev/null
        ./scripts/build-with-docker.sh 2>&1 | tail -5
        popd > /dev/null
        log_and_echo "   ✅ SVM: on-chain programs (intent_inflow_escrow, intent_gmp, intent_outflow_validator)"
    fi

    pushd "$PROJECT_ROOT/coordinator" > /dev/null
    cargo build --bin coordinator 2>&1 | tail -5
    popd > /dev/null
    log_and_echo "   ✅ Coordinator: coordinator"

    case "$E2E_CHAIN" in
        mvm)
            pushd "$PROJECT_ROOT/integrated-gmp" > /dev/null
            cargo build --bin integrated-gmp --bin generate_keys 2>&1 | tail -5
            popd > /dev/null
            log_and_echo "   ✅ Integrated-GMP: integrated-gmp, generate_keys"

            pushd "$PROJECT_ROOT/solver" > /dev/null
            cargo build --bin solver --bin sign_intent 2>&1 | tail -5
            popd > /dev/null
            log_and_echo "   ✅ Solver: solver, sign_intent"
            ;;
        evm)
            pushd "$PROJECT_ROOT/integrated-gmp" > /dev/null
            cargo build --bin integrated-gmp --bin generate_keys --bin get_approver_eth_address 2>&1 | tail -5
            popd > /dev/null
            log_and_echo "   ✅ Integrated-GMP: integrated-gmp, generate_keys, get_approver_eth_address"

            pushd "$PROJECT_ROOT/solver" > /dev/null
            cargo build --bin solver --bin sign_intent 2>&1 | tail -5
            popd > /dev/null
            log_and_echo "   ✅ Solver: solver, sign_intent"
            ;;
        svm)
            pushd "$PROJECT_ROOT/integrated-gmp" > /dev/null
            cargo build --bin integrated-gmp --bin generate_keys 2>&1 | tail -5
            popd > /dev/null
            log_and_echo "   ✅ Integrated-GMP: integrated-gmp, generate_keys"

            pushd "$PROJECT_ROOT/solver" > /dev/null
            cargo build --bin solver 2>&1 | tail -5
            popd > /dev/null
            log_and_echo "   ✅ Solver: solver"

            pushd "$PROJECT_ROOT/intent-frameworks/svm" > /dev/null
            cargo build -p intent_escrow_cli 2>&1 | tail -5
            popd > /dev/null
            log_and_echo "   ✅ SVM: intent_escrow_cli"
            ;;
    esac
}

# ------------------------------------------------------------------------------
# e2e_setup_chains
#
# Setup chains, deploy contracts, fund accounts.
# Uses chain-specific script paths.
# ------------------------------------------------------------------------------
e2e_setup_chains() {
    log_and_echo " Setting up chains and deploying contracts..."
    log_and_echo "======================================================"

    ./testing-infra/ci-e2e/chain-hub/setup-chain.sh
    ./testing-infra/ci-e2e/chain-hub/setup-requester-solver.sh
    ./testing-infra/ci-e2e/chain-connected-${E2E_CHAIN}/setup-chain.sh
    ./testing-infra/ci-e2e/chain-connected-${E2E_CHAIN}/setup-requester-solver.sh
    ./testing-infra/ci-e2e/chain-hub/deploy-contracts.sh
    ./testing-infra/ci-e2e/chain-connected-${E2E_CHAIN}/deploy-contracts.sh
}

# ------------------------------------------------------------------------------
# e2e_start_services
#
# Start coordinator, integrated-gmp, and solver.
# Verify they are running.
# ------------------------------------------------------------------------------
e2e_start_services() {
    log_and_echo ""
    log_and_echo " Starting coordinator and integrated-gmp..."
    log_and_echo "=========================================================================="
    ./testing-infra/ci-e2e/e2e-tests-${E2E_CHAIN}/start-coordinator.sh
    ./testing-infra/ci-e2e/e2e-tests-${E2E_CHAIN}/start-integrated-gmp.sh

    log_and_echo ""
    log_and_echo " Starting solver service..."
    log_and_echo "======================================="
    ./testing-infra/ci-e2e/e2e-tests-${E2E_CHAIN}/start-solver.sh

    # Verify services started successfully
    ./testing-infra/ci-e2e/verify-solver-running.sh
    ./testing-infra/ci-e2e/verify-integrated-gmp-running.sh
}

# ------------------------------------------------------------------------------
# e2e_wait_for_fulfillment FLOW_TYPE TIMEOUT
#
# Load intent info and wait for solver to fulfill.
# FLOW_TYPE: inflow | outflow
# TIMEOUT: seconds to wait
# ------------------------------------------------------------------------------
e2e_wait_for_fulfillment() {
    local flow_type="$1"
    local timeout="$2"

    if ! load_intent_info "INTENT_ID"; then
        log_and_echo "❌ ERROR: Failed to load intent info"
        exit 1
    fi

    log_and_echo ""
    log_and_echo " Waiting for solver to automatically fulfill..."
    log_and_echo "==========================================================="

    if ! wait_for_solver_fulfillment "$INTENT_ID" "$flow_type" "$timeout"; then
        log_and_echo "❌ ERROR: Solver did not fulfill the intent automatically"
        display_service_logs "Solver fulfillment timeout"
        exit 1
    fi

    log_and_echo "✅ Solver fulfilled the intent automatically!"
    log_and_echo ""
}

# ------------------------------------------------------------------------------
# e2e_cleanup_post
#
# Final cleanup step.
# ------------------------------------------------------------------------------
e2e_cleanup_post() {
    log_and_echo ""
    log_and_echo "✅ E2E ${E2E_FLOW} test completed!"
    log_and_echo ""
    log_and_echo " Cleaning up chains, accounts and processes..."
    log_and_echo "========================================================"
    ./testing-infra/ci-e2e/chain-connected-${E2E_CHAIN}/cleanup.sh
}
