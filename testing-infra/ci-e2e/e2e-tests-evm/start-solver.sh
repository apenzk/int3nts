#!/bin/bash

# Start Solver Service for E2E Tests (EVM Connected Chain)
# 
# This script generates a solver configuration for EVM connected chain tests
# and starts the solver service.
#
# Required environment variables (set by run-tests-*.sh):
# - CHAIN1_URL: Hub chain RPC URL (Move VM)
# - EVM_RPC_URL: Connected chain RPC URL (EVM)
# - HUB_CHAIN_ID: Hub chain ID
# - EVM_CHAIN_ID: Connected EVM chain ID
# - ACCOUNT_ADDR: Hub chain module address
# - ESCROW_GMP_ADDR: EVM IntentInflowEscrow contract address
# - EVM_PRIVATE_KEY_ENV: Environment variable name for EVM private key

set -e

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/../util_evm.sh"
source "$SCRIPT_DIR/../chain-connected-evm/utils.sh"

# Setup project root and logging
setup_project_root
setup_logging "solver-start-evm"
cd "$PROJECT_ROOT"

log ""
log " Starting Solver Service (EVM Connected Chain)..."
log "========================================"
log_and_echo " All output logged to: $LOG_FILE"
log ""

# Generate solver config for EVM E2E tests
generate_solver_config_evm() {
    local config_file="$1"
    
    # Get addresses from aptos CLI profiles
    local test_tokens_chain1=$(get_profile_address "test-tokens-chain1")
    local solver_chain1_addr=$(get_profile_address "solver-chain1")
    local chain1_addr=$(get_profile_address "intent-account-chain1")
    
    # Get USDhub metadata on hub chain (32-byte Move address)
    local usdhub_metadata_chain1=$(get_usdxyz_metadata_addr "0x${test_tokens_chain1}" "1")
    
    # Get EVM USDcon address from chain-info.env and pad to 32 bytes
    if [ -f "$PROJECT_ROOT/.tmp/chain-info.env" ]; then
        source "$PROJECT_ROOT/.tmp/chain-info.env"
    fi
    local evm_token_addr="${USD_EVM_ADDR:-}"
    if [ -z "$evm_token_addr" ]; then
        log_and_echo "❌ ERROR: USD_EVM_ADDR not found in chain-info.env"
        exit 1
    fi
    local escrow_addr="${ESCROW_GMP_ADDR:-}"
    if [ -z "$escrow_addr" ]; then
        log_and_echo "❌ ERROR: ESCROW_GMP_ADDR not found in chain-info.env"
        exit 1
    fi
    local outflow_validator_addr="${OUTFLOW_VALIDATOR_ADDR:-}"
    local gmp_endpoint_addr="${GMP_ENDPOINT_ADDR:-}"
    # Lowercase and pad to 32 bytes for Move compatibility (hub chain uses 32-byte addresses)
    local evm_token_no_prefix="${evm_token_addr#0x}"
    local evm_token_lower=$(echo "$evm_token_no_prefix" | tr '[:upper:]' '[:lower:]')
    local usdcon_metadata_evm="0x000000000000000000000000${evm_token_lower}"
    
    # Use environment variables from test setup
    local coordinator_url="${COORDINATOR_URL:-http://127.0.0.1:3333}"
    local hub_rpc="${CHAIN1_URL:-http://127.0.0.1:8080/v1}"
    local evm_rpc="${EVM_RPC_URL:-http://127.0.0.1:8545}"
    local hub_chain_id="${HUB_CHAIN_ID:-1}"
    local evm_chain_id="${EVM_CHAIN_ID:-3}"
    local module_addr="0x${chain1_addr}"
    local escrow_contract="${escrow_addr}"
    local solver_addr="0x${solver_chain1_addr}"
    local evm_private_key_env="${EVM_PRIVATE_KEY_ENV:-SOLVER_EVM_PRIVATE_KEY}"
    
    log "   Generating solver config:"
    log "   - Coordinator URL: $coordinator_url"
    log "   - Hub RPC: $hub_rpc (chain ID: $hub_chain_id)"
    log "   - EVM RPC: $evm_rpc (chain ID: $evm_chain_id)"
    log "   - Hub module address: $module_addr"
    log "   - EVM escrow contract: $escrow_contract"
    log "   - Solver address: $solver_addr"
    log "   - USDhub metadata (hub): $usdhub_metadata_chain1"
    log "   - USDcon metadata (EVM, padded): $usdcon_metadata_evm"
    log "   - EVM outflow validator: $outflow_validator_addr"
    log "   - EVM GMP endpoint: $gmp_endpoint_addr"
    
    cat > "$config_file" << EOF
# Auto-generated solver config for EVM E2E tests
# Generated at: $(date)

[service]
coordinator_url = "$coordinator_url"
polling_interval_ms = 1000  # Poll frequently for tests
e2e_mode = true  # Use aptos CLI with profiles for E2E tests

[hub_chain]
name = "Hub Chain (E2E Test)"
rpc_url = "$hub_rpc"
chain_id = $hub_chain_id
module_addr = "$module_addr"
profile = "solver-chain1"
e2e_mode = true  # Use aptos CLI with profiles for E2E tests

[[connected_chain]]
type = "evm"
name = "EVM Connected Chain (E2E Test)"
rpc_url = "$evm_rpc"
chain_id = $evm_chain_id
escrow_contract_addr = "$escrow_contract"
private_key_env = "$evm_private_key_env"
outflow_validator_addr = "$outflow_validator_addr"
gmp_endpoint_addr = "$gmp_endpoint_addr"

[acceptance]
# Accept USDhub/USDcon swaps at 1:1 rate for E2E testing
# Inflow: offered on EVM (connected), desired on hub
[[acceptance.tokenpair]]
source_chain_id = $evm_chain_id
source_token = "$usdcon_metadata_evm"
target_chain_id = $hub_chain_id
target_token = "$usdhub_metadata_chain1"
ratio = 1.0

# Outflow: offered on hub, desired on EVM (connected)
[[acceptance.tokenpair]]
source_chain_id = $hub_chain_id
source_token = "$usdhub_metadata_chain1"
target_chain_id = $evm_chain_id
target_token = "$usdcon_metadata_evm"
ratio = 1.0

[solver]
profile = "solver-chain1"
address = "$solver_addr"
EOF

    log "   ✅ Config written to: $config_file"
}

# Generate the config file
SOLVER_CONFIG="$PROJECT_ROOT/.tmp/solver-e2e-evm.toml"
mkdir -p "$(dirname "$SOLVER_CONFIG")"
generate_solver_config_evm "$SOLVER_CONFIG"

# Export solver's EVM address for auto-registration
# Hardhat account #2 is used for solver
export SOLVER_EVM_ADDR=$(get_hardhat_account_address "2")
if [ -z "$SOLVER_EVM_ADDR" ]; then
    log_and_echo "❌ ERROR: Failed to get solver EVM address"
    log_and_echo "   Make sure Hardhat is running and get_hardhat_account_address is available"
    exit 1
fi
log "   Exported SOLVER_EVM_ADDR=$SOLVER_EVM_ADDR"

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
