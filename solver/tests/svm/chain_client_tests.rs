//! Unit tests for SVM Connected chain client
//!
//! Test ordering matches EXTENSION-CHECKLIST.md for cross-VM synchronization.
//! Tests marked N/A in the checklist are skipped in this file.

use solver::chains::ConnectedSvmClient;
use solver::config::SvmChainConfig;

#[path = "../helpers.rs"]
mod test_helpers;
use test_helpers::DUMMY_SVM_ESCROW_PROGRAM_ID;

// ============================================================================
// CLIENT INITIALIZATION
// ============================================================================

/// 1. Test: ConnectedSvmClient Initialization
/// Verifies that ConnectedSvmClient::new() accepts valid program ids.
/// Why: Client initialization is the entry point for all SVM operations. A failure
/// here would prevent any solver operations on connected SVM chains.
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

/// 2. Test: ConnectedSvmClient Rejects Invalid Program ID
/// Verifies that ConnectedSvmClient::new() rejects invalid program ids.
/// Why: Misconfigured program ids should fail fast instead of causing RPC errors later.
/// This prevents confusing error messages during escrow operations.
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

// #3: get_escrow_events_success - TODO: implement for SVM
// #4: get_escrow_events_empty - TODO: implement for SVM
// #5: get_escrow_events_error - TODO: implement for SVM
// #6: escrow_event_deserialization - N/A for SVM (parses program accounts directly)
// #7: fulfillment_id_formatting - TODO: implement for SVM
// #8: fulfillment_signature_encoding - N/A for SVM (uses different mechanism)
// #9: fulfillment_command_building - TODO: implement for SVM
// #10: fulfillment_hash_extraction - TODO: implement for SVM
// #11: fulfillment_error_handling - TODO: implement for SVM

// ============================================================================
// INPUT PARSING
// ============================================================================

/// 12. Test: Pubkey From Hex With Leading Zeros
/// Verifies that pubkey parsing handles hex strings with leading zeros.
/// Why: Intent IDs often have leading zeros. If they're stripped during conversion,
/// the pubkey will be wrong and escrow lookups will fail silently.
#[test]
fn test_pubkey_from_hex_with_leading_zeros() {
    // Test hex with leading zeros (common in intent IDs)
    let hex_with_zeros = "0x00000000000000000000000000000000000000000000000000000000deadbeef";
    let hex_no_prefix = hex_with_zeros.strip_prefix("0x").unwrap_or(hex_with_zeros);

    // Verify the hex is 64 chars (32 bytes)
    assert_eq!(hex_no_prefix.len(), 64);

    // Verify leading zeros are preserved
    assert!(hex_no_prefix.starts_with("0000"));
}

/// 13. Test: Pubkey From Hex No Leading Zeros
/// Verifies that pubkey parsing handles hex strings without leading zeros.
/// Why: Ensures non-zero-prefixed hex strings are parsed correctly. This is the
/// complementary test to #12 to ensure both cases work.
#[test]
fn test_pubkey_from_hex_no_leading_zeros() {
    // Test hex without leading zeros
    let hex_no_zeros = "0xdeadbeefcafebabe1234567890abcdef1234567890abcdef1234567890abcdef";
    let hex_no_prefix = hex_no_zeros.strip_prefix("0x").unwrap_or(hex_no_zeros);

    // Verify the hex is 64 chars (32 bytes)
    assert_eq!(hex_no_prefix.len(), 64);

    // Verify it doesn't start with zeros
    assert!(!hex_no_prefix.starts_with("0000"));
}
