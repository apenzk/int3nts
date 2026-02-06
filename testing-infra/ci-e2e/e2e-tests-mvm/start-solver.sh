#!/bin/bash

# Start Solver Service for E2E Tests
# 
# This script generates a solver configuration from aptos CLI profiles
# and starts the solver service.
#
# Optional environment variables:
# - CHAIN1_URL: Hub chain RPC URL (default: http://127.0.0.1:8080/v1)
# - MVMCON_URL: Connected chain RPC URL (default: http://127.0.0.1:8082/v1)
# - HUB_CHAIN_ID: Hub chain ID (default: 1)
# - MVMCON_CHAIN_ID: Connected chain ID (default: 2)
# - COORDINATOR_URL: Coordinator URL (default: http://127.0.0.1:3333)

set -e

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"

# Setup project root and logging
setup_project_root
setup_logging "solver-start"
cd "$PROJECT_ROOT"

log ""
log " Starting Solver Service..."
log "========================================"
log_and_echo " All output logged to: $LOG_FILE"
log ""

# Generate solver config for MVM E2E tests
generate_solver_config_mvm() {
    local config_file="$1"
    
    # Get addresses from aptos CLI profiles (same as other test scripts)
    local chain1_addr=$(get_profile_address "intent-account-chain1")
    local mvmcon_module_addr=$(get_profile_address "intent-account-chain2")
    local solver_chain1_addr=$(get_profile_address "solver-chain1")
    local test_tokens_chain1=$(get_profile_address "test-tokens-chain1")
    local test_tokens_mvm_con=$(get_profile_address "test-tokens-chain2")
    
    # Get USDhub/USDcon metadata addresses (for acceptance config)
    local usdhub_metadata_chain1=$(get_usdxyz_metadata_addr "0x${test_tokens_chain1}" "1")
    local usd_con_mvm_con_address=$(get_usdxyz_metadata_addr "0x${test_tokens_mvm_con}" "2")
    
    # Use environment variables or defaults for URLs
    local coordinator_url="${COORDINATOR_URL:-http://127.0.0.1:3333}"
    local hub_rpc="${CHAIN1_URL:-http://127.0.0.1:8080/v1}"
    local connected_rpc="${MVMCON_URL:-http://127.0.0.1:8082/v1}"
    local hub_chain_id="${HUB_CHAIN_ID:-1}"
    local connected_chain_id="${MVMCON_CHAIN_ID:-2}"
    local hub_module_addr="0x${chain1_addr}"
    local connected_module_addr="0x${mvmcon_module_addr}"
    local solver_addr="0x${solver_chain1_addr}"
    
    log "   Generating solver config:"
    log "   - Coordinator URL: $coordinator_url"
    log "   - Hub RPC: $hub_rpc (chain ID: $hub_chain_id)"
    log "   - Connected RPC: $connected_rpc (chain ID: $connected_chain_id)"
    log "   - Hub module address: $hub_module_addr"
    log "   - Connected module address: $connected_module_addr"
    log "   - Solver address: $solver_addr"
    log "   - USDhub metadata chain 1: $usdhub_metadata_chain1"
    log "   - USDcon metadata chain 2: $usd_con_mvm_con_address"
    
    cat > "$config_file" << EOF
# Auto-generated solver config for MVM E2E tests
# Generated at: $(date)

[service]
coordinator_url = "$coordinator_url"
polling_interval_ms = 1000  # Poll frequently for tests
e2e_mode = true  # Use aptos CLI with profiles for E2E tests

[hub_chain]
name = "Hub Chain (E2E Test)"
rpc_url = "$hub_rpc"
chain_id = $hub_chain_id
module_addr = "$hub_module_addr"
profile = "solver-chain1"
e2e_mode = true  # Use aptos CLI with profiles for E2E tests

[[connected_chain]]
type = "mvm"
name = "Connected Chain (E2E Test)"
rpc_url = "$connected_rpc"
chain_id = $connected_chain_id
module_addr = "$connected_module_addr"
profile = "solver-chain2"
e2e_mode = true  # Use aptos CLI with profiles for E2E tests

[acceptance]
# Accept USDhub/USDcon swaps at 1:1 rate for E2E testing
# Inflow: offered on connected chain (2), desired on hub chain (1)
[[acceptance.tokenpair]]
source_chain_id = $connected_chain_id
source_token = "$usd_con_mvm_con_address"
target_chain_id = $hub_chain_id
target_token = "$usdhub_metadata_chain1"
ratio = 1.0

# Outflow: offered on hub chain (1), desired on connected chain (2)
[[acceptance.tokenpair]]
source_chain_id = $hub_chain_id
source_token = "$usdhub_metadata_chain1"
target_chain_id = $connected_chain_id
target_token = "$usd_con_mvm_con_address"
ratio = 1.0

[solver]
profile = "solver-chain1"
address = "$solver_addr"
EOF

    log "   ✅ Config written to: $config_file"
}

# Generate the config file
SOLVER_CONFIG="$PROJECT_ROOT/.tmp/solver-e2e.toml"
mkdir -p "$(dirname "$SOLVER_CONFIG")"
generate_solver_config_mvm "$SOLVER_CONFIG"

# Export solver's connected chain address for auto-registration
SOLVER_MVMCON_ADDR=$(get_profile_address "solver-chain2")
if [ -z "$SOLVER_MVMCON_ADDR" ]; then
    log_and_echo "❌ ERROR: Failed to get solver Chain 2 address"
    log_and_echo "   Make sure solver-chain2 profile exists"
    exit 1
fi
export SOLVER_MVMCON_ADDR="0x${SOLVER_MVMCON_ADDR}"
log "   Exported SOLVER_MVMCON_ADDR=$SOLVER_MVMCON_ADDR"

# Unset testnet keys to prevent accidental use (E2E tests use profiles only)
unset MOVEMENT_SOLVER_PRIVATE_KEY
log "   Unset MOVEMENT_SOLVER_PRIVATE_KEY (E2E tests use profile keys only)"

# Start the solver service
if start_solver "$LOG_DIR/solver.log" "info" "$SOLVER_CONFIG"; then
    log ""
    log_and_echo "✅ Solver started successfully"
    log_and_echo "   PID: $SOLVER_PID"
    log_and_echo "   Config: $SOLVER_CONFIG"
    log_and_echo "   Logs: $LOG_DIR/solver.log"
else
    log ""
    log_and_echo "❌ PANIC: Solver failed to start. Step 1 (build binaries) failed."
    exit 1
fi
