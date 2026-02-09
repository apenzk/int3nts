//! Shared test helpers for unit tests
//!
//! This module provides helper functions used by unit tests.
//!
//! The module is organized into several categories:
//! - **Configuration Builders**: Functions to create test configurations (MVM, EVM, with mock servers)
//! - **Default Event Creators**: Functions to create default test events (intents, escrows, fulfillments)
//! - **Default Transaction Creators**: Functions to create default test transactions (MVM)

use coordinator::config::{
    ApiConfig, ChainConfig, Config, CoordinatorConfig, EvmChainConfig, SvmChainConfig,
};
use coordinator::monitor::{ChainType, EscrowEvent, FulfillmentEvent, IntentEvent};
use coordinator::mvm_client::MvmTransaction;

// ============================================================================
// CONSTANTS
// ============================================================================

// --------------------------------- IDs ----------------------------------

/// Dummy intent ID (64 hex characters, valid hex format)
pub const DUMMY_INTENT_ID: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000001";

/// Dummy escrow ID (Move VM format, 64 hex characters)
pub const DUMMY_ESCROW_ID_MVM: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000002";

/// Dummy intent ID for normalization tests (64 hex chars, full format)
/// Used in svm_tests.rs, config_tests.rs, and signature_test.rs
#[allow(dead_code)]
pub const DUMMY_INTENT_ID_FULL: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000003";

// -------------------------------- USERS ---------------------------------

/// Dummy requester address on hub chain (Move VM format, 32 bytes)
pub const DUMMY_REQUESTER_ADDR_HUB: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000004";

/// Dummy requester address on connected chain (Move VM format, 32 bytes)
pub const DUMMY_REQUESTER_ADDR_MVMCON: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000005";

/// Dummy requester address (EVM format, 20 bytes)
pub const DUMMY_REQUESTER_ADDR_EVM: &str = "0x0000000000000000000000000000000000000006";

/// Dummy solver address on hub chain (Move VM format, 32 bytes)
pub const DUMMY_SOLVER_ADDR_HUB: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000007";

/// Dummy solver address on connected chain (Move VM format, 32 bytes)
pub const DUMMY_SOLVER_ADDR_MVMCON: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000008";

/// Dummy solver address (EVM format, 20 bytes)
pub const DUMMY_SOLVER_ADDR_EVM: &str = "0x0000000000000000000000000000000000000009";

/// Dummy requester address (SVM format, 32 bytes)
#[allow(dead_code)]
pub const DUMMY_REQUESTER_ADDR_SVM: &str =
    "0x000000000000000000000000000000000000000000000000000000000000000a";

/// Dummy solver address (SVM format, 32 bytes)
#[allow(dead_code)]
pub const DUMMY_SOLVER_ADDR_SVM: &str =
    "0x000000000000000000000000000000000000000000000000000000000000000b";

/// Dummy integrated-gmp EVM public key hash (keccak256 hash of ECDSA public key, last 20 bytes)
#[allow(dead_code)]
pub const DUMMY_INTEGRATED_GMP_EVM_PUBKEY_HASH: &str = "0x000000000000000000000000000000000000000c";

// ------------------------- TOKENS AND CONTRACTS -------------------------

/// Dummy intent address (Move VM format, 64 hex characters)
/// This represents the Move VM object address of an intent on the hub chain
#[allow(dead_code)]
pub const DUMMY_INTENT_ADDR_HUB: &str =
    "0x000000000000000000000000000000000000000000000000000000000000000d";

/// Dummy token address (EVM format, 20 bytes)
pub const DUMMY_TOKEN_ADDR_EVM: &str = "0x000000000000000000000000000000000000000e";

/// Dummy token address used in config tests (Move VM format, 64 hex characters)
#[allow(dead_code)]
pub const DUMMY_TOKEN_ADDR_FANTOM: &str =
    "0x000000000000000000000000000000000000000000000000000000000000000f";

/// Dummy escrow contract address (EVM format, 20 bytes)
#[allow(dead_code)]
pub const DUMMY_ESCROW_CONTRACT_ADDR_EVM: &str = "0x0000000000000000000000000000000000000010";

/// Dummy metadata object address (Move VM format, 32 bytes)
#[allow(dead_code)]
pub const DUMMY_METADATA_ADDR_MVM: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000011";

// -------------------------------- OTHER ---------------------------------

/// Dummy transaction hash (64 hex characters)
#[allow(dead_code)]
pub const DUMMY_TX_HASH: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000012";

/// Dummy escrow program id (valid base58 pubkey string).
/// Base58 is required for Solana program ids; this is not a 0x hex value.
#[allow(dead_code)]
pub const DUMMY_SVM_ESCROW_PROGRAM_ID: &str = "11111111111111111111111111111111";

/// Dummy timestamp for solver registration (arbitrary test value)
#[allow(dead_code)]
pub const DUMMY_REGISTERED_AT: u64 = 1234567890;

/// Dummy expiry timestamp (far future timestamp for tests)
#[allow(dead_code)]
pub const DUMMY_EXPIRY: u64 = 9999999999;

