#!/bin/bash

# Deploy SVM intent_inflow_escrow, intent-gmp, and intent-outflow-validator programs

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_svm.sh"

setup_project_root
setup_logging "deploy-svm-programs"
cd "$PROJECT_ROOT"

log " Deploying SVM programs (intent_inflow_escrow, intent-gmp, intent-outflow-validator)..."
log_and_echo " All output logged to: $LOG_FILE"

SVM_RPC_URL="http://127.0.0.1:8899"
SVM_CHAIN_ID=901
CHAIN_INFO="$PROJECT_ROOT/.tmp/chain-info.env"

if [ -f "$CHAIN_INFO" ]; then
    source "$CHAIN_INFO"
fi

if [ -z "$SVM_PAYER_KEYPAIR" ]; then
    log_and_echo "❌ ERROR: SVM_PAYER_KEYPAIR not found. Run setup-requester-solver.sh first."
    exit 1
fi

PROGRAM_DIR="$PROJECT_ROOT/intent-frameworks/svm"

# ============================================================================
# Deploy intent_inflow_escrow program
# ============================================================================
log ""
log " Deploying intent_inflow_escrow program..."
ESCROW_KEYPAIR="$PROGRAM_DIR/target/deploy/intent_inflow_escrow-keypair.json"
ESCROW_SO="$PROGRAM_DIR/target/deploy/intent_inflow_escrow.so"

if [ ! -f "$ESCROW_KEYPAIR" ]; then
    log "   Generating program keypair..."
    svm_cmd "solana-keygen new --no-bip39-passphrase --silent -o \"$ESCROW_KEYPAIR\""
fi

svm_cmd "solana program deploy --url \"$SVM_RPC_URL\" --keypair \"$SVM_PAYER_KEYPAIR\" \"$ESCROW_SO\" --program-id \"$ESCROW_KEYPAIR\"" >> "$LOG_FILE" 2>&1
SVM_PROGRAM_ID=$(svm_cmd "solana address -k \"$ESCROW_KEYPAIR\"" | tail -n 1)
log "   ✅ intent_inflow_escrow deployed: $SVM_PROGRAM_ID"

# ============================================================================
# Deploy intent-gmp program
# ============================================================================
log ""
log " Deploying intent-gmp program..."
GMP_KEYPAIR="$PROGRAM_DIR/target/deploy/intent_gmp-keypair.json"
GMP_SO="$PROGRAM_DIR/target/deploy/intent_gmp.so"

if [ ! -f "$GMP_KEYPAIR" ]; then
    log "   Generating program keypair..."
    svm_cmd "solana-keygen new --no-bip39-passphrase --silent -o \"$GMP_KEYPAIR\""
fi

svm_cmd "solana program deploy --url \"$SVM_RPC_URL\" --keypair \"$SVM_PAYER_KEYPAIR\" \"$GMP_SO\" --program-id \"$GMP_KEYPAIR\"" >> "$LOG_FILE" 2>&1
SVM_GMP_ENDPOINT_ID=$(svm_cmd "solana address -k \"$GMP_KEYPAIR\"" | tail -n 1)
log "   ✅ intent-gmp deployed: $SVM_GMP_ENDPOINT_ID"

# ============================================================================
# Deploy intent-outflow-validator program
# ============================================================================
log ""
log " Deploying intent-outflow-validator program..."
OUTFLOW_KEYPAIR="$PROGRAM_DIR/target/deploy/intent_outflow_validator-keypair.json"
OUTFLOW_SO="$PROGRAM_DIR/target/deploy/intent_outflow_validator.so"

if [ ! -f "$OUTFLOW_KEYPAIR" ]; then
    log "   Generating program keypair..."
    svm_cmd "solana-keygen new --no-bip39-passphrase --silent -o \"$OUTFLOW_KEYPAIR\""
fi

svm_cmd "solana program deploy --url \"$SVM_RPC_URL\" --keypair \"$SVM_PAYER_KEYPAIR\" \"$OUTFLOW_SO\" --program-id \"$OUTFLOW_KEYPAIR\"" >> "$LOG_FILE" 2>&1
SVM_OUTFLOW_VALIDATOR_ID=$(svm_cmd "solana address -k \"$OUTFLOW_KEYPAIR\"" | tail -n 1)
log "   ✅ intent-outflow-validator deployed: $SVM_OUTFLOW_VALIDATOR_ID"

# Wait for all program accounts to be available
log ""
log "   ⏳ Waiting for program accounts to be available..."
sleep 10

