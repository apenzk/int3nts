#!/bin/bash

# Run Coordinator Locally (Against Testnets)
#
# This script runs the coordinator service locally, connecting to:
#   - Movement Bardock Testnet (hub chain)
#   - Base Sepolia (connected chain)
#
# Use this to test before deploying to EC2.
#
# Prerequisites:
#   - coordinator/config/coordinator_testnet.toml configured with actual deployed addresses
#   - Rust toolchain installed
#
# Usage:
#   ./run-coordinator-local.sh
#   ./run-coordinator-local.sh --release  # Run release build (faster)

set -e

# Get the script directory and project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"

echo " Running Coordinator Locally (Testnet Mode)"
echo "==============================================="
echo ""

# Check config exists
COORDINATOR_CONFIG="$PROJECT_ROOT/coordinator/config/coordinator_testnet.toml"

if [ ! -f "$COORDINATOR_CONFIG" ]; then
    echo "❌ ERROR: coordinator_testnet.toml not found at $COORDINATOR_CONFIG"
    echo ""
    echo "   Create it from the template:"
    echo "   cp coordinator/config/coordinator.template.toml coordinator/config/coordinator_testnet.toml"
    echo ""
    echo "   Then populate with actual deployed contract addresses:"
    echo "   - intent_module_addr (hub_chain section)"
    echo "   - escrow_contract_addr (connected_chain_evm section)"
    echo "   - approver_evm_pubkey_hash (connected_chain_evm section)"
    exit 1
fi

# Validate config has actual addresses (not placeholders)
# Check for common placeholder patterns
if grep -qE "(0x123|0x\.\.\.|0xalice|0xbob)" "$COORDINATOR_CONFIG"; then
    echo "❌ ERROR: coordinator_testnet.toml still has placeholder addresses"
    echo ""
    echo "   Update the config file with actual deployed addresses:"
    echo "   - intent_module_addr (hub_chain section)"
    echo "   - escrow_contract_addr (connected_chain_evm section)"
    echo "   - approver_evm_pubkey_hash (connected_chain_evm section)"
    echo ""
    echo "   Contract addresses should be read from your deployment logs."
    exit 1
fi

# Extract config values for display
HUB_RPC=$(grep -A5 "\[hub_chain\]" "$COORDINATOR_CONFIG" | grep "rpc_url" | head -1 | sed 's/.*= *"\(.*\)".*/\1/')
EVM_RPC=$(grep -A5 "\[connected_chain_evm\]" "$COORDINATOR_CONFIG" | grep "rpc_url" | head -1 | sed 's/.*= *"\(.*\)".*/\1/')
API_PORT=$(grep -A5 "\[api\]" "$COORDINATOR_CONFIG" | grep "port" | head -1 | sed 's/.*= *\([0-9]*\).*/\1/')
INTENT_MODULE=$(grep -A5 "\[hub_chain\]" "$COORDINATOR_CONFIG" | grep "intent_module_addr" | head -1 | sed 's/.*= *"\(.*\)".*/\1/')
ESCROW_CONTRACT=$(grep -A5 "\[connected_chain_evm\]" "$COORDINATOR_CONFIG" | grep "escrow_contract_addr" | head -1 | sed 's/.*= *"\(.*\)".*/\1/')

# Check for API key placeholders in RPC URLs
if [[ "$HUB_RPC" == *"ALCHEMY_API_KEY"* ]] || [[ "$EVM_RPC" == *"ALCHEMY_API_KEY"* ]]; then
    echo "️  WARNING: RPC URLs contain API key placeholders (ALCHEMY_API_KEY)"
    echo "   The coordinator service does not substitute placeholders - use full URLs in config"
    echo "   Or use the public RPC URLs from testnet-assets.toml"
    echo ""
fi

echo " Configuration:"
echo "   Config file: $COORDINATOR_CONFIG"
echo ""
echo "   Hub Chain:"
echo "     RPC:              $HUB_RPC"
echo "     Intent Module:     $INTENT_MODULE"
echo ""
echo "   EVM Chain:"
echo "     RPC:              $EVM_RPC"
echo "     Escrow Contract:  $ESCROW_CONTRACT"
echo ""
echo "   API Server:"
echo "     Port:             ${API_PORT:-3333}"
echo ""

cd "$PROJECT_ROOT/coordinator"

# Parse flags
USE_RELEASE=false
USE_DEBUG_LOG=false
for arg in "$@"; do
    case "$arg" in
        --release) USE_RELEASE=true ;;
        --debug)   USE_DEBUG_LOG=true ;;
    esac
done

# Set log level
if $USE_DEBUG_LOG; then
    LOG_LEVEL="debug"
    echo "   Log level: debug"
else
    LOG_LEVEL="info"
fi

# Set up log file
LOG_DIR="$SCRIPT_DIR/logs"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/coordinator-$(date +%Y%m%d-%H%M%S).log"

# Check if --release flag is passed
if $USE_RELEASE; then
    echo " Building release binary..."
    nix develop "$PROJECT_ROOT/nix" --command bash -c "cd '$PROJECT_ROOT/coordinator' && cargo build --release"
    echo ""
    echo " Starting coordinator (release mode)..."
    echo "   Log file: $LOG_FILE"
    echo "   Press Ctrl+C to stop"
    echo ""
    RUST_LOG=$LOG_LEVEL ./target/release/coordinator --testnet 2>&1 | tee "$LOG_FILE"
else
    echo " Starting coordinator (debug mode)..."
    echo "   Log file: $LOG_FILE"
    echo "   Press Ctrl+C to stop"
    echo "   (Use --release for faster performance)"
    echo ""
    nix develop "$PROJECT_ROOT/nix" --command bash -c "cd '$PROJECT_ROOT/coordinator' && RUST_LOG=$LOG_LEVEL cargo run --bin coordinator -- --testnet" 2>&1 | tee "$LOG_FILE"
fi

