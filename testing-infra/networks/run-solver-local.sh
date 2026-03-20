#!/bin/bash

# Run Solver Locally
#
# Runs the solver service locally against a specified network.
#
# Prerequisites:
#   - solver/config/solver_${NETWORK}.toml configured with actual deployed addresses
#   - .env.${NETWORK} with SOLVER_EVM_PRIVATE_KEY
#   - Coordinator and integrated-gmp running
#
# Usage:
#   ./run-solver-local.sh --network testnet
#   ./run-solver-local.sh --network mainnet --release
#   ./run-solver-local.sh --network testnet --debug

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

# Load env file
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

# Check SOLVER_EVM_PRIVATE_KEY (required for EVM transactions)
if [ -z "$SOLVER_EVM_PRIVATE_KEY" ]; then
    echo "WARNING: SOLVER_EVM_PRIVATE_KEY not set in .env.${NETWORK}"
    echo "   EVM transactions will fail if an EVM connected chain is configured."
    echo ""
fi

# Check SOLANA_SOLVER_PRIVATE_KEY (required for SVM transactions)
if [ -z "$SOLANA_SOLVER_PRIVATE_KEY" ]; then
    echo "WARNING: SOLANA_SOLVER_PRIVATE_KEY not set in .env.${NETWORK}"
    echo "   SVM transactions will fail if an SVM connected chain is configured."
    echo ""
fi

# Check config exists
SOLVER_CONFIG="$PROJECT_ROOT/solver/config/solver_${NETWORK}.toml"

if [ ! -f "$SOLVER_CONFIG" ]; then
    echo "ERROR: solver_${NETWORK}.toml not found at $SOLVER_CONFIG"
    echo ""
    echo "   Create it from the template:"
    echo "   cp solver/config/solver.template.toml solver/config/solver_${NETWORK}.toml"
    echo ""
    echo "   Then populate with actual deployed contract addresses."
    exit 1
fi

# Validate config has actual addresses (not placeholders)
if grep -qE "(0x123|0x\.\.\.|0x\.\.\.)" "$SOLVER_CONFIG"; then
    echo "ERROR: solver_${NETWORK}.toml still has placeholder addresses"
    echo ""
    echo "   Update the config file with actual deployed addresses."
    echo "   Contract addresses should be read from your deployment logs."
    exit 1
fi

# Extract config values for display (skip comment lines)
COORDINATOR_URL=$(grep "^coordinator_url" "$SOLVER_CONFIG" | grep -v "^#" | head -1 | sed 's/.*= *"\(.*\)".*/\1/' | sed 's/#.*$//' | xargs)
HUB_RPC=$(grep -A5 "\[hub_chain\]" "$SOLVER_CONFIG" | grep "^rpc_url" | grep -v "^#" | head -1 | sed 's/.*= *"\(.*\)".*/\1/' | sed 's/#.*$//' | xargs)
HUB_MODULE=$(grep -A5 "\[hub_chain\]" "$SOLVER_CONFIG" | grep "^module_addr" | grep -v "^#" | head -1 | sed 's/.*= *"\(.*\)".*/\1/' | sed 's/#.*$//' | xargs)
SOLVER_PROFILE=$(grep -A5 "\[solver\]" "$SOLVER_CONFIG" | grep "^profile" | grep -v "^#" | head -1 | sed 's/.*= *"\(.*\)".*/\1/' | sed 's/#.*$//' | xargs)
SOLVER_ADDR=$(grep -A5 "\[solver\]" "$SOLVER_CONFIG" | grep "^address" | grep -v "^#" | head -1 | sed 's/.*= *"\(.*\)".*/\1/' | sed 's/#.*$//' | xargs)

# Check which connected chains are configured
CONNECTED_TYPES=""
for CONNECTED_TYPE in $(grep -A1 "\[\[connected_chain\]\]" "$SOLVER_CONFIG" | grep "^type" | sed 's/.*= *"\(.*\)".*/\1/' | sed 's/#.*$//' | xargs); do
    CONNECTED_TYPES="${CONNECTED_TYPES:+$CONNECTED_TYPES,}$CONNECTED_TYPE"
done

if [ -z "$CONNECTED_TYPES" ]; then
    echo "ERROR: No [[connected_chain]] configured in $SOLVER_CONFIG"
    echo ""
    echo "   Add at least one [[connected_chain]] section with type = \"evm\", \"svm\", or \"mvm\""
    exit 1
fi

echo " Running Solver Locally (${NETWORK} mode)"
echo "========================================="
echo ""
echo " Configuration:"
echo "   Config file: $SOLVER_CONFIG"
echo "   Keys file:   $ENV_FILE"
echo "   Network:     $NETWORK"
echo "   Connected:   $CONNECTED_TYPES"
echo ""
echo "   Coordinator:"
echo "     URL:              $COORDINATOR_URL"
echo ""
echo "   Hub Chain:"
echo "     RPC:              $HUB_RPC"
echo "     Module Address:   $HUB_MODULE"
echo ""
echo "   Solver:"
echo "     Profile:          $SOLVER_PROFILE"
echo "     Address:          $SOLVER_ADDR"
echo ""

