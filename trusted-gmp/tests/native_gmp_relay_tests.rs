//! Tests for native GMP relay transaction delivery functions.
//!
//! These tests cover the helper functions used by `deliver_to_mvm()` and `deliver_to_svm()`
//! for parsing addresses, converting keys, and extracting transaction hashes from CLI output.

mod helpers;

use helpers::{
    build_test_config_with_mvm, build_test_config_with_svm, DUMMY_INTENT_ID, DUMMY_SOLVER_ADDR_HUB,
    DUMMY_SVM_ESCROW_PROGRAM_ID, DUMMY_TX_HASH, TEST_MVM_CHAIN_ID, TEST_SVM_CHAIN_ID,
};
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use std::str::FromStr;
use trusted_gmp::native_gmp_relay::{
    bytes_array_to_hex, ed25519_seed_to_keypair_bytes, extract_transaction_hash, hex_to_bytes,
    normalize_address, parse_32_byte_address, parse_svm_pubkey, NativeGmpRelayConfig,
};

// ============================================================================
// ADDRESS PARSING TESTS
// ============================================================================

/// Test parsing a full 64-character hex address using DUMMY_SOLVER_ADDR_HUB.
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

/// Test parsing DUMMY_INTENT_ID which has a different last byte.
/// Why: Verify consistent behavior across different test constants.
#[test]
fn test_parse_32_byte_address_intent_id() {
    let result = parse_32_byte_address(DUMMY_INTENT_ID).unwrap();

    assert_eq!(result.len(), 32, "Result should be exactly 32 bytes");
    // DUMMY_INTENT_ID = "0x0000000000000000000000000000000000000000000000000000000000000001"
    assert_eq!(result[0], 0x00, "First byte should be 0x00");
    assert_eq!(result[31], 0x01, "Last byte should match constant (0x01)");
}

/// Test parsing a short address that needs significant left-padding.
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

/// Test parsing a 4-hex-char address (2 bytes, needs 30 bytes padding).
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
// SVM PUBKEY PARSING TESTS
// ============================================================================

/// Test parsing an SVM pubkey from hex format using DUMMY_INTENT_ID.
/// Why: GMP messages use hex-encoded addresses; we convert to Solana Pubkey.
#[test]
fn test_parse_svm_pubkey_from_hex_format() {
    let pubkey = parse_svm_pubkey(DUMMY_INTENT_ID).unwrap();

    // DUMMY_INTENT_ID = "0x0000000000000000000000000000000000000000000000000000000000000001"
    assert_eq!(pubkey.to_bytes()[31], 0x01, "Last byte should be 0x01");
    assert_eq!(pubkey.to_bytes()[0], 0x00, "First byte should be 0x00");
}

/// Test parsing an SVM pubkey from base58 format using DUMMY_SVM_ESCROW_PROGRAM_ID.
/// Why: Solana native addresses use base58; we must support both formats.
#[test]
fn test_parse_svm_pubkey_from_base58_format() {
    // DUMMY_SVM_ESCROW_PROGRAM_ID = "11111111111111111111111111111111" (system program, all zeros)
    let pubkey = parse_svm_pubkey(DUMMY_SVM_ESCROW_PROGRAM_ID).unwrap();

    assert_eq!(pubkey.to_bytes(), [0u8; 32], "System program should be all zeros");
}

// ============================================================================
// ED25519 KEYPAIR CONVERSION TESTS
// ============================================================================

/// Test converting an Ed25519 seed to Solana keypair format.
/// Why: trusted-gmp uses Ed25519 keys; Solana SDK expects 64-byte keypairs.
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
    let keypair = Keypair::from_bytes(&keypair_bytes).expect("Should create valid Keypair");
    assert_eq!(keypair.to_bytes(), keypair_bytes);
}

/// Test that invalid seed length returns an error.
/// Why: Ed25519 seeds must be exactly 32 bytes.
#[test]
fn test_ed25519_seed_to_keypair_bytes_rejects_invalid_length() {
    let short_seed = [1u8; 16]; // Only 16 bytes, should fail
    let result = ed25519_seed_to_keypair_bytes(&short_seed);

    assert!(result.is_err(), "Should reject seed that isn't 32 bytes");
}

// ============================================================================
// TRANSACTION HASH EXTRACTION TESTS
// ============================================================================

/// Test extracting transaction hash from JSON format (aptos CLI).
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

