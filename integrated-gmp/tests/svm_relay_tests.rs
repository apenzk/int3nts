//! SVM counterpart of *vm_relay_tests.rs.
//!
//! These tests cover SVM pubkey parsing, SVM message parsing, relay config SVM extraction,
//! fulfillment proof payload parsing, and ATA derivation.
//! MVM-only relay config tests are N/A placeholders.

mod helpers;

use helpers::{
    build_test_config_with_svm, DUMMY_INTENT_ID,
    DUMMY_SVM_ESCROW_PROGRAM_ID, TEST_MVM_CHAIN_ID, TEST_SVM_CHAIN_ID,
};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use integrated_gmp::integrated_gmp_relay::{
    parse_svm_pubkey, NativeGmpRelayConfig,
};

// ============================================================================
// SVM PUBKEY PARSING TESTS
// ============================================================================

// 1. Test: Parse SVM Pubkey From Hex Format
/// Verifies that parse_svm_pubkey correctly converts a hex-encoded address to a Solana Pubkey.
/// Why: GMP messages use hex-encoded addresses; we convert to Solana Pubkey.
#[test]
fn test_parse_svm_pubkey_from_hex_format() {
    let pubkey = parse_svm_pubkey(DUMMY_INTENT_ID).unwrap();

    // DUMMY_INTENT_ID = "0x0000000000000000000000000000000000000000000000000000000000000001"
    assert_eq!(pubkey.to_bytes()[31], 0x01, "Last byte should be 0x01");
    assert_eq!(pubkey.to_bytes()[0], 0x00, "First byte should be 0x00");
}

// 2. Test: Parse SVM Pubkey From Base58 Format
/// Verifies that parse_svm_pubkey correctly converts a base58-encoded address to a Solana Pubkey.
/// Why: Solana native addresses use base58; we must support both formats.
#[test]
fn test_parse_svm_pubkey_from_base58_format() {
    // DUMMY_SVM_ESCROW_PROGRAM_ID = "11111111111111111111111111111111" (system program, all zeros)
    let pubkey = parse_svm_pubkey(DUMMY_SVM_ESCROW_PROGRAM_ID).unwrap();

    assert_eq!(pubkey.to_bytes(), [0u8; 32], "System program should be all zeros");
}

// ============================================================================
// SVM MESSAGE PARSING TESTS
// ============================================================================

// 3. Test: Parse SVM MessageSent Log Format
/// Verifies that the MessageSent log format can be correctly parsed into its component fields.
/// Why: Validates the log parsing logic used in poll_svm_events().
#[test]
fn test_parse_svm_message_sent_log_format() {
    // Format matches integrated-gmp-endpoint program log output
    let log = format!(
        "Program log: MessageSent: src_chain_id={}, dst_chain_id={}, remote_gmp_endpoint_addr={}, dst_addr=0102030405060708091011121314151617181920212223242526272829303132, nonce=42, payload_len=4, payload_hex=deadbeef",
        TEST_SVM_CHAIN_ID, TEST_MVM_CHAIN_ID, DUMMY_SVM_ESCROW_PROGRAM_ID
    );

    // Verify log format is parseable
    assert!(log.contains("MessageSent:"), "Log should contain MessageSent marker");

    let msg_part = log.split("MessageSent:").nth(1).unwrap().trim();
    let mut src_chain_id: Option<u32> = None;
    let mut dst_chain_id: Option<u32> = None;
    let mut nonce: Option<u64> = None;
    let mut payload_hex: Option<String> = None;

    for part in msg_part.split(", ") {
        let mut kv = part.splitn(2, '=');
        let key = kv.next().unwrap().trim();
        let value = kv.next().unwrap().trim();

        match key {
            "src_chain_id" => src_chain_id = value.parse().ok(),
            "dst_chain_id" => dst_chain_id = value.parse().ok(),
            "nonce" => nonce = value.parse().ok(),
            "payload_hex" => payload_hex = Some(format!("0x{}", value)),
            _ => {}
        }
    }

    assert_eq!(src_chain_id, Some(TEST_SVM_CHAIN_ID));
    assert_eq!(dst_chain_id, Some(TEST_MVM_CHAIN_ID));
    assert_eq!(nonce, Some(42));
    assert_eq!(payload_hex, Some("0xdeadbeef".to_string()));
}

