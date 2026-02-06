//! Unit tests for configuration module

#[path = "helpers.rs"]
mod test_helpers;
use test_helpers::{
    create_default_connected_mvm_chain_config, create_default_solver_config, create_default_token_pair,
    DUMMY_ESCROW_CONTRACT_ADDR_EVM, DUMMY_SVM_ESCROW_PROGRAM_ID, DUMMY_TOKEN_ADDR_EVM, DUMMY_TOKEN_ADDR_MVMCON, DUMMY_TOKEN_ADDR_HUB,
};

use solver::config::{AcceptanceConfig, ConnectedChainConfig, EvmChainConfig, MvmChainConfig, SvmChainConfig, SolverConfig, TokenPairConfig};

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Create a minimal valid SolverConfig for testing
fn create_test_config() -> SolverConfig {
    SolverConfig {
        acceptance: AcceptanceConfig {
            token_pairs: vec![TokenPairConfig {
                source_chain_id: 1,
                source_token: DUMMY_TOKEN_ADDR_HUB.to_string(),
                target_chain_id: 2,
                target_token: DUMMY_TOKEN_ADDR_MVMCON.to_string(),
                ratio: 1.0,
            }],
        },
        ..create_default_solver_config()
    }
}

// ============================================================================
// VALIDATION TESTS
// ============================================================================

/// What is tested: SolverConfig::validate() accepts valid configuration
/// Why: Ensure valid configs pass validation
#[test]
fn test_config_validation_success() {
    let config = create_test_config();
    assert!(config.validate().is_ok());
}

/// What is tested: SolverConfig::validate() accepts multiple connected chains
/// Why: Ensure multiple connected chains can be configured at once
#[test]
fn test_config_validation_multiple_connected_chains() {
    let mut config = create_test_config();

    // Add an EVM connected chain alongside the default MVM chain
    config.connected_chain.push(ConnectedChainConfig::Evm(EvmChainConfig {
        name: "connected-evm".to_string(),
        rpc_url: "http://127.0.0.1:8545".to_string(),
        chain_id: 3,
        escrow_contract_addr: DUMMY_ESCROW_CONTRACT_ADDR_EVM.to_string(),
        private_key_env: "SOLVER_EVM_PRIVATE_KEY".to_string(),
        network_name: "localhost".to_string(),
        outflow_validator_addr: None,
        gmp_endpoint_addr: None,
    }));

    assert!(config.validate().is_ok());
    assert!(config.get_mvm_config().is_some());
    assert!(config.get_evm_config().is_some());
    assert!(config.get_svm_config().is_none());
}

/// What is tested: SolverConfig::validate() rejects duplicate chain IDs
/// Why: Ensure hub and connected chains have different chain IDs
#[test]
fn test_config_validation_duplicate_chain_ids() {
    let mut config = create_test_config();
    // Set connected chain to same ID as hub
    config.connected_chain = vec![
        ConnectedChainConfig::Mvm(MvmChainConfig {
            chain_id: 1, // Same as hub chain
            ..create_default_connected_mvm_chain_config()
        }),
    ];

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("same chain ID"));
}

/// What is tested: SolverConfig::validate() rejects unknown chain IDs in token pairs
/// Why: Ensure token pairs reference configured chain IDs
#[test]
fn test_config_validation_unknown_chain_id_in_token_pair() {
    let mut config = create_test_config();
    config.acceptance.token_pairs = vec![TokenPairConfig {
        source_chain_id: 999,
        source_token: DUMMY_TOKEN_ADDR_HUB.to_string(),
        target_chain_id: 2,
        target_token: DUMMY_TOKEN_ADDR_MVMCON.to_string(),
        ratio: 1.0,
    }];

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unknown source_chain_id"));
}

