#!/bin/bash

# Start Solver Service for MVM E2E Tests (Multi-Instance)
#
# This script generates a solver configuration for both MVM connected chain instances
# and starts the solver service.

set -e

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/../chain-connected-mvm/utils.sh"

# Setup project root and logging
setup_project_root
setup_logging "solver-start"
cd "$PROJECT_ROOT"

log ""
log " Starting Solver Service (MVM Connected Chains)..."
log "========================================"
log_and_echo " All output logged to: $LOG_FILE"
log ""

# Generate solver config for MVM E2E tests (both instances)
generate_solver_config_mvm() {
    local config_file="$1"

    # Get addresses from aptos CLI profiles
    local test_tokens_chain1=$(get_profile_address "test-tokens-chain1")
    local solver_chain1_addr=$(get_profile_address "solver-chain1")
    local chain1_addr=$(get_profile_address "intent-account-chain1")

    # Get USDhub metadata on hub chain (32-byte Move address)
    local usdhub_metadata_chain1=$(get_usdxyz_metadata_addr "0x${test_tokens_chain1}" "1")

    # Use environment variables or defaults for URLs
    local coordinator_url="${COORDINATOR_URL:-http://127.0.0.1:3333}"
    local hub_rpc="${CHAIN1_URL:-http://127.0.0.1:1000/v1}"
    local hub_chain_id="${HUB_CHAIN_ID:-1}"
    local hub_module_addr="0x${chain1_addr}"
    local solver_addr="0x${solver_chain1_addr}"

    log "   Generating solver config:"
    log "   - Coordinator URL: $coordinator_url"
    log "   - Hub RPC: $hub_rpc (chain ID: $hub_chain_id)"
    log "   - Hub module address: $hub_module_addr"
    log "   - Solver address: $solver_addr"
    log "   - USDhub metadata (hub): $usdhub_metadata_chain1"

    # Start the config with common sections
    cat > "$config_file" << EOF
# Auto-generated solver config for MVM E2E tests (multi-instance)
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

[acceptance]
base_fee_in_move = 1000000  # 0.01 MOVE (8 decimals)

[liquidity]
balance_poll_interval_ms = 10000
in_flight_timeout_secs = 300

# Threshold for USDhub on hub chain
[[liquidity.threshold]]
chain_id = $hub_chain_id
token = "$usdhub_metadata_chain1"
min_balance = 1

# Gas token (MOVE) on hub chain
[[liquidity.threshold]]
chain_id = $hub_chain_id
token = "0x000000000000000000000000000000000000000000000000000000000000000a"
min_balance = 1

[solver]
profile = "solver-chain1"
address = "$solver_addr"
EOF

    # Append connected_chain and tokenpair entries for each MVM instance
    for n in 2 3; do
        mvm_instance_vars "$n"
        load_mvm_chain_info "$n"

        local mvmcon_module_addr=$(get_profile_address "intent-account-chain${n}")
        local test_tokens_mvm_con=$(get_profile_address "test-tokens-chain${n}")
        local usd_con_mvm_con_address=$(get_usdxyz_metadata_addr "0x${test_tokens_mvm_con}" "$n")

        log "   - MVM instance $n: RPC=$MVM_RPC_URL chain_id=$MVM_CHAIN_ID"

        cat >> "$config_file" << EOF

[[connected_chain]]
type = "mvm"
name = "MVM Connected Chain $n (E2E Test)"
rpc_url = "$MVM_RPC_URL"
chain_id = $MVM_CHAIN_ID
module_addr = "0x${mvmcon_module_addr}"
profile = "solver-chain${n}"
e2e_mode = true  # Use aptos CLI with profiles for E2E tests

# Accept USDhub/USDcon swaps at 1:1 rate for E2E testing (instance $n)
# Inflow: offered on MVM connected chain ($n), desired on hub chain (1)
[[acceptance.tokenpair]]
source_chain_id = $MVM_CHAIN_ID
source_token = "$usd_con_mvm_con_address"
target_chain_id = $hub_chain_id
target_token = "$usdhub_metadata_chain1"
ratio = 1.0
fee_bps = 50  # 0.5% fee
move_rate = 0.01  # 1 Octa = 0.01 micro-USD (MOVE 8 dec, USD 6 dec, 1:1 price)

# Outflow: offered on hub chain (1), desired on MVM connected chain ($n)
[[acceptance.tokenpair]]
source_chain_id = $hub_chain_id
source_token = "$usdhub_metadata_chain1"
target_chain_id = $MVM_CHAIN_ID
target_token = "$usd_con_mvm_con_address"
ratio = 1.0
fee_bps = 50  # 0.5% fee
move_rate = 0.01  # 1 Octa = 0.01 micro-USD (MOVE 8 dec, USD 6 dec, 1:1 price)

# Gas token (MOVE) on MVM chain instance $n
[[liquidity.threshold]]
chain_id = $MVM_CHAIN_ID
token = "0x000000000000000000000000000000000000000000000000000000000000000a"
min_balance = 1

# USDcon on MVM chain instance $n
[[liquidity.threshold]]
chain_id = $MVM_CHAIN_ID
token = "$usd_con_mvm_con_address"
min_balance = 1
EOF
    done

    log "   ✅ Config written to: $config_file"
}

# Generate the config file
SOLVER_CONFIG="$PROJECT_ROOT/.tmp/solver-e2e.toml"
mkdir -p "$(dirname "$SOLVER_CONFIG")"
generate_solver_config_mvm "$SOLVER_CONFIG"

# Export solver's MVM address for auto-registration
# All MVM instances share the same solver key (via solver-mvm-shared-key.hex), so one address covers both
SOLVER_MVMCON_ADDR=$(get_profile_address "solver-chain2")
SOLVER_MVMCON_ADDR3=$(get_profile_address "solver-chain3")
if [ -z "$SOLVER_MVMCON_ADDR" ]; then
    log_and_echo "❌ ERROR: Failed to get solver MVM address"
    log_and_echo "   Make sure solver-chain2 profile exists"
    exit 1
fi
if [ "$SOLVER_MVMCON_ADDR" != "$SOLVER_MVMCON_ADDR3" ]; then
    log_and_echo "❌ ERROR: Solver addresses differ across MVM instances (shared key broken)"
    log_and_echo "   solver-chain2: $SOLVER_MVMCON_ADDR"
    log_and_echo "   solver-chain3: $SOLVER_MVMCON_ADDR3"
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
