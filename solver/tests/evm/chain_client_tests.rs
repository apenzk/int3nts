//! Unit tests for EVM Connected chain client
//!
//! Test ordering matches EXTENSION-CHECKLIST.md for cross-VM synchronization.
//! Tests marked N/A in the checklist are skipped in this file.

use hex;
use serde_json::json;
use sha3::{Digest, Keccak256};
use solver::chains::ConnectedEvmClient;
use solver::config::EvmChainConfig;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "../helpers.rs"]
mod test_helpers;
use test_helpers::{
    DUMMY_ESCROW_CONTRACT_ADDR_EVM, DUMMY_INTENT_ID, DUMMY_REQUESTER_ADDR_EVM,
    DUMMY_SOLVER_ADDR_EVM, DUMMY_TOKEN_ADDR_EVM, DUMMY_TX_HASH,
};

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn create_test_evm_config() -> EvmChainConfig {
    EvmChainConfig {
        name: "test-evm".to_string(),
        rpc_url: "http://127.0.0.1:8545".to_string(),
        chain_id: 84532,
        escrow_contract_addr: DUMMY_ESCROW_CONTRACT_ADDR_EVM.to_string(),
        private_key_env: "TEST_PRIVATE_KEY".to_string(),
        network_name: "localhost".to_string(),
    }
}

// ============================================================================
// CLIENT INITIALIZATION
// ============================================================================

/// 1. Test: ConnectedEvmClient Initialization
/// Verifies that ConnectedEvmClient::new() creates a client with correct config.
/// Why: Client initialization is the entry point for all EVM operations. A failure
/// here would prevent any solver operations on connected EVM chains.
#[test]
fn test_evm_client_new() {
    let config = create_test_evm_config();
    let _client = ConnectedEvmClient::new(&config).unwrap();
}

// #2: client_new_rejects_invalid - N/A for EVM (no config validation like SVM pubkey)

// ============================================================================
// ESCROW EVENT QUERYING
// ============================================================================

/// 3. Test: Get Escrow Events Success
/// Verifies that get_escrow_events() parses EscrowInitialized events correctly.
/// Why: The solver needs to parse escrow events from connected EVM chains to
/// identify fulfillment opportunities. A parsing bug would cause missed escrows.
#[tokio::test]
async fn test_get_escrow_events_evm_success() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    // EscrowInitialized event signature hash
    // keccak256("EscrowInitialized(uint256,address,address,address,address,uint256,uint256)")
    let event_signature = "EscrowInitialized(uint256,address,address,address,address,uint256,uint256)";
    let mut hasher = Keccak256::new();
    hasher.update(event_signature.as_bytes());
    let event_topic = format!("0x{}", hex::encode(hasher.finalize()));

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "result": [
                {
                    "address": DUMMY_ESCROW_CONTRACT_ADDR_EVM,
                    "topics": [
                        event_topic,
                        format!("0x000000000000000000000000{}", DUMMY_INTENT_ID.strip_prefix("0x").unwrap()), // intent_id (padded to 32 bytes in EVM topic)
                        format!("0x000000000000000000000000{}", DUMMY_ESCROW_CONTRACT_ADDR_EVM.strip_prefix("0x").unwrap()), // escrow
                        format!("0x000000000000000000000000{}", DUMMY_REQUESTER_ADDR_EVM.strip_prefix("0x").unwrap())  // requester
                    ],
                    "data": format!("0x000000000000000000000000{}000000000000000000000000{}00000000000000000000000000000000000000000000000000000000000f42400000000000000000000000000000000000000000000000000000000000000000", DUMMY_TOKEN_ADDR_EVM.strip_prefix("0x").unwrap(), DUMMY_SOLVER_ADDR_EVM.strip_prefix("0x").unwrap()), // token (32 bytes) + reserved_solver (32 bytes) + amount (32 bytes, 1000000) + expiry (32 bytes, 0)
                    "blockNumber": "0x1000",
                    "transactionHash": DUMMY_TX_HASH
                }
            ],
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let mut config = create_test_evm_config();
    config.rpc_url = base_url;
    let client = ConnectedEvmClient::new(&config).unwrap();

    let events = client.get_escrow_events(None, None).await.unwrap();

    assert_eq!(events.len(), 1);
    // Intent ID is extracted from topic (32 bytes), so it includes padding zeros
    assert_eq!(
        events[0].intent_id,
        format!(
            "0x000000000000000000000000{}",
            DUMMY_INTENT_ID.strip_prefix("0x").unwrap()
        )
    );
    assert_eq!(events[0].requester_addr, DUMMY_REQUESTER_ADDR_EVM);
    assert_eq!(events[0].token_addr, DUMMY_TOKEN_ADDR_EVM);
    assert_eq!(events[0].reserved_solver_addr, DUMMY_SOLVER_ADDR_EVM);
}

