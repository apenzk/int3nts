//! Unit tests for configuration management
//!
//! These tests verify configuration loading, parsing, and defaults
//! without requiring external services.

use trusted_gmp::config::{ChainConfig, Config, EvmChainConfig, SvmChainConfig};
use trusted_gmp::monitor::ChainType;
use trusted_gmp::validator::{get_chain_type_from_chain_id, normalize_address};
#[path = "mod.rs"]
mod test_helpers;
use test_helpers::{DUMMY_ESCROW_CONTRACT_ADDR_EVM, DUMMY_INTENT_ID_FULL, DUMMY_SVM_ESCROW_PROGRAM_ID, DUMMY_APPROVER_EVM_PUBKEY_HASH};

/// 1. Test: Default Config Creation
/// Verifies that default configuration creates a valid structure.
/// Why: Default config must be valid and not panic.
#[test]
fn test_default_config_creation() {
    let config = Config::default();

    assert_eq!(config.hub_chain.name, "Hub Chain");
    assert_eq!(config.hub_chain.rpc_url, "http://127.0.0.1:8080");
    assert!(
        config.connected_chain_mvm.is_none(),
        "Default config should have no connected Move VM chain"
    );
    assert!(
        config.connected_chain_evm.is_none(),
        "Default config should have no connected EVM chain"
    );
}

/// 2. Test: Connected Chain MVM With Values
/// Verifies that connected_chain_mvm can be set to Some(ChainConfig).
/// Why: connected_chain_mvm must accept actual values when configured.
#[test]
fn test_connected_chain_mvm_with_values() {
    use trusted_gmp::config::ChainConfig;
    let mut config = Config::default();

    config.connected_chain_mvm = Some(ChainConfig {
        name: "Connected Move VM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8082".to_string(),
        chain_id: 2,
        intent_module_addr: "0x123".to_string(),
        escrow_module_addr: Some("0x123".to_string()),
    });

    assert_eq!(
        config.connected_chain_mvm.as_ref().unwrap().name,
        "Connected Move VM Chain"
    );
}

/// 3. Test: Config Validation Multiple Connected Chains
/// Verifies that Config::validate() accepts multiple connected chains.
/// Why: MVM, EVM, and SVM must all be configurable at once.
#[test]
fn test_config_validation_multiple_connected_chains() {
    let mut config = Config::default();

    config.connected_chain_mvm = Some(ChainConfig {
        name: "MVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8082".to_string(),
        chain_id: 2,
        intent_module_addr: "0x123".to_string(),
        escrow_module_addr: Some("0x123".to_string()),
    });

    config.connected_chain_evm = Some(EvmChainConfig {
        name: "EVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8545".to_string(),
        escrow_contract_addr: DUMMY_ESCROW_CONTRACT_ADDR_EVM.to_string(),
        chain_id: 31337,
        approver_evm_pubkey_hash: DUMMY_APPROVER_EVM_PUBKEY_HASH.to_string(),
        gmp_endpoint_addr: None,
        outflow_validator_addr: None,
    });

    config.connected_chain_svm = Some(SvmChainConfig {
        name: "SVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8899".to_string(),
        chain_id: 901,
        escrow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
        gmp_endpoint_program_id: Some(DUMMY_SVM_ESCROW_PROGRAM_ID.to_string()),
    });

    let result = config.validate();
    assert!(result.is_ok(), "Should accept multiple connected chains");
}

/// 4. Test: Config Serialization
/// Verifies that config can be serialized to TOML and deserialized back.
/// Why: TOML round-trip must preserve configuration values correctly.
#[test]
fn test_config_serialization() {
    let config = Config::default();

    // Serialize to TOML
    let toml = toml::to_string(&config).expect("Should serialize to TOML");

    // Deserialize back
    let deserialized: Config = toml::from_str(&toml).expect("Should deserialize from TOML");

    assert_eq!(config.hub_chain.name, deserialized.hub_chain.name);
    assert_eq!(config.hub_chain.rpc_url, deserialized.hub_chain.rpc_url);
}

