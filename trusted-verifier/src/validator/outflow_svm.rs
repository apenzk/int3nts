//! Outflow SVM-specific fulfillment parsing
//!
//! Parses SVM transactions to extract intent_id memo details and SPL token transfer metadata.

use anyhow::{Context, Result};
use solana_program::pubkey::Pubkey;
use std::str::FromStr;

use crate::validator::generic::FulfillmentTransactionParams;

// SPL Memo program id from Solana program registry (mainnet/devnet).
// https://spl.solana.com/memo
const MEMO_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
const TOKEN_PROGRAM: &str = "spl-token";

#[derive(Debug)]
struct ParsedTransfer {
    destination: String,
    authority: String,
    mint: String,
    amount: u64,
}

/// Extracts fulfillment parameters from an SVM transaction.
///
/// # Requirements
///
/// - The first instruction must be an SPL memo with `intent_id=0x...`.
/// - The transaction must include exactly one `transferChecked` instruction.
/// - The transfer authority must be a signer in the transaction.
pub fn extract_svm_fulfillment_params(tx: &serde_json::Value) -> Result<FulfillmentTransactionParams> {
    let message = tx
        .get("transaction")
        .and_then(|t| t.get("message"))
        .context("Missing transaction message")?;
    let instructions = message
        .get("instructions")
        .and_then(|i| i.as_array())
        .context("Missing instructions")?;

    let (memo_index, memo) = extract_memo(instructions)?;
    if memo_index != 0 {
        anyhow::bail!("SVM memo must be the first instruction");
    }
    let intent_id = parse_intent_id(&memo)?;

    let transfer = extract_transfer_checked(instructions)?;
    if !is_signer(message, &transfer.authority) {
        anyhow::bail!("SVM transfer authority is not a signer");
    }

    Ok(FulfillmentTransactionParams {
        intent_id,
        recipient_addr: pubkey_to_hex(&transfer.destination)?,
        solver_addr: pubkey_to_hex(&transfer.authority)?,
        amount: transfer.amount,
        token_metadata: pubkey_to_hex(&transfer.mint)?,
    })
}

/// Extracts the memo instruction index and memo string.
///
/// # Arguments
///
/// * `instructions` - Parsed transaction instructions
///
/// # Returns
///
/// * `Ok((usize, String))` - Memo instruction index and memo contents
/// * `Err(anyhow::Error)` - Memo missing or multiple memos found
fn extract_memo(instructions: &[serde_json::Value]) -> Result<(usize, String)> {
    let mut memo_matches = Vec::new();
    for (index, instruction) in instructions.iter().enumerate() {
        if let Some(memo) = parse_memo_instruction(instruction) {
            memo_matches.push((index, memo));
        }
    }
    if memo_matches.len() != 1 {
        anyhow::bail!("SVM transaction must contain exactly one memo instruction");
    }
    Ok(memo_matches.remove(0))
}

/// Parses a memo instruction and returns its content if present.
///
/// # Arguments
///
/// * `instruction` - Parsed instruction value
///
/// # Returns
///
/// * `Some(String)` - Memo content
/// * `None` - Not a memo instruction
fn parse_memo_instruction(instruction: &serde_json::Value) -> Option<String> {
    let program = instruction.get("program").and_then(|p| p.as_str());
    let program_id = instruction.get("programId").and_then(|p| p.as_str());
    if program != Some("spl-memo") && program_id != Some(MEMO_PROGRAM_ID) {
        return None;
    }

    if let Some(parsed) = instruction.get("parsed") {
        if let Some(memo) = parsed.as_str() {
            return Some(memo.to_string());
        }
        if let Some(memo) = parsed
            .get("info")
            .and_then(|info| info.get("memo"))
            .and_then(|memo| memo.as_str())
        {
            return Some(memo.to_string());
        }
    }

    None
}

