//! Tests for GMP MVM client helper functions.
//!
//! These tests cover the helper functions extracted from `integrated_gmp_relay.rs`
//! into the `mvm_client` module: address normalization, transaction hash extraction,
//! VM status checking, view function byte parsing, and byte array conversion.

mod helpers;

use helpers::DUMMY_TX_HASH;
use integrated_gmp::mvm_client::{
    check_vm_status_success, extract_transaction_hash, normalize_address, parse_view_bytes,
};

// ============================================================================
// ADDRESS NORMALIZATION TESTS
// ============================================================================

/// 1. Test: Normalize Address Adds Prefix
/// Verifies that normalize_address adds a 0x prefix to addresses that are missing it.
/// Why: Some Move VM addresses may be stored without prefix.
#[test]
fn test_normalize_address_adds_prefix() {
    assert_eq!(normalize_address("abc123"), "0xabc123");
}

/// 2. Test: Normalize Address Preserves Existing Prefix
/// Verifies that normalize_address does not double-prefix addresses that already have 0x.
/// Why: Should not double-prefix addresses.
#[test]
fn test_normalize_address_preserves_existing_prefix() {
    assert_eq!(normalize_address("0xabc123"), "0xabc123");
}

// ============================================================================
// TRANSACTION HASH EXTRACTION TESTS
// ============================================================================

/// 3. Test: Extract Transaction Hash From JSON Output
/// Verifies that extract_transaction_hash parses the transaction_hash from JSON-formatted aptos CLI output.
/// Why: Modern aptos CLI outputs JSON with transaction_hash field.
#[test]
fn test_extract_transaction_hash_from_json_output() {
    let output = format!(
        r#"{{"Result":{{"transaction_hash":"{}","success":true}}}}"#,
        DUMMY_TX_HASH
    );

    let hash = extract_transaction_hash(&output);

    assert_eq!(hash, Some(DUMMY_TX_HASH.to_string()));
}

/// 4. Test: Extract Transaction Hash Returns None When Missing
/// Verifies that extract_transaction_hash returns None when no transaction hash is present in the output.
/// Why: Some CLI outputs may not contain a transaction hash.
#[test]
fn test_extract_transaction_hash_returns_none_when_missing() {
    let output = "Some output without a transaction hash";

    let hash = extract_transaction_hash(output);

    assert_eq!(hash, None);
}

// ============================================================================
// VM STATUS CHECKING TESTS
// ============================================================================

/// 5. Test: Check VM Status Success With Result Wrapper
/// Verifies that check_vm_status_success parses success from {"Result":{"success":true}}.
/// Why: Standard aptos CLI format wraps output in a Result object.
#[test]
fn test_check_vm_status_success_result_wrapper() {
    let output = r#"{"Result":{"success":true,"vm_status":"Executed successfully"}}"#;
    assert!(check_vm_status_success(output).unwrap());
}

/// 6. Test: Check VM Status Failure With Result Wrapper
/// Verifies that check_vm_status_success detects failure from {"Result":{"success":false}}.
/// Why: VM execution can fail even when CLI exits with code 0.
#[test]
fn test_check_vm_status_failure_result_wrapper() {
    let output = r#"{"Result":{"success":false,"vm_status":"Move abort"}}"#;
    assert!(!check_vm_status_success(output).unwrap());
}

/// 7. Test: Check VM Status Success Top Level
/// Verifies that check_vm_status_success parses success from top-level {"success":true}.
/// Why: Some output formats don't wrap in Result.
#[test]
fn test_check_vm_status_success_top_level() {
    let output = r#"{"success":true}"#;
    assert!(check_vm_status_success(output).unwrap());
}

// ============================================================================
// PARSE VIEW BYTES TESTS
// ============================================================================

/// 8. Test: Parse View Bytes Hex String
/// Verifies that parse_view_bytes handles hex string format ("0x3c44...").
/// Why: Aptos view functions may return hex strings.
#[test]
fn test_parse_view_bytes_hex_string() {
    let value = serde_json::json!("0x3c44cdddb6a900fa2b585dd299e03d12fa4293bc");
    let result = parse_view_bytes(&value).unwrap();
    assert_eq!(result, "3c44cdddb6a900fa2b585dd299e03d12fa4293bc");
}

/// 9. Test: Parse View Bytes Hex String Without Prefix
/// Verifies that parse_view_bytes handles hex string format without 0x prefix.
/// Why: Some responses omit the prefix.
#[test]
fn test_parse_view_bytes_hex_string_no_prefix() {
    let value = serde_json::json!("abcdef");
    let result = parse_view_bytes(&value).unwrap();
    assert_eq!(result, "abcdef");
}

/// 10. Test: Parse View Bytes JSON Array
/// Verifies that parse_view_bytes handles JSON byte array format (["60", "68"]).
/// Why: Aptos view functions may return byte arrays as JSON arrays of decimal strings.
#[test]
fn test_parse_view_bytes_json_array() {
    let value = serde_json::json!(["1", "2", "255"]);
    let result = parse_view_bytes(&value).unwrap();
    assert_eq!(result, "0102ff");
}

/// 11. Test: Parse View Bytes Empty Array
/// Verifies that parse_view_bytes handles an empty JSON array.
/// Why: Edge case for empty payloads.
#[test]
fn test_parse_view_bytes_empty_array() {
    let value = serde_json::json!([]);
    let result = parse_view_bytes(&value).unwrap();
    assert_eq!(result, "");
}