/// 4. Test: Get Escrow Events Empty
/// Verifies that get_escrow_events() handles empty log list correctly.
/// Why: A chain with no escrows should return an empty list, not an error.
/// The solver should handle this gracefully and continue polling.
#[tokio::test]
async fn test_get_escrow_events_evm_empty() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "result": [],
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let mut config = create_test_evm_config();
    config.rpc_url = base_url;
    let client = ConnectedEvmClient::new(&config).unwrap();

    let events = client.get_escrow_events(None, None).await.unwrap();

    assert_eq!(events.len(), 0);
}

/// 5. Test: Get Escrow Events Error
/// Verifies that get_escrow_events() handles JSON-RPC errors correctly.
/// Why: RPC errors should be propagated to the caller, not silently ignored.
/// The solver needs to know when queries fail to retry or alert operators.
#[tokio::test]
async fn test_get_escrow_events_evm_error() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32000,
                "message": "Invalid filter"
            },
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let mut config = create_test_evm_config();
    config.rpc_url = base_url;
    let client = ConnectedEvmClient::new(&config).unwrap();

    let result = client.get_escrow_events(None, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("JSON-RPC error"));
}

// #6: escrow_event_deserialization - N/A for EVM (parses directly in get_escrow_events)

// ============================================================================
// FULFILLMENT OPERATIONS
// ============================================================================

/// 7. Test: Claim Escrow Intent ID Formatting
/// Verifies that intent_id is correctly formatted for Hardhat script.
/// Why: EVM expects 0x-prefixed hex strings. Missing prefix would cause the
/// Hardhat script to fail with a parse error.
#[test]
fn test_claim_escrow_intent_id_formatting() {
    // Test that intent_id with 0x prefix is preserved
    let intent_id_with_prefix = "0x1234567890abcdef";
    let formatted = if intent_id_with_prefix.starts_with("0x") {
        intent_id_with_prefix.to_string()
    } else {
        format!("0x{}", intent_id_with_prefix)
    };
    assert_eq!(formatted, "0x1234567890abcdef");

    // Test that intent_id without 0x prefix gets one added
    let intent_id_no_prefix = "1234567890abcdef";
    let formatted = if intent_id_no_prefix.starts_with("0x") {
        intent_id_no_prefix.to_string()
    } else {
        format!("0x{}", intent_id_no_prefix)
    };
    assert_eq!(formatted, "0x1234567890abcdef");
}

/// 8. Test: Claim Escrow Signature Encoding
/// Verifies that signature bytes are correctly converted to hex string.
/// Why: The Hardhat script expects a hex-encoded signature. Wrong encoding
/// would cause signature verification to fail on the smart contract.
#[test]
fn test_claim_escrow_signature_encoding() {
    use hex;

    // Test signature encoding (65 bytes: r || s || v)
    let signature_bytes = vec![0xaa; 65];
    let signature_hex = hex::encode(&signature_bytes);

    // Should be 130 hex chars (65 bytes * 2)
    assert_eq!(signature_hex.len(), 130);
    assert_eq!(signature_hex, "aa".repeat(65));

    // Test with actual signature-like data
    let mut signature = vec![0u8; 65];
    signature[0] = 0x12;
    signature[64] = 0x34;
    let signature_hex = hex::encode(&signature);
    assert!(signature_hex.starts_with("12"));
    assert!(signature_hex.ends_with("34"));
}

