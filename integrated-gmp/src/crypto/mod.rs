//! Cryptographic Operations Module
//!
//! This module handles cryptographic operations for the integrated-gmp relay,
//! including key management, EVM transaction signing, and address derivation
//! for all supported chain types (MVM, EVM, SVM).
//!
//! ## Security Requirements
//!
//! **CRITICAL**: All cryptographic operations must use secure random number generation
//! and proper key management practices. Private keys must never be exposed or logged.

use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::{SigningKey, VerifyingKey};
use hex;
use k256::ecdsa::{
    Signature as EcdsaSignature, SigningKey as EcdsaSigningKey, VerifyingKey as EcdsaVerifyingKey,
};
use sha3::{Digest, Keccak256, Sha3_256};
use tracing::info;

use crate::config::Config;

// ============================================================================
// CRYPTOGRAPHIC SERVICE IMPLEMENTATION
// ============================================================================

/// Cryptographic service for the integrated-gmp relay.
///
/// Manages Ed25519 keys (for MVM/SVM) and derived ECDSA keys (for EVM).
/// Used by the relay for signing delivery transactions and by utility binaries
/// for deriving relay addresses on each chain.
pub struct CryptoService {
    /// Ed25519 signing key (primary key, loaded from config)
    #[allow(dead_code)]
    signing_key: SigningKey,
    /// Ed25519 verifying key (derived from signing key)
    verifying_key: VerifyingKey,
    /// ECDSA signing key for EVM operations (secp256k1)
    /// Derived from Ed25519 private key by using same 32-byte seed
    ecdsa_signing_key: EcdsaSigningKey,
}

impl CryptoService {
    /// Creates a new cryptographic service from configuration.
    ///
    /// Loads the Ed25519 keypair from environment variables specified in config,
    /// verifies the public key matches, and derives the ECDSA key for EVM.
    pub fn new(config: &Config) -> Result<Self> {
        // Load private key from environment variable
        let private_key_b64 = config.integrated_gmp.get_private_key()?;
        let private_key_bytes = general_purpose::STANDARD.decode(&private_key_b64)?;

        if private_key_bytes.len() != 32 {
            return Err(anyhow::anyhow!(
                "Invalid private key length: expected 32 bytes, got {}",
                private_key_bytes.len()
            ));
        }

        let secret_key_bytes: [u8; 32] = private_key_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to convert private key to array"))?;

        let signing_key = SigningKey::from_bytes(&secret_key_bytes);
        let verifying_key = signing_key.verifying_key();

        // Verify public key matches environment variable
        let expected_public_key_b64 = config.integrated_gmp.get_public_key()?;
        let actual_public_key_b64 = general_purpose::STANDARD.encode(verifying_key.to_bytes());

        if actual_public_key_b64 != expected_public_key_b64 {
            return Err(anyhow::anyhow!(
                "Public key mismatch: environment variable '{}' has {}, but private key corresponds to {}",
                config.integrated_gmp.public_key_env,
                expected_public_key_b64,
                actual_public_key_b64
            ));
        }

        info!("Crypto service initialized with key pair from environment variables");

        // Derive ECDSA key from Ed25519 private key for EVM compatibility
        let ecdsa_secret_bytes: [u8; 32] = secret_key_bytes;
        let ecdsa_signing_key = EcdsaSigningKey::from_bytes(&ecdsa_secret_bytes.into())
            .map_err(|e| anyhow::anyhow!("Failed to create ECDSA signing key: {}", e))?;

        Ok(Self {
            signing_key,
            verifying_key,
            ecdsa_signing_key,
        })
    }

    /// Returns the Ed25519 public key as a base64 string.
    pub fn get_public_key(&self) -> String {
        general_purpose::STANDARD.encode(self.verifying_key.to_bytes())
    }

