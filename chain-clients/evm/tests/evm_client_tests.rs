//! Unit tests for chain-clients-evm EvmClient
//!
//! Test ordering matches chain-clients/extension-checklist.md for cross-VM synchronization.
//! Tests marked N/A in the checklist are skipped in this file.

use chain_clients_evm::{normalize_evm_address, EvmClient};
use serde_json::json;
use sha3::{Digest, Keccak256};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// CONSTANTS
// ============================================================================

const DUMMY_ESCROW_CONTRACT_ADDR: &str = "0x000000000000000000000000000000000000000e";
const DUMMY_INTENT_ID: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000001";
const DUMMY_TOKEN_ADDR: &str = "0x000000000000000000000000000000000000000a";
const DUMMY_REQUESTER_ADDR: &str = "0x0000000000000000000000000000000000000006";
const DUMMY_TX_HASH: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000012";

// ============================================================================
// #1: client_new
// ============================================================================

/// 1. Test: EvmClient initialization
/// Verifies that EvmClient::new() creates a client with correct config.
#[test]
fn test_client_new() {
    let client = EvmClient::new("http://127.0.0.1:8545", DUMMY_ESCROW_CONTRACT_ADDR).unwrap();
    assert_eq!(client.base_url(), "http://127.0.0.1:8545");
    assert_eq!(client.escrow_contract_addr(), DUMMY_ESCROW_CONTRACT_ADDR);
}

// #2: client_new_rejects_invalid — N/A for EVM (no config validation like SVM pubkey)

// ============================================================================
// #3-5: is_escrow_released
// ============================================================================

/// 3. Test: is_escrow_released returns true when escrow is released
/// Verifies eth_call to isReleased(bytes32) parses ABI bool correctly.
#[tokio::test]
async fn test_is_escrow_released_success() {
    let mock_server = MockServer::start().await;

    // ABI-encoded true: 32 bytes with last byte = 1
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "result": format!("0x{:064x}", 1u64),
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client =
        EvmClient::new(&mock_server.uri(), DUMMY_ESCROW_CONTRACT_ADDR).unwrap();
    let released = client.is_escrow_released(DUMMY_INTENT_ID).await.unwrap();
    assert!(released);
}

/// 4. Test: is_escrow_released returns false when escrow is not released
#[tokio::test]
async fn test_is_escrow_released_false() {
    let mock_server = MockServer::start().await;

    // ABI-encoded false: 32 bytes all zeros
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "result": format!("0x{:064x}", 0u64),
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client =
        EvmClient::new(&mock_server.uri(), DUMMY_ESCROW_CONTRACT_ADDR).unwrap();
    let released = client.is_escrow_released(DUMMY_INTENT_ID).await.unwrap();
    assert!(!released);
}

/// 5. Test: is_escrow_released propagates RPC errors
#[tokio::test]
async fn test_is_escrow_released_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "error": { "code": -32000, "message": "execution reverted" },
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client =
        EvmClient::new(&mock_server.uri(), DUMMY_ESCROW_CONTRACT_ADDR).unwrap();
    let result = client.is_escrow_released(DUMMY_INTENT_ID).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("isReleased failed"));
}

// ============================================================================
// #6-13: balance queries
// ============================================================================

/// 6. Test: get_token_balance returns correct ERC20 balance
#[tokio::test]
async fn test_get_token_balance_success() {
    let mock_server = MockServer::start().await;

    let balance_hex = format!("0x{:064x}", 1_000_000u64);

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "result": balance_hex,
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client =
        EvmClient::new(&mock_server.uri(), DUMMY_ESCROW_CONTRACT_ADDR).unwrap();
    let balance = client
        .get_token_balance(DUMMY_TOKEN_ADDR, DUMMY_REQUESTER_ADDR)
        .await
        .unwrap();
    assert_eq!(balance, 1_000_000);
}

/// 7. Test: get_token_balance propagates RPC errors
#[tokio::test]
async fn test_get_token_balance_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "error": { "code": -32000, "message": "execution reverted" },
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client =
        EvmClient::new(&mock_server.uri(), DUMMY_ESCROW_CONTRACT_ADDR).unwrap();
    let result = client
        .get_token_balance(DUMMY_TOKEN_ADDR, DUMMY_REQUESTER_ADDR)
        .await;
    assert!(result.is_err());
}

