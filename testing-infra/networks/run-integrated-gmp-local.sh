#!/bin/bash

# Run Integrated GMP Locally
#
# Runs the integrated-gmp service locally against a specified network.
#
# Usage:
#   ./run-integrated-gmp-local.sh --network testnet
#   ./run-integrated-gmp-local.sh --network mainnet --release
#   ./run-integrated-gmp-local.sh --network testnet --debug

set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"

# Parse arguments
NETWORK=""
USE_RELEASE=false
USE_DEBUG_LOG=false
shift_next=false
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
INTEGRATED_GMP_CONFIG="$PROJECT_ROOT/integrated-gmp/config/integrated-gmp_${NETWORK}.toml"

if [ ! -f "$INTEGRATED_GMP_CONFIG" ]; then
    echo "ERROR: integrated-gmp_${NETWORK}.toml not found at $INTEGRATED_GMP_CONFIG"
    echo ""
    echo "   Create it from the template:"
    echo "   cp integrated-gmp/config/integrated-gmp.template.toml integrated-gmp/config/integrated-gmp_${NETWORK}.toml"
    echo ""
    echo "   Then populate with actual deployed contract addresses."
    exit 1
fi

# Load env file for keys
ENV_FILE="$NETWORK_DIR/.env.${NETWORK}"

if [ ! -f "$ENV_FILE" ]; then
    echo "ERROR: .env.${NETWORK} not found at $ENV_FILE"
    echo ""
    echo "   Create it from the template in the ${NETWORK} directory:"
    echo "   cp env.${NETWORK}.example .env.${NETWORK}"
    echo ""
    echo "   Then populate with your keys."
    exit 1
fi

source "$ENV_FILE"

# Check required environment variables
REQUIRED_VARS=(
    "INTEGRATED_GMP_PRIVATE_KEY"
    "INTEGRATED_GMP_PUBLIC_KEY"
)

MISSING_VARS=()
for var in "${REQUIRED_VARS[@]}"; do
    if [ -z "${!var}" ]; then
        MISSING_VARS+=("$var")
    fi
done

if [ ${#MISSING_VARS[@]} -ne 0 ]; then
    echo "ERROR: Missing required environment variables in .env.${NETWORK}:"
    for var in "${MISSING_VARS[@]}"; do
        echo "   - $var"
    done
    echo ""
    echo "   These keys are required for the integrated-gmp service to sign approvals."
    exit 1
fi

# Validate config has actual addresses (not placeholders)
if grep -qE "(0x123|0x\.\.\.|0xalice|0xbob)" "$INTEGRATED_GMP_CONFIG"; then
    echo "ERROR: integrated-gmp_${NETWORK}.toml still has placeholder addresses"
    echo ""
    echo "   Update the config file with actual deployed addresses."
    echo "   Contract addresses should be read from your deployment logs."
    exit 1
fi

# Extract config values for display
HUB_RPC=$(grep -A5 "\[hub_chain\]" "$INTEGRATED_GMP_CONFIG" | grep "rpc_url" | head -1 | sed 's/.*= *"\(.*\)".*/\1/')
INTENT_MODULE=$(grep -A5 "\[hub_chain\]" "$INTEGRATED_GMP_CONFIG" | grep "intent_module_addr" | head -1 | sed 's/.*= *"\(.*\)".*/\1/')

echo " Running Integrated GMP Locally (${NETWORK} mode)"
echo "================================================"
echo ""
echo " Configuration:"
echo "   Config file: $INTEGRATED_GMP_CONFIG"
echo "   Keys file:   $ENV_FILE"
echo "   Network:     $NETWORK"
echo ""
echo "   Hub Chain:"
echo "     RPC:              $HUB_RPC"
echo "     Intent Module:     $INTENT_MODULE"
echo ""

cd "$PROJECT_ROOT/integrated-gmp"

# Export environment variables for integrated-gmp keys
export INTEGRATED_GMP_PRIVATE_KEY
export INTEGRATED_GMP_PUBLIC_KEY

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
LOG_FILE="$LOG_DIR/integrated-gmp-$(date +%Y%m%d-%H%M%S).log"

if $USE_RELEASE; then
    echo " Building release binary..."
    nix develop "$PROJECT_ROOT/nix" --command bash -c "cd '$PROJECT_ROOT/integrated-gmp' && cargo build --release"
    echo ""
    echo " Starting integrated-gmp (release mode)..."
    echo "   Log file: $LOG_FILE"
    echo "   Press Ctrl+C to stop"
    echo ""
    RUST_LOG=$LOG_LEVEL ./target/release/integrated-gmp --${NETWORK} 2>&1 | tee "$LOG_FILE"
else
    echo " Starting integrated-gmp (debug mode)..."
    echo "   Log file: $LOG_FILE"
    echo "   Press Ctrl+C to stop"
    echo "   (Use --release for faster performance)"
    echo ""
    nix develop "$PROJECT_ROOT/nix" --command bash -c "cd '$PROJECT_ROOT/integrated-gmp' && RUST_LOG=$LOG_LEVEL cargo run --bin integrated-gmp -- --${NETWORK}" 2>&1 | tee "$LOG_FILE"
fi