    /// Signs a raw EVM transaction hash with the ECDSA key.
    ///
    /// This does NOT apply the Ethereum signed message prefix — the caller is expected
    /// to pass a keccak256 hash of a RLP-encoded transaction.
    ///
    /// # Returns
    ///
    /// * `Ok((r, s, recovery_id))` — r and s are 32-byte big-endian, recovery_id is 0 or 1
    pub fn sign_evm_transaction_hash(
        &self,
        tx_hash: &[u8; 32],
    ) -> Result<([u8; 32], [u8; 32], u8)> {
        use k256::ecdsa::signature::hazmat::PrehashSigner;
        let signature: EcdsaSignature = self
            .ecdsa_signing_key
            .sign_prehash(tx_hash)
            .map_err(|e| anyhow::anyhow!("Failed to sign transaction hash: {}", e))?;

        let sig_bytes = signature.to_bytes();
        if sig_bytes.len() != 64 {
            return Err(anyhow::anyhow!(
                "Invalid signature length: expected 64 bytes, got {}",
                sig_bytes.len()
            ));
        }

        let mut r = [0u8; 32];
        let mut s = [0u8; 32];
        r.copy_from_slice(&sig_bytes[..32]);
        s.copy_from_slice(&sig_bytes[32..64]);

        // Calculate recovery ID by trying both 0 and 1
        let verifying_key = self.ecdsa_signing_key.verifying_key();
        let public_key_point = verifying_key.to_encoded_point(false);
        let public_key_bytes = public_key_point.as_bytes();

        let recovery_id_0 = k256::ecdsa::RecoveryId::try_from(0u8).unwrap();
        let recovery_id = if let Ok(recovered) =
            EcdsaVerifyingKey::recover_from_prehash(tx_hash, &signature, recovery_id_0)
        {
            let recovered_point = recovered.to_encoded_point(false);
            if recovered_point.as_bytes() == public_key_bytes {
                0u8
            } else {
                1u8
            }
        } else {
            1u8
        };

        Ok((r, s, recovery_id))
    }

    /// Derives the Ethereum address from the ECDSA public key.
    ///
    /// The Ethereum address is computed as:
    /// keccak256(uncompressed_public_key)[12:32] (last 20 bytes)
    pub fn get_ethereum_address(&self) -> Result<String> {
        let verifying_key = self.ecdsa_signing_key.verifying_key();
        let public_key_point = verifying_key.to_encoded_point(false); // Uncompressed format
        let public_key_bytes = public_key_point.as_bytes();

        // Remove the 0x04 prefix (uncompressed point indicator)
        // Uncompressed format: 0x04 || x (32 bytes) || y (32 bytes) = 65 bytes total
        if public_key_bytes.len() != 65 || public_key_bytes[0] != 0x04 {
            return Err(anyhow::anyhow!(
                "Invalid public key format: expected 65 bytes with 0x04 prefix"
            ));
        }

        // Hash the public key (without the 0x04 prefix)
        let mut hasher = Keccak256::new();
        hasher.update(&public_key_bytes[1..]);
        let hash = hasher.finalize();

        // Ethereum address is the last 20 bytes of the hash
        let address_bytes = &hash[12..32];
        let address_hex = format!("0x{}", hex::encode(address_bytes));

        Ok(address_hex)
    }

    /// Derives the Move VM (Aptos) address from the Ed25519 public key.
    ///
    /// The Move address is computed as:
    /// sha3_256(public_key || 0x00) for Ed25519 single-key accounts
    pub fn get_move_address(&self) -> Result<String> {
        let public_key_bytes = self.verifying_key.as_bytes();

        // Aptos Ed25519 address derivation: sha3_256(public_key || 0x00)
        let mut hasher = Sha3_256::new();
        hasher.update(public_key_bytes);
        hasher.update(&[0x00u8]); // Ed25519 single-key scheme identifier
        let hash = hasher.finalize();

        let address_hex = format!("0x{}", hex::encode(hash));

        Ok(address_hex)
    }

    /// Derives the Solana address from the Ed25519 public key.
    ///
    /// The Solana address is the base58 encoding of the Ed25519 public key bytes.
    pub fn get_solana_address(&self) -> String {
        bs58::encode(self.verifying_key.as_bytes()).into_string()
    }
}
