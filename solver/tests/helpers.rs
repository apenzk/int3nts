//! Shared test helpers for solver unit tests
//!
//! This module provides constants and helper functions used by solver unit tests.

#![allow(dead_code)]

// ============================================================================
// CONSTANTS
// ============================================================================

// --------------------------------- IDs ----------------------------------

/// Dummy draft ID (UUID format)
pub const DUMMY_DRAFT_ID: &str = "11111111-1111-1111-1111-111111111111";

/// Dummy intent ID (64 hex characters, same across all chains)
pub const DUMMY_INTENT_ID: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000001";

/// Dummy escrow ID (EVM format, 40 hex characters)
pub const DUMMY_ESCROW_ID_EVM: &str = "0x0000000000000000000000000000000000000002";

/// Dummy escrow ID (Move VM format, 64 hex characters)
pub const DUMMY_ESCROW_ID_MVM: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000003";

// -------------------------------- USERS ---------------------------------

/// Dummy requester address on hub chain (Move VM format, 64 hex characters)
pub const DUMMY_REQUESTER_ADDR_HUB: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000004";

/// Dummy requester address on connected chain (Move VM format, 64 hex characters)
pub const DUMMY_REQUESTER_ADDR_MVMCON: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000005";

/// Dummy requester address (EVM format, 40 hex characters)
pub const DUMMY_REQUESTER_ADDR_EVM: &str = "0x0000000000000000000000000000000000000006";

/// Dummy solver address on hub chain (Move VM format, 64 hex characters)
pub const DUMMY_SOLVER_ADDR_HUB: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000007";

/// Dummy solver address on connected chain (Move VM format, 64 hex characters)
pub const DUMMY_SOLVER_ADDR_MVMCON: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000008";

/// Dummy solver address (EVM format, 40 hex characters)
pub const DUMMY_SOLVER_ADDR_EVM: &str = "0x0000000000000000000000000000000000000009";

// ------------------------- TOKENS AND CONTRACTS -------------------------

/// Dummy token address (EVM format, 40 hex characters)
pub const DUMMY_TOKEN_ADDR_EVM: &str = "0x000000000000000000000000000000000000000a";

/// Dummy token address on hub chain (Move VM format, 64 hex characters)
pub const DUMMY_TOKEN_ADDR_HUB: &str =
    "0x000000000000000000000000000000000000000000000000000000000000000b";

/// Dummy token address on connected chain (Move VM format, 64 hex characters)
pub const DUMMY_TOKEN_ADDR_MVMCON: &str =
    "0x000000000000000000000000000000000000000000000000000000000000000c";

/// Dummy token address for unsupported token tests (64 hex chars)
#[allow(dead_code)]
pub const DUMMY_TOKEN_ADDR_UNSUPPORTED: &str =
    "0x000000000000000000000000000000000000000000000000000000000000000d";

/// Dummy escrow contract address (EVM format, 40 hex characters)
pub const DUMMY_ESCROW_CONTRACT_ADDR_EVM: &str = "0x000000000000000000000000000000000000000e";

/// Dummy intent address (Move VM format, 64 hex characters, used for intent object address on hub chain)
pub const DUMMY_INTENT_ADDR_HUB: &str =
    "0x000000000000000000000000000000000000000000000000000000000000000f";

/// Dummy escrow program id (valid base58 pubkey string)
pub const DUMMY_SVM_ESCROW_PROGRAM_ID: &str = "11111111111111111111111111111111";

/// Dummy module address for hub chain
pub const DUMMY_MODULE_ADDR_HUB: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000010";

/// Dummy module address for connected chain
pub const DUMMY_MODULE_ADDR_CON: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000011";

// -------------------------------- OTHER ---------------------------------

/// Dummy transaction hash (64 hex characters)
pub const DUMMY_TX_HASH: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000012";

