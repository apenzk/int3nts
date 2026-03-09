//! Unit tests for chain-clients-svm SvmClient
//!
//! Test ordering matches chain-clients/extension-checklist.md for cross-VM synchronization.
//! Tests marked N/A in the checklist are skipped in this file.

use base64::Engine;
use borsh::BorshSerialize;
use chain_clients_svm::{parse_escrow_data, pubkey_from_hex, pubkey_to_hex, EscrowAccount, SvmClient};
use solana_program::pubkey::Pubkey;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// CONSTANTS
// ============================================================================

/// Valid base58 program ID for tests (deterministic, not a real program)
const DUMMY_PROGRAM_ID: &str = "11111111111111111111111111111112";
const DUMMY_INTENT_ID: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000001";

// ============================================================================
// HELPERS
// ============================================================================

/// Creates a test EscrowAccount with the given is_claimed state.
fn make_escrow(is_claimed: bool) -> EscrowAccount {
    EscrowAccount {
        discriminator: [0u8; 8],
        requester: Pubkey::default(),
        token_mint: Pubkey::default(),
        amount: 1_000_000,
        is_claimed,
        expiry: 9999999999,
        reserved_solver: Pubkey::default(),
        intent_id: [0u8; 32],
        bump: 255,
    }
}

/// Serializes an EscrowAccount to base64 for mock RPC responses.
fn escrow_to_base64(escrow: &EscrowAccount) -> String {
    let serialized = escrow.try_to_vec().expect("serialize escrow");
    base64::engine::general_purpose::STANDARD.encode(&serialized)
}

/// Builds a mock getAccountInfo response with escrow data.
fn mock_account_info_response(escrow: &EscrowAccount) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "result": {
            "context": { "slot": 123 },
            "value": {
                "data": [escrow_to_base64(escrow), "base64"],
                "executable": false,
                "lamports": 1_000_000,
                "owner": DUMMY_PROGRAM_ID,
                "rentEpoch": 0
            }
        },
        "id": 1
    })
}

/// Builds a mock getProgramAccounts response with escrow accounts.
fn mock_program_accounts_response(escrows: &[(Pubkey, EscrowAccount)]) -> serde_json::Value {
    let accounts: Vec<serde_json::Value> = escrows
        .iter()
        .map(|(pubkey, escrow)| {
            serde_json::json!({
                "pubkey": pubkey.to_string(),
                "account": {
                    "data": [escrow_to_base64(escrow), "base64"],
                    "executable": false,
                    "lamports": 1_000_000,
                    "owner": DUMMY_PROGRAM_ID,
                    "rentEpoch": 0
                }
            })
        })
        .collect();

    serde_json::json!({
        "jsonrpc": "2.0",
        "result": accounts,
        "id": 1
    })
}

/// Standard JSON-RPC error response for tests.
fn mock_rpc_error(code: i32, message: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "error": { "code": code, "message": message },
        "id": 1
    })
}

// ============================================================================
// #1-2: client initialization
// ============================================================================

/// 1. Test: SvmClient initialization
/// Verifies that SvmClient::new() creates a client with correct config.
#[test]
fn test_client_new() {
    let client = SvmClient::new("http://127.0.0.1:8899", DUMMY_PROGRAM_ID).unwrap();
    assert_eq!(client.rpc_url(), "http://127.0.0.1:8899");
    assert_eq!(client.program_id().to_string(), DUMMY_PROGRAM_ID);
}

/// 2. Test: SvmClient rejects invalid program ID
/// Verifies that SvmClient::new() rejects non-base58 program IDs.
/// Why: Misconfigured program IDs should fail fast instead of causing RPC errors later.
#[test]
fn test_client_new_rejects_invalid() {
    let result = SvmClient::new("http://127.0.0.1:8899", "not-a-valid-pubkey!!!");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid SVM program_id"));
}

// ============================================================================
// #3-5: is_escrow_released
// ============================================================================

/// 3. Test: is_escrow_released returns true when escrow has been released
/// Verifies getAccountInfo + Borsh parsing with is_claimed=true.
#[tokio::test]
async fn test_is_escrow_released_success() {
    let mock_server = MockServer::start().await;

    let escrow = make_escrow(true);
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(mock_account_info_response(&escrow)),
        )
        .mount(&mock_server)
        .await;

    let client = SvmClient::new(&mock_server.uri(), DUMMY_PROGRAM_ID).unwrap();
    let released = client.is_escrow_released(DUMMY_INTENT_ID).await.unwrap();
    assert!(released);
}

/// 4. Test: is_escrow_released returns false when escrow not yet released
#[tokio::test]
async fn test_is_escrow_released_false() {
    let mock_server = MockServer::start().await;

    let escrow = make_escrow(false);
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(mock_account_info_response(&escrow)),
        )
        .mount(&mock_server)
        .await;

    let client = SvmClient::new(&mock_server.uri(), DUMMY_PROGRAM_ID).unwrap();
    let released = client.is_escrow_released(DUMMY_INTENT_ID).await.unwrap();
    assert!(!released);
}

