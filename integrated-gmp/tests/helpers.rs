//! Shared test helpers for unit tests
//!
//! This module provides helper functions used by unit tests.
//!
//! The module is organized into several categories:
//! - **Constants**: Dummy addresses, IDs, and other test values
//! - **Configuration Builders**: Functions to create test configurations (MVM, EVM, SVM)

#![allow(dead_code)]

use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::SigningKey;
use rand::{Rng, RngCore};
use integrated_gmp::config::{
    ApiConfig, ChainConfig, Config, EvmChainConfig, SvmChainConfig, IntegratedGmpConfig,
};

// ============================================================================
// CONSTANTS
// ============================================================================

// --------------------------------- IDs ----------------------------------

/// Dummy intent ID (64 hex characters, valid hex format)
pub const DUMMY_INTENT_ID: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000001";

// -------------------------------- USERS ---------------------------------

/// Dummy solver address on hub chain (Move VM format, 32 bytes)
pub const DUMMY_SOLVER_ADDR_HUB: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000007";

/// Dummy solver address on connected chain (Move VM format, 32 bytes)
pub const DUMMY_SOLVER_ADDR_MVMCON: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000008";

/// Dummy solver address (EVM format, 20 bytes)
pub const DUMMY_SOLVER_ADDR_EVM: &str = "0x0000000000000000000000000000000000000009";

/// Dummy integrated-gmp EVM public key hash (keccak256 hash of ECDSA public key, last 20 bytes; on-chain approver address)
pub const DUMMY_APPROVER_EVM_PUBKEY_HASH: &str = "0x000000000000000000000000000000000000000c";

// ------------------------- TOKENS AND CONTRACTS -------------------------

/// Dummy escrow contract address (EVM format, 20 bytes)
pub const DUMMY_ESCROW_CONTRACT_ADDR_EVM: &str = "0x0000000000000000000000000000000000000010";

// -------------------------------- OTHER ---------------------------------

/// Dummy transaction hash (64 hex characters)
pub const DUMMY_TX_HASH: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000012";

/// Dummy escrow program id (valid base58 pubkey string).
/// Base58 is required for Solana program ids; this is not a 0x hex value.
pub const DUMMY_SVM_ESCROW_PROGRAM_ID: &str = "11111111111111111111111111111111";

/// Dummy timestamp for solver registration (arbitrary test value)
pub const DUMMY_REGISTERED_AT: u64 = 1234567890;

/// Dummy public key bytes used in solver registry responses
pub const DUMMY_PUBLIC_KEY: [u8; 4] = [1, 2, 3, 4];

/// Dummy solver registry address
pub const DUMMY_SOLVER_REGISTRY_ADDR: &str = "0x1";

/// Test MVM chain ID (Movement mainnet)
pub const TEST_MVM_CHAIN_ID: u32 = 30817;

/// Test SVM chain ID (Solana)
pub const TEST_SVM_CHAIN_ID: u32 = 30168;

// ============================================================================
// CONFIGURATION BUILDERS
// ============================================================================

/// Build a valid in-memory test configuration with a fresh Ed25519 keypair.
/// Keys are encoded using standard Base64 and set as environment variables.
/// The config references these env vars via private_key_env/public_key_env.
pub fn build_test_config_with_mvm() -> Config {
    let mut rng = rand::thread_rng();
    let mut sk_bytes = [0u8; 32];
    rng.fill_bytes(&mut sk_bytes);
    let signing_key = SigningKey::from_bytes(&sk_bytes);
    let verifying_key = signing_key.verifying_key();

    let private_key_b64 = general_purpose::STANDARD.encode(signing_key.to_bytes());
    let public_key_b64 = general_purpose::STANDARD.encode(verifying_key.to_bytes());

    // Use unique env var names per invocation to avoid parallel test conflicts
    let unique_id: u64 = rng.gen();
    let private_key_env_name = format!("TEST_APPROVER_PRIVATE_KEY_{}", unique_id);
    let public_key_env_name = format!("TEST_APPROVER_PUBLIC_KEY_{}", unique_id);

    // Set environment variables for the keys (CryptoService reads from env vars)
    std::env::set_var(&private_key_env_name, &private_key_b64);
    std::env::set_var(&public_key_env_name, &public_key_b64);

    Config {
        hub_chain: ChainConfig {
            name: "hub".to_string(),
            rpc_url: "http://127.0.0.1:18080".to_string(),
            chain_id: 1,
            intent_module_addr: "0x1".to_string(),
            escrow_module_addr: None,
        },
        connected_chain_mvm: Some(ChainConfig {
            name: "connected".to_string(),
            rpc_url: "http://127.0.0.1:18082".to_string(),
            chain_id: 2,
            intent_module_addr: "0x2".to_string(),
            escrow_module_addr: Some("0x2".to_string()),
        }),
        integrated_gmp: IntegratedGmpConfig {
            private_key_env: private_key_env_name,
            public_key_env: public_key_env_name,
            polling_interval_ms: 1000,
            validation_timeout_ms: 1000,
        },
        api: ApiConfig {
            host: "127.0.0.1".to_string(),
            port: 3999,
            cors_origins: vec![],
        },
        connected_chain_evm: None,
        connected_chain_svm: None,
    }
}

/// Build a test configuration with EVM chain configuration.
/// Extends build_test_config_with_mvm() to include a populated connected_chain_evm field.
pub fn build_test_config_with_evm() -> Config {
    let mut config = build_test_config_with_mvm();
    config.connected_chain_evm = Some(EvmChainConfig {
        name: "Connected EVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8545".to_string(),
        escrow_contract_addr: DUMMY_ESCROW_CONTRACT_ADDR_EVM.to_string(),
        chain_id: 31337,
        approver_evm_pubkey_hash: DUMMY_APPROVER_EVM_PUBKEY_HASH.to_string(),
        gmp_endpoint_addr: None,
        outflow_validator_addr: None,
    });
    config
}

/// Build a test configuration with SVM chain configuration.
/// Extends build_test_config_with_mvm() to include a populated connected_chain_svm field.
pub fn build_test_config_with_svm() -> Config {
    let mut config = build_test_config_with_mvm();
    config.connected_chain_svm = Some(SvmChainConfig {
        name: "Connected SVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8899".to_string(),
        chain_id: 901,
        escrow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
        outflow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
        gmp_endpoint_program_id: Some(DUMMY_SVM_ESCROW_PROGRAM_ID.to_string()),
    });
    config
}
