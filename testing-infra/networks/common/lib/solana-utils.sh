#!/bin/bash
# Shared Solana utilities for deployment and configuration scripts.
# Provides keypair conversion and CLI build helpers.
# Requires: Node.js, solana-keygen

# Convert a base58 Solana private key to a JSON keypair file.
# Sets DEPLOYER_KEYPAIR to the path. Creates temp dir tracked by TEMP_KEYPAIR_DIR.
# Usage: solana_create_keypair <base58_private_key> [expected_address]
# If expected_address is provided, verifies the derived address matches.
solana_create_keypair() {
    local private_key="$1"
    local expected_addr="${2:-}"

    TEMP_KEYPAIR_DIR=$(mktemp -d)
    DEPLOYER_KEYPAIR="$TEMP_KEYPAIR_DIR/deployer.json"

    echo " Converting deployer private key to keypair file..."

    # Try bs58 module first, fall back to inline decoder
    node -e "
const bs58 = require('bs58');
const keyBytes = bs58.decode('$private_key');
console.log(JSON.stringify(Array.from(keyBytes)));
" > "$DEPLOYER_KEYPAIR" 2>/dev/null

    if [ ! -s "$DEPLOYER_KEYPAIR" ]; then
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
console.log(JSON.stringify(b58decode('$private_key')));
" > "$DEPLOYER_KEYPAIR"
    fi

    if [ ! -s "$DEPLOYER_KEYPAIR" ]; then
        echo "ERROR: Failed to convert private key"
        echo "   Node.js is required (available in nix develop ./nix shell)"
        rm -rf "$TEMP_KEYPAIR_DIR"
        exit 1
    fi

    # Verify keypair
    local derived_addr
    derived_addr=$(solana-keygen pubkey "$DEPLOYER_KEYPAIR" 2>&1) || {
        echo "ERROR: solana-keygen pubkey failed: $derived_addr"
        echo "   Make sure you are running inside nix develop ./nix"
        rm -rf "$TEMP_KEYPAIR_DIR"
        exit 1
    }

    if [ -n "$expected_addr" ] && [ "$derived_addr" != "$expected_addr" ]; then
        echo "ERROR: Derived address does not match expected"
        echo "   Derived:  $derived_addr"
        echo "   Expected: $expected_addr"
        rm -rf "$TEMP_KEYPAIR_DIR"
        exit 1
    fi

    echo "   Deployer verified: $derived_addr"
    echo ""
}

# Convert a base64 public key to base58 (Solana format).
# Usage: base64_to_base58 <base64_key>  => prints base58 string
base64_to_base58() {
    local base64_key="$1"

    node -e "
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
    for (let i = 0; i < bytes.length && bytes[i] === 0; i++) {
        digits.push(0);
    }
    return digits.reverse().map(d => ALPHABET[d]).join('');
}
const keyBytes = Buffer.from('$base64_key', 'base64');
console.log(b58encode(Array.from(keyBytes)));
"
}

# Build the intent_escrow_cli binary.
# Sets CLI_BIN to the path. Exits on failure.
# Usage: build_solana_cli
build_solana_cli() {
    CLI_BIN="$PROJECT_ROOT/intent-frameworks/svm/target/debug/intent_escrow_cli"
    echo " Building CLI tool..."
    cd "$PROJECT_ROOT/intent-frameworks/svm"
    cargo build --bin intent_escrow_cli 2>/dev/null

    if [ ! -x "$CLI_BIN" ]; then
        echo "ERROR: CLI tool not built at $CLI_BIN"
        exit 1
    fi
}
