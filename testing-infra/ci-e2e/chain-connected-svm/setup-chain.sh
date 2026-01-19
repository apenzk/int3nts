#!/bin/bash

# Setup SVM Chain (solana-test-validator)

set -e

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/../util_svm.sh"

# Setup project root and logging
setup_project_root
setup_logging "setup-chain-svm"
cd "$PROJECT_ROOT"

log " SVM CHAIN SETUP"
log "=================="
log_and_echo " All output logged to: $LOG_FILE"

log ""
log " Stopping any existing solana-test-validator..."
pkill -f "solana-test-validator" || true
sleep 2

log ""
log " Starting solana-test-validator on port 8899..."

LEDGER_DIR="$PROJECT_ROOT/.tmp/solana-test-validator"
mkdir -p "$LEDGER_DIR"

svm_cmd "solana-test-validator --reset --ledger \"$LEDGER_DIR\" --rpc-port 8899" > "$LOG_FILE" 2>&1 &
VALIDATOR_PID=$!

mkdir -p "$PROJECT_ROOT/.tmp"
echo "$VALIDATOR_PID" > "$PROJECT_ROOT/.tmp/solana-test-validator.pid"

log "   solana-test-validator started with PID: $VALIDATOR_PID"

log ""
log "⏳ Waiting for SVM chain to be ready..."
for i in {1..120}; do
    if check_svm_chain_running "http://127.0.0.1:8899"; then
        log "   ✅ SVM chain ready!"
        break
    fi
    if [ $((i % 20)) -eq 0 ]; then
        log "   Still waiting... (${i}/120 seconds)"
    fi
    if [ "$i" -eq 120 ]; then
        log_and_echo "   ❌ Timeout waiting for solana-test-validator (120 seconds)"
        log_and_echo "   Last 50 lines of validator log:"
        tail -50 "$LOG_FILE" | while IFS= read -r line; do
            log_and_echo "   $line"
        done
        kill "$VALIDATOR_PID" 2>/dev/null || true
        exit 1
    fi
    sleep 1
done

log ""
log "✅ SVM chain is running!"
log "   RPC URL: http://127.0.0.1:8899"
log "   PID: $VALIDATOR_PID"
