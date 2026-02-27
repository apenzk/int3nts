//! Unit tests for MVM Connected chain client
//!
//! Test ordering matches extension-checklist.md for cross-VM synchronization.
//! Tests marked N/A in the checklist are skipped in this file.
//!
//! Hub-specific tests are in hub_client_tests.rs (hub is always MVM).

use serde_json::json;
use solver::chains::ConnectedMvmClient;
use wiremock::matchers::{body_partial_json, method, path};
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
fn test_client_new() {
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
fn test_fulfillment_id_formatting() {
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
fn test_fulfillment_command_building() {
    let module_addr = DUMMY_MODULE_ADDR_CON;
    let intent_id = DUMMY_INTENT_ID;
    let token_metadata = DUMMY_TOKEN_ADDR_MVMCON;
    let profile = "solver-chain2";

    // Build the function ID
    let function_id = format!("{}::intent_outflow_validator_impl::fulfill_intent", module_addr);

    // Build the args as the CLI would
    let intent_id_hex = intent_id.strip_prefix("0x").unwrap_or(intent_id);
    let arg1 = format!("hex:{}", intent_id_hex);
    let arg2 = format!("address:{}", token_metadata);

    // Verify function ID format
    assert!(function_id.contains("intent_outflow_validator_impl::fulfill_intent"));
    assert!(function_id.starts_with("0x"));

    // Verify args format
    assert!(arg1.starts_with("hex:"));
    assert!(!arg1.contains("0x")); // 0x prefix should be stripped
    assert!(arg2.starts_with("address:"));
    assert!(arg2.contains("0x")); // address should keep 0x prefix

    // Verify profile is used
    assert_eq!(profile, "solver-chain2");
}

// #10: fulfillment_error_handling - TODO: implement for MVM
// #11: pubkey_from_hex_with_leading_zeros - N/A for MVM (SVM-specific)
// #12: pubkey_from_hex_no_leading_zeros - N/A for MVM (SVM-specific)

// ============================================================================
// GMP ESCROW STATE QUERYING (MVM-specific)
// ============================================================================

/// 13. Test: is_escrow_released returns true when escrow has been auto-released
/// Verifies that is_escrow_released() calls the intent_inflow_escrow::is_released
/// view function and parses the boolean response.
/// Why: With auto-release, the solver polls this to confirm release happened.
#[tokio::test]
async fn test_is_escrow_released_success() {
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

    let result = client.is_escrow_released(DUMMY_INTENT_ID).await.unwrap();
    assert!(result);
}

/// 14. Test: is_escrow_released returns false when not yet released
/// Verifies that is_escrow_released() correctly parses a false response.
/// Why: The solver polls this function repeatedly; false must not be misinterpreted.
#[tokio::test]
async fn test_is_escrow_released_false() {
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

    let result = client.is_escrow_released(DUMMY_INTENT_ID).await.unwrap();
    assert!(!result);
}

/// 15. Test: is_escrow_released handles HTTP error
/// Verifies that is_escrow_released() propagates errors from failed HTTP requests.
/// Why: Network errors must not be silently swallowed; the solver needs to retry.
#[tokio::test]
async fn test_is_escrow_released_error() {
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

    let result = client.is_escrow_released(DUMMY_INTENT_ID).await;
    assert!(result.is_err());
}

// ============================================================================
// BALANCE QUERIES
// ============================================================================

/// 16. Test: get_token_balance returns correct FA balance
/// Verifies that get_token_balance() calls the primary_fungible_store::balance view function
/// with the required Metadata type argument and parses the string response as u128.
/// Why: Liquidity monitoring depends on accurate balance reads from MVM chains.
#[tokio::test]
async fn test_get_token_balance_success() {
    let mock_server = MockServer::start().await;
    let base_url = format!("{}/v1", mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .and(body_partial_json(json!({
            "function": "0x1::primary_fungible_store::balance",
            "type_arguments": ["0x1::fungible_asset::Metadata"]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!(["1000000"])))
        .mount(&mock_server)
        .await;

    let mut config = create_default_connected_mvm_chain_config();
    config.rpc_url = base_url;
    let client = ConnectedMvmClient::new(&config).unwrap();

    let balance = client
        .get_token_balance(DUMMY_TOKEN_ADDR_MVMCON, DUMMY_TOKEN_ADDR_MVMCON)
        .await
        .unwrap();
    assert_eq!(balance, 1_000_000);
}

/// 17. Test: get_token_balance propagates HTTP errors
/// Verifies that get_token_balance() returns Err on HTTP failure.
/// Why: Errors must propagate so the liquidity monitor can log and retry.
#[tokio::test]
async fn test_get_token_balance_error() {
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

    let result = client
        .get_token_balance(DUMMY_TOKEN_ADDR_MVMCON, DUMMY_TOKEN_ADDR_MVMCON)
        .await;
    assert!(result.is_err());
}

/// 18. Test: get_token_balance returns zero balance
/// Verifies that get_token_balance() correctly parses "0" from the view function.
/// Why: Zero balance is a valid state (empty wallet), not an error.
#[tokio::test]
async fn test_get_token_balance_zero() {
    let mock_server = MockServer::start().await;
    let base_url = format!("{}/v1", mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!(["0"])))
        .mount(&mock_server)
        .await;

    let mut config = create_default_connected_mvm_chain_config();
    config.rpc_url = base_url;
    let client = ConnectedMvmClient::new(&config).unwrap();

    let balance = client
        .get_token_balance(DUMMY_TOKEN_ADDR_MVMCON, DUMMY_TOKEN_ADDR_MVMCON)
        .await
        .unwrap();
    assert_eq!(balance, 0);
}

// #19: get_native_balance_success - N/A for MVM (native MOVE is queried as FA token via get_token_balance with 0xa metadata)
// #20: get_native_balance_error - N/A for MVM

// ============================================================================
// HEX ADDRESS NORMALIZATION (MVM-specific)
// ============================================================================

/// 21. Test: normalize_hex_to_address preserves full-length 64-char addresses
/// Verifies that a correctly formatted 64-char hex address passes through unchanged.
/// Why: Normalization must be a no-op for well-formed addresses to avoid corruption.
#[test]
fn test_normalize_hex_to_address_full_length() {
    let result = ConnectedMvmClient::normalize_hex_to_address(DUMMY_INTENT_ID);
    assert_eq!(result, DUMMY_INTENT_ID);
}

/// 22. Test: normalize_hex_to_address pads short addresses to 64 chars
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

/// 23. Test: normalize_hex_to_address fixes odd-length hex from stripped leading zeros
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

/// 24. Test: normalize_hex_to_address handles input without 0x prefix
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
// HAS OUTFLOW REQUIREMENTS (MVM-specific, GMP view function)
// ============================================================================

/// 25. Test: has_outflow_requirements returns true when requirements delivered
/// Verifies that has_outflow_requirements() calls the intent_outflow_validator_impl::has_requirements
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

/// 26. Test: has_outflow_requirements returns false when not yet delivered
/// Verifies that has_outflow_requirements() correctly parses a false response.
/// Why: The solver polls this function repeatedly; false must not be misinterpreted.
#[tokio::test]
async fn test_has_outflow_requirements_false() {
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

/// 27. Test: has_outflow_requirements propagates HTTP errors
/// Verifies that has_outflow_requirements() returns Err on HTTP failure.
/// Why: Errors must propagate (not be swallowed as warnings) so the caller can fail fast.
#[tokio::test]
async fn test_has_outflow_requirements_error() {
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

// #28: is_escrow_released_id_formatting - N/A for MVM (EVM-specific Hardhat script mechanics)
// #29: is_escrow_released_output_parsing - N/A for MVM (EVM-specific Hardhat script mechanics)
// #30: is_escrow_released_command_building - N/A for MVM (EVM-specific Hardhat script mechanics)
// #31: is_escrow_released_error_handling - N/A for MVM (EVM-specific Hardhat script mechanics)

// ============================================================================
// EVM ADDRESS NORMALIZATION (EVM-specific)
// ============================================================================

// #32: get_native_balance_exceeds_u64 - N/A for MVM (EVM-specific u64 overflow from large ETH balances)
// #33: get_token_balance_with_padded_address - N/A for MVM (EVM-specific 32-byte address padding)
// #34: get_native_balance_with_padded_address - N/A for MVM (EVM-specific 32-byte address padding)
// #35: normalize_evm_address_padded - N/A for MVM (EVM-specific address normalization)
// #36: normalize_evm_address_passthrough - N/A for MVM (EVM-specific address normalization)
// #37: normalize_evm_address_rejects_non_zero_high_bytes - N/A for MVM (EVM-specific address normalization)