# ============================================================================
# Initialize intent_inflow_escrow
# ============================================================================
log ""
log " Initializing intent_inflow_escrow..."
if [ -z "$E2E_INTEGRATED_GMP_PUBLIC_KEY" ]; then
    load_integrated_gmp_keys
fi

SVM_APPROVER_PUBKEY=$(svm_base64_to_base58 "$E2E_INTEGRATED_GMP_PUBLIC_KEY")

set +e
init_success=0
for attempt in 1 2 3 4 5; do
    nix develop "$PROJECT_ROOT/nix" -c bash -c "cd \"$PROGRAM_DIR\" && SVM_APPROVER_PUBKEY=\"$SVM_APPROVER_PUBKEY\" SVM_PROGRAM_ID=\"$SVM_PROGRAM_ID\" SVM_RPC_URL=\"$SVM_RPC_URL\" SVM_PAYER_KEYPAIR=\"$SVM_PAYER_KEYPAIR\" bash ./scripts/initialize.sh" >> "$LOG_FILE" 2>&1
    status=$?
    if [ "$status" -eq 0 ]; then
        log "   ✅ intent_inflow_escrow initialized"
        init_success=1
        break
    fi
    log "   Initialize failed (attempt $attempt), retrying..."
    sleep 2
done
set -e

if [ "$init_success" -ne 1 ]; then
    log_and_echo "❌ PANIC: intent_inflow_escrow initialization failed after 5 attempts"
    exit 1
fi

# ============================================================================
# Initialize intent-gmp
# ============================================================================
log ""
log " Initializing intent-gmp (chain_id=$SVM_CHAIN_ID)..."

# Get relay pubkey for authorization
SVM_RELAY_PUBKEY=$(svm_base64_to_base58 "$E2E_INTEGRATED_GMP_PUBLIC_KEY")
log "   Relay pubkey: $SVM_RELAY_PUBKEY"

# Get hub module address as 32-byte hex for remote GMP endpoint
HUB_MODULE_ADDR_HEX=""
if [ -n "$HUB_MODULE_ADDR" ]; then
    HUB_MODULE_ADDR_CLEAN=$(echo "$HUB_MODULE_ADDR" | sed 's/^0x//')
    HUB_MODULE_ADDR_HEX=$(printf "%064s" "$HUB_MODULE_ADDR_CLEAN" | tr ' ' '0')
fi

# Initialize GMP endpoint
set +e
gmp_init_success=0
for attempt in 1 2 3; do
    nix develop "$PROJECT_ROOT/nix" -c bash -c "
cd \"$PROGRAM_DIR\"
cargo run -p intent_escrow_cli --quiet -- \
    gmp-init \
    --gmp-program-id \"$SVM_GMP_ENDPOINT_ID\" \
    --payer \"$SVM_PAYER_KEYPAIR\" \
    --chain-id $SVM_CHAIN_ID \
    --rpc \"$SVM_RPC_URL\"
" >> "$LOG_FILE" 2>&1
    status=$?
    if [ "$status" -eq 0 ]; then
        log "   ✅ intent-gmp initialized"
        gmp_init_success=1
        break
    fi
    log "   GMP init failed (attempt $attempt), retrying..."
    sleep 2
done
set -e

if [ "$gmp_init_success" -ne 1 ]; then
    log_and_echo "❌ PANIC: intent-gmp initialization failed"
    exit 1
fi

# Add integrated-gmp relay as authorized relay
log " Adding integrated-gmp relay to GMP endpoint..."
nix develop "$PROJECT_ROOT/nix" -c bash -c "
cd \"$PROGRAM_DIR\"
cargo run -p intent_escrow_cli --quiet -- \
    gmp-add-relay \
    --gmp-program-id \"$SVM_GMP_ENDPOINT_ID\" \
    --payer \"$SVM_PAYER_KEYPAIR\" \
    --relay \"$SVM_RELAY_PUBKEY\" \
    --rpc \"$SVM_RPC_URL\"
" >> "$LOG_FILE" 2>&1 || {
    log_and_echo "❌ PANIC: Failed to add relay to GMP endpoint"
    exit 1
}
log "   ✅ Integrated-GMP relay added to GMP endpoint"

# Set hub as remote GMP endpoint (if hub module address available)
if [ -n "$HUB_MODULE_ADDR_HEX" ]; then
    log " Setting hub (chain_id=1) as remote GMP endpoint..."
    nix develop "$PROJECT_ROOT/nix" -c bash -c "
