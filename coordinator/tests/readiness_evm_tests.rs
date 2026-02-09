//! EVM-specific readiness tracking tests
//!
//! These tests verify that the coordinator correctly monitors IntentRequirementsReceived
//! events from EVM connected chains and marks intents as ready.

use coordinator::monitor::{EventMonitor, poll_evm_requirements_received};
use serde_json::json;
use wiremock::matchers::{body_json_string, body_partial_json, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "mod.rs"]
mod test_helpers;
use test_helpers::{build_test_config_with_evm, create_default_intent_evm, DUMMY_INTENT_ID};

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Mount an eth_blockNumber mock returning block 0x3e8 (1000).
/// With event_block_range=1000, fromBlock = 1000 - 1000 = 0 = "0x0".
async fn mount_eth_block_number_mock(mock_server: &MockServer) {
    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "eth_blockNumber"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": "0x3e8"
        })))
        .mount(mock_server)
        .await;
}

/// Create a mock eth_getLogs response with IntentRequirementsReceived event
fn create_eth_get_logs_response(intent_id: &str) -> serde_json::Value {
    // Remove 0x prefix if present for data field
    let intent_id_hex = intent_id.strip_prefix("0x").unwrap_or(intent_id);

    // Pad intent_id to 64 hex characters (32 bytes)
    let intent_id_padded = format!("{:0>64}", intent_id_hex);

    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": [{
            "address": "0x0000000000000000000000000000000000000010",
            "topics": [
                // IntentRequirementsReceived event signature (keccak256)
                "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            ],
            "data": format!(
                "0x{}{}{}{}{}{}{}",
                intent_id_padded,
                "0000000000000000000000000000000000000000000000000000000000000001", // src_chain_id
                "000000000000000000000000000000000000000000000000000000000000abc", // requester_addr
                "00000000000000000000000000000000000000000000000000000000000003e8", // amount_required (1000)
                "000000000000000000000000000000000000000000000000000000000000token", // token_addr
                "000000000000000000000000000000000000000000000000000000000solver", // solver_addr
                "00000000000000000000000000000000000000000000000000000002540be3ff"  // expiry (9999999999)
            ),
            "blockNumber": "0x64",
            "transactionHash": "0xabc123",
            "logIndex": "0x0"
        }]
    })
}

// ============================================================================
// TESTS
// ============================================================================

// 1. Test: poll_evm_requirements_received parses IntentRequirementsReceived events
/// Test that poll_evm_requirements_received parses IntentRequirementsReceived events
/// What is tested: Event parsing and intent_id extraction from EVM events
/// Why: Coordinator must correctly parse EVM event format to mark intents as ready
#[tokio::test]
async fn test_poll_evm_requirements_received_parses_event() {
    let mock_server = MockServer::start().await;

    mount_eth_block_number_mock(&mock_server).await;

    // Mock the eth_getLogs endpoint
    let expected_body = json!({
        "jsonrpc": "2.0",
        "method": "eth_getLogs",
        "params": [{
            "address": "0x0000000000000000000000000000000000000010",
            "fromBlock": "0x0",
            "topics": ["0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"]
        }],
        "id": 1
    });

    Mock::given(method("POST"))
        .and(body_json_string(expected_body.to_string()))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            create_eth_get_logs_response(DUMMY_INTENT_ID),
        ))
        .mount(&mock_server)
        .await;

    // Create config with mock server URL
    let mut config = build_test_config_with_evm();
    config.connected_chain_evm.as_mut().unwrap().rpc_url = mock_server.uri();

    let monitor = EventMonitor::new(&config).await.unwrap();

    // Add a test intent to the cache
    let intent = create_default_intent_evm();
    {
        let mut cache = monitor.event_cache.write().await;
        cache.push(intent);
    }

    // Poll for requirements received events
    let result = poll_evm_requirements_received(&monitor).await;
    assert!(result.is_ok(), "Polling should succeed");

    let count = result.unwrap();
    assert_eq!(count, 1, "Should process one event");

    // Verify intent is marked as ready
    let cached = monitor.get_cached_events().await;
    assert_eq!(cached.len(), 1);
    assert_eq!(cached[0].ready_on_connected_chain, true);
}

// 2. Test: poll_evm_requirements_received handles empty event list
/// Test that poll_evm_requirements_received handles empty event list
/// What is tested: Handling of no new events
/// Why: Polling should succeed even when no events are found
#[tokio::test]
async fn test_poll_evm_requirements_received_handles_empty_events() {
    let mock_server = MockServer::start().await;

    mount_eth_block_number_mock(&mock_server).await;

    // Mock empty event response
    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "eth_getLogs"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": []
        })))
        .mount(&mock_server)
        .await;

    let mut config = build_test_config_with_evm();
    config.connected_chain_evm.as_mut().unwrap().rpc_url = mock_server.uri();

    let monitor = EventMonitor::new(&config).await.unwrap();

    let result = poll_evm_requirements_received(&monitor).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0, "Should process zero events");
}

