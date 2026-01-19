//! Unit tests for ConnectedSvmClient initialization

use solver::chains::ConnectedSvmClient;
use solver::config::SvmChainConfig;

#[path = "../helpers.rs"]
mod test_helpers;
use test_helpers::DUMMY_SVM_ESCROW_PROGRAM_ID;

/// Test that ConnectedSvmClient rejects invalid program ids
/// Why: Misconfigured program ids should fail fast instead of causing RPC errors later
#[test]
fn test_new_rejects_invalid_program_id() {
    let config = SvmChainConfig {
        name: "svm".to_string(),
        rpc_url: "http://127.0.0.1:8899".to_string(),
        chain_id: 4,
        escrow_program_id: "not-a-pubkey".to_string(),
        private_key_env: "SOLANA_SOLVER_PRIVATE_KEY".to_string(),
    };

    let result = ConnectedSvmClient::new(&config);
    assert!(result.is_err(), "Expected invalid program id to fail");
}

/// Test that ConnectedSvmClient accepts valid program ids
/// Why: Ensure basic config validation allows known-good program ids
#[test]
fn test_new_accepts_valid_program_id() {
    let config = SvmChainConfig {
        name: "svm".to_string(),
        rpc_url: "http://127.0.0.1:8899".to_string(),
        chain_id: 4,
        escrow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
        private_key_env: "SOLANA_SOLVER_PRIVATE_KEY".to_string(),
    };

    let result = ConnectedSvmClient::new(&config);
    assert!(result.is_ok(), "Expected valid program id to succeed");
}
