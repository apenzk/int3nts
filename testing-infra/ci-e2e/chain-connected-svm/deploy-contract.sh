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

# Get hub module address as 32-byte hex for trusted remote
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
        log "   ✅ native-gmp-endpoint initialized"
        gmp_init_success=1
        break
    fi
    log "   GMP init failed (attempt $attempt), retrying..."
    sleep 2
done
set -e

if [ "$gmp_init_success" -ne 1 ]; then
    log_and_echo "❌ PANIC: native-gmp-endpoint initialization failed"
    exit 1
fi

# Add trusted-gmp relay as authorized relay
log " Adding trusted-GMP relay to GMP endpoint..."
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
log "   ✅ Trusted-GMP relay added to GMP endpoint"

# Set hub as trusted remote (if hub module address available)
if [ -n "$HUB_MODULE_ADDR_HEX" ]; then
    log " Setting hub (chain_id=1) as trusted remote on GMP endpoint..."
    nix develop "$PROJECT_ROOT/nix" -c bash -c "
cd \"$PROGRAM_DIR\"
cargo run -p intent_escrow_cli --quiet -- \
    gmp-set-trusted-remote \
    --gmp-program-id \"$SVM_GMP_ENDPOINT_ID\" \
    --payer \"$SVM_PAYER_KEYPAIR\" \
    --src-chain-id 1 \
    --trusted-addr \"$HUB_MODULE_ADDR_HEX\" \
    --rpc \"$SVM_RPC_URL\"
" >> "$LOG_FILE" 2>&1 || {
        log_and_echo "❌ PANIC: Failed to set trusted remote on GMP endpoint"
        exit 1
    }
    log "   ✅ Hub set as trusted remote on GMP endpoint"
fi

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
            log "   ✅ outflow-validator initialized"
            outflow_init_success=1
            break
        fi
        log "   Outflow init failed (attempt $attempt), retrying..."
        sleep 2
    done
    set -e

    if [ "$outflow_init_success" -ne 1 ]; then
        log_and_echo "❌ PANIC: outflow-validator initialization failed"
        exit 1
    fi

    # ============================================================================
    # Configure intent_escrow GMP config (for receiving IntentRequirements)
    # ============================================================================
    log ""
    log " Configuring intent_escrow GMP config..."
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
            log "   ✅ intent_escrow GMP config set"
            escrow_gmp_success=1
            break
        fi
        log "   Escrow GMP config failed (attempt $attempt), retrying..."
        sleep 2
    done
    set -e

    if [ "$escrow_gmp_success" -ne 1 ]; then
        log_and_echo "❌ PANIC: intent_escrow GMP config failed"
        exit 1
    fi
else
    log_and_echo "❌ PANIC: HUB_MODULE_ADDR not found, cannot initialize outflow-validator"
    exit 1
fi