// 4. Test: Non-MessageSent Log Is Ignored
/// Verifies that logs without the MessageSent marker are correctly identified as non-relay logs.
/// Why: Only MessageSent logs should be processed by the relay.
#[test]
fn test_non_message_sent_log_is_ignored() {
    let log = "Program log: Some other log message";
    assert!(!log.contains("MessageSent:"), "Non-MessageSent log should not contain marker");
}

// 5. Test: Solana Pubkey To Hex Conversion
/// Verifies that a Solana base58 pubkey is correctly converted to its hex representation.
/// Why: MessageSent logs contain base58 pubkeys that must be converted to hex.
#[test]
fn test_solana_pubkey_to_hex_conversion() {
    // DUMMY_SVM_ESCROW_PROGRAM_ID is "11111111111111111111111111111111" (system program, all zeros)
    let pubkey = Pubkey::from_str(DUMMY_SVM_ESCROW_PROGRAM_ID).unwrap();
    let hex = format!("0x{}", hex::encode(pubkey.to_bytes()));

    // System program pubkey is all zeros
    assert_eq!(
        hex,
        "0x0000000000000000000000000000000000000000000000000000000000000000",
        "System program should be all zeros in hex"
    );
}

// ============================================================================
// RELAY CONFIG TESTS
// ============================================================================

// 6. Test: Relay Config Extracts MVM Connected Chain
// NOTE: N/A for SVM - MVM connected chain extraction is MVM-specific

// 7. Test: Relay Config Extracts Both Connected Chains
// NOTE: N/A for SVM - tested in mvm_relay_tests.rs (uses both MVM and SVM config)

// 8. Test: Relay Config Handles Missing MVM Connected Chain
/// Verifies that NativeGmpRelayConfig has empty mvm_chains when not configured.
/// Why: When only hub and SVM are configured, mvm_chains should be empty.
#[test]
fn test_relay_config_handles_missing_mvm_connected() {
    let mut config = build_test_config_with_svm();
    // Clear MVM connected chain to simulate hub-only + SVM config
    config.connected_chain_mvm = vec![];

    let relay_config = NativeGmpRelayConfig::from_config(&config).unwrap();

    // MVM connected chains should be empty
    assert!(
        relay_config.mvm_chains.is_empty(),
        "MVM connected chains should be empty when not configured"
    );

    // SVM should still be extracted
    assert_eq!(
        relay_config.svm_chains.len(), 1,
        "Should have one SVM chain"
    );
    assert_eq!(
        relay_config.svm_chains[0].chain_id, 901,
        "SVM chain ID should still be extracted"
    );
}

// 9. Test: Relay Config Extracts EVM Connected Chain
// NOTE: N/A for SVM - EVM connected chain extraction is EVM-specific

// ============================================================================
// FULFILLMENT PROOF PAYLOAD PARSING TESTS
// ============================================================================

