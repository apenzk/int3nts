#!/bin/bash

# Deploy SVM IntentEscrow to Solana Devnet
# Reads keys from .env.testnet and deploys the program

set -e

# Get the script directory and project root
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"
export PROJECT_ROOT

echo " Deploying IntentEscrow to Solana Devnet"
echo "=========================================="
echo ""

# Load .env.testnet
TESTNET_KEYS_FILE="$SCRIPT_DIR/.env.testnet"

if [ ! -f "$TESTNET_KEYS_FILE" ]; then
    echo "❌ ERROR: .env.testnet not found at $TESTNET_KEYS_FILE"
    echo "   Create it from env.testnet.example in this directory"
    exit 1
fi

# Source the keys file
source "$TESTNET_KEYS_FILE"

# Check required variables
if [ -z "$SOLANA_DEPLOYER_PRIVATE_KEY" ]; then
    echo "❌ ERROR: SOLANA_DEPLOYER_PRIVATE_KEY not set in .env.testnet"
    exit 1
fi

if [ -z "$SOLANA_DEPLOYER_ADDR" ]; then
    echo "❌ ERROR: SOLANA_DEPLOYER_ADDR not set in .env.testnet"
    exit 1
fi

# Solana devnet RPC
SOLANA_RPC_URL="${SOLANA_RPC_URL:-https://api.devnet.solana.com}"

echo " Configuration:"
echo "   Deployer Address: $SOLANA_DEPLOYER_ADDR"
echo "   Network: Solana Devnet"
echo "   RPC URL: $SOLANA_RPC_URL"
echo ""

# Change to intent-frameworks/svm directory
cd "$PROJECT_ROOT/intent-frameworks/svm"

# Create temporary keypair file from base58 private key
# Solana CLI can read base58 private keys directly if we use solana-keygen recover
TEMP_KEYPAIR_DIR=$(mktemp -d)
DEPLOYER_KEYPAIR="$TEMP_KEYPAIR_DIR/deployer.json"

echo " Converting deployer private key to keypair file..."

# Use Node.js to convert base58 private key to JSON keypair format
# Node.js is available in nix develop ./nix shell
node -e "
const bs58 = require('bs58');
const keyBytes = bs58.decode('$SOLANA_DEPLOYER_PRIVATE_KEY');
console.log(JSON.stringify(Array.from(keyBytes)));
" > "$DEPLOYER_KEYPAIR" 2>/dev/null

# If bs58 module not available, try with inline base58 decoder
if [ ! -s "$DEPLOYER_KEYPAIR" ]; then
    node -e "
// Inline base58 decoder (no external dependencies)
const ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
function b58decode(str) {
    const bytes = [];
    for (let i = 0; i < str.length; i++) {
        const idx = ALPHABET.indexOf(str[i]);
        if (idx < 0) throw new Error('Invalid base58 character');
        let carry = idx;
        for (let j = 0; j < bytes.length; j++) {
            carry += bytes[j] * 58;
            bytes[j] = carry & 0xff;
            carry >>= 8;
        }
        while (carry > 0) {
            bytes.push(carry & 0xff);
            carry >>= 8;
        }
    }
    // Add leading zeros
    for (let i = 0; i < str.length && str[i] === '1'; i++) {
        bytes.push(0);
    }
    return bytes.reverse();
}
console.log(JSON.stringify(b58decode('$SOLANA_DEPLOYER_PRIVATE_KEY')));
" > "$DEPLOYER_KEYPAIR"
fi

if [ ! -s "$DEPLOYER_KEYPAIR" ]; then
    echo "❌ ERROR: Failed to convert private key"
    echo "   Node.js is required (available in nix develop ./nix shell)"
    rm -rf "$TEMP_KEYPAIR_DIR"
    exit 1
fi

echo "✅ Deployer keypair created"

