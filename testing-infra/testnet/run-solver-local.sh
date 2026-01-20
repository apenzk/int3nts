#!/bin/bash

# Run Solver Locally (Against Testnets)
#
# This script runs the solver service locally, connecting to:
#   - Local or remote verifier (default: localhost:3333)
#   - Movement Bardock Testnet (hub chain)
#   - Base Sepolia (connected chain)
#
# Use this to test before deploying to EC2.
#
# Prerequisites:
#   - solver/config/solver_testnet.toml configured with actual deployed addresses
#   - .env.testnet with BASE_SOLVER_PRIVATE_KEY
#   - Movement CLI profile configured for solver (uses MOVEMENT_SOLVER_PRIVATE_KEY from .env.testnet)
#   - Verifier running (locally or remotely)
#   - Rust toolchain installed
#
# Usage:
#   ./run-solver-local.sh
#   ./run-solver-local.sh --release  # Run release build (faster)

set -e

# Get the script directory and project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"

echo " Running Solver Locally (Testnet Mode)"
echo "========================================="
echo ""

# Load .env.testnet for BASE_SOLVER_PRIVATE_KEY
TESTNET_KEYS_FILE="$SCRIPT_DIR/.env.testnet"

if [ ! -f "$TESTNET_KEYS_FILE" ]; then
    echo "❌ ERROR: .env.testnet not found at $TESTNET_KEYS_FILE"
    echo ""
    echo "   Create it from the template in this directory:"
    echo "   cp env.testnet.example .env.testnet"
    echo ""
    echo "   Then populate with your testnet keys."
    exit 1
fi

source "$TESTNET_KEYS_FILE"

# Check BASE_SOLVER_PRIVATE_KEY (required for EVM transactions)
if [ -z "$BASE_SOLVER_PRIVATE_KEY" ]; then
    echo "⚠️  WARNING: BASE_SOLVER_PRIVATE_KEY not set in .env.testnet"
    echo "   EVM transactions will fail if an EVM connected chain is configured."
    echo ""
fi

# Check SOLANA_SOLVER_PRIVATE_KEY (required for SVM transactions)
if [ -z "$SOLANA_SOLVER_PRIVATE_KEY" ]; then
    echo "⚠️  WARNING: SOLANA_SOLVER_PRIVATE_KEY not set in .env.testnet"
    echo "   SVM transactions will fail if an SVM connected chain is configured."
    echo "   This should be the base58-encoded 64-byte keypair (seed + pubkey)."
    echo ""
fi

# Check config exists
SOLVER_CONFIG="$PROJECT_ROOT/solver/config/solver_testnet.toml"

if [ ! -f "$SOLVER_CONFIG" ]; then
    echo "❌ ERROR: solver_testnet.toml not found at $SOLVER_CONFIG"
    echo ""
    echo "   Create it from the template:"
    echo "   cp solver/config/solver.template.toml solver/config/solver_testnet.toml"
    echo ""
    echo "   Then populate with actual deployed contract addresses:"
    echo "   - module_addr (hub_chain section)"
    echo "   - escrow_contract_addr (connected_chain_evm section)"
    echo "   - escrow_program_id (connected_chain_svm section)"
    echo "   - address (solver section)"
    echo "   - verifier_url (service section - use localhost:3333 for local testing)"
    exit 1
fi

# Validate config has actual addresses (not placeholders)
# Check for common placeholder patterns
if grep -qE "(0x123|0x\.\.\.|0x\.\.\.)" "$SOLVER_CONFIG"; then
    echo "❌ ERROR: solver_testnet.toml still has placeholder addresses"
    echo ""
    echo "   Update the config file with actual deployed addresses:"
    echo "   - module_addr (hub_chain section)"
    echo "   - escrow_contract_addr (connected_chain_evm section)"
    echo "   - escrow_program_id (connected_chain_svm section)"
    echo "   - address (solver section)"
    echo "   - verifier_url (service section - use localhost:3333 for local testing)"
    echo ""
    echo "   Contract addresses should be read from your deployment logs."
    exit 1
