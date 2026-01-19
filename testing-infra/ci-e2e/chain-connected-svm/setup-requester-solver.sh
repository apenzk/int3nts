#!/bin/bash

# Setup SVM requester/solver accounts and test mint

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_svm.sh"

setup_project_root
setup_logging "setup-svm-requester-solver"
cd "$PROJECT_ROOT"

log " Requester and Solver Account Setup - SVM CHAIN"
log "================================================="
log_and_echo " All output logged to: $LOG_FILE"

SVM_RPC_URL="http://127.0.0.1:8899"
E2E_DIR="$PROJECT_ROOT/.tmp/svm-e2e"
mkdir -p "$E2E_DIR"

PAYER_KEYPAIR="$E2E_DIR/payer.json"
REQUESTER_KEYPAIR="$E2E_DIR/requester.json"
SOLVER_KEYPAIR="$E2E_DIR/solver.json"

log ""
log " Creating keypairs..."
ensure_svm_keypair "$PAYER_KEYPAIR"
ensure_svm_keypair "$REQUESTER_KEYPAIR"
ensure_svm_keypair "$SOLVER_KEYPAIR"

PAYER_PUBKEY=$(get_svm_pubkey "$PAYER_KEYPAIR")
REQUESTER_SVM_PUBKEY=$(get_svm_pubkey "$REQUESTER_KEYPAIR")
SOLVER_SVM_PUBKEY=$(get_svm_pubkey "$SOLVER_KEYPAIR")

log "   ✅ Payer:     $PAYER_PUBKEY"
log "   ✅ Requester: $REQUESTER_SVM_PUBKEY"
log "   ✅ Solver:    $SOLVER_SVM_PUBKEY"

log ""
log " Airdropping SOL..."
airdrop_svm "$PAYER_PUBKEY" 10 "$SVM_RPC_URL"
airdrop_svm "$REQUESTER_SVM_PUBKEY" 10 "$SVM_RPC_URL"
airdrop_svm "$SOLVER_SVM_PUBKEY" 10 "$SVM_RPC_URL"

log ""
log " Creating test USDcon SPL mint..."
MINT_ADDR=$(create_svm_mint "$PAYER_KEYPAIR" "$SVM_RPC_URL")
if [ -z "$MINT_ADDR" ]; then
    log_and_echo "❌ ERROR: Failed to create USDcon SPL mint"
    exit 1
fi
log "   ✅ Mint: $MINT_ADDR"

log ""
log " Creating USDcon token accounts..."
REQUESTER_SVM_TOKEN_ACCOUNT=$(create_svm_token_account "$MINT_ADDR" "$REQUESTER_SVM_PUBKEY" "$PAYER_KEYPAIR" "$SVM_RPC_URL")
SOLVER_SVM_TOKEN_ACCOUNT=$(create_svm_token_account "$MINT_ADDR" "$SOLVER_SVM_PUBKEY" "$PAYER_KEYPAIR" "$SVM_RPC_URL")

log "   ✅ Requester USDcon token account: $REQUESTER_SVM_TOKEN_ACCOUNT"
log "   ✅ Solver USDcon token account:    $SOLVER_SVM_TOKEN_ACCOUNT"

log ""
log " Verifying derived USDcon token accounts..."
log "   Requester ATA (expected): $REQUESTER_SVM_TOKEN_ACCOUNT"
log "   Solver ATA (expected):    $SOLVER_SVM_TOKEN_ACCOUNT"
log "   Requester ATA (cli):      $(get_svm_associated_token_address "$MINT_ADDR" "$REQUESTER_SVM_PUBKEY" "$SVM_RPC_URL")"
log "   Solver ATA (cli):         $(get_svm_associated_token_address "$MINT_ADDR" "$SOLVER_SVM_PUBKEY" "$SVM_RPC_URL")"

log ""
log " Minting USDcon..."
# spl-token mint expects token units; 1 token = 1_000_000 base units at 6 decimals.
log "   Minting to requester..."
REQUESTER_MINT_OUTPUT=$(mint_svm_tokens "$MINT_ADDR" 1 "$REQUESTER_SVM_TOKEN_ACCOUNT" "$PAYER_KEYPAIR" "$SVM_RPC_URL" 2>&1 || true)
log "$REQUESTER_MINT_OUTPUT"
log "   Minting to solver..."
SOLVER_MINT_OUTPUT=$(mint_svm_tokens "$MINT_ADDR" 1 "$SOLVER_SVM_TOKEN_ACCOUNT" "$PAYER_KEYPAIR" "$SVM_RPC_URL" 2>&1 || true)
log "$SOLVER_MINT_OUTPUT"

log ""
log " Post-mint token balances (base units):"

