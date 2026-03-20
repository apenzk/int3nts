#!/bin/bash

# Source common utilities
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"
source "$SCRIPT_DIR/utils.sh"

# Setup project root and logging
setup_project_root

# Accept instance number as argument (default: 1)
evm_instance_vars "${1:-1}"

setup_logging "stop-chain-evm${EVM_INSTANCE}"
cd "$PROJECT_ROOT"

log " EVM CHAIN CLEANUP (instance $EVM_INSTANCE)"
log "===================="
log_and_echo " All output logged to: $LOG_FILE"

log ""
log " Stopping Hardhat node (instance $EVM_INSTANCE)..."

# Kill by PID if exists
if [ -f "$EVM_PID_FILE" ]; then
    log "   - Found PID file, stopping processes..."
    while read -r pid; do
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            log "     Killing process (PID: $pid)..."
            kill "$pid"
        fi
    done < "$EVM_PID_FILE"
    sleep 1
    # Force kill any remaining
    while read -r pid; do
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            kill -9 "$pid"
        fi
    done < "$EVM_PID_FILE"
    rm -f "$EVM_PID_FILE"
    log "   ✅ Stopped processes from PID file"
else
    log "   - No PID file found"
fi

# Wait a moment for processes to fully terminate
sleep 1

# Verify port is free
if lsof -i :$EVM_PORT >/dev/null 2>&1; then
    log "   ️  Warning: Port $EVM_PORT is still in use"
    log "   - Killing process on port $EVM_PORT..."
    lsof -ti :$EVM_PORT | xargs kill -9
    sleep 1
    log "   ✅ Process on port $EVM_PORT killed"
fi

log ""
log_and_echo "✅ EVM chain cleanup complete (instance $EVM_INSTANCE)"
log ""