cd \"$PROGRAM_DIR\"
cargo run -p intent_escrow_cli --quiet -- \
    gmp-set-remote-gmp-endpoint-addr \
    --gmp-program-id \"$SVM_GMP_ENDPOINT_ID\" \
    --payer \"$SVM_PAYER_KEYPAIR\" \
    --src-chain-id 1 \
    --addr \"$HUB_MODULE_ADDR_HEX\" \
    --rpc \"$SVM_RPC_URL\"
" >> "$LOG_FILE" 2>&1 || {
        log_and_echo "❌ PANIC: Failed to set remote GMP endpoint"
        exit 1
    }
    log "   ✅ Hub set as remote GMP endpoint"
fi

# ============================================================================
# Initialize intent-outflow-validator with hub config
# ============================================================================
log ""
log " Initializing intent-outflow-validator..."

# Get hub module address (32-byte hex)
if [ -n "$HUB_MODULE_ADDR" ]; then
    HUB_ADDR_CLEAN=$(echo "$HUB_MODULE_ADDR" | sed 's/^0x//')
    # Pad to 64 hex characters (32 bytes)
    HUB_ADDR_PADDED=$(printf "%064s" "$HUB_ADDR_CLEAN" | tr ' ' '0')
    log "   Hub address (padded): 0x$HUB_ADDR_PADDED"

    set +e
    outflow_init_success=0
    for attempt in 1 2 3; do
        nix develop "$PROJECT_ROOT/nix" -c bash -c "
cd \"$PROGRAM_DIR\"
cargo run -p intent_escrow_cli --quiet -- \
    outflow-init \
    --outflow-program-id \"$SVM_OUTFLOW_VALIDATOR_ID\" \
    --payer \"$SVM_PAYER_KEYPAIR\" \
    --gmp-endpoint \"$SVM_GMP_ENDPOINT_ID\" \
    --hub-chain-id 1 \
    --hub-address \"$HUB_ADDR_PADDED\" \
    --rpc \"$SVM_RPC_URL\"
" >> "$LOG_FILE" 2>&1
        status=$?
        if [ "$status" -eq 0 ]; then
            log "   ✅ intent-outflow-validator initialized"
            outflow_init_success=1
            break
        fi
        log "   Outflow init failed (attempt $attempt), retrying..."
        sleep 2
    done
    set -e

    if [ "$outflow_init_success" -ne 1 ]; then
        log_and_echo "❌ PANIC: intent-outflow-validator initialization failed"
        exit 1
    fi

    # ============================================================================
    # Configure intent_inflow_escrow GMP config (for receiving IntentRequirements)
    # ============================================================================
    log ""
    log " Configuring intent_inflow_escrow GMP config..."
    set +e
    escrow_gmp_success=0
    for attempt in 1 2 3; do
        nix develop "$PROJECT_ROOT/nix" -c bash -c "
cd \"$PROGRAM_DIR\"
cargo run -p intent_escrow_cli --quiet -- \
    escrow-set-gmp-config \
    --program-id \"$SVM_PROGRAM_ID\" \
    --payer \"$SVM_PAYER_KEYPAIR\" \
    --hub-chain-id 1 \
    --hub-address \"$HUB_ADDR_PADDED\" \
    --gmp-endpoint \"$SVM_GMP_ENDPOINT_ID\" \
    --rpc \"$SVM_RPC_URL\"
" >> "$LOG_FILE" 2>&1
        status=$?
        if [ "$status" -eq 0 ]; then
            log "   ✅ intent_inflow_escrow GMP config set"
            escrow_gmp_success=1
            break
        fi
        log "   intent_inflow_escrow GMP config failed (attempt $attempt), retrying..."
        sleep 2
    done
    set -e

    if [ "$escrow_gmp_success" -ne 1 ]; then
        log_and_echo "❌ PANIC: intent_inflow_escrow GMP config failed"
        exit 1
    fi
else
    log_and_echo "❌ PANIC: HUB_MODULE_ADDR not found, cannot initialize intent-outflow-validator"
    exit 1
fi