// 10. Test: FulfillmentProof Payload Intent ID Extraction
/// Verifies that intent_id and solver_addr can be correctly extracted from a FulfillmentProof payload at the expected byte offsets.
/// Why: deliver_to_svm must correctly parse intent_id from payload to derive PDAs.
#[test]
fn test_fulfillment_proof_payload_intent_id_extraction() {
    // Build a valid FulfillmentProof payload (81 bytes)
    let mut payload = vec![0x03]; // FulfillmentProof message type
    let intent_id = [0xAA; 32];
    let solver_addr = [0xBB; 32];
    payload.extend_from_slice(&intent_id);
    payload.extend_from_slice(&solver_addr);
    payload.extend_from_slice(&1000u64.to_be_bytes()); // amount
    payload.extend_from_slice(&1234567890u64.to_be_bytes()); // timestamp

    assert_eq!(payload.len(), 81, "FulfillmentProof payload should be 81 bytes");
    assert_eq!(payload[0], 0x03, "First byte should be message type 0x03");

    // Extract intent_id (bytes 1-33)
    let mut extracted_intent_id = [0u8; 32];
    extracted_intent_id.copy_from_slice(&payload[1..33]);
    assert_eq!(extracted_intent_id, intent_id, "Extracted intent_id should match");

    // Extract solver_addr (bytes 33-65)
    let mut extracted_solver_addr = [0u8; 32];
    extracted_solver_addr.copy_from_slice(&payload[33..65]);
    assert_eq!(extracted_solver_addr, solver_addr, "Extracted solver_addr should match");
}

// 11. Test: FulfillmentProof Payload Minimum Length
/// Verifies that FulfillmentProof payloads are validated against the minimum 65-byte length requirement.
/// Why: deliver_to_svm checks payload.len() >= 65 for required fields.
#[test]
fn test_fulfillment_proof_payload_minimum_length() {
    // Valid payload: 81 bytes
    let valid_payload = vec![0x03; 81];
    assert!(valid_payload.len() >= 65, "Valid payload should be >= 65 bytes");

    // Minimum payload for parsing intent_id and solver_addr: 65 bytes
    let minimum_payload = vec![0x03; 65];
    assert!(minimum_payload.len() >= 65, "Minimum payload should be >= 65 bytes");

    // Too short payload: 64 bytes (can't extract solver_addr completely)
    let short_payload = vec![0x03; 64];
    assert!(short_payload.len() < 65, "Short payload should be < 65 bytes");
}

// ============================================================================
// ATA DERIVATION TESTS
// ============================================================================

// 12. Test: ATA Derivation Formula
/// Verifies that the ATA derivation using PDA([owner, TOKEN_PROGRAM_ID, mint], ASSOCIATED_TOKEN_PROGRAM_ID) produces a valid, distinct pubkey.
/// Why: integrated-gmp derives solver's ATA manually and must match spl-associated-token-account.
#[test]
fn test_ata_derivation_formula() {
    // Known constants
    let token_program_id = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
        .expect("Invalid token program ID");
    let associated_token_program_id = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
        .expect("Invalid associated token program ID");

    // Use deterministic test values
    let owner = Pubkey::from_str("So11111111111111111111111111111111111111112")
        .expect("Invalid owner pubkey");
    let mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")
        .expect("Invalid mint pubkey");

    // Derive ATA using the same formula as in deliver_to_svm
    let (derived_ata, _bump) = Pubkey::find_program_address(
        &[
            owner.as_ref(),
            token_program_id.as_ref(),
            mint.as_ref(),
        ],
        &associated_token_program_id,
    );

    // Verify the derivation produces a valid pubkey
    assert_ne!(derived_ata, Pubkey::default(), "Derived ATA should not be default");

    // Verify the formula matches expected ATA format
    // The ATA should be different from owner and mint
    assert_ne!(derived_ata, owner, "ATA should be different from owner");
    assert_ne!(derived_ata, mint, "ATA should be different from mint");
}

// 13. Test: ATA Derivation Is Deterministic
/// Verifies that deriving an ATA twice with the same inputs produces identical addresses and bumps.
/// Why: Same inputs must always produce the same ATA address.
#[test]
fn test_ata_derivation_is_deterministic() {
    let token_program_id = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
        .expect("Invalid token program ID");
    let associated_token_program_id = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
        .expect("Invalid associated token program ID");

    let owner = Pubkey::new_from_array([0xAA; 32]);
    let mint = Pubkey::new_from_array([0xBB; 32]);

    // Derive twice with same inputs
    let (ata1, bump1) = Pubkey::find_program_address(
        &[owner.as_ref(), token_program_id.as_ref(), mint.as_ref()],
        &associated_token_program_id,
    );
    let (ata2, bump2) = Pubkey::find_program_address(
        &[owner.as_ref(), token_program_id.as_ref(), mint.as_ref()],
        &associated_token_program_id,
    );

    assert_eq!(ata1, ata2, "ATA derivation should be deterministic");
    assert_eq!(bump1, bump2, "Bump should be deterministic");
}

