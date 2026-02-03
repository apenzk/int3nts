//! Unit tests for MVM Hub chain client
//!
//! These tests are MVM-only because the hub is always an MVM chain.
//! Hub tests are NOT in EXTENSION-CHECKLIST.md (that's for connected chain tests only).
//! Connected chain tests are in chain_client_tests.rs (synchronized across VMs).

use serde_json::json;
use solver::chains::HubChainClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "../helpers.rs"]
mod test_helpers;
use test_helpers::{
    create_default_hub_chain_config, DUMMY_EXPIRY, DUMMY_INTENT_ADDR_HUB, DUMMY_INTENT_ID,
    DUMMY_MODULE_ADDR_HUB, DUMMY_REQUESTER_ADDR_HUB, DUMMY_SOLVER_ADDR_HUB, DUMMY_TOKEN_ADDR_HUB,
    DUMMY_TOKEN_ADDR_MVMCON,
};

// ============================================================================
// HUB CLIENT INITIALIZATION
// ============================================================================

/// 1. Test: HubChainClient Initialization
/// Verifies that HubChainClient::new() creates a client with correct config.
/// Why: Client initialization is the entry point for all hub operations. A failure
/// here would prevent any solver operations on the hub chain.
#[test]
fn test_hub_client_new() {
    let config = create_default_hub_chain_config();
    let _client = HubChainClient::new(&config).unwrap();
}

// ============================================================================
// INTENT EVENT DESERIALIZATION
// ============================================================================

/// 2. Test: IntentCreatedEvent Deserialization
/// Verifies that IntentCreatedEvent deserializes correctly from JSON.
/// Why: Intent events have a specific JSON structure. A deserialization bug would
/// cause the solver to miss intent opportunities or parse wrong data.
#[test]
fn test_intent_created_event_deserialization() {
    let json = json!({
        "intent_addr": DUMMY_INTENT_ADDR_HUB,
        "intent_id": DUMMY_INTENT_ID,
        "offered_metadata": {"inner": DUMMY_TOKEN_ADDR_HUB},
        "offered_amount": "1000",
        "offered_chain_id": "1",
        "desired_metadata": {"inner": DUMMY_TOKEN_ADDR_MVMCON},
        "desired_amount": "2000",
        "desired_chain_id": "2",
        "requester_addr": DUMMY_REQUESTER_ADDR_HUB,
        "expiry_time": DUMMY_EXPIRY.to_string()
    });

    let event: solver::chains::hub::IntentCreatedEvent = serde_json::from_value(json).unwrap();
    assert_eq!(event.intent_addr, DUMMY_INTENT_ADDR_HUB);
    assert_eq!(event.intent_id, DUMMY_INTENT_ID);
    assert_eq!(event.offered_amount, "1000");
    assert_eq!(event.desired_amount, "2000");
    assert_eq!(event.requester_addr, DUMMY_REQUESTER_ADDR_HUB);
}

// ============================================================================
// INTENT EVENT QUERYING
// ============================================================================

/// 3. Test: Get Intent Events Success
/// Verifies that get_intent_events() parses transaction events correctly.
/// Why: The solver needs to discover intents from the hub chain. A parsing bug
/// would cause missed intent opportunities.
#[tokio::test]
async fn test_get_intent_events_success() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    // Mock transaction response with LimitOrderEvent
    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/accounts/{}/transactions",
            DUMMY_REQUESTER_ADDR_HUB.strip_prefix("0x").unwrap()
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "events": [
                    {
                        "type": format!("{}::fa_intent::LimitOrderEvent", DUMMY_MODULE_ADDR_HUB),
                        "data": {
                            "intent_addr": DUMMY_INTENT_ADDR_HUB,
                            "intent_id": DUMMY_INTENT_ID,
                            "offered_metadata": {"inner": DUMMY_TOKEN_ADDR_HUB},
                            "offered_amount": "1000",
                            "offered_chain_id": "1",
                            "desired_metadata": {"inner": DUMMY_TOKEN_ADDR_MVMCON},
                            "desired_amount": "2000",
                            "desired_chain_id": "2",
                            "requester_addr": DUMMY_REQUESTER_ADDR_HUB,
                            "expiry_time": DUMMY_EXPIRY.to_string(),
                            "revocable": true
                        }
                    }
                ]
            }
        ])))
        .mount(&mock_server)
        .await;

    let mut config = create_default_hub_chain_config();
    config.rpc_url = base_url;
    let client = HubChainClient::new(&config).unwrap();

    let accounts = vec![DUMMY_REQUESTER_ADDR_HUB.to_string()];
    let (events, _tx_hashes) = client.get_intent_events(&accounts, None, None).await.unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].intent_id, DUMMY_INTENT_ID);
    assert_eq!(events[0].offered_amount, "1000");
}