# ============================================================================
# Configure GMP routing for multi-destination delivery (like MVM's route_message)
# ============================================================================
log ""
log " Configuring GMP routing (intent-outflow-validator + intent-inflow-escrow)..."
nix develop "$PROJECT_ROOT/nix" -c bash -c "
cd \"$PROGRAM_DIR\"
cargo run -p intent_escrow_cli --quiet -- \
    gmp-set-routing \
    --gmp-program-id \"$SVM_GMP_ENDPOINT_ID\" \
    --payer \"$SVM_PAYER_KEYPAIR\" \
    --outflow-validator \"$SVM_OUTFLOW_VALIDATOR_ID\" \
    --intent-escrow \"$SVM_PROGRAM_ID\" \
    --rpc \"$SVM_RPC_URL\"
" >> "$LOG_FILE" 2>&1 || {
    log_and_echo "❌ PANIC: Failed to set GMP routing"
    exit 1
}
log "   ✅ GMP routing configured for IntentRequirements multi-destination delivery"

# ============================================================================
# Configure hub chain to trust SVM connected chain
# ============================================================================
log ""
log " Configuring hub chain to trust SVM connected chain..."

if [ -n "$HUB_MODULE_ADDR" ]; then
    # Both SVM programs (intent-outflow-validator and intent-inflow-escrow) use the
    # GMP endpoint program ID as remote_gmp_endpoint_addr when sending messages via CPI.
    # So we register the GMP endpoint as the single remote GMP endpoint for SVM.
    SVM_GMP_ENDPOINT_HEX=$(svm_pubkey_to_hex "$SVM_GMP_ENDPOINT_ID")
    log "   SVM GMP endpoint address (hex): $SVM_GMP_ENDPOINT_HEX"

    # Set remote GMP endpoint on hub's intent_gmp for SVM GMP endpoint
    if aptos move run --profile intent-account-chain1 --assume-yes \
        --function-id ${HUB_MODULE_ADDR}::intent_gmp::set_remote_gmp_endpoint_addr \
        --args u32:$SVM_CHAIN_ID "hex:${SVM_GMP_ENDPOINT_HEX}" >> "$LOG_FILE" 2>&1; then
        log "   ✅ Hub intent_gmp now trusts SVM GMP endpoint"
    else
        log "   ️ Could not set remote GMP endpoint on hub (ignoring)"
    fi

    # Set remote GMP endpoint on hub's intent_gmp_hub for SVM GMP endpoint
    if aptos move run --profile intent-account-chain1 --assume-yes \
        --function-id ${HUB_MODULE_ADDR}::intent_gmp_hub::set_remote_gmp_endpoint_addr \
        --args u32:$SVM_CHAIN_ID "hex:${SVM_GMP_ENDPOINT_HEX}" >> "$LOG_FILE" 2>&1; then
        log "   ✅ Hub intent_gmp_hub now trusts SVM GMP endpoint"
    else
        log "   ️ Could not set remote GMP endpoint in intent_gmp_hub (ignoring)"
    fi
else
    log "   ️ WARNING: HUB_MODULE_ADDR not found, skipping hub trust config for SVM"
fi

# ============================================================================
# Fund integrated-gmp relay on SVM
# ============================================================================
log ""
log " Funding integrated-gmp relay on SVM..."
load_integrated_gmp_keys
if [ -z "$E2E_INTEGRATED_GMP_PUBLIC_KEY" ]; then
    log_and_echo "❌ PANIC: E2E_INTEGRATED_GMP_PUBLIC_KEY not set"
    exit 1
fi
RELAY_PUBKEY_BASE58=$(svm_base64_to_base58 "$E2E_INTEGRATED_GMP_PUBLIC_KEY")
log "   Relay address: $RELAY_PUBKEY_BASE58"
airdrop_svm "$RELAY_PUBKEY_BASE58" 10 "$SVM_RPC_URL"
log "   ✅ Integrated-GMP relay funded on SVM"

# ============================================================================
# Save chain info
# ============================================================================
log ""
log " Saving chain info..."
cat >> "$CHAIN_INFO" << EOF
SVM_PROGRAM_ID=$SVM_PROGRAM_ID
SVM_GMP_ENDPOINT_ID=$SVM_GMP_ENDPOINT_ID
SVM_OUTFLOW_VALIDATOR_ID=$SVM_OUTFLOW_VALIDATOR_ID
SVM_CHAIN_ID=$SVM_CHAIN_ID
EOF

log ""
log "✅ SVM programs deploy + init complete"
log "   - intent_inflow_escrow: $SVM_PROGRAM_ID"
log "   - intent-gmp: $SVM_GMP_ENDPOINT_ID"
log "   - intent-outflow-validator: $SVM_OUTFLOW_VALIDATOR_ID"
