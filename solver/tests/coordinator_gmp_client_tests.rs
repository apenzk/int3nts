//! Unit tests for coordinator/trusted-gmp API client (coordinator_gmp_client module)

use serde_json::json;
use solver::{
    ApiResponse, CoordinatorGmpClient, PendingDraft, SignatureSubmission, SignatureSubmissionResponse,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "helpers.rs"]
mod test_helpers;
use test_helpers::{
    DUMMY_DRAFT_ID, DUMMY_REQUESTER_ADDR_EVM,
    DUMMY_SOLVER_ADDR_HUB,
};

// ============================================================================
// JSON PARSING TESTS
// ============================================================================

/// What is tested: CoordinatorGmpClient::new() creates a client with correct base URL
/// Why: Ensure client initialization works correctly
#[test]
fn test_coordinator_gmp_client_new() {
    let _client = CoordinatorGmpClient::new("http://127.0.0.1:3333");
    // Client should be created successfully
    // We can't easily test the internal state without exposing it, but we can test methods
    // Actual HTTP functionality tested in integration tests
}

/// What is tested: CoordinatorGmpClient methods handle API response format correctly
/// Why: Ensure we correctly parse the ApiResponse<T> wrapper from coordinator/trusted-gmp
#[test]
fn test_api_response_parsing() {
    // Test successful response
    // Using test-specific timestamp (1000000) and expiry_time (2000000) for mock data
    let json = format!(r#"{{
        "success": true,
        "data": [
            {{
                "draft_id": "{}",
                "requester_addr": "{}",
                "draft_data": {{
                    "offered_metadata": {{"inner": "0xa"}},
                    "offered_amount": 1000,
                    "desired_metadata": {{"inner": "0xb"}},
                    "desired_amount": 2000
                }},
                "timestamp": 1000000,
                "expiry_time": 2000000
            }}
        ],
        "error": null
    }}"#, DUMMY_DRAFT_ID, DUMMY_REQUESTER_ADDR_EVM);

    let response: ApiResponse<Vec<PendingDraft>> = serde_json::from_str(&json).unwrap();
    assert!(response.success);
    assert!(response.data.is_some());
    assert!(response.error.is_none());

    let drafts = response.data.unwrap();
    assert_eq!(drafts.len(), 1);
    assert_eq!(
        drafts[0].draft_id,
        DUMMY_DRAFT_ID
    );
    assert_eq!(
        drafts[0].requester_addr,
        DUMMY_REQUESTER_ADDR_EVM
    );
}

/// What is tested: API error response parsing
/// Why: Ensure we correctly handle error responses from coordinator/trusted-gmp
#[test]
fn test_api_error_response_parsing() {
    let json = r#"{
        "success": false,
        "data": null,
        "error": "Draft already signed by another solver"
    }"#;

    let response: ApiResponse<SignatureSubmissionResponse> =
        serde_json::from_str(json).unwrap();
    assert!(!response.success);
    assert!(response.data.is_none());
    assert_eq!(
        response.error,
        Some("Draft already signed by another solver".to_string())
    );
}

/// What is tested: SignatureSubmission serialization
/// Why: Ensure request format matches coordinator/trusted-gmp API expectations
#[test]
fn test_signature_submission_serialization() {
    let submission = SignatureSubmission {
        solver_hub_addr: DUMMY_SOLVER_ADDR_HUB.to_string(),
        signature: "0x".to_string() + &"a".repeat(128), // 128 hex chars = 64 bytes signature (ECDSA format)
        public_key: "0x".to_string() + &"b".repeat(64), // 64 hex chars = 32 bytes public key (Ed25519 format)
    };

    let json = serde_json::to_string(&submission).unwrap();
    let parsed: SignatureSubmission = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.solver_hub_addr, submission.solver_hub_addr);
    assert_eq!(parsed.signature, submission.signature);
    assert_eq!(parsed.public_key, submission.public_key);
}