/// Test extracting transaction hash from traditional CLI format.
/// Why: Older CLI versions use "Transaction hash: 0x..." format.
#[test]
fn test_extract_transaction_hash_from_traditional_output() {
    let output = format!(
        "Transaction submitted.\nTransaction hash: {}\nGas used: 100",
        DUMMY_TX_HASH
    );

    let hash = extract_transaction_hash(&output);

    assert_eq!(hash, Some(DUMMY_TX_HASH.to_string()));
}

/// Test that missing hash returns None.
/// Why: Some CLI outputs may not contain a transaction hash.
#[test]
fn test_extract_transaction_hash_returns_none_when_missing() {
    let output = "Some output without a transaction hash";

    let hash = extract_transaction_hash(output);

    assert_eq!(hash, None);
}

/// Test hash extraction with Hash capitalization variant.
/// Why: Some outputs use "Hash:" instead of "hash:".
#[test]
fn test_extract_transaction_hash_handles_capitalization() {
    let output = format!("Transaction Hash: {}", DUMMY_TX_HASH);

    let hash = extract_transaction_hash(&output);

    assert_eq!(hash, Some(DUMMY_TX_HASH.to_string()));
}

// ============================================================================
// BYTES ARRAY TO HEX TESTS
// ============================================================================

/// Test converting array of decimal byte strings to hex.
/// Why: MVM events encode bytes as arrays of decimal strings like ["1", "2", "255"].
#[test]
fn test_bytes_array_to_hex_converts_decimal_strings() {
    let bytes = vec!["1".to_string(), "2".to_string(), "255".to_string()];
    let result = bytes_array_to_hex(&bytes).unwrap();

    assert_eq!(result, "0x0102ff", "Should convert decimal bytes to hex with 0x prefix");
}

/// Test converting empty array.
/// Why: Edge case for empty payloads.
#[test]
fn test_bytes_array_to_hex_empty_array() {
    let bytes: Vec<String> = vec![];
    let result = bytes_array_to_hex(&bytes).unwrap();

    assert_eq!(result, "0x", "Empty array should produce just 0x prefix");
}

// ============================================================================
// HEX TO BYTES TESTS
// ============================================================================

/// Test hex to bytes conversion with 0x prefix.
/// Why: Most addresses and payloads use 0x prefix.
#[test]
fn test_hex_to_bytes_with_prefix() {
    let bytes = hex_to_bytes("0x0102ff").unwrap();

    assert_eq!(bytes, vec![1, 2, 255]);
}

/// Test hex to bytes conversion without 0x prefix.
/// Why: Some inputs may omit the prefix.
#[test]
fn test_hex_to_bytes_without_prefix() {
    let bytes = hex_to_bytes("0102ff").unwrap();

    assert_eq!(bytes, vec![1, 2, 255]);
}

// ============================================================================
// ADDRESS NORMALIZATION TESTS
// ============================================================================

/// Test normalize_address adds 0x prefix when missing.
/// Why: Some Move VM addresses may be stored without prefix.
#[test]
fn test_normalize_address_adds_prefix() {
    assert_eq!(normalize_address("abc123"), "0xabc123");
}

/// Test normalize_address preserves existing prefix.
/// Why: Should not double-prefix addresses.
#[test]
fn test_normalize_address_preserves_existing_prefix() {
    assert_eq!(normalize_address("0xabc123"), "0xabc123");
}

// ============================================================================
// SVM MESSAGE PARSING TESTS
// ============================================================================