/// 8. Test: get_token_balance returns zero for "0x0" result
#[tokio::test]
async fn test_get_token_balance_zero() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "result": "0x0",
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client =
        EvmClient::new(&mock_server.uri(), DUMMY_ESCROW_CONTRACT_ADDR).unwrap();
    let balance = client
        .get_token_balance(DUMMY_TOKEN_ADDR, DUMMY_REQUESTER_ADDR)
        .await
        .unwrap();
    assert_eq!(balance, 0);
}

/// 9. Test: get_native_balance returns correct ETH balance
#[tokio::test]
async fn test_get_native_balance_success() {
    let mock_server = MockServer::start().await;

    // 0.01 ETH = 10000000000000000 wei
    let balance_hex = format!("0x{:x}", 10_000_000_000_000_000u64);

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "result": balance_hex,
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client =
        EvmClient::new(&mock_server.uri(), DUMMY_ESCROW_CONTRACT_ADDR).unwrap();
    let balance = client.get_native_balance(DUMMY_REQUESTER_ADDR).await.unwrap();
    assert_eq!(balance, 10_000_000_000_000_000);
}

/// 10. Test: get_native_balance propagates RPC errors
#[tokio::test]
async fn test_get_native_balance_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "error": { "code": -32602, "message": "invalid argument" },
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client =
        EvmClient::new(&mock_server.uri(), DUMMY_ESCROW_CONTRACT_ADDR).unwrap();
    let result = client.get_native_balance(DUMMY_REQUESTER_ADDR).await;
    assert!(result.is_err());
}

/// 11. Test: get_native_balance returns exact u128 for large ETH balances exceeding u64
#[tokio::test]
async fn test_get_native_balance_exceeds_u64() {
    let mock_server = MockServer::start().await;

    // 10000 ETH in wei — exceeds u64::MAX
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "result": "0x21e19e0c9bab2400000",
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client =
        EvmClient::new(&mock_server.uri(), DUMMY_ESCROW_CONTRACT_ADDR).unwrap();
    let balance = client.get_native_balance(DUMMY_REQUESTER_ADDR).await.unwrap();
    assert_eq!(balance, 10_000_000_000_000_000_000_000u128);
}

/// 12. Test: get_token_balance succeeds with 32-byte padded token address
#[tokio::test]
async fn test_get_token_balance_with_padded_address() {
    let mock_server = MockServer::start().await;

    let balance_hex = format!("0x{:064x}", 500_000u64);

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "result": balance_hex,
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client =
        EvmClient::new(&mock_server.uri(), DUMMY_ESCROW_CONTRACT_ADDR).unwrap();

    // 32-byte padded address (as stored in solver config for Move compatibility)
    let padded_token = "0x000000000000000000000000a513e6e4b8f2a923d98304ec87f64353c4d5c853";
    let balance = client
        .get_token_balance(padded_token, DUMMY_REQUESTER_ADDR)
        .await
        .unwrap();
    assert_eq!(balance, 500_000);
}

/// 13. Test: get_native_balance succeeds with 32-byte padded account address
#[tokio::test]
async fn test_get_native_balance_with_padded_address() {
    let mock_server = MockServer::start().await;

    let balance_hex = format!("0x{:x}", 1_000_000_000_000_000u64);

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "result": balance_hex,
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client =
        EvmClient::new(&mock_server.uri(), DUMMY_ESCROW_CONTRACT_ADDR).unwrap();

    let padded_account = "0x000000000000000000000000f39fd6e51aad88f6f4ce6ab8827279cfffb92266";
    let balance = client.get_native_balance(padded_account).await.unwrap();
    assert_eq!(balance, 1_000_000_000_000_000);
}

// ============================================================================
// #14-16: escrow event parsing
// ============================================================================

