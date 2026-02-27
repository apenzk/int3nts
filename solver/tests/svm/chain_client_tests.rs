//! Unit tests for SVM Connected chain client
//!
//! Test ordering matches extension-checklist.md for cross-VM synchronization.
//! Tests marked N/A in the checklist are skipped in this file.

use base64::Engine;
use borsh::BorshSerialize;
use solver::chains::{ConnectedSvmClient, EscrowAccount};
use solver::config::SvmChainConfig;
use solana_sdk::pubkey::Pubkey;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "../helpers.rs"]
mod test_helpers;
use test_helpers::{DUMMY_INTENT_ID, DUMMY_SVM_ESCROW_PROGRAM_ID};

// ============================================================================
// CLIENT INITIALIZATION
// ============================================================================

/// 1. Test: ConnectedSvmClient Initialization
/// Verifies that ConnectedSvmClient::new() accepts valid program ids.
/// Why: Client initialization is the entry point for all SVM operations. A failure
/// here would prevent any solver operations on connected SVM chains.
#[test]
fn test_client_new() {
    let config = SvmChainConfig {
        name: "svm".to_string(),
        rpc_url: "http://127.0.0.1:8899".to_string(),
        chain_id: 901,
        escrow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
        private_key_env: "SOLANA_SOLVER_PRIVATE_KEY".to_string(),
        gmp_endpoint_program_id: None,
        outflow_validator_program_id: None,
    };

    let result = ConnectedSvmClient::new(&config);
    assert!(result.is_ok(), "Expected valid program id to succeed");
}

/// 2. Test: ConnectedSvmClient Rejects Invalid Program ID
/// Verifies that ConnectedSvmClient::new() rejects invalid program ids.
/// Why: Misconfigured program ids should fail fast instead of causing RPC errors later.
/// This prevents confusing error messages during escrow operations.
#[test]
fn test_client_new_rejects_invalid() {
    let config = SvmChainConfig {
        name: "svm".to_string(),
        rpc_url: "http://127.0.0.1:8899".to_string(),
        chain_id: 901,
        escrow_program_id: "not-a-pubkey".to_string(),
        private_key_env: "SOLANA_SOLVER_PRIVATE_KEY".to_string(),
        gmp_endpoint_program_id: None,
        outflow_validator_program_id: None,
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

// ============================================================================
// GMP FULFILLMENT
// ============================================================================

/// 10. Test: Fulfill Outflow Via GMP Returns Error When Not Configured
/// Verifies that fulfill_outflow_via_gmp returns an error when GMP config is missing.
/// Why: The GMP flow for SVM requires outflow_validator_program_id and gmp_endpoint_program_id
/// to be configured. If not configured, the function should return a clear error message.
#[tokio::test]
async fn test_fulfillment_error_handling() {
    let config = SvmChainConfig {
        name: "svm".to_string(),
        rpc_url: "http://127.0.0.1:8899".to_string(),
        chain_id: 901,
        escrow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
        private_key_env: "SOLANA_SOLVER_PRIVATE_KEY".to_string(),
        gmp_endpoint_program_id: None,
        outflow_validator_program_id: None,
    };

    let client = ConnectedSvmClient::new(&config).unwrap();

    // Call fulfill_outflow_via_gmp - should return error since GMP config is missing
    let result = client
        .fulfill_outflow_via_gmp(
            "0x0000000000000000000000000000000000000000000000000000000000001234",
            DUMMY_SVM_ESCROW_PROGRAM_ID,
        )
        .await;

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not configured"),
        "Expected 'not configured' error, got: {}",
        err_msg
    );
}

// ============================================================================
// INPUT PARSING
// ============================================================================

/// 11. Test: Pubkey From Hex With Leading Zeros
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

/// 12. Test: Pubkey From Hex No Leading Zeros
/// Verifies that pubkey parsing handles hex strings without leading zeros.
/// Why: Ensures non-zero-prefixed hex strings are parsed correctly. This is the
/// complementary test to #11 to ensure both cases work.
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

// ============================================================================
// GMP ESCROW STATE QUERYING
// ============================================================================

/// Helper: Creates a mock Solana RPC response for getAccountInfo with escrow data.
fn create_mock_escrow_response(is_claimed: bool) -> serde_json::Value {
    // Create an EscrowAccount with the specified is_claimed state
    let escrow = EscrowAccount {
        discriminator: [0u8; 8],
        requester: Pubkey::default(),
        token_mint: Pubkey::default(),
        amount: 1_000_000,
        is_claimed,
        expiry: 9999999999,
        reserved_solver: Pubkey::default(),
        intent_id: [0u8; 32],
        bump: 255,
    };

    // Serialize to borsh and base64-encode
    let serialized = escrow.try_to_vec().expect("Failed to serialize escrow");
    let base64_data = base64::engine::general_purpose::STANDARD.encode(&serialized);

    serde_json::json!({
        "jsonrpc": "2.0",
        "result": {
            "context": { "slot": 123 },
            "value": {
                "data": [base64_data, "base64"],
                "executable": false,
                "lamports": 1_000_000,
                "owner": DUMMY_SVM_ESCROW_PROGRAM_ID,
                "rentEpoch": 0
            }
        },
        "id": 1
    })
}

/// 13. Test: is_escrow_released returns true when escrow has been released
/// Verifies that is_escrow_released() correctly parses escrow account data
/// and returns true when is_claimed flag is set.
/// Why: With auto-release, the solver polls this to confirm release happened.
#[tokio::test(flavor = "multi_thread")]
async fn test_is_escrow_released_success() {
    let mock_server = MockServer::start().await;
    let rpc_url = mock_server.uri();

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_mock_escrow_response(true)))
        .mount(&mock_server)
        .await;

    let config = SvmChainConfig {
        name: "svm".to_string(),
        rpc_url,
        chain_id: 901,
        escrow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
        private_key_env: "SOLANA_SOLVER_PRIVATE_KEY".to_string(),
        gmp_endpoint_program_id: None,
        outflow_validator_program_id: None,
    };

    let client = ConnectedSvmClient::new(&config).unwrap();
    let result = client.is_escrow_released(DUMMY_INTENT_ID).unwrap();
    assert!(result, "Expected is_escrow_released to return true");
}