/// What is tested: PendingDraft deserialization with various draft_data formats
/// Why: Ensure we can handle different draft_data JSON structures from coordinator
#[test]
fn test_pending_draft_deserialization() {
    // Using test-specific timestamp (1000000) and expiry_time (2000000) for mock data
    let json = format!(r#"{{
        "draft_id": "{}",
        "requester_addr": "{}",
        "draft_data": {{
            "offered_metadata": {{"inner": "0xa"}},
            "offered_amount": 1000,
            "offered_chain_id": 1,
            "desired_metadata": {{"inner": "0xb"}},
            "desired_amount": 2000,
            "desired_chain_id": 2
        }},
        "timestamp": 1000000,
        "expiry_time": 2000000
    }}"#, DUMMY_DRAFT_ID, DUMMY_REQUESTER_ADDR_EVM);

    let draft: PendingDraft = serde_json::from_str(&json).unwrap();
    assert_eq!(
        draft.draft_id,
        DUMMY_DRAFT_ID
    );
    assert_eq!(
        draft.requester_addr,
        DUMMY_REQUESTER_ADDR_EVM
    );
    // Assertions use test-specific timestamp (1000000) and expiry_time (2000000)
    assert_eq!(draft.timestamp, 1000000);
    assert_eq!(draft.expiry_time, 2000000);

    // Verify draft_data is accessible as JSON value
    assert!(draft.draft_data.is_object());
    let draft_obj = draft.draft_data.as_object().unwrap();
    assert!(draft_obj.contains_key("offered_amount"));
    assert!(draft_obj.contains_key("desired_amount"));
}

// ============================================================================
// HTTP MOCKING TESTS
// ============================================================================

// ----------------------------------------------------------------------------
// poll_pending_drafts() tests
// ----------------------------------------------------------------------------

/// What is tested: poll_pending_drafts() successfully fetches pending drafts
/// Why: Ensure HTTP GET request works correctly and parses response
#[test]
fn test_poll_pending_drafts_success() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_mock_server, base_url) = rt.block_on(async {
        let mock_server = MockServer::start().await;

        // Using test-specific timestamp (1000000) and expiry_time (2000000) for mock data
        let response = json!({
            "success": true,
            "data": [
                {
                    "draft_id": DUMMY_DRAFT_ID,
                    "requester_addr": DUMMY_REQUESTER_ADDR_EVM,
                    "draft_data": {
                        "offered_metadata": {"inner": "0xa"},
                        "offered_amount": 1000,
                        "desired_metadata": {"inner": "0xb"},
                        "desired_amount": 2000
                    },
                    "timestamp": 1000000,
                    "expiry_time": 2000000
                }
            ],
            "error": null
        });

        Mock::given(method("GET"))
            .and(path("/draftintents/pending"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri().to_string();
        (mock_server, base_url)
    });

    let client = CoordinatorGmpClient::new(base_url);
    let drafts = client.poll_pending_drafts().unwrap();

    assert_eq!(drafts.len(), 1);
    assert_eq!(
        drafts[0].draft_id,
        DUMMY_DRAFT_ID
    );
}

/// What is tested: poll_pending_drafts() handles empty list
/// Why: Ensure empty response is handled correctly
#[test]
fn test_poll_pending_drafts_empty() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_mock_server, base_url) = rt.block_on(async {
        let mock_server = MockServer::start().await;

        let response = json!({
            "success": true,
            "data": [],
            "error": null
        });

        Mock::given(method("GET"))
            .and(path("/draftintents/pending"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri().to_string();
        (mock_server, base_url)
    });

    let client = CoordinatorGmpClient::new(base_url);
    let drafts = client.poll_pending_drafts().unwrap();

    assert_eq!(drafts.len(), 0);
}

/// What is tested: poll_pending_drafts() handles API error response
/// Why: Ensure error responses are properly converted to errors
#[test]
fn test_poll_pending_drafts_error() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_mock_server, base_url) = rt.block_on(async {
        let mock_server = MockServer::start().await;

        let response = json!({
            "success": false,
            "data": null,
            "error": "Internal server error"
        });

        Mock::given(method("GET"))
            .and(path("/draftintents/pending"))
            .respond_with(ResponseTemplate::new(500).set_body_json(response))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri().to_string();
        (mock_server, base_url)
    });

    let client = CoordinatorGmpClient::new(base_url);
    let result = client.poll_pending_drafts();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Internal server error"));
}

// ----------------------------------------------------------------------------
// submit_signature() tests
// ----------------------------------------------------------------------------