# Verify the keypair address matches
DERIVED_ADDR=$(solana-keygen pubkey "$DEPLOYER_KEYPAIR" 2>/dev/null || echo "")
if [ "$DERIVED_ADDR" != "$SOLANA_DEPLOYER_ADDR" ]; then
    echo "❌ ERROR: Derived address does not match SOLANA_DEPLOYER_ADDR"
    echo "   Derived:  $DERIVED_ADDR"
    echo "   Expected: $SOLANA_DEPLOYER_ADDR"
    rm -rf "$TEMP_KEYPAIR_DIR"
    exit 1
fi
echo "✅ Address verified: $DERIVED_ADDR"
echo ""

# Check deployer balance
echo " Checking deployer balance..."
BALANCE=$(solana balance "$SOLANA_DEPLOYER_ADDR" --url "$SOLANA_RPC_URL" 2>/dev/null | awk '{print $1}' || echo "0")
echo "   Balance: $BALANCE SOL"

# Warn if balance is low (need ~2-3 SOL for deployment)
if (( $(echo "$BALANCE < 2" | bc -l) )); then
    echo "⚠️  WARNING: Balance may be insufficient for deployment"
    echo "   Recommended: at least 2-3 SOL"
    echo "   Get devnet SOL from: https://faucet.solana.com/"
fi
echo ""

# Build the program
echo " Building program..."
./scripts/build.sh

PROGRAM_SO="$PROJECT_ROOT/intent-frameworks/svm/target/deploy/intent_escrow.so"
PROGRAM_KEYPAIR_PATH="$PROJECT_ROOT/intent-frameworks/svm/target/deploy/intent_escrow-keypair.json"

if [ ! -f "$PROGRAM_SO" ]; then
    echo "❌ ERROR: Program binary not found at $PROGRAM_SO"
    rm -rf "$TEMP_KEYPAIR_DIR"
    exit 1
fi

# Create program keypair if it doesn't exist
if [ ! -f "$PROGRAM_KEYPAIR_PATH" ]; then
    echo " Creating program keypair..."
    solana-keygen new -o "$PROGRAM_KEYPAIR_PATH" --no-bip39-passphrase --force
fi

PROGRAM_ID=$(solana-keygen pubkey "$PROGRAM_KEYPAIR_PATH")
echo " Program ID will be: $PROGRAM_ID"
echo ""

# Deploy the program
echo " Deploying program to Solana Devnet..."
solana program deploy \
    --url "$SOLANA_RPC_URL" \
    --keypair "$DEPLOYER_KEYPAIR" \
    "$PROGRAM_SO" \
    --program-id "$PROGRAM_KEYPAIR_PATH"

DEPLOY_EXIT_CODE=$?

# Clean up temporary keypair
rm -rf "$TEMP_KEYPAIR_DIR"

if [ $DEPLOY_EXIT_CODE -ne 0 ]; then
    echo "❌ Deployment failed with exit code $DEPLOY_EXIT_CODE"
    exit 1
fi

echo ""
echo " Deployment Complete!"
echo "======================"
echo ""
echo " Deployed program ID: $PROGRAM_ID"
echo ""

# =============================================================================
# Initialize the program with verifier public key
# =============================================================================

echo ""
echo " Initializing program with verifier..."
echo ""

# Check for verifier public key
if [ -z "$VERIFIER_PUBLIC_KEY" ]; then
    echo "⚠️  WARNING: VERIFIER_PUBLIC_KEY not set in .env.testnet"
    echo "   Skipping initialization - you'll need to run it manually later"
    echo ""