/// 5. Test: is_escrow_released propagates RPC errors
#[tokio::test]
async fn test_is_escrow_released_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(mock_rpc_error(-32602, "Invalid param: could not find account")),
        )
        .mount(&mock_server)
        .await;

    let client = SvmClient::new(&mock_server.uri(), DUMMY_PROGRAM_ID).unwrap();
    let result = client.is_escrow_released(DUMMY_INTENT_ID).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("SVM RPC error"));
}

// ============================================================================
// #6-13: balance queries
// ============================================================================

/// 6. Test: get_token_balance returns correct SPL token balance
/// Verifies ATA derivation + getTokenAccountBalance parsing.
#[tokio::test]
async fn test_get_token_balance_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "result": {
                "context": { "slot": 123 },
                "value": {
                    "amount": "1000000",
                    "decimals": 6,
                    "uiAmount": 1.0,
                    "uiAmountString": "1.0"
                }
            },
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client = SvmClient::new(&mock_server.uri(), DUMMY_PROGRAM_ID).unwrap();
    let mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
    let owner = DUMMY_PROGRAM_ID;

    let balance = client.get_token_balance(mint, owner).await.unwrap();
    assert_eq!(balance, 1_000_000);
}

/// 7. Test: get_token_balance propagates RPC errors
#[tokio::test]
async fn test_get_token_balance_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(mock_rpc_error(-32602, "Invalid param: could not find account")),
        )
        .mount(&mock_server)
        .await;

    let client = SvmClient::new(&mock_server.uri(), DUMMY_PROGRAM_ID).unwrap();
    let mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
    let owner = DUMMY_PROGRAM_ID;

    let result = client.get_token_balance(mint, owner).await;
    assert!(result.is_err());
}

// #8: test_get_token_balance_zero — N/A for SVM (token accounts don't return zero; they don't exist if unfunded)

/// 9. Test: get_native_balance returns correct SOL balance in lamports
#[tokio::test]
async fn test_get_native_balance_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "result": {
                "context": { "slot": 123 },
                "value": 100_000_000u64
            },
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client = SvmClient::new(&mock_server.uri(), DUMMY_PROGRAM_ID).unwrap();
    let owner = DUMMY_PROGRAM_ID;

    let balance = client.get_native_balance(owner).await.unwrap();
    assert_eq!(balance, 100_000_000);
}

/// 10. Test: get_native_balance propagates RPC errors
#[tokio::test]
async fn test_get_native_balance_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(mock_rpc_error(-32602, "Invalid param: could not find account")),
        )
        .mount(&mock_server)
        .await;

    let client = SvmClient::new(&mock_server.uri(), DUMMY_PROGRAM_ID).unwrap();
    let owner = DUMMY_PROGRAM_ID;

    let result = client.get_native_balance(owner).await;
    assert!(result.is_err());
}

// #11: test_get_native_balance_exceeds_u64 — N/A for SVM (EVM-specific)
// #12: test_get_token_balance_with_padded_address — N/A for SVM (EVM-specific)
// #13: test_get_native_balance_with_padded_address — N/A for SVM (EVM-specific)

// ============================================================================
// #14-17: escrow event parsing
// ============================================================================

/// 14. Test: get_escrow_events parses program accounts into escrow events
#[tokio::test]
async fn test_get_escrow_events_success() {
    let mock_server = MockServer::start().await;

    let mut escrow = make_escrow(false);
    escrow.intent_id = [0u8; 32];
    escrow.intent_id[31] = 1; // intent_id = 0x...0001
    let escrow_pubkey = Pubkey::new_from_array([42u8; 32]);

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(mock_program_accounts_response(&[(escrow_pubkey, escrow)])),
        )
        .mount(&mock_server)
        .await;

    let client = SvmClient::new(&mock_server.uri(), DUMMY_PROGRAM_ID).unwrap();
    let events = client.get_escrow_events().await.unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].intent_id, DUMMY_INTENT_ID);
    assert_eq!(events[0].escrow_id, pubkey_to_hex(&escrow_pubkey));
}

/// 15. Test: get_escrow_events handles empty program accounts
#[tokio::test]
async fn test_get_escrow_events_empty() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "result": [],
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client = SvmClient::new(&mock_server.uri(), DUMMY_PROGRAM_ID).unwrap();
    let events = client.get_escrow_events().await.unwrap();
    assert_eq!(events.len(), 0);
}

/// 16. Test: get_escrow_events propagates RPC errors
#[tokio::test]
async fn test_get_escrow_events_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(mock_rpc_error(-32000, "getProgramAccounts disabled")),
        )
        .mount(&mock_server)
        .await;

    let client = SvmClient::new(&mock_server.uri(), DUMMY_PROGRAM_ID).unwrap();
    let result = client.get_escrow_events().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("SVM RPC error"));
}

