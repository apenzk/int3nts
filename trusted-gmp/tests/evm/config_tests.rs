//! Unit tests for EVM configuration management
//!
//! These tests verify EVM chain configuration loading, parsing, and defaults
//! without requiring external services.

use trusted_gmp::config::Config;

#[path = "../mod.rs"]
mod test_helpers;
use test_helpers::{build_test_config_with_evm, DUMMY_ESCROW_CONTRACT_ADDR_EVM, DUMMY_APPROVER_EVM_PUBKEY_HASH};

/// 1. Test: EVM Chain Config Structure
/// Verifies that EvmChainConfig structure has all required fields.
/// Why: Missing config fields would cause runtime failures when connecting to EVM chains.
#[test]
fn test_evm_chain_config_structure() {
    use trusted_gmp::config::EvmChainConfig;

    let evm_config = EvmChainConfig {
        name: "Connected EVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8545".to_string(),
        escrow_contract_addr: DUMMY_ESCROW_CONTRACT_ADDR_EVM.to_string(),
        chain_id: 31337,
        approver_evm_pubkey_hash: DUMMY_APPROVER_EVM_PUBKEY_HASH.to_string(),
        gmp_endpoint_addr: None,
        outflow_validator_addr: None,
    };

    assert_eq!(evm_config.name, "Connected EVM Chain");
    assert_eq!(evm_config.rpc_url, "http://127.0.0.1:8545");
    assert_eq!(
        evm_config.escrow_contract_addr,
        DUMMY_ESCROW_CONTRACT_ADDR_EVM
    );
    assert_eq!(evm_config.chain_id, 31337);
    assert_eq!(
        evm_config.approver_evm_pubkey_hash,
        DUMMY_APPROVER_EVM_PUBKEY_HASH
    );
}

/// 2. Test: Connected Chain EVM with Values
/// Verifies that connected_chain_evm can be set to Some(EvmChainConfig) with actual values.
/// Why: The EVM chain config must be settable for multi-chain deployments.
#[test]
fn test_connected_chain_evm_with_values() {
    use trusted_gmp::config::EvmChainConfig;
    let mut config = Config::default();

    config.connected_chain_evm = Some(EvmChainConfig {
        name: "Connected EVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8545".to_string(),
        escrow_contract_addr: DUMMY_ESCROW_CONTRACT_ADDR_EVM.to_string(),
        chain_id: 31337,
        approver_evm_pubkey_hash: DUMMY_APPROVER_EVM_PUBKEY_HASH.to_string(),
        gmp_endpoint_addr: None,
        outflow_validator_addr: None,
    });

    assert!(config.connected_chain_evm.is_some());
    let evm_config = config.connected_chain_evm.as_ref().unwrap();
    assert_eq!(evm_config.name, "Connected EVM Chain");
    assert_eq!(evm_config.rpc_url, "http://127.0.0.1:8545");
    assert_eq!(
        evm_config.escrow_contract_addr,
        DUMMY_ESCROW_CONTRACT_ADDR_EVM
    );
    assert_eq!(evm_config.chain_id, 31337);
    assert_eq!(
        evm_config.approver_evm_pubkey_hash,
        DUMMY_APPROVER_EVM_PUBKEY_HASH
    );
}

/// 3. Test: EVM Config Serialization
/// Verifies that EVM config can be serialized to and deserialized from TOML.
/// Why: Config persistence requires correct serialization round-tripping.
#[test]
fn test_evm_config_serialization() {
    let config = build_test_config_with_evm();

    // Serialize to TOML
    let toml = toml::to_string(&config).expect("Should serialize to TOML");

    // Deserialize back
    let deserialized: Config = toml::from_str(&toml).expect("Should deserialize from TOML");

    // Verify EVM config fields
    assert!(deserialized.connected_chain_evm.is_some());
    let evm_config = deserialized.connected_chain_evm.as_ref().unwrap();
    assert_eq!(evm_config.name, "Connected EVM Chain");
    assert_eq!(evm_config.rpc_url, "http://127.0.0.1:8545");
    assert_eq!(
        evm_config.escrow_contract_addr,
        DUMMY_ESCROW_CONTRACT_ADDR_EVM
    );
    assert_eq!(evm_config.chain_id, 31337);
    assert_eq!(
        evm_config.approver_evm_pubkey_hash,
        DUMMY_APPROVER_EVM_PUBKEY_HASH
    );
}

/// 4. Test: EVM Chain Config with All Fields
/// Verifies that EVM chain config has all fields populated correctly.
/// Why: Incomplete config would cause failures during EVM chain operations.
#[test]
fn test_evm_chain_config_with_all_fields() {
    let config = build_test_config_with_evm();

    assert!(
        config.connected_chain_evm.is_some(),
        "EVM chain should be configured"
    );

    let evm_config = config.connected_chain_evm.as_ref().unwrap();
    assert!(!evm_config.name.is_empty(), "Name should be set");
    assert!(!evm_config.rpc_url.is_empty(), "RPC URL should be set");
    assert!(
        !evm_config.escrow_contract_addr.is_empty(),
        "Escrow contract address should be set"
    );
    assert!(evm_config.chain_id > 0, "Chain ID should be set");
    assert!(
        !evm_config.approver_evm_pubkey_hash.is_empty(),
        "Approver address should be set"
    );

    // Verify specific values from build_test_config_with_evm
    assert_eq!(evm_config.name, "Connected EVM Chain");
    assert_eq!(evm_config.rpc_url, "http://127.0.0.1:8545");
    assert_eq!(
        evm_config.escrow_contract_addr,
        DUMMY_ESCROW_CONTRACT_ADDR_EVM
    );
    assert_eq!(evm_config.chain_id, 31337);
    assert_eq!(
        evm_config.approver_evm_pubkey_hash,
        DUMMY_APPROVER_EVM_PUBKEY_HASH
    );
}

/// 5. Test: EVM Config Loading
/// Verifies that config with EVM chain can be loaded and cloned.
/// Why: Config must be loadable and clonable for service initialization.
#[test]
fn test_evm_config_loading() {
    let config = build_test_config_with_evm();

    // Verify config structure is valid
    assert!(config.connected_chain_evm.is_some());

    // Verify all required fields are present
    let evm_config = config.connected_chain_evm.as_ref().unwrap();
    assert!(!evm_config.name.is_empty());
    assert!(!evm_config.rpc_url.is_empty());
    assert!(!evm_config.escrow_contract_addr.is_empty());
    assert!(evm_config.chain_id > 0);
    assert!(!evm_config.approver_evm_pubkey_hash.is_empty());

    // Verify config can be cloned (tests structure validity)
    let cloned_config = config.clone();
    assert!(cloned_config.connected_chain_evm.is_some());
}
