#!/bin/bash

# Setup SVM Chain (solana-test-validator)
# Accepts instance number as argument (default: 2).

set -e

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/utils.sh"

# Setup project root and logging
setup_project_root

# Accept instance number as argument
svm_instance_vars "${1:-2}"

setup_logging "setup-chain-svm${SVM_INSTANCE}"
cd "$PROJECT_ROOT"

log " SVM CHAIN SETUP (instance $SVM_INSTANCE)"
log "=================="
log_and_echo " All output logged to: $LOG_FILE"

log ""
log " Stopping any existing solana-test-validator on port $SVM_PORT..."
if [ -f "$SVM_PID_FILE" ]; then
    while IFS= read -r pid; do
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            log "   Killing process (PID: $pid)..."
            kill "$pid" 2>/dev/null || true
        fi
    done < "$SVM_PID_FILE"
    rm -f "$SVM_PID_FILE"
fi
# Also kill any process on our port
if lsof -i :$SVM_PORT >/dev/null 2>&1; then
    lsof -ti :$SVM_PORT | xargs kill 2>/dev/null || true
fi
sleep 2

log ""
log " Starting solana-test-validator on port $SVM_PORT..."

mkdir -p "$SVM_LEDGER_DIR"

svm_cmd "solana-test-validator --reset --ledger \"$SVM_LEDGER_DIR\" --rpc-port $SVM_PORT --faucet-port $SVM_FAUCET_PORT --gossip-port $SVM_GOSSIP_PORT --dynamic-port-range $SVM_DYNAMIC_PORT_RANGE" > "$LOG_FILE" 2>&1 &
VALIDATOR_PID=$!

mkdir -p "$PROJECT_ROOT/.tmp"
echo "$VALIDATOR_PID" > "$SVM_PID_FILE"

log "   solana-test-validator started with PID: $VALIDATOR_PID"

log ""
log "⏳ Waiting for SVM chain to be ready..."
for i in {1..120}; do
    if check_svm_chain_running "$SVM_RPC_URL"; then
        log "   ✅ SVM chain ready!"
        break
    fi
    if [ $((i % 20)) -eq 0 ]; then
        log "   Still waiting... (${i}/120 seconds)"
    fi
    if [ "$i" -eq 120 ]; then
        log_and_echo "   ❌ Timeout waiting for solana-test-validator (120 seconds)"
        log_and_echo "   Last 50 lines of setup log:"
        tail -50 "$LOG_FILE" | while IFS= read -r line; do
            log_and_echo "   $line"
        done
        INTERNAL_LOG="$SVM_LEDGER_DIR/validator.log"
        if [ -f "$INTERNAL_LOG" ]; then
            log_and_echo "   Last 100 lines of validator internal log ($INTERNAL_LOG):"
            tail -100 "$INTERNAL_LOG" | while IFS= read -r line; do
                log_and_echo "   $line"
            done
        else
            log_and_echo "   No validator internal log found at $INTERNAL_LOG"
        fi
        kill "$VALIDATOR_PID" 2>/dev/null || true
        exit 1
    fi
    sleep 1
done

log ""
log "✅ SVM chain is running (instance $SVM_INSTANCE)!"
log "   RPC URL: $SVM_RPC_URL"
log "   PID: $VALIDATOR_PID"