/// 17. Test: get_all_escrows parses program accounts with Borsh data
/// Verifies that getProgramAccounts returns parsed EscrowWithPubkey structs.
#[tokio::test]
async fn test_get_all_escrows_parses_program_accounts() {
    let mock_server = MockServer::start().await;

    let mut escrow1 = make_escrow(false);
    escrow1.intent_id[31] = 1;
    escrow1.amount = 500_000;
    let pk1 = Pubkey::new_from_array([10u8; 32]);

    let mut escrow2 = make_escrow(true);
    escrow2.intent_id[31] = 2;
    escrow2.amount = 750_000;
    let pk2 = Pubkey::new_from_array([20u8; 32]);

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            mock_program_accounts_response(&[(pk1, escrow1), (pk2, escrow2)]),
        ))
        .mount(&mock_server)
        .await;

    let client = SvmClient::new(&mock_server.uri(), DUMMY_PROGRAM_ID).unwrap();
    let escrows = client.get_all_escrows().await.unwrap();

    assert_eq!(escrows.len(), 2);
    assert_eq!(escrows[0].pubkey, pk1);
    assert_eq!(escrows[0].escrow.amount, 500_000);
    assert!(!escrows[0].escrow.is_claimed);
    assert_eq!(escrows[1].pubkey, pk2);
    assert_eq!(escrows[1].escrow.amount, 750_000);
    assert!(escrows[1].escrow.is_claimed);
}

// ============================================================================
// #18-24: address normalization
// ============================================================================

// #18: test_normalize_hex_to_address_full_length — N/A for SVM (MVM-specific)
// #19: test_normalize_hex_to_address_short_address — N/A for SVM (MVM-specific)
// #20: test_normalize_hex_to_address_odd_length — N/A for SVM (MVM-specific)
// #21: test_normalize_hex_to_address_no_prefix — N/A for SVM (MVM-specific)
// #22: test_normalize_evm_address_padded — N/A for SVM (EVM-specific)
// #23: test_normalize_evm_address_passthrough — N/A for SVM (EVM-specific)
// #24: test_normalize_evm_address_rejects_non_zero_high_bytes — N/A for SVM (EVM-specific)

/// 25. Test: pubkey_from_hex handles hex with leading zeros
/// Verifies that leading zeros stripped by Move are restored to produce correct 32-byte Pubkey.
#[test]
fn test_pubkey_from_hex_with_leading_zeros() {
    let full_hex = "0x00aabbccdd00aabbccdd00aabbccdd00aabbccdd00aabbccdd00aabbccdd0011";
    let pk1 = pubkey_from_hex(full_hex).expect("full hex");

    // Stripped leading zeros (Move address format)
    let stripped_hex = "0xaabbccdd00aabbccdd00aabbccdd00aabbccdd00aabbccdd00aabbccdd0011";
    let pk2 = pubkey_from_hex(stripped_hex).expect("stripped hex");

    assert_eq!(pk1, pk2, "Leading zeros should be restored");
    assert_eq!(pk1.to_bytes()[0], 0x00, "First byte should be zero");
}

/// 26. Test: pubkey_from_hex works for addresses without leading zeros
#[test]
fn test_pubkey_from_hex_no_leading_zeros() {
    let hex = "0xaa11223344556677aa11223344556677aa11223344556677aa11223344556677";
    let pk = pubkey_from_hex(hex).expect("parse hex");
    assert_eq!(pk.to_bytes()[0], 0xaa, "First byte should be 0xaa");
}

// ============================================================================
// #27-28: SVM escrow parsing
// ============================================================================

/// 27. Test: EscrowAccount Borsh roundtrip serialization
/// Verifies that an EscrowAccount can be serialized and deserialized via base64.
/// Why: All escrow reads depend on correct Borsh parsing. A serialization mismatch
/// would cause all escrow lookups to fail.
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
        bump: 1,
    };

    let serialized = escrow.try_to_vec().expect("serialize escrow");
    let encoded = base64::engine::general_purpose::STANDARD.encode(&serialized);
    let parsed = parse_escrow_data(&encoded).expect("parse escrow");

    assert_eq!(parsed.discriminator, escrow.discriminator);
    assert_eq!(parsed.requester, escrow.requester);
    assert_eq!(parsed.token_mint, escrow.token_mint);
    assert_eq!(parsed.amount, escrow.amount);
    assert_eq!(parsed.is_claimed, escrow.is_claimed);
    assert_eq!(parsed.expiry, escrow.expiry);
    assert_eq!(parsed.reserved_solver, escrow.reserved_solver);
    assert_eq!(parsed.intent_id, escrow.intent_id);
    assert_eq!(parsed.bump, escrow.bump);
}

/// 28. Test: parse_escrow_data returns None for invalid base64
/// Verifies that invalid base64 input is handled gracefully.
/// Why: Corrupt or non-escrow accounts should be skipped, not crash.
#[test]
fn test_escrow_account_invalid_base64() {
    let result = parse_escrow_data("not-valid-base64!!!");
    assert!(result.is_none());

    // Valid base64 but too short for EscrowAccount
    let too_short = base64::engine::general_purpose::STANDARD.encode(b"short");
    let result = parse_escrow_data(&too_short);
    assert!(result.is_none());
}