// ============================================================================
// CHAIN TYPE UTILITIES TESTS
// ============================================================================

/// 5. Test: Get Chain Type From Chain ID EVM
/// Verifies that get_chain_type_from_chain_id returns Evm for an EVM chain ID.
/// Why: The function must correctly identify EVM chains from their chain ID.
#[test]
fn test_get_chain_type_from_chain_id_evm() {
    let mut config = Config::default();
    config.connected_chain_evm = Some(EvmChainConfig {
        name: "EVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8545".to_string(),
        escrow_contract_addr: DUMMY_ESCROW_CONTRACT_ADDR_EVM.to_string(),
        chain_id: 31337,
        approver_evm_pubkey_hash: DUMMY_APPROVER_EVM_PUBKEY_HASH.to_string(),
        gmp_endpoint_addr: None,
        outflow_validator_addr: None,
    });

    let result = get_chain_type_from_chain_id(31337, &config);
    assert!(result.is_ok(), "Should successfully identify EVM chain");
    assert_eq!(result.unwrap(), ChainType::Evm);
}

/// 6. Test: Get Chain Type From Chain ID MVM
/// Verifies that get_chain_type_from_chain_id returns Mvm for an MVM chain ID.
/// Why: The function must correctly identify MVM chains from their chain ID.
#[test]
fn test_get_chain_type_from_chain_id_mvm() {
    let mut config = Config::default();
    config.connected_chain_mvm = Some(ChainConfig {
        name: "MVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8082".to_string(),
        chain_id: 2,
        intent_module_addr: "0x123".to_string(),
        escrow_module_addr: Some("0x123".to_string()),
    });

    let result = get_chain_type_from_chain_id(2, &config);
    assert!(result.is_ok(), "Should successfully identify MVM chain");
    assert_eq!(result.unwrap(), ChainType::Mvm);
}

/// 7. Test: Get Chain Type From Chain ID Unknown
/// Verifies that get_chain_type_from_chain_id returns an error for an unknown chain ID.
/// Why: Chain IDs that do not match any configured chain must be rejected.
#[test]
fn test_get_chain_type_from_chain_id_unknown() {
    let config = Config::default();

    let result = get_chain_type_from_chain_id(999, &config);
    assert!(result.is_err(), "Should return error for unknown chain ID");
    assert!(result.unwrap_err().to_string().contains("does not match any configured connected chain"));
}

/// 8. Test: Get Chain Type From Chain ID Duplicate Chain ID Error
/// Verifies that get_chain_type_from_chain_id returns an error when EVM and MVM have the same chain ID.
/// Why: Invalid configurations with duplicate chain IDs must be rejected.
#[test]
fn test_get_chain_type_from_chain_id_duplicate_chain_id_error() {
    let mut config = Config::default();
    // Set both EVM and MVM to same chain_id (invalid configuration)
    config.connected_chain_evm = Some(EvmChainConfig {
        name: "EVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8545".to_string(),
        escrow_contract_addr: DUMMY_ESCROW_CONTRACT_ADDR_EVM.to_string(),
        chain_id: 100,
        approver_evm_pubkey_hash: DUMMY_APPROVER_EVM_PUBKEY_HASH.to_string(),
        gmp_endpoint_addr: None,
        outflow_validator_addr: None,
    });
    config.connected_chain_mvm = Some(ChainConfig {
        name: "MVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8082".to_string(),
        chain_id: 100,
        intent_module_addr: "0x123".to_string(),
        escrow_module_addr: Some("0x123".to_string()),
    });

    // Should return error for duplicate chain IDs
    let result = get_chain_type_from_chain_id(100, &config);
    assert!(result.is_err(), "Should reject duplicate chain IDs");
    assert!(result.unwrap_err().to_string().contains("same chain ID"), "Error message should mention duplicate chain ID");
}

