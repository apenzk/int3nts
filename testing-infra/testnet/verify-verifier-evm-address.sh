#!/bin/bash

# Verify Verifier EVM Address Script (Testnet)
# 
# Verifies that VERIFIER_EVM_PUBKEY_HASH in this directory's .env.testnet matches
# the EVM address derived from VERIFIER_PRIVATE_KEY, and optionally checks
# that the on-chain IntentEscrow contract has the correct verifier address.
#
# Usage: ./verify-verifier-evm-address.sh
#
# Checks:
#   1. Computes EVM address from VERIFIER_PRIVATE_KEY
#   2. Compares to VERIFIER_EVM_PUBKEY_HASH in this directory's .env.testnet
#   3. Queries on-chain IntentEscrow contract's verifier() function (if config available)
#   4. Compares on-chain address to computed address
#
# This ensures the config is correct and the deployed contract matches.

# Get the script directory and project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"
export PROJECT_ROOT

# Use .env.testnet from script directory
ENV_FILE="$SCRIPT_DIR/.env.testnet"

# Source env file if it exists
if [ -f "$ENV_FILE" ]; then
    source "$ENV_FILE"
else
    echo "❌ ERROR: Environment file not found: $ENV_FILE"
    exit 1
fi

# Check if VERIFIER_EVM_PUBKEY_HASH is set
if [ -z "$VERIFIER_EVM_PUBKEY_HASH" ]; then
    echo "❌ ERROR: VERIFIER_EVM_PUBKEY_HASH not set in $ENV_FILE"
    exit 1
fi

# Check if VERIFIER_PRIVATE_KEY is set
if [ -z "$VERIFIER_PRIVATE_KEY" ]; then
    echo "❌ ERROR: VERIFIER_PRIVATE_KEY not set in $ENV_FILE"
    exit 1
fi

echo " Verifying Verifier EVM Address"
echo "=================================="
echo ""
echo "   Config file: $ENV_FILE"
echo "   Expected:    $VERIFIER_EVM_PUBKEY_HASH"
echo ""

# Compute EVM address from private key
echo "   Computing EVM address from VERIFIER_PRIVATE_KEY..."
COMPUTED_ADDR=$(cd "$PROJECT_ROOT/verifier" && \
    VERIFIER_CONFIG_PATH=config/verifier_testnet.toml \
    nix develop "$PROJECT_ROOT/nix" -c bash -c "cargo run --bin get_verifier_eth_address --quiet 2>&1" | grep -E '^0x[a-fA-F0-9]{40}$' | head -1)

if [ -z "$COMPUTED_ADDR" ]; then
    echo "❌ ERROR: Failed to compute EVM address from private key"
    echo "   Make sure VERIFIER_PRIVATE_KEY and VERIFIER_PUBLIC_KEY are set correctly"
    exit 1
fi

echo "   Computed:    $COMPUTED_ADDR"
echo ""

# Normalize addresses for comparison (lowercase)
EXPECTED_NORM=$(echo "$VERIFIER_EVM_PUBKEY_HASH" | tr '[:upper:]' '[:lower:]')
COMPUTED_NORM=$(echo "$COMPUTED_ADDR" | tr '[:upper:]' '[:lower:]')

# Compare env file vs computed
ENV_MATCH=false
if [ "$EXPECTED_NORM" = "$COMPUTED_NORM" ]; then
    ENV_MATCH=true
    echo "✅ Config file matches computed address"
else
    echo "❌ MISMATCH: Config file address does not match computed!"
    echo ""
    echo "   Expected: $VERIFIER_EVM_PUBKEY_HASH"
    echo "   Computed: $COMPUTED_ADDR"
    echo ""
    echo "   Action: Update VERIFIER_EVM_PUBKEY_HASH in $ENV_FILE to:"
    echo "   VERIFIER_EVM_PUBKEY_HASH=$COMPUTED_ADDR"
fi