else
    # Convert verifier public key from base64 to base58 (Solana format)
    VERIFIER_PUBKEY_BASE58=$(node -e "
// Inline base58 encoder
const ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
function b58encode(bytes) {
    const digits = [0];
    for (let i = 0; i < bytes.length; i++) {
        let carry = bytes[i];
        for (let j = 0; j < digits.length; j++) {
            carry += digits[j] << 8;
            digits[j] = carry % 58;
            carry = (carry / 58) | 0;
        }
        while (carry > 0) {
            digits.push(carry % 58);
            carry = (carry / 58) | 0;
        }
    }
    // Add leading zeros
    for (let i = 0; i < bytes.length && bytes[i] === 0; i++) {
        digits.push(0);
    }
    return digits.reverse().map(d => ALPHABET[d]).join('');
}
const base64Key = '$VERIFIER_PUBLIC_KEY';
const keyBytes = Buffer.from(base64Key, 'base64');
console.log(b58encode(Array.from(keyBytes)));
")

    if [ -z "$VERIFIER_PUBKEY_BASE58" ]; then
        echo "❌ ERROR: Failed to convert verifier public key to base58"
        echo "   Skipping initialization - you'll need to run it manually"
    else
        echo " Verifier public key (base58): $VERIFIER_PUBKEY_BASE58"
        
        # Recreate deployer keypair for initialization
        TEMP_KEYPAIR_DIR=$(mktemp -d)
        DEPLOYER_KEYPAIR="$TEMP_KEYPAIR_DIR/deployer.json"
        
        node -e "
const ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
function b58decode(str) {
    const bytes = [];
    for (let i = 0; i < str.length; i++) {
        const idx = ALPHABET.indexOf(str[i]);
        if (idx < 0) throw new Error('Invalid base58 character');
        let carry = idx;
        for (let j = 0; j < bytes.length; j++) {
            carry += bytes[j] * 58;
            bytes[j] = carry & 0xff;
            carry >>= 8;
        }
        while (carry > 0) {
            bytes.push(carry & 0xff);
            carry >>= 8;
        }
    }
    for (let i = 0; i < str.length && str[i] === '1'; i++) {
        bytes.push(0);
    }
    return bytes.reverse();
}
console.log(JSON.stringify(b58decode('$SOLANA_DEPLOYER_PRIVATE_KEY')));
" > "$DEPLOYER_KEYPAIR"
        
        # Build CLI if needed
        CLI_BIN="$PROJECT_ROOT/intent-frameworks/svm/target/debug/intent_escrow_cli"
        if [ ! -x "$CLI_BIN" ]; then
            echo " Building CLI tool..."
            cd "$PROJECT_ROOT/intent-frameworks/svm"
            cargo build --bin intent_escrow_cli 2>/dev/null
        fi
        
        if [ -x "$CLI_BIN" ]; then
            echo " Running initialize command..."
            "$CLI_BIN" initialize \
                --program-id "$PROGRAM_ID" \
                --payer "$DEPLOYER_KEYPAIR" \
                --verifier "$VERIFIER_PUBKEY_BASE58" \
                --rpc "$SOLANA_RPC_URL" && {
                echo "✅ Program initialized with verifier"
            } || {
                INIT_EXIT=$?
                if [ $INIT_EXIT -eq 0 ]; then
                    echo "✅ Program initialized with verifier"
                else
                    echo "⚠️  Initialization returned exit code $INIT_EXIT"
                    echo "   This may be OK if the program was already initialized"
                fi
            }
        else
            echo "⚠️  CLI not built - skipping initialization"
            echo "   Run manually: ./intent-frameworks/svm/scripts/initialize.sh"
        fi
        
        rm -rf "$TEMP_KEYPAIR_DIR"
    fi
fi

echo ""
echo " Update the following:"
echo ""
echo "   1. .env.testnet"
echo "      SOLANA_PROGRAM_ID=$PROGRAM_ID"
echo ""
echo "   2. verifier/config/verifier_testnet.toml"
echo "      Add [connected_chain_svm] section with:"
echo "      escrow_program_id = \"$PROGRAM_ID\""
echo ""
echo "   3. solver/config/solver_testnet.toml"
echo "      Add SVM connected chain section with program ID"
echo ""
echo "   4. Run ./testing-infra/testnet/check-testnet-preparedness.sh to verify"
echo ""