# Balance reads can lag immediately after mint. Retry to avoid false negatives.
BALANCE_ATTEMPTS=15
BALANCE_RETRY_DELAY=2

fetch_svm_balance_with_retry() {
    local label="$1"
    local token_account="$2"
    local attempts="${3:-$BALANCE_ATTEMPTS}"
    local delay_seconds="${4:-$BALANCE_RETRY_DELAY}"
    local expected="${5:-1000000}"
    local attempt=1
    local output=""
    local status=1
    local balance=""

    while [ "$attempt" -le "$attempts" ]; do
        set +e
        output=$(SVM_TOKEN_ACCOUNT="$token_account" SVM_RPC_URL="$SVM_RPC_URL" \
            bash "$PROJECT_ROOT/svm-intent-framework/scripts/get-token-balance.sh" 2>&1)
        status=$?
        set -e

        balance=$(echo "$output" | grep -Eo 'Balance: [0-9]+' | awk '{print $2}' | tail -1 | tr -d '\n')
        if [ "$status" -eq 0 ] && [ "$balance" = "$expected" ]; then
            echo "$output"
            return 0
        fi

        log "   ${label} balance attempt ${attempt}/${attempts} failed; retrying..."
        sleep "$delay_seconds"
        attempt=$((attempt + 1))
    done

    echo "$output"
    return 1
}

set +e
REQUESTER_BALANCE_OUTPUT=$(fetch_svm_balance_with_retry "Requester" "$REQUESTER_SVM_TOKEN_ACCOUNT")
REQUESTER_BALANCE_STATUS=$?
SOLVER_BALANCE_OUTPUT=$(fetch_svm_balance_with_retry "Solver" "$SOLVER_SVM_TOKEN_ACCOUNT")
SOLVER_BALANCE_STATUS=$?
set -e

log "   Requester balance output:"
log "$REQUESTER_BALANCE_OUTPUT"
log "   Solver balance output:"
log "$SOLVER_BALANCE_OUTPUT"

REQUESTER_BALANCE=$(echo "$REQUESTER_BALANCE_OUTPUT" | grep -Eo 'Balance: [0-9]+' | awk '{print $2}' | tail -1 | tr -d '\n')
SOLVER_BALANCE=$(echo "$SOLVER_BALANCE_OUTPUT" | grep -Eo 'Balance: [0-9]+' | awk '{print $2}' | tail -1 | tr -d '\n')

if [ "$REQUESTER_BALANCE_STATUS" -ne 0 ] || [ "$SOLVER_BALANCE_STATUS" -ne 0 ]; then
    log_and_echo "❌ ERROR: Token balance command failed after minting"
    log_and_echo "   Requester status: $REQUESTER_BALANCE_STATUS"
    log_and_echo "   Solver status:    $SOLVER_BALANCE_STATUS"
    exit 1
fi

if [ -z "$REQUESTER_BALANCE" ] || [ -z "$SOLVER_BALANCE" ]; then
    log_and_echo "❌ ERROR: Failed to read token balances after minting"
    log_and_echo "   Requester balance: ${REQUESTER_BALANCE:-missing}"
    log_and_echo "   Solver balance: ${SOLVER_BALANCE:-missing}"
    exit 1
fi

if [ "$REQUESTER_BALANCE" != "1000000" ] || [ "$SOLVER_BALANCE" != "1000000" ]; then
    log_and_echo "❌ ERROR: Token balances do not match expected values after minting"
    log_and_echo "   Requester balance: $REQUESTER_BALANCE (expected 1000000)"
    log_and_echo "   Solver balance:    $SOLVER_BALANCE (expected 1000000)"
    exit 1
fi

log ""
log " Saving chain info..."
CHAIN_INFO="$PROJECT_ROOT/.tmp/chain-info.env"
cat >> "$CHAIN_INFO" << EOF
SVM_RPC_URL=$SVM_RPC_URL
SVM_PAYER_KEYPAIR=$PAYER_KEYPAIR
SVM_REQUESTER_KEYPAIR=$REQUESTER_KEYPAIR
SVM_SOLVER_KEYPAIR=$SOLVER_KEYPAIR
REQUESTER_SVM_PUBKEY=$REQUESTER_SVM_PUBKEY
SOLVER_SVM_PUBKEY=$SOLVER_SVM_PUBKEY
USD_SVM_MINT_ADDR=$MINT_ADDR
REQUESTER_SVM_TOKEN_ACCOUNT=$REQUESTER_SVM_TOKEN_ACCOUNT
SOLVER_SVM_TOKEN_ACCOUNT=$SOLVER_SVM_TOKEN_ACCOUNT
SVM_CHAIN_ID=4
EOF

log ""
log "✅ SVM requester/solver setup complete"
