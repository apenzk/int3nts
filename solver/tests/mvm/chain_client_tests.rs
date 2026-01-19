//! Unit tests for MVM chain clients

use serde_json::json;
use solver::chains::{ConnectedMvmClient, HubChainClient};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "../helpers.rs"]
mod test_helpers;
use test_helpers::{
    create_default_connected_mvm_chain_config, create_default_hub_chain_config,
    DUMMY_ESCROW_ID_MVM, DUMMY_EXPIRY, DUMMY_INTENT_ADDR_HUB, DUMMY_INTENT_ID,
    DUMMY_MODULE_ADDR_CON, DUMMY_MODULE_ADDR_HUB, DUMMY_REQUESTER_ADDR_MVMCON,
    DUMMY_REQUESTER_ADDR_HUB, DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_ADDR_MVMCON,
    DUMMY_TOKEN_ADDR_HUB, DUMMY_TOKEN_ADDR_MVMCON,
};

// ============================================================================
// JSON PARSING TESTS
// ============================================================================

/// What is tested: IntentCreatedEvent deserialization
/// Why: Ensure we can parse intent creation events from hub chain
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

/// What is tested: EscrowEvent deserialization (MVM)
/// Why: Ensure we can parse escrow events from connected MVM chain
/// Note: Field names match Move's OracleLimitOrderEvent (intent_addr, requester, reserved_solver as Move Option)
#[test]
fn test_escrow_event_deserialization() {
    let json = json!({
        "intent_addr": DUMMY_ESCROW_ID_MVM,
        "intent_id": DUMMY_INTENT_ID,
        "requester_addr": DUMMY_REQUESTER_ADDR_MVMCON,
        "offered_metadata": {"inner": DUMMY_TOKEN_ADDR_HUB},
        "offered_amount": "1000",
        "desired_metadata": {"inner": DUMMY_TOKEN_ADDR_MVMCON},
        "desired_amount": "2000",
        "expiry_time": DUMMY_EXPIRY.to_string(),
        "revocable": true,
        "reserved_solver": {"vec": [DUMMY_SOLVER_ADDR_MVMCON]}
    });

    let event: solver::chains::connected_mvm::EscrowEvent = serde_json::from_value(json).unwrap();
    assert_eq!(event.escrow_id, DUMMY_ESCROW_ID_MVM);
    assert_eq!(event.intent_id, DUMMY_INTENT_ID);
    assert_eq!(event.requester_addr, DUMMY_REQUESTER_ADDR_MVMCON);
    assert_eq!(event.offered_amount, "1000");
    // reserved_solver is a Move Option wrapper
    let solver = event.reserved_solver.and_then(|opt| opt.into_option());
    assert_eq!(solver, Some(DUMMY_SOLVER_ADDR_MVMCON.to_string()));
}

// ============================================================================
// HUB CHAIN CLIENT TESTS
// ============================================================================

/// What is tested: HubChainClient::new() creates a client with correct config
/// Why: Ensure client initialization works correctly
#[test]
fn test_hub_client_new() {
    let config = create_default_hub_chain_config();
    let _client = HubChainClient::new(&config).unwrap();
}

/// What is tested: get_intent_events() parses transaction events correctly
/// Why: Ensure we can extract intent creation events from transaction history
/// Note: The account parameter is the requester address (the account that created the intent).
///       We query transactions sent by this requester to find intent creation events.
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

/// What is tested: get_intent_events() handles empty transaction list
/// Why: Ensure we handle accounts with no transactions gracefully
/// Note: The account parameter is the requester address (the account that created the intent).
///       We query transactions sent by this requester to find intent creation events.
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

/// What is tested: is_solver_registered() returns true for registered solver
/// Why: Ensure we can check if a solver is registered on-chain
#[tokio::test]
async fn test_is_solver_registered_true() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    // Mock view function response - returns [true] for registered solver
    // Note: HubChainClient calls /v1/view on the base_url
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

