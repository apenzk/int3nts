#!/bin/bash

# Stop SVM chain (solana-test-validator)

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/../util.sh"

setup_project_root
setup_logging "stop-chain-svm"
cd "$PROJECT_ROOT"

PID_FILE="$PROJECT_ROOT/.tmp/solana-test-validator.pid"

if [ -f "$PID_FILE" ]; then
    while IFS= read -r pid; do
        if [ -n "$pid" ]; then
            log "Stopping solana-test-validator (PID: $pid)..."
            kill "$pid" 2>/dev/null || true
        fi
    done < "$PID_FILE"
    rm -f "$PID_FILE"
fi

pkill -f "solana-test-validator" || true
log "✅ SVM chain stopped"