/// Extracts the single transferChecked instruction.
///
/// # Arguments
///
/// * `instructions` - Parsed transaction instructions
///
/// # Returns
///
/// * `Ok(ParsedTransfer)` - Parsed transfer details
/// * `Err(anyhow::Error)` - Missing or multiple transferChecked instructions
fn extract_transfer_checked(instructions: &[serde_json::Value]) -> Result<ParsedTransfer> {
    let mut transfers = Vec::new();
    for instruction in instructions {
        let program = instruction.get("program").and_then(|p| p.as_str());
        if program != Some(TOKEN_PROGRAM) {
            continue;
        }
        if let Some(parsed) = instruction.get("parsed") {
            let instruction_type = parsed.get("type").and_then(|t| t.as_str());
            if instruction_type != Some("transferChecked") {
                continue;
            }
            if let Some(info) = parsed.get("info") {
                let amount = parse_amount(info)
                    .context("Missing transferChecked amount")?;
                let destination = info
                    .get("destination")
                    .and_then(|v| v.as_str())
                    .context("Missing transferChecked destination")?;
                let authority = info
                    .get("authority")
                    .and_then(|v| v.as_str())
                    .context("Missing transferChecked authority")?;
                let mint = info
                    .get("mint")
                    .and_then(|v| v.as_str())
                    .context("Missing transferChecked mint")?;
                transfers.push(ParsedTransfer {
                    destination: destination.to_string(),
                    authority: authority.to_string(),
                    mint: mint.to_string(),
                    amount,
                });
            }
        }
    }

    if transfers.len() != 1 {
        anyhow::bail!("SVM transaction must contain exactly one transferChecked instruction");
    }
    Ok(transfers.remove(0))
}

/// Parses the transfer amount from a transferChecked info object.
///
/// # Arguments
///
/// * `info` - Parsed transfer info
///
/// # Returns
///
/// * `Ok(u64)` - Amount in base units
/// * `Err(anyhow::Error)` - Amount missing or invalid
fn parse_amount(info: &serde_json::Value) -> Result<u64> {
    if let Some(amount) = info.get("amount") {
        if let Some(amount_str) = amount.as_str() {
            return amount_str
                .parse::<u64>()
                .context("Invalid transfer amount");
        }
        if let Some(amount_num) = amount.as_u64() {
            return Ok(amount_num);
        }
    };
    if let Some(token_amount) = info.get("tokenAmount") {
        if let Some(amount_str) = token_amount.get("amount").and_then(|v| v.as_str()) {
            return amount_str
                .parse::<u64>()
                .context("Invalid transfer amount");
        }
    };
    anyhow::bail!("Missing transfer amount");
}

/// Parses and validates the intent_id memo value.
///
/// # Arguments
///
/// * `memo` - Memo string contents
///
/// # Returns
///
/// * `Ok(String)` - Normalized intent_id (0x-prefixed)
/// * `Err(anyhow::Error)` - Memo format or hex invalid
fn parse_intent_id(memo: &str) -> Result<String> {
    let memo = memo.trim();
    let rest = memo
        .strip_prefix("intent_id=")
        .ok_or_else(|| anyhow::anyhow!("Invalid memo format (expected intent_id=0x...)"))?;
    let intent_id = rest
        .strip_prefix("0x")
        .ok_or_else(|| anyhow::anyhow!("Intent ID must be 0x-prefixed"))?;
    if intent_id.len() != 64 {
        anyhow::bail!("Intent ID must be 32 bytes (64 hex chars)");
    }
    hex::decode(intent_id).context("Invalid intent_id hex")?;
    Ok(format!("0x{}", intent_id))
}

/// Checks if an account key is marked as a signer in the transaction message.
///
/// # Arguments
///
/// * `message` - Transaction message object
/// * `signer` - Expected signer pubkey
///
/// # Returns
///
/// * `true` - Signer is present and marked as signer
/// * `false` - Signer missing or not marked
fn is_signer(message: &serde_json::Value, signer: &str) -> bool {
    let Some(keys) = message.get("accountKeys").and_then(|v| v.as_array()) else {
        return false;
    };

    for key in keys {
        if let Some(pubkey) = key.get("pubkey").and_then(|v| v.as_str()) {
            let is_signer = key
                .get("signer")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if pubkey == signer && is_signer {
                return true;
            }
        }
    }
    false
}

fn pubkey_to_hex(pubkey_str: &str) -> Result<String> {
    let pubkey = Pubkey::from_str(pubkey_str)
        .context("Invalid pubkey string")?;
    Ok(format!("0x{}", hex::encode(pubkey.to_bytes())))
}
