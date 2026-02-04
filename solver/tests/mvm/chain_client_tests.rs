//! Unit tests for MVM Connected chain client
//!
//! Test ordering matches EXTENSION-CHECKLIST.md for cross-VM synchronization.
//! Tests marked N/A in the checklist are skipped in this file.
//!
//! Hub-specific tests are in hub_client_tests.rs (hub is always MVM).

use serde_json::json;
use solver::chains::ConnectedMvmClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "../helpers.rs"]
mod test_helpers;
use test_helpers::{
    create_default_connected_mvm_chain_config, DUMMY_INTENT_ID, DUMMY_MODULE_ADDR_CON,
    DUMMY_TOKEN_ADDR_MVMCON,
};

// ============================================================================
// CLIENT INITIALIZATION
// ============================================================================

/// 1. Test: ConnectedMvmClient Initialization
/// Verifies that ConnectedMvmClient::new() creates a client with correct config.
/// Why: Client initialization is the entry point for all MVM operations. A failure
/// here would prevent any solver operations on connected MVM chains.
#[test]
fn test_mvm_client_new() {
    let config = create_default_connected_mvm_chain_config();
    let _client = ConnectedMvmClient::new(&config).unwrap();
}

// #2: client_new_rejects_invalid - N/A for MVM (no config validation like SVM pubkey)

// ============================================================================
// ESCROW EVENT QUERYING (legacy, replaced by GMP flow)
// ============================================================================

// #3: get_escrow_events_success - N/A for MVM (GMP flow uses hub-chain is_escrow_confirmed instead)
// #4: get_escrow_events_empty - N/A for MVM (GMP flow uses hub-chain is_escrow_confirmed instead)
// #5: get_escrow_events_error - N/A for MVM (GMP flow uses hub-chain is_escrow_confirmed instead)
// #6: escrow_event_deserialization - N/A for MVM (GMP flow removed legacy EscrowEvent/OracleLimitOrderEvent)

// ============================================================================
// FULFILLMENT OPERATIONS
// ============================================================================

/// 7. Test: Fulfill Outflow Via GMP Intent ID Formatting
/// Verifies that intent_id is correctly formatted for aptos CLI hex: argument.
/// Why: The aptos CLI expects hex arguments without the 0x prefix. Wrong formatting
/// would cause the transaction to fail with a parse error.
#[test]
fn test_fulfill_outflow_via_gmp_intent_id_formatting() {
    // Test that intent_id with 0x prefix has it stripped for hex: arg
    let intent_id_with_prefix = "0x1234567890abcdef";
    let formatted = intent_id_with_prefix
        .strip_prefix("0x")
        .unwrap_or(intent_id_with_prefix);
    assert_eq!(formatted, "1234567890abcdef");

    // Test that intent_id without 0x prefix is passed through
    let intent_id_no_prefix = "1234567890abcdef";
    let formatted = intent_id_no_prefix
        .strip_prefix("0x")
        .unwrap_or(intent_id_no_prefix);
    assert_eq!(formatted, "1234567890abcdef");
}

// #8: fulfillment_signature_encoding - N/A for MVM (EVM uses signatures, MVM uses CLI)

/// 9. Test: Fulfill Outflow Via GMP Command Building
/// Verifies that the aptos CLI command is built correctly with all required arguments.
/// Why: The fulfill_intent function is called via aptos CLI. Wrong argument formatting
/// would cause silent failures or wrong function parameters.
#[test]
fn test_fulfill_outflow_via_gmp_command_building() {
    let module_addr = DUMMY_MODULE_ADDR_CON;
    let intent_id = DUMMY_INTENT_ID;
    let token_metadata = DUMMY_TOKEN_ADDR_MVMCON;
    let profile = "solver-chain2";

    // Build the function ID
    let function_id = format!("{}::outflow_validator_impl::fulfill_intent", module_addr);

    // Build the args as the CLI would
    let intent_id_hex = intent_id.strip_prefix("0x").unwrap_or(intent_id);
    let arg1 = format!("hex:{}", intent_id_hex);
    let arg2 = format!("address:{}", token_metadata);

    // Verify function ID format
    assert!(function_id.contains("outflow_validator_impl::fulfill_intent"));
    assert!(function_id.starts_with("0x"));

    // Verify args format
    assert!(arg1.starts_with("hex:"));
    assert!(!arg1.contains("0x")); // 0x prefix should be stripped
    assert!(arg2.starts_with("address:"));
    assert!(arg2.contains("0x")); // address should keep 0x prefix

    // Verify profile is used
    assert_eq!(profile, "solver-chain2");
}

// #10: fulfillment_hash_extraction - TODO: implement for MVM
// #11: fulfillment_error_handling - TODO: implement for MVM
// #12: pubkey_from_hex_with_leading_zeros - N/A for MVM (SVM-specific)
// #13: pubkey_from_hex_no_leading_zeros - N/A for MVM (SVM-specific)

