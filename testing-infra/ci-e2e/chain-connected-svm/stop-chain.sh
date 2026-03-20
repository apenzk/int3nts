#!/bin/bash

# Stop SVM chain (solana-test-validator)
# Accepts instance number as argument (default: stops all instances).

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/utils.sh"

setup_project_root
setup_logging "stop-chain-svm"
cd "$PROJECT_ROOT"

stop_svm_instance() {
    local n="$1"
    svm_instance_vars "$n"

    if [ -f "$SVM_PID_FILE" ]; then
        while IFS= read -r pid; do
            if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
                log "Stopping solana-test-validator instance $n (PID: $pid)..."
                kill "$pid"
            fi
        done < "$SVM_PID_FILE"
        rm -f "$SVM_PID_FILE"
    fi

    # Kill any process on this instance's port
    if lsof -i :$SVM_PORT >/dev/null 2>&1; then
        log "Killing remaining process on port $SVM_PORT..."
        lsof -ti :$SVM_PORT | xargs kill 2>/dev/null || true
    fi
}

if [ -n "$1" ]; then
    # Stop specific instance
    stop_svm_instance "$1"
    log "✅ SVM chain stopped (instance $1)"
else
    # Stop all known instances
    stop_svm_instance 2
    stop_svm_instance 3
    # Also kill any remaining validator processes
    if pgrep -f "solana-test-validator" >/dev/null; then
        log "Killing remaining solana-test-validator processes..."
        pkill -f "solana-test-validator" || true
    fi
    log "✅ SVM chains stopped"
fi
