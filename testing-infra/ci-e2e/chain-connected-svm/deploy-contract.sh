#!/bin/bash

# Deploy SVM intent escrow, native-gmp-endpoint, and outflow-validator programs

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_svm.sh"

setup_project_root
setup_logging "deploy-svm-programs"
cd "$PROJECT_ROOT"

log " Deploying SVM programs (escrow, GMP endpoint, outflow validator)..."
log_and_echo " All output logged to: $LOG_FILE"

SVM_RPC_URL="http://127.0.0.1:8899"
SVM_CHAIN_ID=4
CHAIN_INFO="$PROJECT_ROOT/.tmp/chain-info.env"

if [ -f "$CHAIN_INFO" ]; then
    source "$CHAIN_INFO"
fi

if [ -z "$SVM_PAYER_KEYPAIR" ]; then
    log_and_echo "❌ ERROR: SVM_PAYER_KEYPAIR not found. Run setup-requester-solver.sh first."
    exit 1
fi

PROGRAM_DIR="$PROJECT_ROOT/intent-frameworks/svm"

# Build all programs
log "   Building all SVM programs..."
nix develop "$PROJECT_ROOT/nix" -c bash -c "cd \"$PROGRAM_DIR\" && ./scripts/build.sh" >> "$LOG_FILE" 2>&1

# ============================================================================
# Deploy intent_escrow program
# ============================================================================
log ""
log " Deploying intent_escrow program..."
ESCROW_KEYPAIR="$PROGRAM_DIR/target/deploy/intent_escrow-keypair.json"
ESCROW_SO="$PROGRAM_DIR/target/deploy/intent_escrow.so"

if [ ! -f "$ESCROW_KEYPAIR" ]; then
    log "   Generating program keypair..."
    svm_cmd "solana-keygen new --no-bip39-passphrase --silent -o \"$ESCROW_KEYPAIR\""
fi

svm_cmd "solana program deploy --url \"$SVM_RPC_URL\" --keypair \"$SVM_PAYER_KEYPAIR\" \"$ESCROW_SO\" --program-id \"$ESCROW_KEYPAIR\"" >> "$LOG_FILE" 2>&1
SVM_PROGRAM_ID=$(svm_cmd "solana address -k \"$ESCROW_KEYPAIR\"" | tail -n 1)
log "   ✅ intent_escrow deployed: $SVM_PROGRAM_ID"

# ============================================================================
# Deploy native-gmp-endpoint program
# ============================================================================
log ""
log " Deploying native-gmp-endpoint program..."
GMP_KEYPAIR="$PROGRAM_DIR/target/deploy/native_gmp_endpoint-keypair.json"
GMP_SO="$PROGRAM_DIR/target/deploy/native_gmp_endpoint.so"

if [ ! -f "$GMP_KEYPAIR" ]; then
    log "   Generating program keypair..."
    svm_cmd "solana-keygen new --no-bip39-passphrase --silent -o \"$GMP_KEYPAIR\""
fi

svm_cmd "solana program deploy --url \"$SVM_RPC_URL\" --keypair \"$SVM_PAYER_KEYPAIR\" \"$GMP_SO\" --program-id \"$GMP_KEYPAIR\"" >> "$LOG_FILE" 2>&1
SVM_GMP_ENDPOINT_ID=$(svm_cmd "solana address -k \"$GMP_KEYPAIR\"" | tail -n 1)
log "   ✅ native-gmp-endpoint deployed: $SVM_GMP_ENDPOINT_ID"

# ============================================================================
# Deploy outflow-validator program
# ============================================================================
log ""
log " Deploying outflow-validator program..."
OUTFLOW_KEYPAIR="$PROGRAM_DIR/target/deploy/outflow_validator-keypair.json"
OUTFLOW_SO="$PROGRAM_DIR/target/deploy/outflow_validator.so"

if [ ! -f "$OUTFLOW_KEYPAIR" ]; then
    log "   Generating program keypair..."
    svm_cmd "solana-keygen new --no-bip39-passphrase --silent -o \"$OUTFLOW_KEYPAIR\""
fi

svm_cmd "solana program deploy --url \"$SVM_RPC_URL\" --keypair \"$SVM_PAYER_KEYPAIR\" \"$OUTFLOW_SO\" --program-id \"$OUTFLOW_KEYPAIR\"" >> "$LOG_FILE" 2>&1
SVM_OUTFLOW_VALIDATOR_ID=$(svm_cmd "solana address -k \"$OUTFLOW_KEYPAIR\"" | tail -n 1)
log "   ✅ outflow-validator deployed: $SVM_OUTFLOW_VALIDATOR_ID"

# Wait for all program accounts to be available
log ""
log "   ⏳ Waiting for program accounts to be available..."
sleep 10

# ============================================================================
# Initialize intent_escrow
# ============================================================================
log ""
log " Initializing intent_escrow..."
if [ -z "$E2E_TRUSTED_GMP_PUBLIC_KEY" ]; then
    load_trusted_gmp_keys
fi

SVM_APPROVER_PUBKEY=$(svm_base64_to_base58 "$E2E_TRUSTED_GMP_PUBLIC_KEY")