/// What is tested: is_solver_registered() returns false for unregistered solver
/// Why: Ensure we correctly detect when a solver is not registered
#[tokio::test]
async fn test_is_solver_registered_false() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    // Mock view function response - returns [false] for unregistered solver
    // Note: HubChainClient calls /v1/view on the base_url
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

/// What is tested: is_solver_registered() handles address normalization (with/without 0x prefix)
/// Why: Ensure address format doesn't matter
#[tokio::test]
async fn test_is_solver_registered_address_normalization() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    // Note: HubChainClient calls /v1/view on the base_url
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

/// What is tested: is_solver_registered() handles HTTP errors
/// Why: Ensure network errors are properly propagated
#[tokio::test]
async fn test_is_solver_registered_http_error() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    // Mock HTTP 500 error
    // Note: HubChainClient calls /v1/view on the base_url
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

/// What is tested: is_solver_registered() handles invalid JSON response
/// Why: Ensure malformed responses are handled gracefully
#[tokio::test]
async fn test_is_solver_registered_invalid_json() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    // Mock invalid JSON response (not an array, or wrong format)
    // Note: HubChainClient calls /v1/view on the base_url
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

/// What is tested: is_solver_registered() handles unexpected response format
/// Why: Ensure we handle cases where response is not a boolean array
#[tokio::test]
async fn test_is_solver_registered_unexpected_format() {
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri().to_string();

    // Mock response with wrong format (empty array or non-boolean)
    // Note: HubChainClient calls /v1/view on the base_url
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

// ============================================================================
// CONNECTED MVM CLIENT TESTS
// ============================================================================

/// What is tested: ConnectedMvmClient::new() creates a client with correct config
/// Why: Ensure client initialization works correctly
#[test]
fn test_mvm_client_new() {
    let config = create_default_connected_mvm_chain_config();
    let _client = ConnectedMvmClient::new(&config).unwrap();
}

/// What is tested: get_escrow_events() parses OracleLimitOrderEvent correctly
/// Why: Ensure we can extract escrow events from connected MVM chain
/// Note: The account parameter is the requester address (the account that created the escrow).
///       We query transactions sent by this requester to find escrow creation events.
#[tokio::test]
async fn test_get_escrow_events_success() {
    let mock_server = MockServer::start().await;
    // Note: base_url simulates the full RPC URL including /v1 suffix
    let base_url = format!("{}/v1", mock_server.uri());

    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/accounts/{}/transactions",
            DUMMY_REQUESTER_ADDR_MVMCON.strip_prefix("0x").unwrap()
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "events": [
                    {
                        "type": format!(
                            "{}::fa_intent_with_oracle::OracleLimitOrderEvent",
                            DUMMY_MODULE_ADDR_CON
                        ),
                        "data": {
                            "intent_addr": DUMMY_ESCROW_ID_MVM,
                            "intent_id": DUMMY_INTENT_ID,
                            "requester_addr": DUMMY_REQUESTER_ADDR_MVMCON,
                            "offered_metadata": {"inner": DUMMY_TOKEN_ADDR_MVMCON},
                            "offered_amount": "1000",
                            "desired_metadata": {"inner": DUMMY_TOKEN_ADDR_MVMCON},
                            "desired_amount": "2000",
                            "expiry_time": DUMMY_EXPIRY.to_string(),
                            "revocable": true,
                            "reserved_solver": {"vec": [DUMMY_SOLVER_ADDR_MVMCON]}
                        }
                    }
                ]
            }
        ])))
        .mount(&mock_server)
        .await;

    let mut config = create_default_connected_mvm_chain_config();
    config.rpc_url = base_url;
    let client = ConnectedMvmClient::new(&config).unwrap();

    let accounts = vec![DUMMY_REQUESTER_ADDR_MVMCON.to_string()];
    let events = client.get_escrow_events(&accounts, None).await.unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].intent_id, DUMMY_INTENT_ID);
    assert_eq!(events[0].escrow_id, DUMMY_ESCROW_ID_MVM);
}