/// Dummy public key bytes used in solver registry responses
#[allow(dead_code)]
pub const DUMMY_PUBLIC_KEY: [u8; 4] = [1, 2, 3, 4];

/// Dummy solver registry address
#[allow(dead_code)]
pub const DUMMY_SOLVER_REGISTRY_ADDR: &str = "0x1";

// ============================================================================
// CONFIGURATION BUILDERS
// ============================================================================

/// Build a valid in-memory test configuration.
/// The coordinator doesn't use crypto keys — only timing parameters.
#[allow(dead_code)]
pub fn build_test_config_with_mvm() -> Config {
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
        coordinator: CoordinatorConfig {
            polling_interval_ms: 1000,
            validation_timeout_ms: 1000,
        },
        api: ApiConfig {
            host: "127.0.0.1".to_string(),
            port: 3999,
            cors_origins: vec![],
        },
        connected_chain_evm: None, // No connected EVM chain for unit tests
        connected_chain_svm: None, // No connected SVM chain for unit tests
        acceptance: None, // No acceptance criteria for unit tests
    }
}

/// Build a test configuration with EVM chain configuration.
/// Extends build_test_config_with_mvm() to include a populated connected_chain_evm field.
#[allow(dead_code)]
pub fn build_test_config_with_evm() -> Config {
    let mut config = build_test_config_with_mvm();
    config.connected_chain_evm = Some(EvmChainConfig {
        name: "Connected EVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8545".to_string(),
        escrow_contract_addr: DUMMY_ESCROW_CONTRACT_ADDR_EVM.to_string(),
        outflow_validator_contract_addr: "0x0000000000000000000000000000000000000010".to_string(),
        chain_id: 31337,
        event_block_range: 1000,
    });
    config
}

/// Build a test configuration with SVM chain configuration.
/// Extends build_test_config_with_mvm() to include a populated connected_chain_svm field.
#[allow(dead_code)]
pub fn build_test_config_with_svm() -> Config {
    let mut config = build_test_config_with_mvm();
    config.connected_chain_svm = Some(SvmChainConfig {
        name: "Connected SVM Chain".to_string(),
        rpc_url: "http://127.0.0.1:8899".to_string(),
        chain_id: 4,
        escrow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
    });
    config
}

/// Build a test config with a mock server URL
#[allow(dead_code)]
pub fn build_test_config_with_mock_server(mock_server_url: &str) -> Config {
    let mut config = build_test_config_with_mvm();
    config.hub_chain.rpc_url = mock_server_url.to_string();
    config
}

// ============================================================================
// DEFAULT EVENT CREATORS
// ============================================================================

/// Create a default intent event with test values for Move VM hub chain.
/// This can be customized using Rust's struct update syntax:
/// ```
/// let intent = create_default_intent_mvm();
/// let custom_intent = IntentEvent {
///     desired_amount: 500,
///     expiry_time: 1000000,
///     ..intent
/// };
/// ```
#[allow(dead_code)]
pub fn create_default_intent_mvm() -> IntentEvent {
    IntentEvent {
        intent_id: DUMMY_INTENT_ID.to_string(),
        offered_metadata: "{\"inner\":\"offered_meta\"}".to_string(),
        offered_amount: 1000,
        desired_metadata: "{\"inner\":\"desired_meta\"}".to_string(),
        desired_amount: 0,
        revocable: false,
        requester_addr: DUMMY_REQUESTER_ADDR_HUB.to_string(), // Hub chain requester (Move VM format, 32 bytes)
        requester_addr_connected_chain: Some(DUMMY_REQUESTER_ADDR_MVMCON.to_string()), // Required for outflow intents (connected_chain_id is Some). Move VM address format (32 bytes)
        reserved_solver_addr: Some(DUMMY_SOLVER_ADDR_HUB.to_string()), // Move VM address format (32 bytes)
        connected_chain_id: Some(2),
        expiry_time: 0, // Should be set explicitly in tests
        timestamp: 0,
        ready_on_connected_chain: false,
    }
}

