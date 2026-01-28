//! Unit tests for configuration management
//!
//! These tests verify configuration loading, parsing, and defaults
//! without requiring external services.

use coordinator::config::{AcceptanceConfig, ChainConfig, Config, EvmChainConfig, SvmChainConfig, TokenPairConfig};
#[path = "mod.rs"]
mod test_helpers;
use test_helpers::{DUMMY_ESCROW_CONTRACT_ADDR_EVM, DUMMY_SVM_ESCROW_PROGRAM_ID, DUMMY_TOKEN_ADDR_FANTOM};

/// Test that default configuration creates valid structure
/// Why: Verify default config is valid and doesn't panic
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

/// Test that connected_chain_mvm can be set to Some(ChainConfig)
/// Why: Verify connected_chain_mvm accepts actual values when configured
#[test]
fn test_connected_chain_mvm_with_values() {
    use coordinator::config::ChainConfig;
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

/// What is tested: AcceptanceConfig parses pairs list without ratios
/// Why: Coordinator should only store pairs and fetch ratios live from solver
#[test]
fn test_acceptance_pairs_deserialize() {
    let toml = format!(
        r#"
solver_url = "http://127.0.0.1:4444"
pairs = [
  {{ source_chain_id = 250, source_token = "{}", target_chain_id = 84532, target_token = "0x036CbD53842c5426634e7929541eC2318f3dCF7e" }},
  {{ source_chain_id = 250, source_token = "{}", target_chain_id = 901, target_token = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU" }}
]
"#,
        DUMMY_TOKEN_ADDR_FANTOM, DUMMY_TOKEN_ADDR_FANTOM
    );

    let acceptance: AcceptanceConfig = toml::from_str(&toml).expect("Should deserialize acceptance config");
    assert_eq!(acceptance.solver_url, "http://127.0.0.1:4444");
    assert_eq!(acceptance.pairs.len(), 2);
}

/// What is tested: Config::validate() accepts base58 SVM mints in pairs
/// Why: SVM tokens must be base58, not hex
#[test]
fn test_config_validate_acceptance_svm_base58() {
    let mut config = Config::default();
    config.hub_chain.chain_id = 250;
    config.connected_chain_svm = Some(SvmChainConfig {
        name: "SVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8899".to_string(),
        chain_id: 901,
        escrow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
    });
    config.acceptance = Some(AcceptanceConfig {
        solver_url: "http://127.0.0.1:4444".to_string(),
        pairs: vec![TokenPairConfig {
            source_chain_id: 250,
            source_token: DUMMY_TOKEN_ADDR_FANTOM.to_string(),
            target_chain_id: 901,
            target_token: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
        }],
    });

    let result = config.validate();
    assert!(result.is_ok(), "Should accept base58 SVM mints");
}

/// What is tested: Config::validate() accepts multiple connected chains
/// Why: Ensure MVM, EVM, and SVM can all be configured at once
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

    });

    config.connected_chain_svm = Some(SvmChainConfig {
        name: "SVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8899".to_string(),
        chain_id: 901,
        escrow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
    });

    let result = config.validate();
    assert!(result.is_ok(), "Should accept multiple connected chains");
}

/// Test that config can be serialized and deserialized
/// Why: Verify TOML round-trip works correctly
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
// CONFIG VALIDATION TESTS
// ============================================================================

/// Test that config.validate() returns error when hub and MVM chains have same chain ID
/// Why: Verify configuration validation catches duplicate chain IDs at load time
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

/// Test that config.validate() returns error when hub and EVM chains have same chain ID
/// Why: Verify configuration validation catches duplicate chain IDs at load time
#[test]
fn test_config_validate_hub_evm_duplicate_chain_id() {
    let mut config = Config::default();
    config.hub_chain.chain_id = 100;
    config.connected_chain_evm = Some(EvmChainConfig {
        name: "EVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8545".to_string(),
        escrow_contract_addr: DUMMY_ESCROW_CONTRACT_ADDR_EVM.to_string(),
        chain_id: 100, // Same as hub

    });

    let result = config.validate();
    assert!(result.is_err(), "Should reject duplicate chain IDs");
    assert!(result.unwrap_err().to_string().contains("Hub chain and connected EVM chain have the same chain ID"), "Error message should mention hub and EVM duplicate");
}

/// Test that config.validate() returns error when MVM and EVM chains have same chain ID
/// Why: Verify configuration validation catches duplicate chain IDs at load time
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

    });

    let result = config.validate();
    assert!(result.is_err(), "Should reject duplicate chain IDs");
    assert!(result.unwrap_err().to_string().contains("Connected MVM chain and connected EVM chain have the same chain ID"), "Error message should mention MVM and EVM duplicate");
}

/// Test that config.validate() succeeds when all chain IDs are unique
/// Why: Verify configuration validation passes for valid configurations
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

    });

    let result = config.validate();
    assert!(result.is_ok(), "Should accept unique chain IDs");
}
