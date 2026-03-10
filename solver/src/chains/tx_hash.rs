//! Transaction hash extraction from CLI output
//!
//! Shared utility for parsing transaction hashes from aptos CLI and Hardhat
//! script output. Both MVM and EVM clients use this to avoid duplicated parsing logic.

use anyhow::Result;

/// Extracts a transaction hash from CLI output.
///
/// Tries two strategies in order:
/// 1. JSON parsing — aptos CLI outputs `{"Result": {"transaction_hash": "0x..."}}`
/// 2. Line-based parsing — finds a line containing "hash"/"Hash" and extracts the `0x...` value
///
/// # Arguments
///
/// * `output` - Raw stdout from the CLI command
/// * `context` - Description of the command (used in error messages)
///
/// # Returns
///
/// The extracted `0x`-prefixed transaction hash string.
pub fn extract_tx_hash(output: &str, context: &str) -> Result<String> {
    // Strategy 1: JSON (aptos CLI outputs {"Result": {"transaction_hash": "0x..."}})
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(hash) = json
            .get("Result")
            .and_then(|r| r.get("transaction_hash"))
            .and_then(|h| h.as_str())
        {
            return Ok(hash.to_string());
        }
    }

    // Strategy 2: line-based ("Transaction hash: 0x..." or similar)
    if let Some(hash_line) = output
        .lines()
        .find(|l| l.contains("hash") || l.contains("Hash"))
    {
        // Unquoted: "Transaction hash: 0x1234..."
        if let Some(hash) = hash_line
            .split_whitespace()
            .find(|s| s.starts_with("0x"))
        {
            return Ok(hash.to_string());
        }
        // Quoted: "transaction_hash": "0x1234..."
        if let Some(start) = hash_line.find("\"0x") {
            if let Some(end) = hash_line[start + 1..].find('"') {
                return Ok(hash_line[start + 1..start + 1 + end].to_string());
            }
        }
    }

    anyhow::bail!("Could not extract transaction hash from {} output: {}", context, output)
}