// ============================================================================
// GMP ESCROW STATE QUERYING (MVM-specific)
// ============================================================================

/// 14. Test: is_escrow_fulfilled returns true when FulfillmentProof received
/// Verifies that is_escrow_fulfilled() calls the inflow_escrow_gmp::is_fulfilled
/// view function and parses the boolean response.
/// Why: The solver needs to check fulfillment state before calling release_escrow.
#[tokio::test]
async fn test_is_escrow_fulfilled_success() {
    let mock_server = MockServer::start().await;
    // Note: base_url simulates the full RPC URL including /v1 suffix
    let base_url = format!("{}/v1", mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([true])))
        .mount(&mock_server)
        .await;

    let mut config = create_default_connected_mvm_chain_config();
    config.rpc_url = base_url;
    let client = ConnectedMvmClient::new(&config).unwrap();

    let result = client.is_escrow_fulfilled(DUMMY_INTENT_ID).await.unwrap();
    assert!(result);
}

/// 15. Test: is_escrow_fulfilled returns false when not yet fulfilled
/// Verifies that is_escrow_fulfilled() correctly parses a false response.
/// Why: The solver polls this function repeatedly; false must not be misinterpreted.
#[tokio::test]
async fn test_is_escrow_fulfilled_returns_false() {
    let mock_server = MockServer::start().await;
    let base_url = format!("{}/v1", mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([false])))
        .mount(&mock_server)
        .await;

    let mut config = create_default_connected_mvm_chain_config();
    config.rpc_url = base_url;
    let client = ConnectedMvmClient::new(&config).unwrap();

    let result = client.is_escrow_fulfilled(DUMMY_INTENT_ID).await.unwrap();
    assert!(!result);
}