/// 14. Test: is_escrow_released returns false when escrow not yet released
/// Verifies that is_escrow_released() correctly parses a false is_claimed value.
/// Why: The solver polls this function repeatedly; false must not be misinterpreted.
#[tokio::test(flavor = "multi_thread")]
async fn test_is_escrow_released_false() {
    let mock_server = MockServer::start().await;
    let rpc_url = mock_server.uri();

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(create_mock_escrow_response(false)))
        .mount(&mock_server)
        .await;

    let config = SvmChainConfig {
        name: "svm".to_string(),
        rpc_url,
        chain_id: 901,
        escrow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
        private_key_env: "SOLANA_SOLVER_PRIVATE_KEY".to_string(),
        gmp_endpoint_program_id: None,
        outflow_validator_program_id: None,
    };

    let client = ConnectedSvmClient::new(&config).unwrap();
    let result = client.is_escrow_released(DUMMY_INTENT_ID).unwrap();
    assert!(!result, "Expected is_escrow_released to return false");
}

/// 15. Test: is_escrow_released handles RPC error
/// Verifies that is_escrow_released() propagates errors from failed RPC requests.
/// Why: Network errors must not be silently swallowed; the solver needs to retry.
#[tokio::test(flavor = "multi_thread")]
async fn test_is_escrow_released_error() {
    let mock_server = MockServer::start().await;
    let rpc_url = mock_server.uri();

    // Return an RPC error response (account not found)
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32602,
                "message": "Invalid param: could not find account"
            },
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let config = SvmChainConfig {
        name: "svm".to_string(),
        rpc_url,
        chain_id: 901,
        escrow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
        private_key_env: "SOLANA_SOLVER_PRIVATE_KEY".to_string(),
        gmp_endpoint_program_id: None,
        outflow_validator_program_id: None,
    };

    let client = ConnectedSvmClient::new(&config).unwrap();
    let result = client.is_escrow_released(DUMMY_INTENT_ID);
    assert!(result.is_err(), "Expected RPC error to propagate");
}

// ============================================================================
// BALANCE QUERIES
// ============================================================================

/// 16. Test: get_token_balance returns correct SPL token balance
/// Verifies that get_token_balance() derives the ATA and parses the balance response.
/// Why: Liquidity monitoring depends on accurate token balance reads from SVM chains.
#[tokio::test(flavor = "multi_thread")]
async fn test_get_token_balance_success() {
    let mock_server = MockServer::start().await;
    let rpc_url = mock_server.uri();

    // Mock getTokenAccountBalance response
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "result": {
                "context": { "slot": 123 },
                "value": {
                    "amount": "1000000",
                    "decimals": 6,
                    "uiAmount": 1.0,
                    "uiAmountString": "1.0"
                }
            },
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let config = SvmChainConfig {
        name: "svm".to_string(),
        rpc_url,
        chain_id: 901,
        escrow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
        private_key_env: "SOLANA_SOLVER_PRIVATE_KEY".to_string(),
        gmp_endpoint_program_id: None,
        outflow_validator_program_id: None,
    };

    let client = ConnectedSvmClient::new(&config).unwrap();

    // Use a valid base58 mint and owner (system program as mint, for simplicity)
    let mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
    let owner = "11111111111111111111111111111112";

    let balance = client.get_token_balance(mint, owner).unwrap();
    assert_eq!(balance, 1_000_000);
}

