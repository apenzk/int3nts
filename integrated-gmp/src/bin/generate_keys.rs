//! Ed25519 Key Generation Utility
//!
//! This binary generates a new Ed25519 key pair for the integrated-gmp service
//! and derives the relay address on each supported chain.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --bin generate_keys
//! ```
//!
//! ## Output
//!
//! - Private key (base64 encoded) - for signing operations
//! - Public key (base64 encoded) - for signature verification
//! - MVM address (hex) - Move address for on-chain relay authorization
//! - EVM address (hex) - Ethereum address derived from ECDSA key
//! - SVM address (base58) - Solana address from Ed25519 public key
//!
//! Copy the output values to your `.env.testnet` file.

use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::SigningKey;
use k256::ecdsa::SigningKey as EcdsaSigningKey;
use rand::Rng;
use sha3::{Digest, Keccak256, Sha3_256};

fn main() {
    // Generate a new Ed25519 key pair
    let mut rng = rand::rngs::OsRng;
    let mut secret_key_bytes = [0u8; 32];
    rng.fill(&mut secret_key_bytes);
    let signing_key = SigningKey::from_bytes(&secret_key_bytes);
    let verifying_key = signing_key.verifying_key();

    // Encode keys as base64
    let private_key_b64 = general_purpose::STANDARD.encode(signing_key.as_bytes());
    let public_key_b64 = general_purpose::STANDARD.encode(verifying_key.as_bytes());

    // Derive MVM address: sha3_256(public_key || 0x00)
    let mut hasher = Sha3_256::new();
    hasher.update(verifying_key.as_bytes());
    hasher.update(&[0x00u8]); // Ed25519 single-key scheme identifier
    let hash = hasher.finalize();
    let mvm_address = format!("0x{}", hex::encode(hash));

    // Derive EVM address: keccak256(ecdsa_uncompressed_pubkey)[12:32]
    let ecdsa_signing_key = EcdsaSigningKey::from_bytes(&secret_key_bytes.into())
        .expect("Failed to create ECDSA signing key");
    let ecdsa_verifying_key = ecdsa_signing_key.verifying_key();
    let public_key_point = ecdsa_verifying_key.to_encoded_point(false);
    let public_key_bytes = public_key_point.as_bytes();
    // Skip the 0x04 prefix, hash the 64 bytes of x || y
    let mut keccak = Keccak256::new();
    keccak.update(&public_key_bytes[1..]);
    let keccak_hash = keccak.finalize();
    let evm_address = format!("0x{}", hex::encode(&keccak_hash[12..32]));

    // Derive SVM address: base58-encode of the Ed25519 public key
    let svm_address = bs58::encode(verifying_key.as_bytes()).into_string();

    println!("Generated Ed25519 Key Pair:");
    println!();
    println!("INTEGRATED_GMP_PRIVATE_KEY={}", private_key_b64);
    println!("INTEGRATED_GMP_PUBLIC_KEY={}", public_key_b64);
    println!("INTEGRATED_GMP_MVM_ADDR={}", mvm_address);
    println!("INTEGRATED_GMP_EVM_PUBKEY_HASH={}", evm_address);
    println!("INTEGRATED_GMP_SVM_ADDR={}", svm_address);
    println!();
    println!("Update these files with the values above:");
    println!("  - testing-infra/testnet/.env.testnet                       (all values)");
    println!("  - integrated-gmp/config/integrated-gmp_testnet.toml        (private_key, public_key, approver_evm_pubkey_hash)");
}