/// What is tested: SolverConfig::validate() rejects invalid-length hex SVM tokens
/// Why: SVM hex tokens must be 32 bytes (like Move addresses on hub chain)
#[test]
fn test_config_validation_rejects_svm_invalid_hex_length() {
    let mut config = create_test_config();
    config.connected_chain.push(ConnectedChainConfig::Svm(SvmChainConfig {
        name: "svm".to_string(),
        rpc_url: "http://127.0.0.1:8899".to_string(),
        chain_id: 901,
        escrow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
        private_key_env: "SOLANA_SOLVER_PRIVATE_KEY".to_string(),
        gmp_endpoint_program_id: None,
        outflow_validator_program_id: None,
    }));
    config.acceptance.token_pairs = vec![TokenPairConfig {
        source_chain_id: 1,
        source_token: DUMMY_TOKEN_ADDR_HUB.to_string(),
        target_chain_id: 901,
        target_token: "0xdeadbeef".to_string(), // Invalid: 4 bytes, not 32
        ratio: 1.0,
    }];

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("expected 32 bytes"));
}

/// What is tested: SolverConfig::validate() rejects invalid base58 SVM tokens
/// Why: Prevent malformed SVM mints in acceptance config
#[test]
fn test_config_validation_rejects_invalid_svm_base58_token() {
    let mut config = create_test_config();
    config.connected_chain.push(ConnectedChainConfig::Svm(SvmChainConfig {
        name: "svm".to_string(),
        rpc_url: "http://127.0.0.1:8899".to_string(),
        chain_id: 901,
        escrow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
        private_key_env: "SOLANA_SOLVER_PRIVATE_KEY".to_string(),
        gmp_endpoint_program_id: None,
        outflow_validator_program_id: None,
    }));
    config.acceptance.token_pairs = vec![TokenPairConfig {
        source_chain_id: 1,
        source_token: DUMMY_TOKEN_ADDR_HUB.to_string(),
        target_chain_id: 901,
        target_token: "not_base58".to_string(),
        ratio: 1.0,
    }];

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid base58 SVM mint"));
}

/// What is tested: SolverConfig::validate() rejects non-positive exchange rates
/// Why: Ensure exchange rates are positive
#[test]
fn test_config_validation_negative_exchange_rate() {
    let mut config = create_test_config();
    config.acceptance.token_pairs = vec![TokenPairConfig {
        source_chain_id: 1,
        source_token: DUMMY_TOKEN_ADDR_HUB.to_string(),
        target_chain_id: 2,
        target_token: DUMMY_TOKEN_ADDR_MVMCON.to_string(),
        ratio: -1.0,
    }];

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must be positive"));
}

/// What is tested: SolverConfig::validate() rejects zero exchange rate
/// Why: Ensure exchange rates are positive (not zero)
#[test]
fn test_config_validation_zero_exchange_rate() {
    let mut config = create_test_config();
    config.acceptance.token_pairs = vec![TokenPairConfig {
        source_chain_id: 1,
        source_token: DUMMY_TOKEN_ADDR_HUB.to_string(),
        target_chain_id: 2,
        target_token: DUMMY_TOKEN_ADDR_MVMCON.to_string(),
        ratio: 0.0,
    }];

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must be positive"));
}

// ============================================================================
// TOKEN PAIR CONVERSION TESTS
// ============================================================================

/// What is tested: SolverConfig::get_token_pairs() converts configs to TokenPair structs
/// Why: Ensure token pairs are correctly converted into TokenPair structs
#[test]
fn test_get_token_pairs_success() {
    let config = create_test_config();
    let pairs = config.get_token_pairs().unwrap();

    assert_eq!(pairs.len(), 1);
    
    let expected_pair = create_default_token_pair();
    
    assert!(pairs.contains_key(&expected_pair));
    assert_eq!(pairs[&expected_pair], 1.0);
}

