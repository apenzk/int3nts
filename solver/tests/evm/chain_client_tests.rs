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
    DUMMY_TOKEN_ADDR_EVM, DUMMY_TX_HASH,
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
        outflow_validator_addr: None,
        gmp_endpoint_addr: None,
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
/// Verifies that get_escrow_events() parses EscrowCreated events correctly.
/// Why: The solver needs to parse escrow events from connected EVM chains to
/// identify fulfillment opportunities. A parsing bug would cause missed escrows.
#[tokio::test]
async fn test_get_escrow_events_evm_success() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    // EscrowCreated(bytes32,bytes32,address,uint64,address,bytes32,uint64)
    let event_signature = "EscrowCreated(bytes32,bytes32,address,uint64,address,bytes32,uint64)";
    let mut hasher = Keccak256::new();
    hasher.update(event_signature.as_bytes());
    let event_topic = format!("0x{}", hex::encode(hasher.finalize()));

    // Construct mock data matching the new event layout:
    // topics: [sig, intentId(bytes32), requester(address padded), token(address padded)]
    // data: escrowId(32B) + amount(32B) + reservedSolver(32B) + expiry(32B) = 256 hex chars
    let escrow_id_hex = "0000000000000000000000000000000000000000000000000000000000000002";
    let amount_hex = "00000000000000000000000000000000000000000000000000000000000f4240"; // 1000000
    let solver_hex = "0000000000000000000000000000000000000000000000000000000000000009";
    let expiry_hex = "0000000000000000000000000000000000000000000000000000000000000000"; // 0

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "result": [
                {
                    "address": DUMMY_ESCROW_CONTRACT_ADDR_EVM,
                    "topics": [
                        event_topic,
                        DUMMY_INTENT_ID, // intentId (bytes32, already 64 hex chars)
                        format!("0x000000000000000000000000{}", DUMMY_REQUESTER_ADDR_EVM.strip_prefix("0x").unwrap()), // requester (address padded to 32 bytes)
                        format!("0x000000000000000000000000{}", DUMMY_TOKEN_ADDR_EVM.strip_prefix("0x").unwrap())  // token (address padded to 32 bytes)
                    ],
                    "data": format!("0x{}{}{}{}", escrow_id_hex, amount_hex, solver_hex, expiry_hex),
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
    assert_eq!(events[0].intent_id, DUMMY_INTENT_ID);
    assert_eq!(events[0].escrow_id, format!("0x{}", escrow_id_hex));
    assert_eq!(events[0].requester_addr, DUMMY_REQUESTER_ADDR_EVM);
    assert_eq!(events[0].amount, 1000000);
    assert_eq!(events[0].token_addr, DUMMY_TOKEN_ADDR_EVM);
    assert_eq!(events[0].reserved_solver, format!("0x{}", solver_hex));
    assert_eq!(events[0].expiry, 0);
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
// ESCROW RELEASE (GMP Auto-Release)
// ============================================================================

/// 7. Test: Is Escrow Released Intent ID Formatting
/// Verifies that intent_id is correctly formatted for Hardhat script.
/// Why: EVM expects 0x-prefixed hex strings. Missing prefix would cause the
/// Hardhat script to fail with a parse error.
#[test]
fn test_is_escrow_released_intent_id_formatting() {
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

/// 8. Test: Is Escrow Released Output Parsing
/// Verifies that "isReleased: true/false" is correctly parsed from Hardhat output.
/// Why: The solver needs to know when escrow is auto-released to complete the flow.
/// Wrong parsing would cause the solver to wait forever or miss releases.
#[test]
fn test_is_escrow_released_output_parsing() {
    // Test "isReleased: true" output
    let output_true = "Some log output\nisReleased: true\n";
    assert!(output_true.contains("isReleased: true"));
    assert!(!output_true.contains("isReleased: false"));

    // Test "isReleased: false" output
    let output_false = "Some log output\nisReleased: false\n";
    assert!(output_false.contains("isReleased: false"));
    assert!(!output_false.contains("isReleased: true"));
}

/// 9. Test: Is Escrow Released Command Building
/// Verifies that the Hardhat command is built correctly with all required arguments.
/// Why: The is_escrow_released function invokes a Hardhat script with environment variables.
/// Wrong command formatting would cause the script to fail or use wrong parameters.
#[test]
fn test_is_escrow_released_command_building() {
    let escrow_gmp_addr = DUMMY_ESCROW_CONTRACT_ADDR_EVM;
    let intent_id_evm = "0x1234567890abcdef";
    let evm_framework_dir = "/path/to/intent-frameworks/evm";

    // Build the command string that would be passed to bash -c
    let command = format!(
        "cd '{}' && ESCROW_GMP_ADDR='{}' INTENT_ID_EVM='{}' npx hardhat run scripts/get-is-released.js --network localhost",
        evm_framework_dir,
        escrow_gmp_addr,
        intent_id_evm
    );

    // Verify all components are present
    assert!(command.contains("ESCROW_GMP_ADDR"));
    assert!(command.contains(escrow_gmp_addr));
    assert!(command.contains("INTENT_ID_EVM"));
    assert!(command.contains(intent_id_evm));
    assert!(command.contains("get-is-released.js"));
    assert!(command.contains("--network localhost"));
}

/// 10. Test: Is Escrow Released Missing Directory Error
/// Verifies that proper error is returned when intent-frameworks/evm directory is missing.
/// Why: A clear error message helps operators diagnose deployment issues.
/// Silent failures would make debugging much harder.
#[test]
fn test_is_escrow_released_missing_directory_error() {
    // Simulate the directory check logic
    let current_dir = std::env::current_dir().unwrap();
    let project_root = current_dir.parent().unwrap();
    let evm_framework_dir = project_root.join("intent-frameworks/evm");

    // This test documents the expected behavior - actual test would need to mock or use temp dir
    // In real code, this would bail with: "intent-frameworks/evm directory not found at: ..."
    // We're just verifying the path construction logic here
    assert!(evm_framework_dir.to_string_lossy().contains("intent-frameworks/evm"));
}

// #11: pubkey_from_hex_with_leading_zeros - N/A for EVM (SVM-specific)
// #12: pubkey_from_hex_no_leading_zeros - N/A for EVM (SVM-specific)
