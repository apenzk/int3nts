//! Tests for integrated GMP relay generic helper functions.
//!
//! These tests cover the generic helper functions used by the relay:
//! address parsing, Ed25519 keypair conversion, hex-to-bytes, and delivery retry tracking.
//!
//! VM-specific tests are in relay_vm_tests.rs.

mod helpers;

use helpers::{DUMMY_INTENT_ID, DUMMY_SOLVER_ADDR_HUB};
use solana_sdk::signature::Keypair;
use integrated_gmp::integrated_gmp_relay::{
    ed25519_seed_to_keypair_bytes, hex_to_bytes,
    parse_32_byte_address, DeliveryAttempt,
};
use integrated_gmp::MAX_DELIVERY_RETRIES;

// ============================================================================
// ADDRESS PARSING TESTS
// ============================================================================

/// 1. Test: Parse Full-Length 64-Character Hex Address
/// Verifies that parse_32_byte_address correctly parses a full 64-character hex address using DUMMY_SOLVER_ADDR_HUB.
/// Why: This is the standard Move VM address format used throughout the codebase.
#[test]
fn test_parse_32_byte_address_full_length() {
    let result = parse_32_byte_address(DUMMY_SOLVER_ADDR_HUB).unwrap();

    assert_eq!(result.len(), 32, "Result should be exactly 32 bytes");
    // DUMMY_SOLVER_ADDR_HUB = "0x0000000000000000000000000000000000000000000000000000000000000007"
    // All zeros except last byte which is 0x07
    assert_eq!(result[0], 0x00, "First byte should be 0x00");
    assert_eq!(result[31], 0x07, "Last byte should match constant (0x07)");
}

/// 2. Test: Parse Intent ID Address
/// Verifies that parse_32_byte_address correctly parses DUMMY_INTENT_ID which has a different last byte.
/// Why: Verify consistent behavior across different test constants.
#[test]
fn test_parse_32_byte_address_intent_id() {
    let result = parse_32_byte_address(DUMMY_INTENT_ID).unwrap();

    assert_eq!(result.len(), 32, "Result should be exactly 32 bytes");
    // DUMMY_INTENT_ID = "0x0000000000000000000000000000000000000000000000000000000000000001"
    assert_eq!(result[0], 0x00, "First byte should be 0x00");
    assert_eq!(result[31], 0x01, "Last byte should match constant (0x01)");
}

/// 3. Test: Parse Short Address With Leading Zero Restoration
/// Verifies that parse_32_byte_address correctly left-pads a short address to 32 bytes.
/// Why: Move VM may strip leading zeros; we must restore them for 32-byte SVM pubkeys.
#[test]
fn test_parse_32_byte_address_restores_leading_zeros() {
    // Simulate a Move address with stripped leading zeros (just "1" = 1 hex char)
    let short_addr = "0x1";
    let result = parse_32_byte_address(short_addr).unwrap();

    assert_eq!(result.len(), 32, "Result should be exactly 32 bytes");
    // Should be left-padded with 31 zero bytes, then 0x01
    for i in 0..31 {
        assert_eq!(result[i], 0x00, "Byte {} should be padded zero", i);
    }
    assert_eq!(result[31], 0x01, "Last byte should be 0x01");
}

/// 4. Test: Parse Two-Byte Address With Significant Padding
/// Verifies that parse_32_byte_address correctly pads a 4-hex-char (2 byte) address to 32 bytes.
/// Why: Verify significant padding works correctly.
#[test]
fn test_parse_32_byte_address_two_bytes_input() {
    // 4 hex chars = 2 bytes
    let addr = "0xabcd";
    let result = parse_32_byte_address(addr).unwrap();

    assert_eq!(result.len(), 32, "Result should be exactly 32 bytes");
    // First 30 bytes should be zeros, then 0xab, 0xcd
    for i in 0..30 {
        assert_eq!(result[i], 0x00, "Byte {} should be padded zero", i);
    }
    assert_eq!(result[30], 0xab, "Second-to-last byte should be 0xab");
    assert_eq!(result[31], 0xcd, "Last byte should be 0xcd");
}

// ============================================================================
// ED25519 KEYPAIR CONVERSION TESTS
// ============================================================================