# Check coordinator is reachable
echo "   Checking coordinator health..."
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" --max-time 5 "$COORDINATOR_URL/health" 2>/dev/null || echo "000")

if [ "$HTTP_CODE" = "200" ]; then
    echo "   Coordinator is healthy"
else
    echo "   Coordinator not responding at $COORDINATOR_URL (HTTP $HTTP_CODE)"
    echo ""
    echo "   Make sure coordinator is running first:"
    echo "   ./testing-infra/networks/run-coordinator-local.sh --network $NETWORK"
    echo ""
    read -p "   Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo ""

cd "$PROJECT_ROOT"

# Export environment variables for solver
export SOLVER_EVM_PRIVATE_KEY
export SOLANA_SOLVER_PRIVATE_KEY
export BASE_SOLVER_ADDR
if [ -z "$SOLVER_EVM_ADDR" ] && [ -n "$BASE_SOLVER_ADDR" ]; then
    SOLVER_EVM_ADDR="$BASE_SOLVER_ADDR"
fi
export SOLVER_EVM_ADDR
if [ -n "$MOVEMENT_SOLVER_PRIVATE_KEY" ]; then
    export MOVEMENT_SOLVER_PRIVATE_KEY
fi

# Convert SOLANA_SOLVER_ADDR (base58) to hex for solver registration
if [ -n "$SOLANA_SOLVER_ADDR" ]; then
    SOLVER_SVM_ADDR=$(node -e "const bs58 = require('bs58'); console.log('0x' + Buffer.from(bs58.decode('$SOLANA_SOLVER_ADDR')).toString('hex'))")
    export SOLVER_SVM_ADDR
fi

export HUB_RPC_URL="$HUB_RPC"

# Set log level
if $USE_DEBUG_LOG; then
    SOLVER_LOG_LEVEL="debug"
    echo "   Log level: debug"
else
    SOLVER_LOG_LEVEL="info,solver::service::tracker=debug,solver::chains::hub=debug"
fi

# Prepare environment variables for nix develop
ENV_VARS="SOLVER_CONFIG_PATH='$SOLVER_CONFIG' RUST_LOG=$SOLVER_LOG_LEVEL HUB_RPC_URL='$HUB_RPC'"
[ -n "$SOLVER_EVM_PRIVATE_KEY" ] && ENV_VARS="$ENV_VARS SOLVER_EVM_PRIVATE_KEY='$SOLVER_EVM_PRIVATE_KEY'"
[ -n "$BASE_SOLVER_ADDR" ] && ENV_VARS="$ENV_VARS BASE_SOLVER_ADDR='$BASE_SOLVER_ADDR'"
[ -n "$SOLVER_EVM_ADDR" ] && ENV_VARS="$ENV_VARS SOLVER_EVM_ADDR='$SOLVER_EVM_ADDR'"
[ -n "$MOVEMENT_SOLVER_PRIVATE_KEY" ] && ENV_VARS="$ENV_VARS MOVEMENT_SOLVER_PRIVATE_KEY='$MOVEMENT_SOLVER_PRIVATE_KEY'"
[ -n "$SOLANA_SOLVER_PRIVATE_KEY" ] && ENV_VARS="$ENV_VARS SOLANA_SOLVER_PRIVATE_KEY='$SOLANA_SOLVER_PRIVATE_KEY'"
[ -n "$SOLVER_SVM_ADDR" ] && ENV_VARS="$ENV_VARS SOLVER_SVM_ADDR='$SOLVER_SVM_ADDR'"

# Set up log file
LOG_DIR="$NETWORK_DIR/logs"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/solver-$(date +%Y%m%d-%H%M%S).log"

if $USE_RELEASE; then
    echo " Building release binary..."
    nix develop "$PROJECT_ROOT/nix" --command bash -c "cargo build --release --manifest-path solver/Cargo.toml"
    echo ""
    echo " Starting solver (release mode)..."
    echo "   Log file: $LOG_FILE"
    echo "   Press Ctrl+C to stop"
    echo ""
    eval "$ENV_VARS ./solver/target/release/solver" 2>&1 | tee "$LOG_FILE"
else
    echo " Starting solver (debug mode)..."
    echo "   Log file: $LOG_FILE"
    echo "   Press Ctrl+C to stop"
    echo "   (Use --release for faster performance)"
    echo ""
    nix develop "$PROJECT_ROOT/nix" --command bash -c "$ENV_VARS cargo run --manifest-path solver/Cargo.toml --bin solver" 2>&1 | tee "$LOG_FILE"
fi