// ============================================================================
// CONFIG VALIDATION TESTS
// ============================================================================

/// 9. Test: Config Validate Hub MVM Duplicate Chain ID
/// Verifies that config.validate() returns an error when hub and MVM chains have the same chain ID.
/// Why: Configuration validation must catch duplicate chain IDs between hub and MVM at load time.
#[test]
fn test_config_validate_hub_mvm_duplicate_chain_id() {
    let mut config = Config::default();
    config.hub_chain.chain_id = 100;
    config.connected_chain_mvm = Some(ChainConfig {
        name: "MVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8082".to_string(),
        chain_id: 100, // Same as hub
        intent_module_addr: "0x123".to_string(),
        escrow_module_addr: Some("0x123".to_string()),
    });

    let result = config.validate();
    assert!(result.is_err(), "Should reject duplicate chain IDs");
    assert!(result.unwrap_err().to_string().contains("Hub chain and connected MVM chain have the same chain ID"), "Error message should mention hub and MVM duplicate");
}

/// 10. Test: Config Validate Hub EVM Duplicate Chain ID
/// Verifies that config.validate() returns an error when hub and EVM chains have the same chain ID.
/// Why: Configuration validation must catch duplicate chain IDs between hub and EVM at load time.
#[test]
fn test_config_validate_hub_evm_duplicate_chain_id() {
    let mut config = Config::default();
    config.hub_chain.chain_id = 100;
    config.connected_chain_evm = Some(EvmChainConfig {
        name: "EVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8545".to_string(),
        escrow_contract_addr: DUMMY_ESCROW_CONTRACT_ADDR_EVM.to_string(),
        chain_id: 100, // Same as hub
        approver_evm_pubkey_hash: DUMMY_APPROVER_EVM_PUBKEY_HASH.to_string(),
        gmp_endpoint_addr: None,
        outflow_validator_addr: None,
    });

    let result = config.validate();
    assert!(result.is_err(), "Should reject duplicate chain IDs");
    assert!(result.unwrap_err().to_string().contains("Hub chain and connected EVM chain have the same chain ID"), "Error message should mention hub and EVM duplicate");
}

/// 11. Test: Config Validate MVM EVM Duplicate Chain ID
/// Verifies that config.validate() returns an error when MVM and EVM chains have the same chain ID.
/// Why: Configuration validation must catch duplicate chain IDs between MVM and EVM at load time.
#[test]
fn test_config_validate_mvm_evm_duplicate_chain_id() {
    let mut config = Config::default();
    config.connected_chain_mvm = Some(ChainConfig {
        name: "MVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8082".to_string(),
        chain_id: 100,
        intent_module_addr: "0x123".to_string(),
        escrow_module_addr: Some("0x123".to_string()),
    });
    config.connected_chain_evm = Some(EvmChainConfig {
        name: "EVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8545".to_string(),
        escrow_contract_addr: DUMMY_ESCROW_CONTRACT_ADDR_EVM.to_string(),
        chain_id: 100, // Same as MVM
        approver_evm_pubkey_hash: DUMMY_APPROVER_EVM_PUBKEY_HASH.to_string(),
        gmp_endpoint_addr: None,
        outflow_validator_addr: None,
    });

    let result = config.validate();
    assert!(result.is_err(), "Should reject duplicate chain IDs");
    assert!(result.unwrap_err().to_string().contains("Connected MVM chain and connected EVM chain have the same chain ID"), "Error message should mention MVM and EVM duplicate");
}