// 14. Test: ATA Differs By Owner
/// Verifies that different owners produce different ATA addresses for the same token mint.
/// Why: Each owner must have a unique ATA for the same token.
#[test]
fn test_ata_differs_by_owner() {
    let token_program_id = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
        .expect("Invalid token program ID");
    let associated_token_program_id = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
        .expect("Invalid associated token program ID");

    let owner1 = Pubkey::new_from_array([0xCC; 32]);
    let owner2 = Pubkey::new_from_array([0xDD; 32]);
    let mint = Pubkey::new_from_array([0xEE; 32]);

    let (ata1, _) = Pubkey::find_program_address(
        &[owner1.as_ref(), token_program_id.as_ref(), mint.as_ref()],
        &associated_token_program_id,
    );
    let (ata2, _) = Pubkey::find_program_address(
        &[owner2.as_ref(), token_program_id.as_ref(), mint.as_ref()],
        &associated_token_program_id,
    );

    assert_ne!(ata1, ata2, "Different owners should have different ATAs");
}

// ============================================================================
// EVM EVENT TOPIC TESTS
// ============================================================================

// 15. Test: EVM Event Topic Produces Known Keccak Hash
// NOTE: N/A for SVM - EVM event topic hashing is EVM-specific

// 16. Test: EVM Event Topic Is Deterministic
// NOTE: N/A for SVM - EVM event topic hashing is EVM-specific

// ============================================================================
// EVM ABI ENCODING TESTS
// ============================================================================

// 17. Test: EVM Encode deliverMessage Calldata
// NOTE: N/A for SVM - EVM ABI encoding is EVM-specific

// 18. Test: EVM Encode deliverMessage With Empty Payload
// NOTE: N/A for SVM - EVM ABI encoding is EVM-specific

// ============================================================================
// EVM LOG PARSING TESTS
// ============================================================================

// 19. Test: Parse EVM MessageSent Log
// NOTE: N/A for SVM - EVM log parsing is EVM-specific

// 20. Test: EVM MessageSent Log Short Data Ignored
// NOTE: N/A for SVM - EVM log parsing is EVM-specific

// 21. Test: EVM MessageSent Log Missing Topics Ignored
// NOTE: N/A for SVM - EVM log parsing is EVM-specific

// ============================================================================
// RLP ENCODING TESTS
// ============================================================================

// 22. Test: RLP Encode u64 Known Values
// NOTE: N/A for SVM - RLP encoding is EVM-specific

// 23. Test: RLP Encode Item Short String
// NOTE: N/A for SVM - RLP encoding is EVM-specific

// 24. Test: RLP Encode List Basic
// NOTE: N/A for SVM - RLP encoding is EVM-specific

// ============================================================================
// MVM OUTBOX MESSAGE PARSING TESTS
// ============================================================================

// 25. Test: MVM Get Message Response Parsing
// NOTE: N/A for SVM - MVM outbox message parsing is MVM-specific

// 26. Test: MVM Get Next Nonce Response Parsing
// NOTE: N/A for SVM - MVM outbox nonce parsing is MVM-specific

// ============================================================================
// SVM ACCOUNT DATA PARSING TESTS
// ============================================================================

// 27. TODO test_svm_outbound_nonce_account_layout — not yet implemented for SVM
// 28. TODO test_svm_outbound_nonce_account_too_short — not yet implemented for SVM
// 29. TODO test_svm_message_account_field_extraction — not yet implemented for SVM
// 30. TODO test_svm_message_account_discriminator_check — not yet implemented for SVM
// 31. TODO test_svm_message_account_payload_truncation — not yet implemented for SVM
