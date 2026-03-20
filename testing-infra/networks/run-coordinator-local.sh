#!/bin/bash

# Run Coordinator Locally
#
# Runs the coordinator service locally against a specified network.
#
# Usage:
#   ./run-coordinator-local.sh --network testnet
#   ./run-coordinator-local.sh --network mainnet --release
#   ./run-coordinator-local.sh --network testnet --debug

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"

# Parse arguments
NETWORK=""
USE_RELEASE=false
USE_DEBUG_LOG=false
for arg in "$@"; do
    case "$arg" in
        --network)  shift_next=true ;;
        --release)  USE_RELEASE=true ;;
        --debug)    USE_DEBUG_LOG=true ;;
        *)
            if [ "$shift_next" = true ]; then
                NETWORK="$arg"
                shift_next=false
            fi
            ;;
    esac
done

# Handle --network=value syntax
for arg in "$@"; do
    case "$arg" in
        --network=*) NETWORK="${arg#--network=}" ;;
    esac
done

if [ -z "$NETWORK" ]; then
    echo "Usage: $0 --network testnet|mainnet [--release] [--debug]" >&2
    exit 1
fi

NETWORK_DIR="$SCRIPT_DIR/$NETWORK"

if [ ! -d "$NETWORK_DIR" ]; then
    echo "ERROR: Network directory not found: $NETWORK_DIR" >&2
    exit 1
fi

# Check config exists
COORDINATOR_CONFIG="$PROJECT_ROOT/coordinator/config/coordinator_${NETWORK}.toml"

if [ ! -f "$COORDINATOR_CONFIG" ]; then
    echo "ERROR: coordinator_${NETWORK}.toml not found at $COORDINATOR_CONFIG"
    echo ""
    echo "   Create it from the template:"
    echo "   cp coordinator/config/coordinator.template.toml coordinator/config/coordinator_${NETWORK}.toml"
    echo ""
    echo "   Then populate with actual deployed contract addresses."
    exit 1
fi

# Validate config has actual addresses (not placeholders)
if grep -qE "(0x123|0x\.\.\.|0xalice|0xbob)" "$COORDINATOR_CONFIG"; then
    echo "ERROR: coordinator_${NETWORK}.toml still has placeholder addresses"
    echo ""
    echo "   Update the config file with actual deployed addresses."
    echo "   Contract addresses should be read from your deployment logs."
    exit 1
fi

# Extract config values for display
HUB_RPC=$(grep -A5 "\[hub_chain\]" "$COORDINATOR_CONFIG" | grep "rpc_url" | head -1 | sed 's/.*= *"\(.*\)".*/\1/')
INTENT_MODULE=$(grep -A5 "\[hub_chain\]" "$COORDINATOR_CONFIG" | grep "intent_module_addr" | head -1 | sed 's/.*= *"\(.*\)".*/\1/')

echo " Running Coordinator Locally (${NETWORK} mode)"
echo "==============================================="
echo ""
echo " Configuration:"
echo "   Config file: $COORDINATOR_CONFIG"
echo "   Network:     $NETWORK"
echo ""
echo "   Hub Chain:"
echo "     RPC:              $HUB_RPC"
echo "     Intent Module:     $INTENT_MODULE"
echo ""

cd "$PROJECT_ROOT/coordinator"

# Set log level
if $USE_DEBUG_LOG; then
    LOG_LEVEL="debug"
    echo "   Log level: debug"
else
    LOG_LEVEL="info"
fi

# Set up log file
LOG_DIR="$NETWORK_DIR/logs"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/coordinator-$(date +%Y%m%d-%H%M%S).log"

if $USE_RELEASE; then
    echo " Building release binary..."
    nix develop "$PROJECT_ROOT/nix" --command bash -c "cd '$PROJECT_ROOT/coordinator' && cargo build --release"
    echo ""
    echo " Starting coordinator (release mode)..."
    echo "   Log file: $LOG_FILE"
    echo "   Press Ctrl+C to stop"
    echo ""
    RUST_LOG=$LOG_LEVEL ./target/release/coordinator --${NETWORK} 2>&1 | tee "$LOG_FILE"
else
    echo " Starting coordinator (debug mode)..."
    echo "   Log file: $LOG_FILE"
    echo "   Press Ctrl+C to stop"
    echo "   (Use --release for faster performance)"
    echo ""
    nix develop "$PROJECT_ROOT/nix" --command bash -c "cd '$PROJECT_ROOT/coordinator' && RUST_LOG=$LOG_LEVEL cargo run --bin coordinator -- --${NETWORK}" 2>&1 | tee "$LOG_FILE"
fi
