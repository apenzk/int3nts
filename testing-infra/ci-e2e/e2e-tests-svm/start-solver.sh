#!/bin/bash

# Start Solver Service for E2E Tests (SVM Connected Chain)

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_mvm.sh"
source "$SCRIPT_DIR/../util_svm.sh"

setup_project_root
setup_logging "solver-start-svm"
cd "$PROJECT_ROOT"

log ""
log " Starting Solver Service (SVM Connected Chain)..."
log "========================================"
log_and_echo " All output logged to: $LOG_FILE"
log ""

generate_solver_config_svm() {
    local config_file="$1"

    local test_tokens_chain1=$(get_profile_address "test-tokens-chain1")
    local solver_chain1_addr=$(get_profile_address "solver-chain1")
    local chain1_addr=$(get_profile_address "intent-account-chain1")
    local usdhub_metadata_chain1=$(get_usdxyz_metadata_addr "0x${test_tokens_chain1}" "1")

    if [ -f "$PROJECT_ROOT/.tmp/chain-info.env" ]; then
        source "$PROJECT_ROOT/.tmp/chain-info.env"
    fi

    if [ -z "$USD_SVM_MINT_ADDR" ] || [ -z "$SOLVER_SVM_PUBKEY" ] || [ -z "$SVM_SOLVER_KEYPAIR" ]; then
        log_and_echo "❌ ERROR: Missing SVM chain info. Run chain-connected-svm/setup-requester-solver.sh first."
        exit 1
    fi
    if [ -z "$SVM_PROGRAM_ID" ]; then
        log_and_echo "❌ ERROR: SVM_PROGRAM_ID not found. Run chain-connected-svm/deploy-contract.sh first."
        exit 1
    fi

    local verifier_url="${VERIFIER_URL:-http://127.0.0.1:3333}"
    local hub_rpc="${CHAIN1_URL:-http://127.0.0.1:8080/v1}"
    local hub_chain_id="${HUB_CHAIN_ID:-1}"
    local svm_rpc="${SVM_RPC_URL:-http://127.0.0.1:8899}"
    local svm_chain_id="${SVM_CHAIN_ID:-4}"
    local module_addr="0x${chain1_addr}"
    local solver_addr="0x${solver_chain1_addr}"

    local svm_token_mint_base58
    svm_token_mint_base58="$USD_SVM_MINT_ADDR"
    
    # Convert SVM token from base58 to 32-byte hex (matches hub chain format)
    local svm_token_mint_hex
    svm_token_mint_hex=$(svm_pubkey_to_hex "$svm_token_mint_base58")

    log "   Generating solver config:"
    log "   - Verifier URL: $verifier_url"
    log "   - Hub RPC: $hub_rpc (chain ID: $hub_chain_id)"
    log "   - SVM RPC: $svm_rpc (chain ID: $svm_chain_id)"
    log "   - Hub module address: $module_addr"
    log "   - SVM program id: $SVM_PROGRAM_ID"
    log "   - Solver address: $solver_addr"
    log "   - USDhub metadata (hub): $usdhub_metadata_chain1"
    log "   - SVM token (base58): $svm_token_mint_base58"
    log "   - SVM token (hex): $svm_token_mint_hex"

    cat > "$config_file" << EOF
# Auto-generated solver config for SVM E2E tests
# Generated at: $(date)

[service]
verifier_url = "$verifier_url"
polling_interval_ms = 1000
e2e_mode = true

[hub_chain]
name = "Hub Chain (E2E Test)"
rpc_url = "$hub_rpc"
chain_id = $hub_chain_id
module_addr = "$module_addr"
profile = "solver-chain1"
e2e_mode = true

[[connected_chain]]
type = "svm"
name = "SVM Connected Chain (E2E Test)"
rpc_url = "$svm_rpc"
chain_id = $svm_chain_id
escrow_program_id = "$SVM_PROGRAM_ID"
private_key_env = "SOLANA_SOLVER_PRIVATE_KEY"

[acceptance]
[[acceptance.tokenpair]]
source_chain_id = $svm_chain_id
source_token = "$svm_token_mint_hex"
target_chain_id = $hub_chain_id
target_token = "$usdhub_metadata_chain1"
ratio = 1.0

[[acceptance.tokenpair]]
source_chain_id = $hub_chain_id
source_token = "$usdhub_metadata_chain1"
target_chain_id = $svm_chain_id
target_token = "$svm_token_mint_hex"
ratio = 1.0

[solver]
profile = "solver-chain1"
address = "$solver_addr"
EOF

    log "   ✅ Config written to: $config_file"
}

SOLVER_CONFIG="$PROJECT_ROOT/.tmp/solver-e2e-svm.toml"
mkdir -p "$(dirname "$SOLVER_CONFIG")"
generate_solver_config_svm "$SOLVER_CONFIG"

# Convert keypair file (JSON byte array) to base58 private key for solver
# Solana keypairs are 64 bytes: first 32 bytes seed + last 32 bytes public key
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
