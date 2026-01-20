//! Unit tests for SVM escrow account parsing
//!
//! These tests validate Borsh serialization/deserialization of SVM escrow data.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use borsh::BorshSerialize;
use solana_program::pubkey::Pubkey;
use verifier::svm_client::EscrowAccount;

/// Test that Borsh-encoded escrow data round-trips through base64 encoding
/// Why: SVM RPC returns base64-encoded account data that must deserialize correctly
#[test]
fn test_escrow_account_borsh_roundtrip() {
    let escrow = EscrowAccount {
        discriminator: [7u8; 8],
        requester: Pubkey::new_from_array([1u8; 32]),
        token_mint: Pubkey::new_from_array([2u8; 32]),
        amount: 42,
        is_claimed: false,
        expiry: 123456,
        reserved_solver: Pubkey::new_from_array([3u8; 32]),
        intent_id: [4u8; 32],
        bump: 255,
    };

    let serialized = escrow.try_to_vec().expect("Serialize escrow");
    let encoded = STANDARD.encode(serialized);

    let decoded = STANDARD.decode(encoded).expect("Decode base64");
    let parsed = EscrowAccount::try_from_slice(&decoded).expect("Deserialize escrow");

    assert_eq!(parsed.requester, escrow.requester);
    assert_eq!(parsed.token_mint, escrow.token_mint);
    assert_eq!(parsed.amount, escrow.amount);
    assert_eq!(parsed.is_claimed, escrow.is_claimed);
    assert_eq!(parsed.expiry, escrow.expiry);
    assert_eq!(parsed.reserved_solver, escrow.reserved_solver);
    assert_eq!(parsed.intent_id, escrow.intent_id);
    assert_eq!(parsed.bump, escrow.bump);
}

/// Test that invalid base64 data fails to parse
/// Why: Malformed RPC responses should not be accepted as valid escrow data
#[test]
fn test_escrow_account_invalid_base64() {
    let bad_base64 = "not_base64";
    assert!(STANDARD.decode(bad_base64).is_err());
}