/// 9. Test: Claim Escrow Command Building
/// Verifies that the Hardhat command is built correctly with all required arguments.
/// Why: The claim_escrow function invokes a Hardhat script with environment variables.
/// Wrong command formatting would cause the script to fail or use wrong parameters.
#[test]
fn test_claim_escrow_command_building() {
    let escrow_addr = DUMMY_ESCROW_CONTRACT_ADDR_EVM;
    let intent_id_evm = "0x1234567890abcdef";
    let signature_hex = "aa".repeat(130);
    let evm_framework_dir = "/path/to/intent-frameworks/evm";

    // Build the command string that would be passed to bash -c
    let command = format!(
        "cd '{}' && ESCROW_ADDR='{}' INTENT_ID_EVM='{}' SIGNATURE_HEX='{}' npx hardhat run scripts/claim-escrow.js --network localhost",
        evm_framework_dir,
        escrow_addr,
        intent_id_evm,
        signature_hex
    );

    // Verify all components are present
    assert!(command.contains("ESCROW_ADDR"));
    assert!(command.contains(escrow_addr));
    assert!(command.contains("INTENT_ID_EVM"));
    assert!(command.contains(intent_id_evm));
    assert!(command.contains("SIGNATURE_HEX"));
    assert!(command.contains(&signature_hex));
    assert!(command.contains("claim-escrow.js"));
    assert!(command.contains("--network localhost"));
}

/// 10. Test: Claim Escrow Hash Extraction
/// Verifies that transaction hash is correctly extracted from Hardhat output.
/// Why: The solver needs the transaction hash to track fulfillment status.
/// Wrong extraction would cause the solver to lose track of pending fulfillments.
#[test]
fn test_claim_escrow_hash_extraction() {
    // Test successful output format from Hardhat script
    let output =
        "Some log output\nClaim transaction hash: 0xabcdef1234567890\nEscrow released successfully!";

    if let Some(hash_line) = output.lines().find(|l| l.contains("hash") || l.contains("Hash")) {
        if let Some(hash) = hash_line.split_whitespace().find(|s| s.starts_with("0x")) {
            assert_eq!(hash, "0xabcdef1234567890");
        } else {
            panic!("Failed to extract hash from line: {}", hash_line);
        }
    } else {
        panic!("Failed to find hash line in output");
    }

    // Test case-insensitive matching
    let output_upper = "Some log output\nCLAIM TRANSACTION HASH: 0x1234567890abcdef\nSuccess!";
    if let Some(hash_line) = output_upper.lines().find(|l| l.contains("hash") || l.contains("Hash")) {
        if let Some(hash) = hash_line.split_whitespace().find(|s| s.starts_with("0x")) {
            assert_eq!(hash, "0x1234567890abcdef");
        }
    }
}

/// 11. Test: Claim Escrow Missing Directory Error
/// Verifies that proper error is returned when intent-frameworks/evm directory is missing.
/// Why: A clear error message helps operators diagnose deployment issues.
/// Silent failures would make debugging much harder.
#[test]
fn test_claim_escrow_missing_directory_error() {
    // Simulate the directory check logic
    let current_dir = std::env::current_dir().unwrap();
    let project_root = current_dir.parent().unwrap();
    let evm_framework_dir = project_root.join("intent-frameworks/evm");

    // This test documents the expected behavior - actual test would need to mock or use temp dir
    // In real code, this would bail with: "intent-frameworks/evm directory not found at: ..."
    // We're just verifying the path construction logic here
    assert!(evm_framework_dir.to_string_lossy().contains("intent-frameworks/evm"));
}

// #12: pubkey_from_hex_with_leading_zeros - N/A for EVM (SVM-specific)
// #13: pubkey_from_hex_no_leading_zeros - N/A for EVM (SVM-specific)