fi

# Extract config values for display (skip comment lines)
VERIFIER_URL=$(grep "^verifier_url" "$SOLVER_CONFIG" | grep -v "^#" | head -1 | sed 's/.*= *"\(.*\)".*/\1/' | sed 's/#.*$//' | xargs)
HUB_RPC=$(grep -A5 "\[hub_chain\]" "$SOLVER_CONFIG" | grep "^rpc_url" | grep -v "^#" | head -1 | sed 's/.*= *"\(.*\)".*/\1/' | sed 's/#.*$//' | xargs)
HUB_MODULE=$(grep -A5 "\[hub_chain\]" "$SOLVER_CONFIG" | grep "^module_addr" | grep -v "^#" | head -1 | sed 's/.*= *"\(.*\)".*/\1/' | sed 's/#.*$//' | xargs)
SOLVER_PROFILE=$(grep -A5 "\[solver\]" "$SOLVER_CONFIG" | grep "^profile" | grep -v "^#" | head -1 | sed 's/.*= *"\(.*\)".*/\1/' | sed 's/#.*$//' | xargs)
SOLVER_ADDR=$(grep -A5 "\[solver\]" "$SOLVER_CONFIG" | grep "^address" | grep -v "^#" | head -1 | sed 's/.*= *"\(.*\)".*/\1/' | sed 's/#.*$//' | xargs)

# Check which connected chains are configured by parsing [[connected_chain]] sections
HAS_EVM=false
HAS_SVM=false
HAS_MVM=false
CONNECTED_TYPES=""

# Extract types from [[connected_chain]] sections
for CONNECTED_TYPE in $(grep -A1 "\[\[connected_chain\]\]" "$SOLVER_CONFIG" | grep "^type" | sed 's/.*= *"\(.*\)".*/\1/' | sed 's/#.*$//' | xargs); do
    case "$CONNECTED_TYPE" in
        evm)
            HAS_EVM=true
            CONNECTED_TYPES="${CONNECTED_TYPES:+$CONNECTED_TYPES,}evm"
            # Find the EVM section and extract values
            EVM_ESCROW_CONTRACT=$(awk '/\[\[connected_chain\]\]/{found=0} /type *= *"evm"/{found=1} found && /^escrow_contract_addr/{gsub(/.*= *"|".*/, ""); print; exit}' "$SOLVER_CONFIG")
            EVM_RPC=$(awk '/\[\[connected_chain\]\]/{found=0} /type *= *"evm"/{found=1} found && /^rpc_url/{gsub(/.*= *"|".*/, ""); print; exit}' "$SOLVER_CONFIG")
            ;;
        svm)
            HAS_SVM=true
            CONNECTED_TYPES="${CONNECTED_TYPES:+$CONNECTED_TYPES,}svm"
            SVM_PROGRAM_ID=$(awk '/\[\[connected_chain\]\]/{found=0} /type *= *"svm"/{found=1} found && /^escrow_program_id/{gsub(/.*= *"|".*/, ""); print; exit}' "$SOLVER_CONFIG")
            SVM_RPC=$(awk '/\[\[connected_chain\]\]/{found=0} /type *= *"svm"/{found=1} found && /^rpc_url/{gsub(/.*= *"|".*/, ""); print; exit}' "$SOLVER_CONFIG")
            ;;
        mvm)
            HAS_MVM=true
            CONNECTED_TYPES="${CONNECTED_TYPES:+$CONNECTED_TYPES,}mvm"
            MVM_MODULE=$(awk '/\[\[connected_chain\]\]/{found=0} /type *= *"mvm"/{found=1} found && /^module_addr/{gsub(/.*= *"|".*/, ""); print; exit}' "$SOLVER_CONFIG")
            MVM_RPC=$(awk '/\[\[connected_chain\]\]/{found=0} /type *= *"mvm"/{found=1} found && /^rpc_url/{gsub(/.*= *"|".*/, ""); print; exit}' "$SOLVER_CONFIG")
            ;;
    esac