/// What is tested: SolverConfig::get_token_pairs() handles multiple token pairs
/// Why: Ensure multiple token pairs are correctly converted
#[test]
fn test_get_token_pairs_multiple() {
    let mut config = create_test_config();
    config.acceptance.token_pairs.push(TokenPairConfig {
        source_chain_id: 2,
        source_token: DUMMY_TOKEN_ADDR_MVMCON.to_string(),
        target_chain_id: 1,
        target_token: DUMMY_TOKEN_ADDR_HUB.to_string(),
        ratio: 0.5,
    });

    let pairs = config.get_token_pairs().unwrap();
    assert_eq!(pairs.len(), 2);
}

/// What is tested: SolverConfig::get_token_pairs() handles token addresses
/// Why: Ensure all tokens use their actual addresses (hex format)
#[test]
fn test_get_token_pairs_token_address() {
    let mut config = create_test_config();
    config.connected_chain.push(ConnectedChainConfig::Evm(EvmChainConfig {
        name: "connected-evm".to_string(),
        rpc_url: "http://127.0.0.1:8545".to_string(),
        chain_id: 3,
        escrow_contract_addr: DUMMY_ESCROW_CONTRACT_ADDR_EVM.to_string(),
        private_key_env: "SOLVER_EVM_PRIVATE_KEY".to_string(),
        network_name: "localhost".to_string(),
        outflow_validator_addr: None,
        gmp_endpoint_addr: None,
    }));
    config.acceptance.token_pairs = vec![TokenPairConfig {
        source_chain_id: 1,
        source_token: DUMMY_TOKEN_ADDR_HUB.to_string(),
        target_chain_id: 3,
        target_token: DUMMY_TOKEN_ADDR_EVM.to_string(),
        ratio: 0.5,
    }];

    let pairs = config.get_token_pairs().unwrap();
    assert_eq!(pairs.len(), 1);
    
    use solver::TokenPair;
    let expected_pair = TokenPair {
        desired_chain_id: 3,
        desired_token: DUMMY_TOKEN_ADDR_EVM.to_string(),
        ..create_default_token_pair()
    };
    
    assert!(pairs.contains_key(&expected_pair));
    assert_eq!(pairs[&expected_pair], 0.5);
}

// ============================================================================
// TOML SERIALIZATION/DESERIALIZATION TESTS
// ============================================================================

/// What is tested: SolverConfig can be serialized to and deserialized from TOML
/// Why: Ensure config structs work with TOML format
#[test]
fn test_config_toml_roundtrip() {
    let config = create_test_config();
    
    // Serialize to TOML
    let toml_str = toml::to_string(&config).unwrap();
    
    // Deserialize from TOML
    let deserialized: SolverConfig = toml::from_str(&toml_str).unwrap();
    
    // Verify key fields match
    assert_eq!(deserialized.service.coordinator_url, config.service.coordinator_url);
    assert_eq!(deserialized.hub_chain.chain_id, config.hub_chain.chain_id);
    assert_eq!(deserialized.acceptance.token_pairs.len(), config.acceptance.token_pairs.len());
}

/// What is tested: MvmChainConfig can deserialize from TOML
/// Why: Ensure MVM chain config is correctly parsed from TOML
#[test]
fn test_connected_chain_mvm_deserialization() {
    let toml_str = r#"
name = "connected-chain"
rpc_url = "http://127.0.0.1:8082/v1"
chain_id = 2
module_addr = "0x2"
profile = "connected-profile"
"#;

    let config: MvmChainConfig = toml::from_str(toml_str).unwrap();
    
    assert_eq!(config.chain_id, 2);
    assert_eq!(config.name, "connected-chain");
    assert_eq!(config.module_addr, "0x2");
    assert_eq!(config.profile, "connected-profile");
}