/// 14. Test: get_escrow_created_events parses EscrowCreated events correctly
#[tokio::test]
async fn test_get_escrow_events_success() {
    let mock_server = MockServer::start().await;

    let event_signature = "EscrowCreated(bytes32,bytes32,address,uint64,address,bytes32,uint64)";
    let mut hasher = Keccak256::new();
    hasher.update(event_signature.as_bytes());
    let event_topic = format!("0x{}", hex::encode(hasher.finalize()));

    let escrow_id_hex = "0000000000000000000000000000000000000000000000000000000000000002";
    let amount_hex = "00000000000000000000000000000000000000000000000000000000000f4240"; // 1000000
    let solver_hex = "0000000000000000000000000000000000000000000000000000000000000009";
    let expiry_hex = "0000000000000000000000000000000000000000000000000000000000000000";

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "result": [
                {
                    "address": DUMMY_ESCROW_CONTRACT_ADDR,
                    "topics": [
                        event_topic,
                        DUMMY_INTENT_ID,
                        format!("0x000000000000000000000000{}", DUMMY_REQUESTER_ADDR.strip_prefix("0x").unwrap()),
                        format!("0x000000000000000000000000{}", DUMMY_TOKEN_ADDR.strip_prefix("0x").unwrap())
                    ],
                    "data": format!("0x{}{}{}{}", escrow_id_hex, amount_hex, solver_hex, expiry_hex),
                    "blockNumber": "0x1000",
                    "transactionHash": DUMMY_TX_HASH,
                    "logIndex": "0x0"
                }
            ],
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client =
        EvmClient::new(&mock_server.uri(), DUMMY_ESCROW_CONTRACT_ADDR).unwrap();
    let events = client
        .get_escrow_created_events(None, None)
        .await
        .unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].intent_id, DUMMY_INTENT_ID);
    assert_eq!(events[0].escrow_id, format!("0x{}", escrow_id_hex));
    assert_eq!(events[0].requester_addr, DUMMY_REQUESTER_ADDR);
    assert_eq!(events[0].amount, 1000000);
    assert_eq!(events[0].token_addr, DUMMY_TOKEN_ADDR);
    assert_eq!(events[0].reserved_solver, format!("0x{}", solver_hex));
    assert_eq!(events[0].expiry, 0);
}

/// 15. Test: get_escrow_created_events handles empty log list correctly
#[tokio::test]
async fn test_get_escrow_events_empty() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "result": [],
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client =
        EvmClient::new(&mock_server.uri(), DUMMY_ESCROW_CONTRACT_ADDR).unwrap();
    let events = client
        .get_escrow_created_events(None, None)
        .await
        .unwrap();
    assert_eq!(events.len(), 0);
}

/// 16. Test: get_escrow_created_events propagates JSON-RPC errors
#[tokio::test]
async fn test_get_escrow_events_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "error": { "code": -32000, "message": "Invalid filter" },
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client =
        EvmClient::new(&mock_server.uri(), DUMMY_ESCROW_CONTRACT_ADDR).unwrap();
    let result = client.get_escrow_created_events(None, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("JSON-RPC error"));
}

// #17: get_all_escrows_parses_program_accounts — N/A for EVM

// ============================================================================
// #18-21: address normalization (MVM-specific, N/A for EVM)
// ============================================================================

// #18-21: normalize_hex_to_address — N/A for EVM (MVM-specific Move address normalization)

// ============================================================================
// #22-24: EVM address normalization
// ============================================================================

/// 22. Test: normalize_evm_address extracts 20 bytes from 32-byte padded address
#[test]
fn test_normalize_evm_address_padded() {
    let padded = "0x000000000000000000000000a513e6e4b8f2a923d98304ec87f64353c4d5c853";
    let result = normalize_evm_address(padded).unwrap();
    assert_eq!(result, "0xa513e6e4b8f2a923d98304ec87f64353c4d5c853");
}

/// 23. Test: normalize_evm_address passes through 20-byte addresses unchanged
#[test]
fn test_normalize_evm_address_passthrough() {
    let normal = "0xa513e6e4b8f2a923d98304ec87f64353c4d5c853";
    let result = normalize_evm_address(normal).unwrap();
    assert_eq!(result, normal);
}

/// 24. Test: normalize_evm_address rejects 32-byte address with non-zero high bytes
#[test]
fn test_normalize_evm_address_rejects_non_zero_high_bytes() {
    let bad = "0x0000000000000000000000010000000000000000000000000000000000000001";
    let result = normalize_evm_address(bad);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("non-zero high bytes"));
}

// #25-26: pubkey_from_hex — N/A for EVM (SVM-specific)
// #27-28: escrow_account_borsh — N/A for EVM (SVM-specific)