done

# Check at least one connected chain is configured
if [ -z "$CONNECTED_TYPES" ]; then
    echo "❌ ERROR: No [[connected_chain]] configured in $SOLVER_CONFIG"
    echo ""
    echo "   Add at least one [[connected_chain]] section with type = \"evm\", \"svm\", or \"mvm\""
    exit 1
fi

# Check for API key placeholders in RPC URLs
if [[ "$HUB_RPC" == *"ALCHEMY_API_KEY"* ]] || ($HAS_EVM && [[ "$EVM_RPC" == *"ALCHEMY_API_KEY"* ]]); then
    echo "❌ WARNING: RPC URLs contain API key placeholders (ALCHEMY_API_KEY)"
    echo "   The solver service does not substitute placeholders - use full URLs in config"
    echo "   Or use the public RPC URLs from testnet-assets.toml"
    echo ""
fi

echo " Configuration:"
echo "   Config file: $SOLVER_CONFIG"
echo "   Keys file:   $TESTNET_KEYS_FILE"
echo "   Connected:   $CONNECTED_TYPES"
echo ""
echo "   Verifier:"
echo "     URL:              $VERIFIER_URL"
echo ""
echo "   Hub Chain:"
echo "     RPC:              $HUB_RPC"
echo "     Module Address:   $HUB_MODULE"
echo ""
if $HAS_EVM; then
    echo "   Connected Chain (EVM):"
    echo "     Escrow Contract:  $EVM_ESCROW_CONTRACT"
    echo "     RPC:              $EVM_RPC"
    echo ""
fi
if $HAS_SVM; then
    echo "   Connected Chain (SVM):"
    echo "     Program ID:       $SVM_PROGRAM_ID"
    echo "     RPC:              $SVM_RPC"
    echo ""
fi
if $HAS_MVM; then
    echo "   Connected Chain (MVM):"
    echo "     Module Address:   $MVM_MODULE"
    echo "     RPC:              $MVM_RPC"
    echo ""
fi
echo "   Solver:"
echo "     Profile:          $SOLVER_PROFILE"
echo "     Address:          $SOLVER_ADDR"
echo ""

# Check verifier is reachable
echo "   Checking verifier health..."
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" --max-time 5 "$VERIFIER_URL/health" 2>/dev/null || echo "000")

if [ "$HTTP_CODE" = "200" ]; then
    echo "   ✅ Verifier is healthy"
else
    echo "   ️  Verifier not responding at $VERIFIER_URL (HTTP $HTTP_CODE)"
    echo ""
    echo "   Make sure verifier is running first:"
    echo "   ./testing-infra/testnet/run-verifier-local.sh"
    echo ""
    echo "   Quick check: curl $VERIFIER_URL/health"
    echo ""
    read -p "   Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo ""

# Check Movement CLI profile exists
if [ -n "$SOLVER_PROFILE" ]; then
    echo "   Checking Movement CLI profile '$SOLVER_PROFILE'..."
    if movement config show-profiles --profile "$SOLVER_PROFILE" &>/dev/null; then
        echo "   ✅ Profile exists"
    else
        echo "   ️  Profile '$SOLVER_PROFILE' not found"
        echo ""
        echo "   Create it with:"
        echo "   movement init --profile $SOLVER_PROFILE \\"
        echo "     --network custom \\"
        echo "     --rest-url https://testnet.movementnetwork.xyz/v1 \\"
        echo "     --private-key \"\$MOVEMENT_SOLVER_PRIVATE_KEY\" \\"
        echo "     --skip-faucet --assume-yes"
        echo ""
        echo "   Note: MOVEMENT_SOLVER_PRIVATE_KEY should be set in .env.testnet"
        echo ""
        read -p "   Continue anyway? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi
fi

echo ""

cd "$PROJECT_ROOT"

