//! Unit tests for API error handling and request validation
//!
//! These tests verify the trusted GMP service API endpoints work correctly.

use trusted_gmp::api::{ApiResponse, ApiServer};
use trusted_gmp::crypto::CryptoService;
use trusted_gmp::monitor::EventMonitor;
use trusted_gmp::validator::CrossChainValidator;
use warp::http::StatusCode;
use warp::test::request;

#[path = "mod.rs"]
mod test_helpers;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Create a test API server with minimal configuration
async fn create_test_api_server() -> ApiServer {
    let config = test_helpers::build_test_config_with_mvm();
    let monitor = EventMonitor::new(&config).await.unwrap();
    let validator = CrossChainValidator::new(&config).await.unwrap();
    let crypto_service = CryptoService::new(&config).unwrap();

    ApiServer::new(config, monitor, validator, crypto_service)
}

// ============================================================================
// HEALTH ENDPOINT TESTS
// ============================================================================

/// Test that health endpoint returns success
/// What is tested: Basic health check endpoint
/// Why: Ensures service is running and responsive
#[tokio::test]
async fn test_health_endpoint() {
    let api_server = create_test_api_server().await;
    let routes = api_server.test_routes();

    let response = request()
        .method("GET")
        .path("/health")
        .reply(&routes)
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: ApiResponse<String> = serde_json::from_slice(response.body()).unwrap();
    assert!(body.success);
    assert!(body.data.is_some());
}

// ============================================================================
// EVENTS ENDPOINT TESTS
// ============================================================================

/// Test that events endpoint returns success
/// What is tested: Events retrieval endpoint
/// Why: Ensures monitored events can be retrieved
#[tokio::test]
async fn test_events_endpoint() {
    let api_server = create_test_api_server().await;
    let routes = api_server.test_routes();

    let response = request()
        .method("GET")
        .path("/events")
        .reply(&routes)
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = serde_json::from_slice(response.body()).unwrap();
    assert!(body.success);
}

// ============================================================================
// APPROVALS ENDPOINT TESTS
// ============================================================================

/// Test that approvals endpoint returns success
/// What is tested: Approvals retrieval endpoint
/// Why: Ensures cached approvals can be retrieved
#[tokio::test]
async fn test_approvals_endpoint() {
    let api_server = create_test_api_server().await;
    let routes = api_server.test_routes();

    let response = request()
        .method("GET")
        .path("/approvals")
        .reply(&routes)
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: ApiResponse<serde_json::Value> = serde_json::from_slice(response.body()).unwrap();
    assert!(body.success);
}

// ============================================================================
// PUBLIC KEY ENDPOINT TESTS
// ============================================================================

/// Test that public-key endpoint returns success
/// What is tested: Public key retrieval endpoint
/// Why: Ensures trusted-gmp public key can be retrieved for signature verification
#[tokio::test]
async fn test_public_key_endpoint() {
    let api_server = create_test_api_server().await;
    let routes = api_server.test_routes();

    let response = request()
        .method("GET")
        .path("/public-key")
        .reply(&routes)
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: ApiResponse<String> = serde_json::from_slice(response.body()).unwrap();
    assert!(body.success);
    assert!(body.data.is_some());
}
