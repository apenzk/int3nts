#!/bin/bash

# Start Solver Service for E2E Tests (SVM Connected Chains)
#
# This script generates a solver configuration for both SVM connected chain instances
# and starts the solver service.

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/../util_svm.sh"
source "$SCRIPT_DIR/../chain-connected-svm/utils.sh"

setup_project_root
setup_logging "solver-start-svm"
cd "$PROJECT_ROOT"

log ""
log " Starting Solver Service (SVM Connected Chains)..."
log "========================================"
log_and_echo " All output logged to: $LOG_FILE"
log ""

generate_solver_config_svm() {
    local config_file="$1"

    local test_tokens_chain1=$(get_profile_address "test-tokens-chain1")
    local solver_chain1_addr=$(get_profile_address "solver-chain1")
    local chain1_addr=$(get_profile_address "intent-account-chain1")
    local usdhub_metadata_chain1=$(get_usdxyz_metadata_addr "0x${test_tokens_chain1}" "1")

    local coordinator_url="${COORDINATOR_URL:-http://127.0.0.1:3333}"
    local hub_rpc="${CHAIN1_URL:-http://127.0.0.1:1000/v1}"
    local hub_chain_id="${HUB_CHAIN_ID:-1}"
    local module_addr="0x${chain1_addr}"
    local solver_addr="0x${solver_chain1_addr}"

    log "   Generating solver config:"
    log "   - Coordinator URL: $coordinator_url"
    log "   - Hub RPC: $hub_rpc (chain ID: $hub_chain_id)"
    log "   - Hub module address: $module_addr"
    log "   - Solver address: $solver_addr"
    log "   - USDhub metadata (hub): $usdhub_metadata_chain1"

    # Start the config with common sections
    cat > "$config_file" << EOF
# Auto-generated solver config for SVM E2E tests (multi-instance)
# Generated at: $(date)

[service]
coordinator_url = "$coordinator_url"
polling_interval_ms = 1000
e2e_mode = true

[hub_chain]
name = "Hub Chain (E2E Test)"
rpc_url = "$hub_rpc"
chain_id = $hub_chain_id
module_addr = "$module_addr"
profile = "solver-chain1"
e2e_mode = true

[acceptance]
base_fee_in_move = 1000000  # 0.01 MOVE (8 decimals) — covers solver gas costs

[liquidity]
balance_poll_interval_ms = 10000
in_flight_timeout_secs = 300

# Threshold for USDhub on hub chain (inflow target token)
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

    # Append connected_chain and tokenpair entries for each SVM instance
    for n in 2 3; do
        svm_instance_vars "$n"
        source "$SVM_CHAIN_INFO_FILE" 2>/dev/null || true

        if [ -z "$USD_SVM_MINT_ADDR" ] || [ -z "$SOLVER_SVM_PUBKEY" ] || [ -z "$SVM_SOLVER_KEYPAIR" ]; then
            log_and_echo "❌ ERROR: Missing SVM chain info for instance $n. Run chain-connected-svm/setup-requester-solver.sh $n first."
            exit 1
        fi
        if [ -z "$SVM_PROGRAM_ID" ] || [ -z "$SVM_GMP_ENDPOINT_ID" ] || [ -z "$SVM_OUTFLOW_VALIDATOR_ID" ]; then
            log_and_echo "❌ ERROR: SVM program IDs not found for instance $n. Run chain-connected-svm/deploy-contracts.sh $n first."
            exit 1
        fi

        local svm_token_mint_base58="$USD_SVM_MINT_ADDR"
        local svm_token_mint_hex
        svm_token_mint_hex=$(svm_pubkey_to_hex "$svm_token_mint_base58")

        log "   - SVM instance $n: RPC=$SVM_RPC_URL chain_id=$SVM_CHAIN_ID"
        log "     program_id=$SVM_PROGRAM_ID"
        log "     token (base58): $svm_token_mint_base58"
        log "     token (hex): $svm_token_mint_hex"

        cat >> "$config_file" << EOF

[[connected_chain]]
type = "svm"
name = "SVM Connected Chain $n (E2E Test)"
rpc_url = "$SVM_RPC_URL"
chain_id = $SVM_CHAIN_ID
escrow_program_id = "$SVM_PROGRAM_ID"
gmp_endpoint_program_id = "$SVM_GMP_ENDPOINT_ID"
outflow_validator_program_id = "$SVM_OUTFLOW_VALIDATOR_ID"
private_key_env = "SOLANA_SOLVER_PRIVATE_KEY"

# Accept USDhub/USDcon swaps at 1:1 rate for E2E testing (instance $n)
# Inflow: offered on SVM (connected), desired on hub
[[acceptance.tokenpair]]
source_chain_id = $SVM_CHAIN_ID
source_token = "$svm_token_mint_hex"
target_chain_id = $hub_chain_id
target_token = "$usdhub_metadata_chain1"
ratio = 1.0
fee_bps = 50  # 0.5% fee
move_rate = 0.01  # 1 Octa = 0.01 micro-USD (MOVE 8 dec, USD 6 dec, 1:1 price)

# Outflow: offered on hub, desired on SVM (connected)
[[acceptance.tokenpair]]
source_chain_id = $hub_chain_id
source_token = "$usdhub_metadata_chain1"
target_chain_id = $SVM_CHAIN_ID
target_token = "$svm_token_mint_hex"
ratio = 1.0
fee_bps = 50  # 0.5% fee
move_rate = 0.01  # 1 Octa = 0.01 micro-USD (MOVE 8 dec, USD 6 dec, 1:1 price)

# Gas token (SOL) on SVM chain instance $n
[[liquidity.threshold]]
chain_id = $SVM_CHAIN_ID
token = "11111111111111111111111111111111"
min_balance = 1

# USDsvm on SVM chain instance $n
[[liquidity.threshold]]
chain_id = $SVM_CHAIN_ID
token = "$svm_token_mint_hex"
min_balance = 1
EOF
    done

    log "   ✅ Config written to: $config_file"
}

SOLVER_CONFIG="$PROJECT_ROOT/.tmp/solver-e2e-svm.toml"
mkdir -p "$(dirname "$SOLVER_CONFIG")"
generate_solver_config_svm "$SOLVER_CONFIG"

# Convert keypair file (JSON byte array) to base58 private key for solver
# Use the shared solver keypair (same on both instances)
svm_instance_vars 2
source "$SVM_CHAIN_INFO_FILE" 2>/dev/null || true
export SOLANA_SOLVER_PRIVATE_KEY=$(svm_keypair_to_base58 "$SVM_SOLVER_KEYPAIR")
export SOLVER_SVM_ADDR=$(svm_pubkey_to_hex "$SOLVER_SVM_PUBKEY")

unset MOVEMENT_SOLVER_PRIVATE_KEY
log "   Unset MOVEMENT_SOLVER_PRIVATE_KEY (E2E tests use profile keys only)"

if start_solver "$LOG_DIR/solver.log" "info" "$SOLVER_CONFIG"; then
    log ""
    log_and_echo "✅ Solver started successfully"
    log_and_echo "   PID: $SOLVER_PID"
    log_and_echo "   Config: $SOLVER_CONFIG"
    log_and_echo "   Logs: $LOG_DIR/solver.log"
else
    log ""
    log_and_echo "❌ ERROR: Solver failed to start"
    exit 1
fi
