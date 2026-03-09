use chain_clients_common::{normalize_intent_id, normalize_intent_id_to_64_chars};

// ============================================================================
// NORMALIZE_INTENT_ID TESTS
// ============================================================================

/// 1. Test: normalize_intent_id strips leading zeros
/// Verifies that intent IDs with leading zeros are normalized to match those without.
/// Why: EVM and Move VM may format the same intent_id differently (with/without leading zeros).
#[test]
fn test_normalize_intent_id_strips_leading_zeros() {
    // Synthetic 63-char hex (odd length triggers the leading-zero difference)
    let with_leading_zero =
        "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde";
    let without_leading_zero =
        "0x123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde";

    let normalized_with = normalize_intent_id(with_leading_zero);
    let normalized_without = normalize_intent_id(without_leading_zero);

    assert_eq!(
        normalized_with, normalized_without,
        "Intent IDs with and without leading zeros should normalize to the same value"
    );
    assert_eq!(
        normalized_with,
        "0x123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde"
    );
}

/// 2. Test: normalize_intent_id lowercases
/// Verifies that uppercase hex characters are normalized to lowercase.
/// Why: Ensures consistent comparison regardless of input case.
#[test]
fn test_normalize_intent_id_lowercases() {
    assert_eq!(normalize_intent_id("0xABCDEF"), "0xabcdef");
    assert_eq!(normalize_intent_id("0xabcdef"), "0xabcdef");
    assert_eq!(
        normalize_intent_id("0xAbCdEf123"),
        "0xabcdef123"
    );
}

/// 3. Test: normalize_intent_id all zeros
/// Verifies that all-zero intent IDs are normalized to "0x0".
/// Why: Edge case where all hex digits are zero must produce a valid normalized result.
#[test]
fn test_normalize_intent_id_all_zeros() {
    assert_eq!(normalize_intent_id("0x0000"), "0x0");
    assert_eq!(normalize_intent_id("0x0"), "0x0");
    assert_eq!(
        normalize_intent_id("0x0000000000000000000000000000000000000000000000000000000000000000"),
        "0x0"
    );
}

/// 4. Test: normalize_intent_id no prefix
/// Verifies that intent IDs without 0x prefix are handled correctly.
/// Why: Some sources may omit the 0x prefix; normalization must still work.
#[test]
fn test_normalize_intent_id_no_prefix() {
    assert_eq!(normalize_intent_id("abcdef"), "0xabcdef");
    assert_eq!(normalize_intent_id("00abcdef"), "0xabcdef");
    assert_eq!(normalize_intent_id("0000"), "0x0");
}

// ============================================================================
// NORMALIZE_INTENT_ID_TO_64_CHARS TESTS
// ============================================================================

/// 5. Test: normalize_intent_id_to_64_chars pads
/// Verifies that short intent IDs are padded to 64 hex characters with leading zeros.
/// Why: 32-byte hex representation is required for safe cross-chain parsing.
#[test]
fn test_normalize_intent_id_to_64_chars_pads() {
    let short_id = "0xabc";
    let result = normalize_intent_id_to_64_chars(short_id);
    assert_eq!(
        result,
        "0x0000000000000000000000000000000000000000000000000000000000000abc"
    );

    // Already 64 chars — no padding needed
    let full_id = "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let result = normalize_intent_id_to_64_chars(full_id);
    assert_eq!(
        result,
        "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
}

/// 6. Test: normalize_intent_id_to_64_chars lowercases
/// Verifies that uppercase hex characters are lowercased during padding.
/// Why: Consistent casing prevents comparison mismatches across chains.
#[test]
fn test_normalize_intent_id_to_64_chars_lowercases() {
    let result = normalize_intent_id_to_64_chars("0xABCDEF");
    assert_eq!(
        result,
        "0x0000000000000000000000000000000000000000000000000000000000abcdef"
    );
}

/// 7. Test: normalize_intent_id_to_64_chars no prefix
/// Verifies that intent IDs without 0x prefix are padded correctly.
/// Why: Some sources may omit the 0x prefix; padding must still work.
#[test]
fn test_normalize_intent_id_to_64_chars_no_prefix() {
    let result = normalize_intent_id_to_64_chars("abcdef");
    assert_eq!(
        result,
        "0x0000000000000000000000000000000000000000000000000000000000abcdef"
    );
}