# ============================================================================
# Configure GMP routing for multi-destination delivery (like MVM's route_message)
# ============================================================================
log ""
log " Configuring GMP routing (outflow-validator + intent-escrow)..."
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
    # Convert SVM program addresses to 32-byte hex for GMP addressing.
    # Different message types come from different programs:
    # - FulfillmentProof (outflow): src_addr = outflow-validator program ID
    # - EscrowConfirmation (inflow): src_addr = intent-escrow program ID
    SVM_OUTFLOW_ADDR_HEX=$(svm_pubkey_to_hex "$SVM_OUTFLOW_VALIDATOR_ID")
    SVM_ESCROW_ADDR_HEX=$(svm_pubkey_to_hex "$SVM_PROGRAM_ID")
    log "   SVM outflow-validator address (hex): $SVM_OUTFLOW_ADDR_HEX"
    log "   SVM intent-escrow address (hex): $SVM_ESCROW_ADDR_HEX"

    # Set trusted remote on hub's native_gmp_endpoint for outflow-validator (FulfillmentProof)
    if aptos move run --profile intent-account-chain1 --assume-yes \
        --function-id ${HUB_MODULE_ADDR}::native_gmp_endpoint::set_trusted_remote \
        --args u32:$SVM_CHAIN_ID "hex:${SVM_OUTFLOW_ADDR_HEX}" >> "$LOG_FILE" 2>&1; then
        log "   ✅ Hub native_gmp_endpoint now trusts SVM outflow-validator"
    else
        log "   ️ Could not set trusted remote on hub (ignoring)"
    fi

    # Set trusted remote on hub's intent_gmp_hub for outflow-validator (FulfillmentProof)
    if aptos move run --profile intent-account-chain1 --assume-yes \
        --function-id ${HUB_MODULE_ADDR}::intent_gmp_hub::set_trusted_remote \
        --args u32:$SVM_CHAIN_ID "hex:${SVM_OUTFLOW_ADDR_HEX}" >> "$LOG_FILE" 2>&1; then
        log "   ✅ Hub intent_gmp_hub now trusts SVM outflow-validator"
    else
        log "   ️ Could not set trusted remote in intent_gmp_hub (ignoring)"
    fi

    # Also trust intent-escrow for EscrowConfirmation messages (inflow flow)
    # Note: MVM may only support one trusted remote per chain, so this might override the previous.
    # If so, we may need to update the MVM to support multiple trusted remotes or use a single
    # "SVM gateway" address that aggregates all SVM program messages.
    if aptos move run --profile intent-account-chain1 --assume-yes \
        --function-id ${HUB_MODULE_ADDR}::native_gmp_endpoint::add_trusted_remote \
        --args u32:$SVM_CHAIN_ID "hex:${SVM_ESCROW_ADDR_HEX}" >> "$LOG_FILE" 2>&1; then
        log "   ✅ Hub native_gmp_endpoint now also trusts SVM intent-escrow"
    elif aptos move run --profile intent-account-chain1 --assume-yes \
        --function-id ${HUB_MODULE_ADDR}::native_gmp_endpoint::set_trusted_remote \
        --args u32:$SVM_CHAIN_ID "hex:${SVM_ESCROW_ADDR_HEX}" >> "$LOG_FILE" 2>&1; then
        log "   ️ Hub native_gmp_endpoint: set_trusted_remote to intent-escrow (replaces outflow)"
    else
        log "   ️ Could not add intent-escrow to trusted remotes (ignoring)"
    fi

    if aptos move run --profile intent-account-chain1 --assume-yes \
        --function-id ${HUB_MODULE_ADDR}::intent_gmp_hub::add_trusted_remote \
        --args u32:$SVM_CHAIN_ID "hex:${SVM_ESCROW_ADDR_HEX}" >> "$LOG_FILE" 2>&1; then
        log "   ✅ Hub intent_gmp_hub now also trusts SVM intent-escrow"
    elif aptos move run --profile intent-account-chain1 --assume-yes \
        --function-id ${HUB_MODULE_ADDR}::intent_gmp_hub::set_trusted_remote \
        --args u32:$SVM_CHAIN_ID "hex:${SVM_ESCROW_ADDR_HEX}" >> "$LOG_FILE" 2>&1; then
        log "   ️ Hub intent_gmp_hub: set_trusted_remote to intent-escrow (replaces outflow)"
    else
        log "   ️ Could not add intent-escrow to trusted remotes (ignoring)"
    fi
else
    log "   ️ WARNING: HUB_MODULE_ADDR not found, skipping hub trust config for SVM"
fi

# ============================================================================
# Fund trusted-GMP relay on SVM
# ============================================================================
log ""
log " Funding trusted-GMP relay on SVM..."
load_trusted_gmp_keys
if [ -z "$E2E_TRUSTED_GMP_PUBLIC_KEY" ]; then
    log_and_echo "❌ PANIC: E2E_TRUSTED_GMP_PUBLIC_KEY not set"
    exit 1
fi
RELAY_PUBKEY_BASE58=$(svm_base64_to_base58 "$E2E_TRUSTED_GMP_PUBLIC_KEY")
log "   Relay address: $RELAY_PUBKEY_BASE58"
airdrop_svm "$RELAY_PUBKEY_BASE58" 10 "$SVM_RPC_URL"
log "   ✅ Trusted-GMP relay funded on SVM"

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