/// Create a default intent event with test values for EVM connected chain.
/// This uses `create_default_intent_mvm()` as a base and overrides EVM-specific fields.
/// For inflow intents, offered_metadata uses {"token":"0x..."} format to match EVM escrow format.
/// This can be customized using Rust's struct update syntax:
/// ```
/// let intent = create_default_intent_evm();
/// let custom_intent = IntentEvent {
///     desired_amount: 500,
///     expiry_time: 1000000,
///     ..intent
/// };
/// ```
#[allow(dead_code)]
pub fn create_default_intent_evm() -> IntentEvent {
    IntentEvent {
        offered_metadata: format!(r#"{{"token":"{}"}}"#, DUMMY_TOKEN_ADDR_EVM), // EVM token address format for cross-chain
        connected_chain_id: Some(31337), // EVM chain ID (matches build_test_config_with_evm)
        requester_addr_connected_chain: Some(DUMMY_REQUESTER_ADDR_EVM.to_string()), // EVM address format (20 bytes)
        ..create_default_intent_mvm()
    }
}

/// Create a default intent event with test values for Solana connected chain.
/// This uses `create_default_intent_mvm()` as a base and overrides SVM-specific fields.
/// ```
/// let intent = create_default_intent_svm();
/// ```
#[allow(dead_code)]
pub fn create_default_intent_svm() -> IntentEvent {
    IntentEvent {
        connected_chain_id: Some(4), // SVM chain ID (matches build_test_config_with_svm)
        requester_addr_connected_chain: Some(DUMMY_REQUESTER_ADDR_SVM.to_string()), // SVM address format (base58)
        ..create_default_intent_mvm()
    }
}

/// Create a default fulfillment event with test values.
/// This can be customized using Rust's struct update syntax:
/// ```
/// let fulfillment = create_default_fulfillment();
/// let custom_fulfillment = FulfillmentEvent {
///     timestamp: 1000000,
///     provided_amount: 500,
///     provided_metadata: "{\"token\":\"USDC\"}".to_string(),
///     ..fulfillment
/// };
/// ```
#[allow(dead_code)]
pub fn create_default_fulfillment() -> FulfillmentEvent {
    FulfillmentEvent {
        intent_id: DUMMY_INTENT_ID.to_string(),
        intent_addr: DUMMY_INTENT_ADDR_HUB.to_string(),
        solver_hub_addr: DUMMY_SOLVER_ADDR_MVMCON.to_string(),
        provided_metadata: "{}".to_string(),
        provided_amount: 0,
        timestamp: 0, // Should be set explicitly in tests
    }
}

/// Create a default escrow event with test values for Move VM connected chain.
/// This can be customized using Rust's struct update syntax:
/// ```
/// let escrow = create_default_escrow_event();
/// let custom_escrow = EscrowEvent {
///     escrow_id: "0xescrow_id".to_string(),
///     intent_id: "0xintent_id".to_string(),
///     offered_amount: 1000,
///     ..escrow
/// };
/// ```
#[allow(dead_code)]
pub fn create_default_escrow_event() -> EscrowEvent {
    EscrowEvent {
        escrow_id: DUMMY_ESCROW_ID_MVM.to_string(),
        intent_id: DUMMY_INTENT_ID.to_string(),
        offered_metadata: "{\"inner\":\"offered_meta\"}".to_string(),
        offered_amount: 1000,
        desired_metadata: "{\"inner\":\"desired_meta\"}".to_string(),
        desired_amount: 0, // Escrow desired_amount must be 0 (validation requirement)
        revocable: false,
        requester_addr: DUMMY_REQUESTER_ADDR_MVMCON.to_string(), // EscrowEvent.requester_addr is the requester who created the escrow and locked funds (for inflow escrows on connected chain)
        reserved_solver_addr: Some(DUMMY_SOLVER_ADDR_HUB.to_string()),
        chain_id: 2,
        chain_type: ChainType::Mvm,
        expiry_time: 0,    // Should be set explicitly in tests
        timestamp: 0, // Should be set explicitly in tests
    }
}

/// Create a default escrow event with test values for EVM connected chain.
/// This reflects real EVM escrow behavior where desired_metadata is always empty
/// because the EVM IntentEscrow contract doesn't store this field.
#[allow(dead_code)]
pub fn create_default_escrow_event_evm() -> EscrowEvent {
    EscrowEvent {
        escrow_id: DUMMY_INTENT_ID.to_string(), // For EVM, escrow_id = intent_id
        intent_id: DUMMY_INTENT_ID.to_string(),
        offered_metadata: format!("{{\"token\":\"{}\"}}", DUMMY_TOKEN_ADDR_EVM), // Token address in JSON
        offered_amount: 1000,
        desired_metadata: "{}".to_string(), // EVM escrows don't store desired_metadata on-chain
        desired_amount: 0, // Not used for EVM inflow escrows
        revocable: false,
        requester_addr: DUMMY_REQUESTER_ADDR_EVM.to_string(), // EVM address format (20 bytes)
        reserved_solver_addr: Some(DUMMY_SOLVER_ADDR_EVM.to_string()), // EVM address format (20 bytes)
        chain_id: 31337, // Matches build_test_config_with_evm
        chain_type: ChainType::Evm,
        expiry_time: 0,    // Should be set explicitly in tests
        timestamp: 0, // Should be set explicitly in tests
    }
}

/// Create a default Move VM transaction with test values.
/// This can be customized using Rust's struct update syntax:
/// ```
/// let default = create_default_mvm_transaction();
/// let custom = MvmTransaction {
///     hash: "0x123123".to_string(),
///     success: false,
///     ..default
/// };
/// ```
#[allow(dead_code)]
pub fn create_default_mvm_transaction() -> MvmTransaction {
    MvmTransaction {
        version: "12345".to_string(),
        hash: "0x123123".to_string(), // Transaction hash - arbitrary test value
        success: true,
        events: vec![],
    }
}
