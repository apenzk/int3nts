/// Normalizes an intent ID by removing leading zeros after the 0x prefix and converting to lowercase.
///
/// This ensures that intent IDs like "0x0911..." and "0x911..." are treated as the same value.
///
/// # Arguments
///
/// * `intent_id` - The intent ID to normalize (e.g., "0x0911..." or "0x911...")
///
/// # Returns
///
/// Normalized intent ID with 0x prefix, no leading zeros, lowercase (e.g., "0x911...")
pub fn normalize_intent_id(intent_id: &str) -> String {
    let stripped = intent_id.strip_prefix("0x").unwrap_or(intent_id);
    let trimmed = stripped.trim_start_matches('0');
    let hex_part = if trimmed.is_empty() { "0" } else { trimmed };
    format!("0x{}", hex_part.to_lowercase())
}

/// Normalizes an intent ID to 64 hex characters (32 bytes) by padding with leading zeros.
///
/// This ensures that intent IDs can be safely parsed as hex, even if they have an odd number
/// of hex characters or are shorter than 64 characters.
///
/// # Arguments
///
/// * `intent_id` - The intent ID to normalize (e.g., "0xabc..." or "0x0abc...")
///
/// # Returns
///
/// Normalized intent ID with 0x prefix, padded to 64 hex characters, lowercase
pub fn normalize_intent_id_to_64_chars(intent_id: &str) -> String {
    let stripped = intent_id.strip_prefix("0x").unwrap_or(intent_id);
    format!("0x{:0>64}", stripped.to_lowercase())
}