/// 12. Test: Config Validate Unique Chain IDs
/// Verifies that config.validate() succeeds when all chain IDs are unique.
/// Why: Configuration validation must pass for valid configurations with distinct chain IDs.
#[test]
fn test_config_validate_unique_chain_ids() {
    let mut config = Config::default();
    config.hub_chain.chain_id = 1;
    config.connected_chain_mvm = Some(ChainConfig {
        name: "MVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8082".to_string(),
        chain_id: 2, // Different from hub
        intent_module_addr: "0x123".to_string(),
        escrow_module_addr: Some("0x123".to_string()),
    });
    config.connected_chain_evm = Some(EvmChainConfig {
        name: "EVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8545".to_string(),
        escrow_contract_addr: DUMMY_ESCROW_CONTRACT_ADDR_EVM.to_string(),
        chain_id: 31337, // Different from hub and MVM
        approver_evm_pubkey_hash: DUMMY_APPROVER_EVM_PUBKEY_HASH.to_string(),
        gmp_endpoint_addr: None,
        outflow_validator_addr: None,
    });

    let result = config.validate();
    assert!(result.is_ok(), "Should accept unique chain IDs");
}

// ============================================================================
// ADDR NORMALIZATION TESTS
// ============================================================================

/// 13. Test: Normalize Address MVM Pads Short Address
/// Verifies that normalize_address pads Move VM addresses with leading zeros.
/// Why: Move VM addresses can be serialized without leading zeros (63 chars) and must be padded to 64.
#[test]
fn test_normalize_address_mvm_pads_short_address() {
    let address = "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"; // 63 chars
    let normalized = normalize_address(address, ChainType::Mvm);

    assert_eq!(
        normalized.len(),
        66,
        "Should be 0x + 64 hex chars = 66 total"
    );
    assert!(normalized.starts_with("0x"), "Should have 0x prefix");
    assert_eq!(&normalized[2..3], "0", "Should be padded with leading zero");
    assert_eq!(
        &normalized[3..],
        "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        "Rest should match"
    );
}

/// 14. Test: Normalize Address MVM Keeps Full Address
/// Verifies that normalize_address does not pad Move VM addresses that are already 64 chars.
/// Why: Addresses that are already the correct length must not be modified.
#[test]
fn test_normalize_address_mvm_keeps_full_address() {
    let address = DUMMY_INTENT_ID_FULL; // 64 chars
    let normalized = normalize_address(address, ChainType::Mvm);

    assert_eq!(normalized, address, "Should remain unchanged");
}

/// 15. Test: Normalize Address MVM Adds Prefix
/// Verifies that normalize_address adds the 0x prefix to Move VM addresses that lack it.
/// Why: Addresses may arrive without a prefix and must have 0x prepended.
#[test]
fn test_normalize_address_mvm_adds_prefix() {
    let address = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"; // 63 chars, no prefix
    let normalized = normalize_address(address, ChainType::Mvm);

    assert!(normalized.starts_with("0x"), "Should add 0x prefix");
    assert_eq!(
        normalized.len(),
        66,
        "Should be 0x + 64 hex chars = 66 total"
    );
    assert_eq!(&normalized[2..3], "0", "Should be padded with leading zero");
}

/// 16. Test: Normalize Address EVM Pads Short Address
/// Verifies that normalize_address pads short EVM addresses to 40 hex chars.
/// Why: EVM addresses must be padded to 40 hex chars (20 bytes).
#[test]
fn test_normalize_address_evm_pads_short_address() {
    let address = "0xccccccccccccccccccccccccccccccccccccccc"; // 39 chars
    let normalized = normalize_address(address, ChainType::Evm);

    assert_eq!(
        normalized.len(),
        42,
        "Should be 0x + 40 hex chars = 42 total"
    );
    assert!(normalized.starts_with("0x"), "Should have 0x prefix");
    assert_eq!(&normalized[2..3], "0", "Should be padded with leading zero");
}

/// 17. Test: Normalize Address EVM Keeps Full Address
/// Verifies that normalize_address does not pad EVM addresses that are already 40 hex chars.
/// Why: EVM addresses that are already the correct length must not be modified.
#[test]
fn test_normalize_address_evm_keeps_full_address() {
    let address = "0xdddddddddddddddddddddddddddddddddddddddd"; // 40 chars
    let normalized = normalize_address(address, ChainType::Evm);

    assert_eq!(normalized, address, "Should remain unchanged");
}