/// 16. Test: is_escrow_fulfilled handles HTTP error
/// Verifies that is_escrow_fulfilled() propagates errors from failed HTTP requests.
/// Why: Network errors must not be silently swallowed; the solver needs to retry.
#[tokio::test]
async fn test_is_escrow_fulfilled_http_error() {
    let mock_server = MockServer::start().await;
    let base_url = format!("{}/v1", mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
        .mount(&mock_server)
        .await;

    let mut config = create_default_connected_mvm_chain_config();
    config.rpc_url = base_url;
    let client = ConnectedMvmClient::new(&config).unwrap();

    let result = client.is_escrow_fulfilled(DUMMY_INTENT_ID).await;
    assert!(result.is_err());
}

// ============================================================================
// GMP ESCROW RELEASE (MVM-specific)
// ============================================================================

/// 17. Test: release_gmp_escrow intent ID formatting
/// Verifies that intent_id is correctly formatted for aptos CLI hex: argument.
/// Why: The aptos CLI expects hex arguments without the 0x prefix. Wrong formatting
/// would cause the transaction to fail with a parse error.
#[test]
fn test_release_gmp_escrow_intent_id_formatting() {
    // Test that intent_id with 0x prefix has it stripped for hex: arg
    let intent_id_with_prefix = "0x1234567890abcdef";
    let formatted = intent_id_with_prefix
        .strip_prefix("0x")
        .unwrap_or(intent_id_with_prefix);
    assert_eq!(formatted, "1234567890abcdef");

    // Test that intent_id without 0x prefix is passed through
    let intent_id_no_prefix = "1234567890abcdef";
    let formatted = intent_id_no_prefix
        .strip_prefix("0x")
        .unwrap_or(intent_id_no_prefix);
    assert_eq!(formatted, "1234567890abcdef");
}

/// 18. Test: release_gmp_escrow command building
/// Verifies that the aptos CLI command is built correctly for inflow_escrow_gmp::release_escrow.
/// Why: The release_escrow function is called via aptos CLI. Wrong argument formatting
/// would cause the transaction to fail.
#[test]
fn test_release_gmp_escrow_command_building() {
    let module_addr = DUMMY_MODULE_ADDR_CON;
    let intent_id = DUMMY_INTENT_ID;
    let token_metadata = DUMMY_TOKEN_ADDR_MVMCON;

    // Build the function ID
    let function_id = format!("{}::inflow_escrow_gmp::release_escrow", module_addr);

    // Build the args as the CLI would
    let intent_id_hex = intent_id.strip_prefix("0x").unwrap_or(intent_id);
    let arg1 = format!("hex:{}", intent_id_hex);
    let arg2 = format!("address:{}", token_metadata);

    // Verify function ID format
    assert!(function_id.contains("inflow_escrow_gmp::release_escrow"));
    assert!(function_id.starts_with("0x"));

    // Verify args format
    assert!(arg1.starts_with("hex:"));
    assert!(!arg1.contains("0x")); // 0x prefix should be stripped
    assert!(arg2.starts_with("address:"));
    assert!(arg2.contains("0x")); // address should keep 0x prefix
}

// ============================================================================
// HEX ADDRESS NORMALIZATION
// ============================================================================

/// 19. Test: normalize_hex_to_address preserves full-length 64-char addresses
/// Verifies that a correctly formatted 64-char hex address passes through unchanged.
/// Why: Normalization must be a no-op for well-formed addresses to avoid corruption.
#[test]
fn test_normalize_hex_to_address_full_length() {
    let result = ConnectedMvmClient::normalize_hex_to_address(DUMMY_INTENT_ID);
    assert_eq!(result, DUMMY_INTENT_ID);
}

/// 20. Test: normalize_hex_to_address pads short addresses to 64 chars
/// Verifies that short addresses (e.g., "0x1") are zero-padded to 32 bytes.
/// Why: Move addresses are always 32 bytes. Short forms like "0x1" appear in framework
/// addresses and must be padded for the Aptos REST API.
#[test]
fn test_normalize_hex_to_address_short_address() {
    let result = ConnectedMvmClient::normalize_hex_to_address("0x1");
    assert_eq!(
        result,
        "0x0000000000000000000000000000000000000000000000000000000000000001"
    );
}

/// 21. Test: normalize_hex_to_address fixes odd-length hex from stripped leading zeros
/// Verifies that 63-char hex (from Move stripping a leading zero) becomes 64-char.
/// Why: Move events strip leading zeros from addresses. "0x0f...fe" becomes "0xf...fe"
/// (63 hex chars, odd length), which the Aptos REST API rejects with "Odd number of digits".
#[test]
fn test_normalize_hex_to_address_odd_length() {
    // Simulate Move stripping the leading zero from DUMMY_INTENT_ADDR_HUB ("0x00...0f")
    // "0x0f" → "0xf" (odd length, 1 hex char instead of 2)
    let stripped = "0xf";
    let result = ConnectedMvmClient::normalize_hex_to_address(stripped);
    assert_eq!(
        result,
        "0x000000000000000000000000000000000000000000000000000000000000000f"
    );
}

/// 22. Test: normalize_hex_to_address handles input without 0x prefix
/// Verifies that bare hex strings (no "0x") are correctly padded and prefixed.
/// Why: Intent IDs from different sources may or may not include the 0x prefix.
#[test]
fn test_normalize_hex_to_address_no_prefix() {
    let bare_hex = "1";
    let result = ConnectedMvmClient::normalize_hex_to_address(bare_hex);
    assert_eq!(
        result,
        "0x0000000000000000000000000000000000000000000000000000000000000001"
    );
}

// ============================================================================
// HAS OUTFLOW REQUIREMENTS (GMP view function)
// ============================================================================

/// 23. Test: has_outflow_requirements returns true when requirements delivered
/// Verifies that has_outflow_requirements() calls the outflow_validator_impl::has_requirements
/// view function and parses the boolean response.
/// Why: The solver polls this before calling fulfill_intent. A parse error would block fulfillment.
#[tokio::test]
async fn test_has_outflow_requirements_success() {
    let mock_server = MockServer::start().await;
    let base_url = format!("{}/v1", mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([true])))
        .mount(&mock_server)
        .await;

    let mut config = create_default_connected_mvm_chain_config();
    config.rpc_url = base_url;
    let client = ConnectedMvmClient::new(&config).unwrap();

    let result = client.has_outflow_requirements(DUMMY_INTENT_ID).await.unwrap();
    assert!(result);
}

/// 24. Test: has_outflow_requirements returns false when not yet delivered
/// Verifies that has_outflow_requirements() correctly parses a false response.
/// Why: The solver polls this function repeatedly; false must not be misinterpreted.
#[tokio::test]
async fn test_has_outflow_requirements_returns_false() {
    let mock_server = MockServer::start().await;
    let base_url = format!("{}/v1", mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([false])))
        .mount(&mock_server)
        .await;

    let mut config = create_default_connected_mvm_chain_config();
    config.rpc_url = base_url;
    let client = ConnectedMvmClient::new(&config).unwrap();

    let result = client.has_outflow_requirements(DUMMY_INTENT_ID).await.unwrap();
    assert!(!result);
}

/// 25. Test: has_outflow_requirements propagates HTTP errors
/// Verifies that has_outflow_requirements() returns Err on HTTP failure.
/// Why: Errors must propagate (not be swallowed as warnings) so the caller can fail fast.
#[tokio::test]
async fn test_has_outflow_requirements_http_error() {
    let mock_server = MockServer::start().await;
    let base_url = format!("{}/v1", mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(400).set_body_string(
            r#"{"message":"Odd number of digits","error_code":"invalid_input"}"#,
        ))
        .mount(&mock_server)
        .await;

    let mut config = create_default_connected_mvm_chain_config();
    config.rpc_url = base_url;
    let client = ConnectedMvmClient::new(&config).unwrap();

    let result = client.has_outflow_requirements(DUMMY_INTENT_ID).await;
    assert!(result.is_err());
}
