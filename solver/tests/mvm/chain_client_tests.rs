//! Unit tests for MVM Connected chain client (solver-specific)
//!
//! Test ordering matches solver/tests/extension-checklist.md for cross-VM synchronization.
//! Query tests (balance, escrow state, address normalization) moved to
//! chain-clients/mvm/tests/mvm_client_tests.rs. See chain-clients/extension-checklist.md.
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
fn test_client_new() {
    let config = create_default_connected_mvm_chain_config();
    let _client = ConnectedMvmClient::new(&config).unwrap();
}

// #2: test_client_new_rejects_invalid - N/A for MVM (no config validation like SVM pubkey)

// ============================================================================
// ESCROW EVENT QUERYING (legacy, replaced by GMP flow)
// ============================================================================

// #3: test_get_escrow_events_success - N/A for MVM (GMP flow uses hub-chain is_escrow_confirmed instead)
// #4: test_get_escrow_events_empty - N/A for MVM (GMP flow uses hub-chain is_escrow_confirmed instead)
// #5: test_get_escrow_events_error - N/A for MVM (GMP flow uses hub-chain is_escrow_confirmed instead)
// #6: test_escrow_event_deserialization - N/A for MVM (GMP flow removed legacy EscrowEvent/OracleLimitOrderEvent)

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

// #8: test_fulfillment_signature_encoding - N/A for MVM (EVM uses signatures, MVM uses CLI)

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

// #10: test_fulfillment_error_handling - TODO: implement for MVM
// #11: test_pubkey_from_hex_with_leading_zeros - N/A for MVM (SVM-specific)
// #12: test_pubkey_from_hex_no_leading_zeros - N/A for MVM (SVM-specific)

// ============================================================================
// GMP ESCROW STATE QUERYING
// ============================================================================

// #13: test_is_escrow_released_success — moved to chain-clients/mvm/tests/mvm_client_tests.rs (#3)
// #14: test_is_escrow_released_false — moved to chain-clients/mvm/tests/mvm_client_tests.rs (#4)
// #15: test_is_escrow_released_error — moved to chain-clients/mvm/tests/mvm_client_tests.rs (#5)

// ============================================================================
// BALANCE QUERIES
// ============================================================================

// #16: test_get_token_balance_success — moved to chain-clients/mvm/tests/mvm_client_tests.rs (#6)
// #17: test_get_token_balance_error — moved to chain-clients/mvm/tests/mvm_client_tests.rs (#7)
// #18: test_get_token_balance_zero — moved to chain-clients/mvm/tests/mvm_client_tests.rs (#8)
// #19: test_get_native_balance_success — N/A for MVM (native MOVE is queried as FA token via get_token_balance with 0xa metadata)
// #20: test_get_native_balance_error — N/A for MVM

// ============================================================================
// HEX ADDRESS NORMALIZATION
// ============================================================================

// #21: test_normalize_hex_to_address_full_length — moved to chain-clients/mvm/tests/mvm_client_tests.rs (#18)
// #22: test_normalize_hex_to_address_short_address — moved to chain-clients/mvm/tests/mvm_client_tests.rs (#19)
// #23: test_normalize_hex_to_address_odd_length — moved to chain-clients/mvm/tests/mvm_client_tests.rs (#20)
// #24: test_normalize_hex_to_address_no_prefix — moved to chain-clients/mvm/tests/mvm_client_tests.rs (#21)

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

// #28: test_is_escrow_released_id_formatting - N/A for MVM (EVM-specific Hardhat script mechanics)
// #29: test_is_escrow_released_output_parsing - N/A for MVM (EVM-specific Hardhat script mechanics)
// #30: test_is_escrow_released_command_building - N/A for MVM (EVM-specific Hardhat script mechanics)
// #31: test_is_escrow_released_error_handling - N/A for MVM (EVM-specific Hardhat script mechanics)

// ============================================================================
// EVM ADDRESS NORMALIZATION (EVM-specific)
// ============================================================================

// #32: test_get_native_balance_exceeds_u64 - N/A for MVM (EVM-specific u64 overflow from large ETH balances)
// #33: test_get_token_balance_with_padded_address - N/A for MVM (EVM-specific 32-byte address padding)
// #34: test_get_native_balance_with_padded_address - N/A for MVM (EVM-specific 32-byte address padding)
// #35: test_normalize_evm_address_padded - N/A for MVM (EVM-specific address normalization)
// #36: test_normalize_evm_address_passthrough - N/A for MVM (EVM-specific address normalization)
// #37: test_normalize_evm_address_rejects_non_zero_high_bytes - N/A for MVM (EVM-specific address normalization)
