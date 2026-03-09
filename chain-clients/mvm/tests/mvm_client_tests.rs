//! Connected-chain MVM client tests
//!
//! Test ordering matches chain-clients/extension-checklist.md for cross-VM synchronization.
//! Tests marked N/A in the checklist are skipped with comments.

use chain_clients_mvm::{normalize_hex_to_address, MvmClient};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// CONSTANTS
// ============================================================================

const DUMMY_INTENT_ID: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000001";
const DUMMY_MODULE_ADDR: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000011";
const DUMMY_TOKEN_ADDR: &str =
    "0x000000000000000000000000000000000000000000000000000000000000000c";

// ============================================================================
// CLIENT INITIALIZATION
// ============================================================================

/// 1. Test: MvmClient initialization
/// Verifies that MvmClient::new() creates a client successfully.
/// Why: Client initialization is the entry point for all MVM operations.
#[test]
fn test_client_new() {
    let _client = MvmClient::new("http://127.0.0.1:8080").unwrap();
}

// #2: client_new_rejects_invalid - N/A for MVM (accepts any URL, validation at request time)

// ============================================================================
// ESCROW RELEASE CHECK
// ============================================================================

/// 3. Test: is_escrow_released returns true when escrow has been auto-released
/// Verifies that is_escrow_released() calls the view function and parses boolean response.
/// Why: With auto-release, the solver polls this to confirm release happened.
#[tokio::test]
async fn test_is_escrow_released_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([true])))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .is_escrow_released(DUMMY_INTENT_ID, DUMMY_MODULE_ADDR)
        .await
        .unwrap();
    assert!(result);
}

/// 4. Test: is_escrow_released returns false when not yet released
/// Verifies that is_escrow_released() correctly parses a false response.
/// Why: The solver polls this function repeatedly; false must not be misinterpreted.
#[tokio::test]
async fn test_is_escrow_released_false() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([false])))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .is_escrow_released(DUMMY_INTENT_ID, DUMMY_MODULE_ADDR)
        .await
        .unwrap();
    assert!(!result);
}

/// 5. Test: is_escrow_released handles HTTP error
/// Verifies that is_escrow_released() propagates errors from failed HTTP requests.
/// Why: Network errors must not be silently swallowed; the caller needs to retry.
#[tokio::test]
async fn test_is_escrow_released_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .is_escrow_released(DUMMY_INTENT_ID, DUMMY_MODULE_ADDR)
        .await;
    assert!(result.is_err());
}

// ============================================================================
// BALANCE QUERIES
// ============================================================================

/// 6. Test: get_token_balance returns correct FA balance
/// Verifies that get_token_balance() calls primary_fungible_store::balance view function
/// and parses the string response as u128.
/// Why: Liquidity monitoring depends on accurate balance reads from MVM chains.
#[tokio::test]
async fn test_get_token_balance_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!(["1000000"])))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let balance = client
        .get_token_balance(DUMMY_TOKEN_ADDR, DUMMY_TOKEN_ADDR)
        .await
        .unwrap();
    assert_eq!(balance, 1_000_000);
}

/// 7. Test: get_token_balance propagates HTTP errors
/// Verifies that get_token_balance() returns Err on HTTP failure.
/// Why: Errors must propagate so the liquidity monitor can log and retry.
#[tokio::test]
async fn test_get_token_balance_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .get_token_balance(DUMMY_TOKEN_ADDR, DUMMY_TOKEN_ADDR)
        .await;
    assert!(result.is_err());
}

/// 8. Test: get_token_balance returns zero balance
/// Verifies that get_token_balance() correctly parses "0" from the view function.
/// Why: Zero balance is a valid state (empty wallet), not an error.
#[tokio::test]
async fn test_get_token_balance_zero() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!(["0"])))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let balance = client
        .get_token_balance(DUMMY_TOKEN_ADDR, DUMMY_TOKEN_ADDR)
        .await
        .unwrap();
    assert_eq!(balance, 0);
}

// #9: get_native_balance_success - N/A for MVM (native MOVE is queried as FA via get_token_balance)
// #10: get_native_balance_error - N/A for MVM
// #11: get_native_balance_exceeds_u64 - N/A for MVM (EVM-specific)
// #12: get_token_balance_with_padded_address - N/A for MVM (EVM-specific)
// #13: get_native_balance_with_padded_address - N/A for MVM (EVM-specific)

// ============================================================================
// ESCROW EVENT PARSING
// ============================================================================

// #14: get_escrow_events_success - N/A for MVM (events via Aptos REST event stream)
// #15: get_escrow_events_empty - N/A for MVM
// #16: get_escrow_events_error - N/A for MVM
// #17: get_all_escrows_parses_program_accounts - N/A for MVM (SVM-specific)

// ============================================================================
// ADDRESS NORMALIZATION (MVM-specific)
// ============================================================================

/// 18. Test: normalize_hex_to_address preserves full-length 64-char addresses
/// Verifies that a correctly formatted 64-char hex address passes through unchanged.
/// Why: Normalization must be a no-op for well-formed addresses to avoid corruption.
#[test]
fn test_normalize_hex_to_address_full_length() {
    let result = normalize_hex_to_address(DUMMY_INTENT_ID);
    assert_eq!(result, DUMMY_INTENT_ID);
}

/// 19. Test: normalize_hex_to_address pads short addresses to 64 chars
/// Verifies that short addresses (e.g., "0x1") are zero-padded to 32 bytes.
/// Why: Move addresses are always 32 bytes. Short forms like "0x1" appear in framework
/// addresses and must be padded for the Aptos REST API.
#[test]
fn test_normalize_hex_to_address_short_address() {
    let result = normalize_hex_to_address("0x1");
    assert_eq!(
        result,
        "0x0000000000000000000000000000000000000000000000000000000000000001"
    );
}

/// 20. Test: normalize_hex_to_address fixes odd-length hex from stripped leading zeros
/// Verifies that 63-char hex (from Move stripping a leading zero) becomes 64-char.
/// Why: Move events strip leading zeros from addresses. "0x0f...fe" becomes "0xf...fe"
/// (63 hex chars, odd length), which the Aptos REST API rejects.
#[test]
fn test_normalize_hex_to_address_odd_length() {
    let stripped = "0xf";
    let result = normalize_hex_to_address(stripped);
    assert_eq!(
        result,
        "0x000000000000000000000000000000000000000000000000000000000000000f"
    );
}

/// 21. Test: normalize_hex_to_address handles input without 0x prefix
/// Verifies that bare hex strings (no "0x") are correctly padded and prefixed.
/// Why: Intent IDs from different sources may or may not include the 0x prefix.
#[test]
fn test_normalize_hex_to_address_no_prefix() {
    let result = normalize_hex_to_address("1");
    assert_eq!(
        result,
        "0x0000000000000000000000000000000000000000000000000000000000000001"
    );
}

// #22-#24: EVM address normalization - N/A for MVM
// #25-#26: SVM pubkey from hex - N/A for MVM
// #27-#28: SVM escrow parsing - N/A for MVM