# Check on-chain contract (if config is available)
ONCHAIN_MATCH=false
VERIFIER_CONFIG="$PROJECT_ROOT/verifier/config/verifier_testnet.toml"
if [ -f "$VERIFIER_CONFIG" ]; then
    # Extract escrow contract address and RPC URL from config
    ESCROW_ADDR=$(grep -A5 "\[connected_chain_evm\]" "$VERIFIER_CONFIG" | grep "escrow_contract_addr" | sed 's/.*= *"\(.*\)".*/\1/' | tr -d '"' | head -1)
    RPC_URL=$(grep -A5 "\[connected_chain_evm\]" "$VERIFIER_CONFIG" | grep "rpc_url" | sed 's/.*= *"\(.*\)".*/\1/' | tr -d '"' | head -1)
    
    if [ -n "$ESCROW_ADDR" ] && [ -n "$RPC_URL" ]; then
        echo ""
        echo "   Checking on-chain contract..."
        echo "   Contract: $ESCROW_ADDR"
        echo "   RPC: $RPC_URL"
        
        # Query verifier() function (public variable getter)
        # Function selector: keccak256("verifier()")[0:4] = 0x2b7ac3f3
        VERIFIER_SELECTOR="0x2b7ac3f3"
        RPC_RESPONSE=$(curl -s --max-time 10 -X POST "$RPC_URL" \
            -H "Content-Type: application/json" \
            -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_call\",\"params\":[{\"to\":\"$ESCROW_ADDR\",\"data\":\"$VERIFIER_SELECTOR\"},\"latest\"],\"id\":1}" 2>&1)
        
        ONCHAIN_VERIFIER=$(echo "$RPC_RESPONSE" | jq -r '.result // empty' 2>/dev/null)
        RPC_ERROR=$(echo "$RPC_RESPONSE" | jq -r '.error.message // empty' 2>/dev/null)
        
        if [ -n "$RPC_ERROR" ]; then
            echo "️  RPC error: $RPC_ERROR"
        elif [ -n "$ONCHAIN_VERIFIER" ] && [ "$ONCHAIN_VERIFIER" != "null" ] && [ "$ONCHAIN_VERIFIER" != "" ] && [ "${#ONCHAIN_VERIFIER}" -ge 42 ]; then
            # Extract address from result (last 40 hex chars = 20 bytes = address)
            # Result is 0x + 64 hex chars (32 bytes), we want last 40 chars (20 bytes)
            ONCHAIN_ADDR="0x${ONCHAIN_VERIFIER: -40}"
            ONCHAIN_NORM=$(echo "$ONCHAIN_ADDR" | tr '[:upper:]' '[:lower:]')
            
            echo "   On-chain:  $ONCHAIN_ADDR"
            
            if [ "$ONCHAIN_NORM" = "$COMPUTED_NORM" ]; then
                ONCHAIN_MATCH=true
                echo "✅ On-chain contract matches computed address"
            else
                echo "❌ MISMATCH: On-chain contract has wrong verifier address!"
                echo ""
                echo "   On-chain:  $ONCHAIN_ADDR"
                echo "   Computed:  $COMPUTED_ADDR"
                echo ""
                echo "   Action: Redeploy IntentEscrow contract with correct verifier address"
            fi
        else
            echo "️  Could not query on-chain contract"
            if [ -n "$ONCHAIN_VERIFIER" ]; then
                echo "   Response: $ONCHAIN_VERIFIER"
            fi
            echo "   (Contract may not be deployed, RPC unavailable, or function selector incorrect)"
        fi
    fi
fi

# Final summary
echo ""
if [ "$ENV_MATCH" = true ] && [ "$ONCHAIN_MATCH" = true ]; then
    echo "✅ SUCCESS: All checks passed!"
    echo "   - Config file matches computed address"
    echo "   - On-chain contract matches computed address"
    exit 0
elif [ "$ENV_MATCH" = true ]; then
    echo "️  WARNING: Config file is correct, but on-chain contract may need redeployment"
    exit 0
else
    echo "❌ FAILED: Config file does not match computed address"
    exit 1
fi