/// What is tested: EvmChainConfig can deserialize from TOML
/// Why: Ensure EVM chain config is correctly parsed from TOML
#[test]
fn test_connected_chain_evm_deserialization() {
    let toml_str = format!(r#"
name = "Connected EVM Chain"
rpc_url = "https://sepolia.base.org"
chain_id = 84532
escrow_contract_addr = "{}"
private_key_env = "BASE_SOLVER_PRIVATE_KEY"
"#, DUMMY_ESCROW_CONTRACT_ADDR_EVM);

    let config: EvmChainConfig = toml::from_str(&toml_str).unwrap();
    
    assert_eq!(config.chain_id, 84532);
    assert_eq!(config.name, "Connected EVM Chain");
    assert_eq!(config.escrow_contract_addr, DUMMY_ESCROW_CONTRACT_ADDR_EVM);
    assert_eq!(config.private_key_env, "BASE_SOLVER_PRIVATE_KEY");
}

/// What is tested: SvmChainConfig can deserialize from TOML
/// Why: Ensure SVM chain config is correctly parsed from TOML
#[test]
fn test_connected_chain_svm_deserialization() {
    let toml_str = format!(
        r#"
name = "Connected SVM Chain"
rpc_url = "http://127.0.0.1:8899"
chain_id = 100
escrow_program_id = "{}"
private_key_env = "SOLANA_SOLVER_PRIVATE_KEY"
"#,
        DUMMY_SVM_ESCROW_PROGRAM_ID
    );

    let config: SvmChainConfig = toml::from_str(&toml_str).unwrap();

    assert_eq!(config.chain_id, 100);
    assert_eq!(config.name, "Connected SVM Chain");
    assert_eq!(config.escrow_program_id, DUMMY_SVM_ESCROW_PROGRAM_ID);
    assert_eq!(config.private_key_env, "SOLANA_SOLVER_PRIVATE_KEY");
}

// ============================================================================
// FILE LOADING TESTS
// ============================================================================

/// What is tested: SolverConfig::load() loads configuration from TOML file
/// Why: Ensure config can be loaded from actual TOML file
#[test]
fn test_config_load_from_file() {
    use std::fs;
    
    // Create a temporary config file
    let test_config_dir = ".tmp/test_config";
    let test_config_file = format!("{}/solver.toml", test_config_dir);
    
    // Ensure directory exists
    fs::create_dir_all(test_config_dir).unwrap();
    
    // Write test config
    let toml_content = format!(
        r#"
[service]
coordinator_url = "http://127.0.0.1:3333"
polling_interval_ms = 2000

[hub_chain]
name = "hub-chain"
rpc_url = "http://127.0.0.1:8080/v1"
chain_id = 1
module_addr = "0x1"
profile = "hub-profile"

[[connected_chain]]
type = "mvm"
name = "connected-chain"
rpc_url = "http://127.0.0.1:8082/v1"
chain_id = 2
module_addr = "0x2"
profile = "connected-profile"

[acceptance]
[[acceptance.tokenpair]]
source_chain_id = 1
source_token = "{}"
target_chain_id = 2
target_token = "{}"
ratio = 1.0

[solver]
profile = "hub-profile"
address = "0xccc"
"#,
        DUMMY_TOKEN_ADDR_HUB, DUMMY_TOKEN_ADDR_MVMCON
    );
    
    fs::write(&test_config_file, toml_content).unwrap();
    
    // Set environment variable to point to test config
    std::env::set_var("SOLVER_CONFIG_PATH", &test_config_file);
    
    // Load config
    let config = SolverConfig::load().unwrap();
    
    // Verify loaded values
    assert_eq!(config.service.coordinator_url, "http://127.0.0.1:3333");
    assert_eq!(config.hub_chain.chain_id, 1);
    assert_eq!(config.acceptance.token_pairs.len(), 1);
    
    // Cleanup
    std::env::remove_var("SOLVER_CONFIG_PATH");
    fs::remove_file(&test_config_file).unwrap();
    fs::remove_dir(test_config_dir).unwrap();
}

/// What is tested: SolverConfig::load() returns error when file doesn't exist
/// Why: Ensure proper error message when config file is missing
#[test]
fn test_config_load_file_not_found() {
    // Use load_from_path directly with explicit non-existent path
    // to avoid parallel test interference with environment variables
    let result = SolverConfig::load_from_path(Some("/tmp/nonexistent/solver.toml"));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