/// 5. Test: Ed25519 Seed To Keypair Bytes Produces Valid Keypair
/// Verifies that ed25519_seed_to_keypair_bytes converts a 32-byte seed into a valid 64-byte Solana keypair.
/// Why: integrated-gmp uses Ed25519 keys; Solana SDK expects 64-byte keypairs.
#[test]
fn test_ed25519_seed_to_keypair_bytes_produces_valid_keypair() {
    // Use a deterministic seed for reproducible tests
    let seed = [1u8; 32];
    let keypair_bytes = ed25519_seed_to_keypair_bytes(&seed).unwrap();

    assert_eq!(keypair_bytes.len(), 64, "Keypair should be 64 bytes");
    assert_eq!(
        &keypair_bytes[..32],
        &seed,
        "First 32 bytes should be the seed"
    );
    assert_ne!(
        &keypair_bytes[32..],
        &[0u8; 32],
        "Public key portion should not be all zeros"
    );

    // Verify the result can create a valid Solana Keypair
    let keypair = Keypair::try_from(keypair_bytes.as_slice()).expect("Should create valid Keypair");
    assert_eq!(keypair.to_bytes(), keypair_bytes);
}

/// 6. Test: Ed25519 Seed To Keypair Bytes Rejects Invalid Length
/// Verifies that ed25519_seed_to_keypair_bytes returns an error for seeds that are not exactly 32 bytes.
/// Why: Ed25519 seeds must be exactly 32 bytes.
#[test]
fn test_ed25519_seed_to_keypair_bytes_rejects_invalid_length() {
    let short_seed = [1u8; 16]; // Only 16 bytes, should fail
    let result = ed25519_seed_to_keypair_bytes(&short_seed);

    assert!(result.is_err(), "Should reject seed that isn't 32 bytes");
}

// ============================================================================
// HEX TO BYTES TESTS
// ============================================================================

/// 7. Test: Hex To Bytes With 0x Prefix
/// Verifies that hex_to_bytes correctly decodes a hex string with 0x prefix into bytes.
/// Why: Most addresses and payloads use 0x prefix.
#[test]
fn test_hex_to_bytes_with_prefix() {
    let bytes = hex_to_bytes("0x0102ff").unwrap();

    assert_eq!(bytes, vec![1, 2, 255]);
}

/// 8. Test: Hex To Bytes Without Prefix
/// Verifies that hex_to_bytes correctly decodes a hex string without 0x prefix into bytes.
/// Why: Some inputs may omit the prefix.
#[test]
fn test_hex_to_bytes_without_prefix() {
    let bytes = hex_to_bytes("0102ff").unwrap();

    assert_eq!(bytes, vec![1, 2, 255]);
}

// ============================================================================
// DELIVERY RETRY TRACKING TESTS
// ============================================================================

/// 9. Test: DeliveryAttempt record_failure increments count and sets backoff
/// Why: First failure must not be terminal — relay must retry with backoff
#[test]
fn test_delivery_attempt_first_failure_sets_backoff() {
    let mut attempt = DeliveryAttempt { count: 0, next_retry_after: 0 };
    let exhausted = attempt.record_failure();

    assert!(!exhausted, "First failure should not exhaust retries");
    assert_eq!(attempt.count, 1);
    assert!(attempt.next_retry_after > 0, "Backoff should be set");
}

/// 10. Test: DeliveryAttempt transitions to exhausted after MAX_DELIVERY_RETRIES
/// Why: After max retries, message must be permanently skipped
#[test]
fn test_delivery_attempt_exhausted_after_max_retries() {
    let mut attempt = DeliveryAttempt { count: 0, next_retry_after: 0 };

    for i in 0..MAX_DELIVERY_RETRIES {
        let exhausted = attempt.record_failure();
        if i + 1 < MAX_DELIVERY_RETRIES {
            assert!(!exhausted, "Attempt {} should not exhaust retries", i + 1);
        } else {
            assert!(exhausted, "Attempt {} should exhaust retries", i + 1);
        }
    }

    assert_eq!(attempt.count, MAX_DELIVERY_RETRIES);
    assert!(attempt.is_exhausted());
}

/// 11. Test: DeliveryAttempt backoff increases with each retry
/// Why: Backoff must increase to avoid hammering a failing chain
#[test]
fn test_delivery_attempt_backoff_increases() {
    let mut attempt = DeliveryAttempt { count: 0, next_retry_after: 0 };

    attempt.record_failure();
    let first_retry_after = attempt.next_retry_after;

    attempt.record_failure();
    let second_retry_after = attempt.next_retry_after;

    assert!(second_retry_after > first_retry_after, "Backoff should increase with each retry");
}

/// 12. Test: DeliveryAttempt is_exhausted returns false when under limit
/// Why: Should only be true after max retries
#[test]
fn test_delivery_attempt_not_exhausted_under_limit() {
    let attempt = DeliveryAttempt { count: MAX_DELIVERY_RETRIES - 1, next_retry_after: 0 };
    assert!(!attempt.is_exhausted());
}