// 3. Test: poll_evm_requirements_received handles multiple events
/// Test that poll_evm_requirements_received handles multiple events
/// What is tested: Processing multiple IntentRequirementsReceived events in one poll
/// Why: Coordinator should handle batch event processing
#[tokio::test]
async fn test_poll_evm_requirements_received_handles_multiple_events() {
    let mock_server = MockServer::start().await;

    mount_eth_block_number_mock(&mock_server).await;

    let intent_id_1 = "0x0000000000000000000000000000000000000000000000000000000000000001";
    let intent_id_2 = "0x0000000000000000000000000000000000000000000000000000000000000002";

    // Mock multiple events
    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "eth_getLogs"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": [
                {
                    "address": "0x0000000000000000000000000000000000000010",
                    "topics": ["0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"],
                    "data": format!(
                        "0x{}000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000abc00000000000000000000000000000000000000000000000000000000000003e8000000000000000000000000000000000000000000000000000000000000token000000000000000000000000000000000000000000000000000000000solver00000000000000000000000000000000000000000000000000000002540be3ff",
                        intent_id_1.trim_start_matches("0x")
                    ),
                    "blockNumber": "0x64",
                    "transactionHash": "0xabc123",
                    "logIndex": "0x0"
                },
                {
                    "address": "0x0000000000000000000000000000000000000010",
                    "topics": ["0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"],
                    "data": format!(
                        "0x{}000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000abc00000000000000000000000000000000000000000000000000000000000007d0000000000000000000000000000000000000000000000000000000000000token000000000000000000000000000000000000000000000000000000000solver00000000000000000000000000000000000000000000000000000002540be3ff",
                        intent_id_2.trim_start_matches("0x")
                    ),
                    "blockNumber": "0x65",
                    "transactionHash": "0xdef456",
                    "logIndex": "0x0"
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let mut config = build_test_config_with_evm();
    config.connected_chain_evm.as_mut().unwrap().rpc_url = mock_server.uri();

    let monitor = EventMonitor::new(&config).await.unwrap();

    // Add two test intents to cache
    {
        let mut cache = monitor.event_cache.write().await;
        let mut intent1 = create_default_intent_evm();
        intent1.intent_id = intent_id_1.to_string();
        cache.push(intent1);

        let mut intent2 = create_default_intent_evm();
        intent2.intent_id = intent_id_2.to_string();
        cache.push(intent2);
    }

    let result = poll_evm_requirements_received(&monitor).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 2, "Should process two events");

    // Verify both intents are marked as ready
    let cached = monitor.get_cached_events().await;
    assert_eq!(cached.len(), 2);
    assert!(cached.iter().all(|i| i.ready_on_connected_chain));
}

// 4. Test: poll_evm_requirements_received handles intent ID normalization
/// Test that poll_evm_requirements_received handles intent ID normalization
/// What is tested: Intent ID normalization (leading zeros)
/// Why: Intent IDs from events may have different leading zero formats
#[tokio::test]
async fn test_poll_evm_requirements_received_normalizes_intent_id() {
    let mock_server = MockServer::start().await;

    mount_eth_block_number_mock(&mock_server).await;

    // Event has intent ID with leading zeros
    let event_intent_id = "0x00000001";
    // Cached intent has normalized ID (no leading zeros)
    let cache_intent_id = "0x1";

    Mock::given(method("POST"))
        .and(body_partial_json(json!({"method": "eth_getLogs"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            create_eth_get_logs_response(event_intent_id),
        ))
        .mount(&mock_server)
        .await;

    let mut config = build_test_config_with_evm();
    config.connected_chain_evm.as_mut().unwrap().rpc_url = mock_server.uri();

    let monitor = EventMonitor::new(&config).await.unwrap();

    // Add intent with normalized ID
    {
        let mut cache = monitor.event_cache.write().await;
        let mut intent = create_default_intent_evm();
        intent.intent_id = cache_intent_id.to_string();
        cache.push(intent);
    }

    let result = poll_evm_requirements_received(&monitor).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1, "Should process one event");

    // Verify intent is marked as ready despite different ID format
    let cached = monitor.get_cached_events().await;
    assert_eq!(cached.len(), 1);
    assert_eq!(cached[0].ready_on_connected_chain, true);
}