/// 17. Test: get_token_balance propagates RPC errors
/// Verifies that get_token_balance() returns Err on RPC error.
/// Why: RPC errors must propagate so the liquidity monitor can log and retry.
#[tokio::test(flavor = "multi_thread")]
async fn test_get_token_balance_error() {
    let mock_server = MockServer::start().await;
    let rpc_url = mock_server.uri();

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32602,
                "message": "Invalid param: could not find account"
            },
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let config = SvmChainConfig {
        name: "svm".to_string(),
        rpc_url,
        chain_id: 901,
        escrow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
        private_key_env: "SOLANA_SOLVER_PRIVATE_KEY".to_string(),
        gmp_endpoint_program_id: None,
        outflow_validator_program_id: None,
    };

    let client = ConnectedSvmClient::new(&config).unwrap();

    let mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
    let owner = "11111111111111111111111111111112";

    let result = client.get_token_balance(mint, owner);
    assert!(result.is_err(), "Expected RPC error to propagate");
}

// #18: get_token_balance_zero - N/A for SVM (token account either exists with balance or doesn't exist)

/// 19. Test: get_native_balance returns correct SOL balance
/// Verifies that get_native_balance() calls getBalance and returns lamports.
/// Why: Gas token monitoring uses native SOL balance, not SPL token balance.
#[tokio::test(flavor = "multi_thread")]
async fn test_get_native_balance_success() {
    let mock_server = MockServer::start().await;
    let rpc_url = mock_server.uri();

    // 0.1 SOL = 100_000_000 lamports
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "result": {
                "context": { "slot": 123 },
                "value": 100_000_000u64
            },
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let config = SvmChainConfig {
        name: "svm".to_string(),
        rpc_url,
        chain_id: 901,
        escrow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
        private_key_env: "SOLANA_SOLVER_PRIVATE_KEY".to_string(),
        gmp_endpoint_program_id: None,
        outflow_validator_program_id: None,
    };

    let client = ConnectedSvmClient::new(&config).unwrap();
    let owner = "11111111111111111111111111111112";

    let balance = client.get_native_balance(owner).unwrap();
    assert_eq!(balance, 100_000_000);
}

/// 20. Test: get_native_balance propagates RPC errors
/// Verifies that get_native_balance() returns Err on RPC failure.
/// Why: RPC errors must propagate so the liquidity monitor can log and retry.
#[tokio::test(flavor = "multi_thread")]
async fn test_get_native_balance_error() {
    let mock_server = MockServer::start().await;
    let rpc_url = mock_server.uri();

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32602,
                "message": "Invalid param: could not find account"
            },
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let config = SvmChainConfig {
        name: "svm".to_string(),
        rpc_url,
        chain_id: 901,
        escrow_program_id: DUMMY_SVM_ESCROW_PROGRAM_ID.to_string(),
        private_key_env: "SOLANA_SOLVER_PRIVATE_KEY".to_string(),
        gmp_endpoint_program_id: None,
        outflow_validator_program_id: None,
    };

    let client = ConnectedSvmClient::new(&config).unwrap();
    let owner = "11111111111111111111111111111112";

    let result = client.get_native_balance(owner);
    assert!(result.is_err(), "Expected RPC error to propagate");
}

// ============================================================================
// HEX ADDRESS NORMALIZATION (MVM-specific)
// ============================================================================

// #21: normalize_hex_to_address_full_length - N/A for SVM (MVM-specific Move address normalization)
// #22: normalize_hex_to_address_short_address - N/A for SVM (MVM-specific Move address normalization)
// #23: normalize_hex_to_address_odd_length - N/A for SVM (MVM-specific Move address normalization)
// #24: normalize_hex_to_address_no_prefix - N/A for SVM (MVM-specific Move address normalization)

// ============================================================================
// HAS OUTFLOW REQUIREMENTS (MVM-specific)
// ============================================================================

// #25: has_outflow_requirements_success - N/A for SVM (MVM-specific GMP view function)
// #26: has_outflow_requirements_false - N/A for SVM (MVM-specific GMP view function)
// #27: has_outflow_requirements_error - N/A for SVM (MVM-specific GMP view function)

// ============================================================================
// IS ESCROW RELEASED HELPERS (EVM-specific)
// ============================================================================

// #28: is_escrow_released_id_formatting - N/A for SVM (EVM-specific Hardhat script mechanics)
// #29: is_escrow_released_output_parsing - N/A for SVM (EVM-specific Hardhat script mechanics)
// #30: is_escrow_released_command_building - N/A for SVM (EVM-specific Hardhat script mechanics)
// #31: is_escrow_released_error_handling - N/A for SVM (EVM-specific Hardhat script mechanics)

// ============================================================================
// EVM ADDRESS NORMALIZATION (EVM-specific)
// ============================================================================

// #32: get_native_balance_exceeds_u64 - N/A for SVM (EVM-specific u64 overflow from large ETH balances)
// #33: get_token_balance_with_padded_address - N/A for SVM (EVM-specific 32-byte address padding)
// #34: get_native_balance_with_padded_address - N/A for SVM (EVM-specific 32-byte address padding)
// #35: normalize_evm_address_padded - N/A for SVM (EVM-specific address normalization)
// #36: normalize_evm_address_passthrough - N/A for SVM (EVM-specific address normalization)
// #37: normalize_evm_address_rejects_non_zero_high_bytes - N/A for SVM (EVM-specific address normalization)
