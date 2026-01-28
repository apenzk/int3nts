//! Unit tests for SVM outflow fulfillment parsing

use serde_json::json;
use solana_program::pubkey::Pubkey;
use std::str::FromStr;
use trusted_gmp::validator::extract_svm_fulfillment_params;

#[path = "mod.rs"]
mod test_helpers;
use test_helpers::DUMMY_INTENT_ID_FULL;

const MEMO_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
// Use deterministic pubkeys derived from fixed byte patterns.

fn hex_pubkey(pubkey: &str) -> String {
    format!(
        "0x{}",
        hex::encode(Pubkey::from_str(pubkey).unwrap().to_bytes())
    )
}

// Helper for deterministic, valid base58 pubkeys in tests.
fn test_pubkey(byte: u8) -> String {
    Pubkey::new_from_array([byte; 32]).to_string()
}

fn build_tx(memo_first: bool, memo: &str) -> serde_json::Value {
    let authority = test_pubkey(1);
    let destination = test_pubkey(2);
    let source = test_pubkey(3);
    let mint = test_pubkey(4);

    let memo_ix = json!({
        "program": "spl-memo",
        "programId": MEMO_PROGRAM_ID,
        "parsed": memo,
    });
    let transfer_ix = json!({
        "program": "spl-token",
        "parsed": {
            "type": "transferChecked",
            "info": {
                "source": source,
                "destination": destination,
                "authority": authority,
                "mint": mint,
                "amount": "1000"
            }
        }
    });

    let instructions = if memo_first {
        vec![memo_ix, transfer_ix]
    } else {
        vec![transfer_ix, memo_ix]
    };

    json!({
        "transaction": {
            "message": {
                "accountKeys": [
                    { "pubkey": authority, "signer": true },
                    { "pubkey": destination, "signer": false }
                ],
                "instructions": instructions
            }
        }
    })
}

/// What is tested: Successful parsing of memo + transferChecked
/// Why: Ensure SVM outflow parsing returns normalized fulfillment parameters
#[test]
fn test_extract_svm_fulfillment_params_success() {
    let intent_id = DUMMY_INTENT_ID_FULL;
    let tx = build_tx(true, &format!("intent_id={}", intent_id));

    let params = extract_svm_fulfillment_params(&tx).unwrap();
    assert_eq!(params.intent_id, intent_id);
    assert_eq!(params.recipient_addr, hex_pubkey(&test_pubkey(2)));
    assert_eq!(params.solver_addr, hex_pubkey(&test_pubkey(1)));
    assert_eq!(params.token_metadata, hex_pubkey(&test_pubkey(4)));
    assert_eq!(params.amount, 1000);
}

/// What is tested: Memo must be the first instruction
/// Why: Enforce strict memo + transfer ordering for outflow validation
#[test]
fn test_extract_svm_fulfillment_params_requires_memo_first() {
    let intent_id = DUMMY_INTENT_ID_FULL;
    let tx = build_tx(false, &format!("intent_id={}", intent_id));

    let err = extract_svm_fulfillment_params(&tx).unwrap_err();
    assert!(err
        .to_string()
        .contains("memo must be the first instruction"));
}

/// What is tested: Memo format validation for intent_id length
/// Why: Prevent malformed memo values from being accepted
#[test]
fn test_extract_svm_fulfillment_params_rejects_invalid_intent_id() {
    let tx = build_tx(true, "intent_id=0x1234");

    let err = extract_svm_fulfillment_params(&tx).unwrap_err();
    assert!(err.to_string().contains("Intent ID must be 32 bytes"));
}