/// 4. Test: Get Intent Events Empty
/// Verifies that get_intent_events() handles empty transaction list correctly.
/// Why: A hub with no intents should return an empty list, not an error.
/// The solver should handle this gracefully and continue polling.
#[tokio::test]
async fn test_get_intent_events_empty() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/accounts/{}/transactions",
            DUMMY_REQUESTER_ADDR_HUB.strip_prefix("0x").unwrap()
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    let mut config = create_default_hub_chain_config();
    config.rpc_url = base_url;
    let client = HubChainClient::new(&config).unwrap();

    let accounts = vec![DUMMY_REQUESTER_ADDR_HUB.to_string()];
    let (events, _tx_hashes) = client.get_intent_events(&accounts, None, None).await.unwrap();

    assert_eq!(events.len(), 0);
}

// ============================================================================
// SOLVER REGISTRATION CHECKS
// ============================================================================

/// 5. Test: Is Solver Registered True
/// Verifies that is_solver_registered() returns true for registered solver.
/// Why: The solver needs to verify its registration before attempting fulfillments.
/// A false negative would cause the solver to skip valid fulfillment opportunities.
#[tokio::test]
async fn test_is_solver_registered_true() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([true])))
        .mount(&mock_server)
        .await;

    let mut config = create_default_hub_chain_config();
    config.rpc_url = base_url;
    let client = HubChainClient::new(&config).unwrap();

    let is_registered = client
        .is_solver_registered(DUMMY_SOLVER_ADDR_HUB)
        .await
        .unwrap();

    assert!(is_registered);
}

/// 6. Test: Is Solver Registered False
/// Verifies that is_solver_registered() returns false for unregistered solver.
/// Why: The solver must correctly detect when it's not registered to avoid
/// wasting gas on transactions that will fail.
#[tokio::test]
async fn test_is_solver_registered_false() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([false])))
        .mount(&mock_server)
        .await;

    let mut config = create_default_hub_chain_config();
    config.rpc_url = base_url;
    let client = HubChainClient::new(&config).unwrap();

    let is_registered = client
        .is_solver_registered(DUMMY_SOLVER_ADDR_HUB)
        .await
        .unwrap();

    assert!(!is_registered);
}

/// 7. Test: Is Solver Registered Address Normalization
/// Verifies that is_solver_registered() handles addresses with/without 0x prefix.
/// Why: Address format shouldn't affect registration checks. Both formats should work.
#[tokio::test]
async fn test_is_solver_registered_address_normalization() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([true])))
        .mount(&mock_server)
        .await;

    let mut config = create_default_hub_chain_config();
    config.rpc_url = base_url;
    let client = HubChainClient::new(&config).unwrap();

    // Test with 0x prefix
    let is_registered1 = client
        .is_solver_registered(DUMMY_SOLVER_ADDR_HUB)
        .await
        .unwrap();
    assert!(is_registered1);

    // Test without 0x prefix (should still work)
    let is_registered2 = client
        .is_solver_registered(DUMMY_SOLVER_ADDR_HUB.strip_prefix("0x").unwrap())
        .await
        .unwrap();
    assert!(is_registered2);
}

/// 8. Test: Is Solver Registered HTTP Error
/// Verifies that is_solver_registered() handles HTTP errors correctly.
/// Why: Network errors should be propagated to the caller, not silently ignored.
/// The solver needs to know when queries fail to retry or alert operators.
#[tokio::test]
async fn test_is_solver_registered_http_error() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let mut config = create_default_hub_chain_config();
    config.rpc_url = base_url;
    let client = HubChainClient::new(&config).unwrap();

    let result = client
        .is_solver_registered(DUMMY_SOLVER_ADDR_HUB)
        .await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to query solver registration"));
}

/// 9. Test: Is Solver Registered Invalid JSON
/// Verifies that is_solver_registered() handles invalid JSON responses correctly.
/// Why: Malformed responses should result in errors, not silent failures or panics.
#[tokio::test]
async fn test_is_solver_registered_invalid_json() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
        .mount(&mock_server)
        .await;

    let mut config = create_default_hub_chain_config();
    config.rpc_url = base_url;
    let client = HubChainClient::new(&config).unwrap();

    let result = client
        .is_solver_registered(DUMMY_SOLVER_ADDR_HUB)
        .await;

    assert!(result.is_err());
}

/// 10. Test: Is Solver Registered Unexpected Format
/// Verifies that is_solver_registered() handles unexpected response formats correctly.
/// Why: A response with wrong format (empty array, non-boolean) should fail clearly.
#[tokio::test]
async fn test_is_solver_registered_unexpected_format() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    let mut config = create_default_hub_chain_config();
    config.rpc_url = base_url;
    let client = HubChainClient::new(&config).unwrap();

    let result = client
        .is_solver_registered(DUMMY_SOLVER_ADDR_HUB)
        .await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Unexpected response format"));
}