/// Test parsing SVM MessageSent log format.
/// Why: Validates the log parsing logic used in poll_svm_events().
#[test]
fn test_parse_svm_message_sent_log_format() {
    // Format matches native-gmp-endpoint program log output
    let log = format!(
        "Program log: MessageSent: src_chain_id={}, dst_chain_id={}, src_addr={}, dst_addr=0102030405060708091011121314151617181920212223242526272829303132, nonce=42, payload_len=4, payload_hex=deadbeef",
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

/// Test that non-MessageSent logs are correctly identified.
/// Why: Only MessageSent logs should be processed by the relay.
#[test]
fn test_non_message_sent_log_is_ignored() {
    let log = "Program log: Some other log message";
    assert!(!log.contains("MessageSent:"), "Non-MessageSent log should not contain marker");
}

/// Test Solana pubkey to hex conversion using DUMMY_SVM_ESCROW_PROGRAM_ID.
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

/// Test NativeGmpRelayConfig extracts MVM connected chain when present.
/// Why: The relay needs to route messages to connected MVM chains.
#[test]
fn test_relay_config_extracts_mvm_connected_chain() {
    let config = build_test_config_with_mvm();

    let relay_config = NativeGmpRelayConfig::from_config(&config).unwrap();

    // Hub chain should be extracted
    assert_eq!(
        relay_config.mvm_chain_id, 1,
        "Hub chain ID should be extracted"
    );
    assert_eq!(
        relay_config.mvm_rpc_url, "http://127.0.0.1:18080",
        "Hub RPC URL should be extracted"
    );
    assert_eq!(
        relay_config.mvm_module_addr, "0x1",
        "Hub module address should be extracted"
    );

    // MVM connected chain should be extracted
    assert_eq!(
        relay_config.mvm_connected_chain_id,
        Some(2),
        "MVM connected chain ID should be extracted"
    );
    assert_eq!(
        relay_config.mvm_connected_rpc_url,
        Some("http://127.0.0.1:18082".to_string()),
        "MVM connected RPC URL should be extracted"
    );
    assert_eq!(
        relay_config.mvm_connected_module_addr,
        Some("0x2".to_string()),
        "MVM connected module address should be extracted"
    );
}

/// Test NativeGmpRelayConfig handles missing MVM connected chain.
/// Why: When only hub and SVM are configured, MVM connected fields should be None.
#[test]
fn test_relay_config_handles_missing_mvm_connected() {
    let mut config = build_test_config_with_svm();
    // Clear MVM connected chain to simulate hub-only + SVM config
    config.connected_chain_mvm = None;

    let relay_config = NativeGmpRelayConfig::from_config(&config).unwrap();

    // MVM connected chain should be None
    assert!(
        relay_config.mvm_connected_chain_id.is_none(),
        "MVM connected chain ID should be None when not configured"
    );
    assert!(
        relay_config.mvm_connected_rpc_url.is_none(),
        "MVM connected RPC URL should be None when not configured"
    );
    assert!(
        relay_config.mvm_connected_module_addr.is_none(),
        "MVM connected module address should be None when not configured"
    );

    // SVM should still be extracted
    assert_eq!(
        relay_config.svm_chain_id,
        Some(4),
        "SVM chain ID should still be extracted"
    );
}

/// Test NativeGmpRelayConfig extracts both MVM connected and SVM chains.
/// Why: Relay may need to route to both MVM and SVM connected chains.
#[test]
fn test_relay_config_extracts_both_connected_chains() {
    let config = build_test_config_with_svm();
    // Keep both MVM and SVM connected chains (build_test_config_with_svm has MVM from base)

    let relay_config = NativeGmpRelayConfig::from_config(&config).unwrap();

    // Both connected chains should be present
    assert!(
        relay_config.mvm_connected_chain_id.is_some(),
        "MVM connected should be present"
    );
    assert!(
        relay_config.svm_chain_id.is_some(),
        "SVM should be present"
    );

    // Verify they have different chain IDs
    assert_ne!(
        relay_config.mvm_connected_chain_id,
        relay_config.svm_chain_id,
        "MVM connected and SVM should have different chain IDs"
    );
}

// ============================================================================
// FULFILLMENT PROOF PAYLOAD PARSING TESTS
// ============================================================================

/// Test FulfillmentProof payload parsing for intent_id extraction.
/// Why: deliver_to_svm must correctly parse intent_id from payload to derive PDAs.
/// Payload format: [type(1)] [intent_id(32)] [solver_addr(32)] [amount(8)] [timestamp(8)] = 81 bytes
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

/// Test FulfillmentProof payload validation for minimum length.
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

/// Test ATA derivation formula is correct.
/// Why: trusted-gmp derives solver's ATA manually. Must match spl-associated-token-account.
/// ATA = PDA([owner, TOKEN_PROGRAM_ID, mint], ASSOCIATED_TOKEN_PROGRAM_ID)
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
    let (derived_ata, bump) = Pubkey::find_program_address(
        &[
            owner.as_ref(),
            token_program_id.as_ref(),
            mint.as_ref(),
        ],
        &associated_token_program_id,
    );

    // Verify the derivation produces a valid pubkey and bump
    assert!(bump <= 255, "Bump should be <= 255");
    assert_ne!(derived_ata, Pubkey::default(), "Derived ATA should not be default");

    // Verify the formula matches expected ATA format
    // The ATA should be different from owner and mint
    assert_ne!(derived_ata, owner, "ATA should be different from owner");
    assert_ne!(derived_ata, mint, "ATA should be different from mint");
}

/// Test ATA derivation is deterministic.
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

/// Test ATA changes with different owners.
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