set +e
init_success=0
for attempt in 1 2 3 4 5; do
    nix develop "$PROJECT_ROOT/nix" -c bash -c "cd \"$PROGRAM_DIR\" && SVM_APPROVER_PUBKEY=\"$SVM_APPROVER_PUBKEY\" SVM_PROGRAM_ID=\"$SVM_PROGRAM_ID\" SVM_RPC_URL=\"$SVM_RPC_URL\" SVM_PAYER_KEYPAIR=\"$SVM_PAYER_KEYPAIR\" bash ./scripts/initialize.sh" >> "$LOG_FILE" 2>&1
    status=$?
    if [ "$status" -eq 0 ]; then
        log "   ✅ intent_escrow initialized"
        init_success=1
        break
    fi
    log "   Initialize failed (attempt $attempt), retrying..."
    sleep 2
done
set -e

if [ "$init_success" -ne 1 ]; then
    log_and_echo "❌ PANIC: intent_escrow initialization failed after 5 attempts"
    exit 1
fi

# ============================================================================
# Initialize native-gmp-endpoint
# ============================================================================
log ""
log " Initializing native-gmp-endpoint (chain_id=$SVM_CHAIN_ID)..."

# Get relay pubkey for authorization
SVM_RELAY_PUBKEY=$(svm_base64_to_base58 "$E2E_TRUSTED_GMP_PUBLIC_KEY")
log "   Relay pubkey: $SVM_RELAY_PUBKEY"

# Initialize GMP endpoint using intent_escrow_cli (we'll add GMP commands to it)
# For now, use a custom initialization script
nix develop "$PROJECT_ROOT/nix" -c bash -c "
cd \"$PROGRAM_DIR\"
cargo run -p intent_escrow_cli --quiet -- \
    gmp-init \
    --rpc-url \"$SVM_RPC_URL\" \
    --payer-keypair \"$SVM_PAYER_KEYPAIR\" \
    --gmp-program-id \"$SVM_GMP_ENDPOINT_ID\" \
    --chain-id $SVM_CHAIN_ID
" >> "$LOG_FILE" 2>&1 || {
    log "   ️ GMP endpoint init via CLI not available, skipping (will be initialized on first use)"
}

# ============================================================================
# Initialize outflow-validator with hub config
# ============================================================================
log ""
log " Initializing outflow-validator..."

# Get hub module address (32-byte hex)
if [ -n "$HUB_MODULE_ADDR" ]; then
    HUB_ADDR_CLEAN=$(echo "$HUB_MODULE_ADDR" | sed 's/^0x//')
    # Pad to 64 hex characters (32 bytes)
    HUB_ADDR_PADDED=$(printf "%064s" "$HUB_ADDR_CLEAN" | tr ' ' '0')
    log "   Hub address (padded): 0x$HUB_ADDR_PADDED"

    nix develop "$PROJECT_ROOT/nix" -c bash -c "
cd \"$PROGRAM_DIR\"
cargo run -p intent_escrow_cli --quiet -- \
    outflow-init \
    --rpc-url \"$SVM_RPC_URL\" \
    --payer-keypair \"$SVM_PAYER_KEYPAIR\" \
    --outflow-program-id \"$SVM_OUTFLOW_VALIDATOR_ID\" \
    --gmp-endpoint \"$SVM_GMP_ENDPOINT_ID\" \
    --hub-chain-id 1 \
    --hub-address \"$HUB_ADDR_PADDED\"
" >> "$LOG_FILE" 2>&1 || {
        log "   ️ Outflow validator init via CLI not available, skipping (will be initialized on first use)"
    }
else
    log "   ️ WARNING: HUB_MODULE_ADDR not found, skipping outflow-validator hub config"
fi

# ============================================================================
# Configure hub chain to trust SVM connected chain
# ============================================================================
log ""
log " Configuring hub chain to trust SVM connected chain..."

if [ -n "$HUB_MODULE_ADDR" ]; then
    # Convert SVM GMP endpoint pubkey to 32-byte hex for GMP addressing
    # Solana pubkeys are already 32 bytes, but we need hex format
    SVM_GMP_ADDR_HEX=$(svm_pubkey_to_hex "$SVM_GMP_ENDPOINT_ID")
    log "   SVM GMP endpoint address (hex): $SVM_GMP_ADDR_HEX"

    # Set trusted remote on hub for SVM chain (chain_id=4)
    aptos move run --profile intent-account-chain1 --assume-yes \
        --function-id ${HUB_MODULE_ADDR}::native_gmp_endpoint::set_trusted_remote \
        --args u32:$SVM_CHAIN_ID "hex:${SVM_GMP_ADDR_HEX}" >> "$LOG_FILE" 2>&1

    if [ $? -eq 0 ]; then
        log "   ✅ Hub now trusts SVM connected chain (chain_id=$SVM_CHAIN_ID)"
    else
        log "   ️ Could not set trusted remote on hub (ignoring)"
    fi

    # Also set trusted remote in intent_gmp_hub
    aptos move run --profile intent-account-chain1 --assume-yes \
        --function-id ${HUB_MODULE_ADDR}::intent_gmp_hub::set_trusted_remote \
        --args u32:$SVM_CHAIN_ID "hex:${SVM_GMP_ADDR_HEX}" >> "$LOG_FILE" 2>&1

    if [ $? -eq 0 ]; then
        log "   ✅ Hub intent_gmp_hub now trusts SVM connected chain"
    else
        log "   ️ Could not set trusted remote in intent_gmp_hub (ignoring)"
    fi
else
    log "   ️ WARNING: HUB_MODULE_ADDR not found, skipping hub trust config for SVM"
fi

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
log "   - intent_escrow: $SVM_PROGRAM_ID"
log "   - native-gmp-endpoint: $SVM_GMP_ENDPOINT_ID"
log "   - outflow-validator: $SVM_OUTFLOW_VALIDATOR_ID"