/// Dummy expiry timestamp (far future timestamp for tests)
pub const DUMMY_EXPIRY: u64 = 9999999999;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Create a default token pair with test values.
/// This can be customized using Rust's struct update syntax:
/// ```
/// let pair = TokenPair {
///     desired_token: "0xccc...".to_string(),
///     ..create_default_token_pair()
/// };
/// ```
pub fn create_default_token_pair() -> solver::acceptance::TokenPair {
    solver::acceptance::TokenPair {
        offered_chain_id: 1,
        offered_token: DUMMY_TOKEN_ADDR_HUB.to_string(),
        desired_chain_id: 2,
        desired_token: DUMMY_TOKEN_ADDR_MVMCON.to_string(),
    }
}

/// Create a default service config with test values.
/// This can be customized using Rust's struct update syntax:
/// ```
/// let service = ServiceConfig {
///     polling_interval_ms: 5000,
///     ..create_default_service_config()
/// };
/// ```
pub fn create_default_service_config() -> solver::config::ServiceConfig {
    solver::config::ServiceConfig {
        coordinator_url: "http://127.0.0.1:3333".to_string(),
        trusted_gmp_url: "http://127.0.0.1:3334".to_string(),
        polling_interval_ms: 2000,
        e2e_mode: false,
        acceptance_api_host: "127.0.0.1".to_string(),
        acceptance_api_port: 4444,
    }
}

/// Create a default hub chain config with test values.
/// This can be customized using Rust's struct update syntax:
/// ```
/// let hub_chain = ChainConfig {
///     profile: "custom-profile".to_string(),
///     ..create_default_hub_chain_config()
/// };
/// ```
pub fn create_default_hub_chain_config() -> solver::config::ChainConfig {
    solver::config::ChainConfig {
        name: "hub-chain".to_string(),
        rpc_url: "http://127.0.0.1:8080/v1".to_string(),
        chain_id: 1,
        module_addr: DUMMY_MODULE_ADDR_HUB.to_string(),
        profile: "hub-profile".to_string(),
        e2e_mode: false,
    }
}

/// Create a default connected MVM chain config with test values.
/// This can be customized using Rust's struct update syntax:
/// ```
/// let connected_chain = MvmChainConfig {
///     profile: "custom-profile".to_string(),
///     ..create_default_connected_mvm_chain_config()
/// };
/// ```
pub fn create_default_connected_mvm_chain_config() -> solver::config::MvmChainConfig {
    solver::config::MvmChainConfig {
        name: "connected-chain".to_string(),
        rpc_url: "http://127.0.0.1:8082/v1".to_string(),
        chain_id: 2,
        module_addr: DUMMY_MODULE_ADDR_CON.to_string(),
        profile: "connected-profile".to_string(),
        e2e_mode: false,
    }
}

/// Create a default solver signing config with test values.
/// This can be customized using Rust's struct update syntax:
/// ```
/// let solver = SolverSigningConfig {
///     profile: "custom-profile".to_string(),
///     ..create_default_solver_signing_config()
/// };
/// ```
pub fn create_default_solver_signing_config() -> solver::config::SolverSigningConfig {
    solver::config::SolverSigningConfig {
        profile: "hub-profile".to_string(),
        address: DUMMY_SOLVER_ADDR_EVM.to_string(),
    }
}

/// Create a default solver config with test values.
/// This can be customized using Rust's struct update syntax:
/// ```
/// let config = SolverConfig {
///     acceptance: AcceptanceConfig {
///         token_pairs: custom_pairs,
///     },
///     ..create_default_solver_config()
/// };
/// ```
pub fn create_default_solver_config() -> solver::config::SolverConfig {
    solver::config::SolverConfig {
        service: create_default_service_config(),
        hub_chain: create_default_hub_chain_config(),
        connected_chain: vec![
            solver::config::ConnectedChainConfig::Mvm(create_default_connected_mvm_chain_config()),
        ],
        acceptance: solver::config::AcceptanceConfig {
            token_pairs: Vec::new(),
        },
        solver: create_default_solver_signing_config(),
    }
}