/// What is tested: submit_signature() successfully submits signature
/// Why: Ensure HTTP POST request works correctly and parses response
#[test]
fn test_submit_signature_success() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_mock_server, base_url) = rt.block_on(async {
        let mock_server = MockServer::start().await;

        let response = json!({
            "success": true,
            "data": {
                "draft_id": DUMMY_DRAFT_ID,
                "status": "signed"
            },
            "error": null
        });

        Mock::given(method("POST"))
            .and(path(format!("/draftintent/{}/signature", DUMMY_DRAFT_ID)))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri().to_string();
        (mock_server, base_url)
    });

    let client = CoordinatorGmpClient::new(base_url);
    let submission = SignatureSubmission {
        solver_hub_addr: DUMMY_SOLVER_ADDR_HUB.to_string(),
        signature: "0x".to_string() + &"a".repeat(128), // 128 hex chars = 64 bytes signature (ECDSA format)
        public_key: "0x".to_string() + &"b".repeat(64), // 64 hex chars = 32 bytes public key (Ed25519 format)
    };

    let result = client
        .submit_signature(DUMMY_DRAFT_ID, &submission)
        .unwrap();

    assert_eq!(result.draft_id, DUMMY_DRAFT_ID);
    assert_eq!(result.status, "signed");
}

/// What is tested: submit_signature() handles FCFS conflict (409 Conflict)
/// Why: Ensure FCFS logic is properly detected and returns appropriate error
/// FCFS = First-Come, First-Served; only the first solver signature is accepted.
#[test]
fn test_submit_signature_conflict() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_mock_server, base_url) = rt.block_on(async {
        let mock_server = MockServer::start().await;

        let response = json!({
            "success": false,
            "data": null,
            "error": "Draft already signed by another solver"
        });

        Mock::given(method("POST"))
            .and(path(format!("/draftintent/{}/signature", DUMMY_DRAFT_ID)))
            .respond_with(ResponseTemplate::new(409).set_body_json(response))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri().to_string();
        (mock_server, base_url)
    });

    let client = CoordinatorGmpClient::new(base_url);
    let submission = SignatureSubmission {
        solver_hub_addr: DUMMY_SOLVER_ADDR_HUB.to_string(),
        signature: "0x".to_string() + &"a".repeat(128), // 128 hex chars = 64 bytes signature (ECDSA format)
        public_key: "0x".to_string() + &"b".repeat(64), // 64 hex chars = 32 bytes public key (Ed25519 format)
    };

    let result = client.submit_signature(DUMMY_DRAFT_ID, &submission);

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Draft already signed by another solver (FCFS)"));
}

/// What is tested: submit_signature() handles other HTTP errors
/// Why: Ensure non-409 errors are handled correctly
#[test]
fn test_submit_signature_other_error() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_mock_server, base_url) = rt.block_on(async {
        let mock_server = MockServer::start().await;

        let response = json!({
            "success": false,
            "data": null,
            "error": "Invalid signature format"
        });

        Mock::given(method("POST"))
            .and(path(format!("/draftintent/{}/signature", DUMMY_DRAFT_ID)))
            .respond_with(ResponseTemplate::new(400).set_body_json(response))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri().to_string();
        (mock_server, base_url)
    });

    let client = CoordinatorGmpClient::new(base_url);
    let submission = SignatureSubmission {
        solver_hub_addr: DUMMY_SOLVER_ADDR_HUB.to_string(),
        signature: "0x".to_string() + &"a".repeat(128), // 128 hex chars = 64 bytes signature (ECDSA format)
        public_key: "0x".to_string() + &"b".repeat(64), // 64 hex chars = 32 bytes public key (Ed25519 format)
    };

    let result = client.submit_signature(DUMMY_DRAFT_ID, &submission);

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    // The error might be wrapped in "Coordinator/Trusted-GMP API error: " prefix
    assert!(
        error_msg.contains("Invalid signature format"),
        "Error message should contain 'Invalid signature format', got: {}",
        error_msg
    );
}


// ----------------------------------------------------------------------------
// Error handling tests
// ----------------------------------------------------------------------------

/// What is tested: HTTP methods handle network errors (connection refused)
/// Why: Ensure network errors are properly propagated
#[test]
fn test_network_error() {
    // Use a port that's definitely not listening (test-specific invalid URL)
    let client = CoordinatorGmpClient::new("http://127.0.0.1:99999");

    let result = client.poll_pending_drafts();

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to send GET /draftintents/pending request"));
}

/// What is tested: HTTP methods handle invalid JSON responses
/// Why: Ensure malformed JSON is handled gracefully
#[test]
fn test_invalid_json_response() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (_mock_server, base_url) = rt.block_on(async {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/draftintents/pending"))
            .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri().to_string();
        (mock_server, base_url)
    });

    let client = CoordinatorGmpClient::new(base_url);
    let result = client.poll_pending_drafts();

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to parse GET /draftintents/pending response"));
}