# Export environment variables for solver (needed for nix develop subprocess)
export BASE_SOLVER_PRIVATE_KEY
export SOLANA_SOLVER_PRIVATE_KEY
# Export solver addresses for auto-registration
# The solver expects SOLVER_EVM_ADDR for registration - use BASE_SOLVER_ADDR if SOLVER_EVM_ADDR is not set
export BASE_SOLVER_ADDR
if [ -z "$SOLVER_EVM_ADDR" ] && [ -n "$BASE_SOLVER_ADDR" ]; then
    SOLVER_EVM_ADDR="$BASE_SOLVER_ADDR"
fi
export SOLVER_EVM_ADDR
# Export Movement solver private key for registration (solver reads from env var first, then profile)
if [ -n "$MOVEMENT_SOLVER_PRIVATE_KEY" ]; then
    export MOVEMENT_SOLVER_PRIVATE_KEY
fi

# Convert SOLANA_SOLVER_ADDR (base58) to hex for solver registration
# The solver expects SOLVER_SVM_ADDR as 0x-prefixed 32-byte hex
if [ -n "$SOLANA_SOLVER_ADDR" ]; then
    SOLVER_SVM_ADDR=$(node -e "const bs58 = require('bs58'); console.log('0x' + Buffer.from(bs58.decode('$SOLANA_SOLVER_ADDR')).toString('hex'))")
    export SOLVER_SVM_ADDR
fi

# Export HUB_RPC_URL for hash calculation
export HUB_RPC_URL="$HUB_RPC"

# Prepare environment variables for nix develop
# Use debug logging for tracker and hub client to see intent detection
ENV_VARS="SOLVER_CONFIG_PATH='$SOLVER_CONFIG' RUST_LOG=info,solver::service::tracker=debug,solver::chains::hub=debug HUB_RPC_URL='$HUB_RPC'"
if [ -n "$BASE_SOLVER_PRIVATE_KEY" ]; then
    ENV_VARS="$ENV_VARS BASE_SOLVER_PRIVATE_KEY='$BASE_SOLVER_PRIVATE_KEY'"
fi
if [ -n "$BASE_SOLVER_ADDR" ]; then
    ENV_VARS="$ENV_VARS BASE_SOLVER_ADDR='$BASE_SOLVER_ADDR'"
fi
if [ -n "$SOLVER_EVM_ADDR" ]; then
    ENV_VARS="$ENV_VARS SOLVER_EVM_ADDR='$SOLVER_EVM_ADDR'"
fi
if [ -n "$MOVEMENT_SOLVER_PRIVATE_KEY" ]; then
    ENV_VARS="$ENV_VARS MOVEMENT_SOLVER_PRIVATE_KEY='$MOVEMENT_SOLVER_PRIVATE_KEY'"
fi
if [ -n "$SOLANA_SOLVER_PRIVATE_KEY" ]; then
    ENV_VARS="$ENV_VARS SOLANA_SOLVER_PRIVATE_KEY='$SOLANA_SOLVER_PRIVATE_KEY'"
fi
if [ -n "$SOLVER_SVM_ADDR" ]; then
    ENV_VARS="$ENV_VARS SOLVER_SVM_ADDR='$SOLVER_SVM_ADDR'"
fi

# Check if --release flag is passed
if [ "$1" = "--release" ]; then
    echo " Building release binary..."
    nix develop "$PROJECT_ROOT/nix" --command bash -c "cargo build --release --manifest-path solver/Cargo.toml"
    echo ""
    echo " Starting solver (release mode)..."
    echo "   Press Ctrl+C to stop"
    echo ""
    eval "$ENV_VARS ./solver/target/release/solver"
else
    echo " Starting solver (debug mode)..."
    echo "   Press Ctrl+C to stop"
    echo "   (Use --release for faster performance)"
    echo ""
    nix develop "$PROJECT_ROOT/nix" --command bash -c "$ENV_VARS cargo run --manifest-path solver/Cargo.toml --bin solver"
fi

