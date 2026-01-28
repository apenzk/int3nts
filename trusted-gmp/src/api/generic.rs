//! Generic API structures and handlers
//!
//! This module contains shared structures, helper functions, and generic API handlers
//! that are used across all flow types (inflow/outflow) and chain types (Move VM/EVM).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use warp::{http::{Method, StatusCode}, Filter, Rejection, Reply};

use crate::config::Config;
use crate::crypto::CryptoService;
use crate::monitor::EventMonitor;
use crate::validator::CrossChainValidator;

// ============================================================================
// SHARED REQUEST/RESPONSE STRUCTURES
// ============================================================================

/// Standardized response structure for all API endpoints.
///
/// This structure provides a consistent response format for all API endpoints,
/// including success/error status and relevant data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Whether the request was successful
    pub success: bool,
    /// Response data (if successful)
    pub data: Option<T>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Request structure for approval signature creation.
///
/// This structure contains the data needed to create an approval or rejection
/// signature for escrow operations.
#[derive(Debug, Deserialize)]
pub struct ApprovalRequest {
    /// Whether to approve (true) or reject (false) the operation
    pub approve: bool,
}

// ============================================================================
// GENERIC API HANDLERS
// ============================================================================

/// Handler for the events endpoint.
///
/// This function retrieves all cached events from the event monitor
/// and returns them as a JSON response.
///
/// # Arguments
///
/// * `monitor` - The event monitor instance
///
/// # Returns
///
/// * `Ok(warp::Reply)` - JSON response with cached events
/// * `Err(warp::Rejection)` - Failed to retrieve events
pub async fn get_events_handler(
    monitor: Arc<RwLock<EventMonitor>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let monitor = monitor.read().await;
    let intent_events = monitor.get_cached_events().await;
    let escrow_events = monitor.get_cached_escrow_events().await;
    let fulfillment_events = monitor.get_cached_fulfillment_events().await;
    let approvals = monitor.get_cached_approvals().await;

    // Return intent, escrow, fulfillment events, and approvals in a combined structure
    #[derive(Debug, Serialize)]
    struct CombinedEvents {
        intent_events: Vec<crate::monitor::IntentEvent>,
        escrow_events: Vec<crate::monitor::EscrowEvent>,
        fulfillment_events: Vec<crate::monitor::FulfillmentEvent>,
        approvals: Vec<crate::monitor::EscrowApproval>,
    }

    let combined = CombinedEvents {
        intent_events,
        escrow_events,
        fulfillment_events,
        approvals,
    };

    Ok(warp::reply::json(&ApiResponse {
        success: true,
        data: Some(combined),
        error: None,
    }))
}

/// Handler for the approvals endpoint.
///
/// This function retrieves all cached approval signatures from the event monitor
/// and returns them as a JSON response.
///
/// # Arguments
///
/// * `monitor` - The event monitor instance
///
/// # Returns
///
/// * `Ok(warp::Reply)` - JSON response with cached approvals
/// * `Err(warp::Rejection)` - Failed to retrieve approvals
pub async fn get_approvals_handler(
    monitor: Arc<RwLock<EventMonitor>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let monitor = monitor.read().await;
    let approvals = monitor.get_cached_approvals().await;

    Ok(warp::reply::json(&ApiResponse {
        success: true,
        data: Some(approvals),
        error: None,
    }))
}

/// Handler for getting approval by escrow ID.
///
/// This function retrieves the approval signature for a specific escrow
/// and returns it as a JSON response.
///
/// # Arguments
///
/// * `escrow_id` - The escrow ID to look up
/// * `monitor` - The event monitor instance
///
/// # Returns
///
/// * `Ok(warp::Reply)` - JSON response with approval signature
/// * `Err(warp::Rejection)` - Failed to retrieve approval
pub async fn get_approval_by_escrow_handler(
    escrow_id: String,
    monitor: Arc<RwLock<EventMonitor>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let monitor = monitor.read().await;
    match monitor.get_approval_for_escrow(&escrow_id).await {
        Some(approval) => Ok(warp::reply::json(&ApiResponse {
            success: true,
            data: Some(approval),
            error: None,
        })),
        None => Ok(warp::reply::json(&ApiResponse::<
            crate::monitor::EscrowApproval,
        > {
            success: false,
            data: None,
            error: Some(format!("No approval found for escrow: {}", escrow_id)),
        })),
    }
}

/// Handler for checking if an intent has been approved.
///
/// This is a simple endpoint for the frontend to check if an outflow intent
/// has received approval from the trusted-gmp.
///
/// # Arguments
///
/// * `intent_id` - The intent ID to check
/// * `monitor` - The event monitor instance
///
/// # Returns
///
/// * `Ok(warp::Reply)` - JSON response with `{ approved: true/false }`
pub async fn is_intent_approved_handler(
    intent_id: String,
    monitor: Arc<RwLock<EventMonitor>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let monitor = monitor.read().await;
    let normalized = crate::monitor::normalize_intent_id(&intent_id);
    let approved = monitor.is_intent_approved(&intent_id).await;

    tracing::info!(
        "Checking approval status: intent_id={}, normalized={}, approved={}",
        intent_id,
        normalized,
        approved
    );

    #[derive(serde::Serialize)]
    struct ApprovalStatus {
        intent_id: String,
        approved: bool,
    }

    Ok(warp::reply::json(&ApiResponse {
        success: true,
        data: Some(ApprovalStatus { intent_id, approved }),
        error: None,
    }))
}

/// Handler for the approval endpoint.
///
/// This function creates an approval or rejection signature based on
/// the request parameters. It validates that escrow intents are
/// non-revocable before creating approval signatures.
///
/// # Arguments
///
/// * `request` - The approval request containing approval decision
/// * `crypto_service` - The cryptographic service instance
///
/// # Returns
///
/// * `Ok(warp::Reply)` - JSON response with approval signature
/// * `Err(warp::Rejection)` - Failed to create signature
pub async fn create_approval_handler(
    request: ApprovalRequest,
    crypto_service: Arc<RwLock<CryptoService>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let crypto_service = crypto_service.read().await;

    // Create the approval signature
    match crypto_service.create_approval_signature(request.approve) {
        Ok(signature) => Ok(warp::reply::json(&ApiResponse {
            success: true,
            data: Some(signature),
            error: None,
        })),
        Err(e) => Ok(warp::reply::json(&ApiResponse::<
            crate::crypto::ApprovalSignature,
        > {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Handler for the public key endpoint.
///
/// This function retrieves the trusted-gmp's public key for external
/// signature verification.
///
/// # Arguments
///
/// * `crypto_service` - The cryptographic service instance
///
/// # Returns
///
/// * `Ok(warp::Reply)` - JSON response with public key
/// * `Err(warp::Rejection)` - Failed to retrieve public key
pub async fn get_public_key_handler(
    crypto_service: Arc<RwLock<CryptoService>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let crypto_service = crypto_service.read().await;
    let public_key = crypto_service.get_public_key();

    Ok(warp::reply::json(&ApiResponse {
        success: true,
        data: Some(public_key),
        error: None,
    }))
}

// ============================================================================
// WARP FILTER HELPERS
// ============================================================================

/// Creates a warp filter that provides access to the event monitor.
///
/// This helper function creates a filter that injects the event monitor
/// into request handlers.
///
/// # Arguments
///
/// * `monitor` - The event monitor instance
///
/// # Returns
///
/// A warp filter that provides the monitor to handlers
pub fn with_monitor(
    monitor: Arc<RwLock<EventMonitor>>,
) -> impl Filter<Extract = (Arc<RwLock<EventMonitor>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || monitor.clone())
}

/// Creates a warp filter that provides access to the cryptographic service.
///
/// This helper function creates a filter that injects the crypto service
/// into request handlers.
///
/// # Arguments
///
/// * `crypto_service` - The cryptographic service instance
///
/// # Returns
///
/// A warp filter that provides the crypto service to handlers
pub fn with_crypto_service(
    crypto_service: Arc<RwLock<CryptoService>>,
) -> impl Filter<Extract = (Arc<RwLock<CryptoService>>,), Error = std::convert::Infallible> + Clone
{
    warp::any().map(move || crypto_service.clone())
}

/// Creates a warp filter that provides access to the cross-chain validator.
///
/// This helper function creates a filter that injects the validator
/// into request handlers.
///
/// # Arguments
///
/// * `validator` - The cross-chain validator instance
///
/// # Returns
///
/// A warp filter that provides the validator to handlers
pub fn with_validator(
    validator: Arc<RwLock<CrossChainValidator>>,
) -> impl Filter<Extract = (Arc<RwLock<CrossChainValidator>>,), Error = std::convert::Infallible> + Clone
{
    warp::any().map(move || validator.clone())
}

// ============================================================================
// CUSTOM REJECTION TYPES
// ============================================================================

/// Custom rejection for JSON deserialization errors
#[derive(Debug)]
pub struct JsonDeserializeError(pub String);

impl warp::reject::Reject for JsonDeserializeError {}

// ============================================================================
// CORS CONFIGURATION
// ============================================================================

/// Creates a CORS filter based on the configured allowed origins.
fn create_cors_filter(allowed_origins: &[String]) -> warp::cors::Builder {
    let methods = vec![
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
    ];

    if allowed_origins.contains(&"*".to_string()) {
        warp::cors()
            .allow_any_origin()
            .allow_methods(methods.clone())
            .allow_headers(vec!["content-type"])
    } else {
        let origins: Vec<&str> = allowed_origins.iter().map(|s| s.as_str()).collect();
        warp::cors()
            .allow_origins(origins)
            .allow_methods(methods)
            .allow_headers(vec!["content-type"])
    }
}

// ============================================================================
// REJECTION HANDLER
// ============================================================================

/// Global rejection handler for all API routes.
///
/// This function handles all warp rejections and converts them into
/// standardized API responses with appropriate HTTP status codes.
///
/// # Arguments
///
/// * `rej` - The warp rejection to handle
///
/// # Returns
///
/// A warp reply with an error response
pub async fn handle_rejection(rej: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    let (status, message) = if let Some(err) = rej.find::<JsonDeserializeError>() {
        (StatusCode::BAD_REQUEST, err.0.clone())
    } else if let Some(err) = rej.find::<warp::filters::body::BodyDeserializeError>() {
        (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", err))
    } else if rej.is_not_found() {
        (StatusCode::NOT_FOUND, "Endpoint not found".to_string())
    } else if rej.find::<warp::reject::MethodNotAllowed>().is_some() {
        (StatusCode::METHOD_NOT_ALLOWED, "Method not allowed".to_string())
    } else {
        error!("Unhandled rejection: {:?}", rej);
        (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&ApiResponse::<()> {
            success: false,
            data: None,
            error: Some(message),
        }),
        status,
    ))
}

// ============================================================================
// API SERVER IMPLEMENTATION
// ============================================================================

/// REST API server for the trusted GMP service.
///
/// This server exposes HTTP endpoints for external systems to interact with
/// the service, including event monitoring, validation, and signature
/// retrieval.
pub struct ApiServer {
    /// Service configuration
    config: Arc<Config>,
    /// Event monitor for blockchain event processing
    monitor: Arc<RwLock<EventMonitor>>,
    /// Cross-chain validator for fulfillment validation
    validator: Arc<RwLock<CrossChainValidator>>,
    /// Cryptographic service for signature operations
    crypto_service: Arc<RwLock<CryptoService>>,
}

impl ApiServer {
    /// Creates a new API server with the given components.
    ///
    /// This function initializes the API server with all necessary components
    /// for handling HTTP requests and providing trusted-gmp functionality.
    ///
    /// # Arguments
    ///
    /// * `config` - Service configuration
    /// * `monitor` - Event monitor instance
    /// * `validator` - Cross-chain validator instance
    /// * `crypto_service` - Cryptographic service instance
    ///
    /// # Returns
    ///
    /// A new API server instance
    pub fn new(
        config: Config,
        monitor: EventMonitor,
        validator: CrossChainValidator,
        crypto_service: CryptoService,
    ) -> Self {
        Self {
            config: Arc::new(config),
            monitor: Arc::new(RwLock::new(monitor)),
            validator: Arc::new(RwLock::new(validator)),
            crypto_service: Arc::new(RwLock::new(crypto_service)),
        }
    }

    /// Starts the API server and begins handling HTTP requests.
    ///
    /// This function configures all API routes and starts the HTTP server
    /// on the configured host and port.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Server started successfully
    /// * `Err(anyhow::Error)` - Failed to start server
    pub async fn run(&self) -> Result<()> {
        info!(
            "Starting API server on {}:{}",
            self.config.api.host, self.config.api.port
        );

        // Create and configure all API routes
        let routes = self.create_routes();

        // Parse host address from config
        let addr: std::net::SocketAddr = format!("{}:{}", self.config.api.host, self.config.api.port)
            .parse()
            .context("Failed to parse API server address")?;

        // Start the HTTP server
        warp::serve(routes)
            .run(addr)
            .await;

        Ok(())
    }

    /// Creates all API routes for the server.
    ///
    /// This function defines all HTTP endpoints and their handlers,
    /// including health checks, event monitoring, validation, and
    /// signature operations.
    ///
    /// # Returns
    ///
    /// A warp filter containing all API routes
    pub(crate) fn create_routes(
        &self,
    ) -> impl Filter<Extract = impl warp::Reply, Error = std::convert::Infallible> + Clone {
        use super::inflow_generic;
        use super::outflow_generic;

        let _config = self.config.clone();
        let monitor = self.monitor.clone();
        let _validator = self.validator.clone();
        let crypto_service = self.crypto_service.clone();

        // Health check endpoint - returns service status
        let health = warp::path("health").and(warp::get()).map(|| {
            warp::reply::json(&ApiResponse::<String> {
                success: true,
                data: Some("Trusted GMP Service is running".to_string()),
                error: None,
            })
        });

        // Get cached events endpoint - returns all monitored events
        let events = warp::path("events")
            .and(warp::get())
            .and(with_monitor(monitor.clone()))
            .and_then(get_events_handler);

        // Get approvals endpoint - returns all cached approval signatures
        let approvals = warp::path("approvals")
            .and(warp::get())
            .and(with_monitor(monitor.clone()))
            .and_then(get_approvals_handler);

        // Get approval for specific escrow endpoint
        let approval_by_escrow_monitor = monitor.clone();
        let approval_by_escrow = warp::path("approvals")
            .and(warp::path::param())
            .and(warp::get())
            .and(with_monitor(approval_by_escrow_monitor))
            .and_then(get_approval_by_escrow_handler);

        // Check if intent is approved endpoint - simple true/false for frontend polling
        let is_approved_monitor = monitor.clone();
        let is_approved = warp::path("approved")
            .and(warp::path::param())
            .and(warp::get())
            .and(with_monitor(is_approved_monitor))
            .and_then(is_intent_approved_handler);

        // Create approval signature endpoint - creates approval/rejection signatures
        let approval = warp::path("approval")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_crypto_service(crypto_service.clone()))
            .and_then(create_approval_handler);

        // Get public key endpoint - returns trusted-gmp's public key
        let public_key = warp::path("public-key")
            .and(warp::get())
            .and(with_crypto_service(crypto_service.clone()))
            .and_then(get_public_key_handler);

        // Outflow validation endpoint - validates connected chain transactions for outflow intents
        // Signature is for hub chain intent fulfillment
        let validate_outflow_monitor = monitor.clone();
        let validate_outflow_validator = _validator.clone();
        let validate_outflow_crypto = crypto_service.clone();
        let validate_outflow = warp::path("validate-outflow-fulfillment")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_monitor(validate_outflow_monitor))
            .and(with_validator(validate_outflow_validator))
            .and(with_crypto_service(validate_outflow_crypto))
            .and_then(outflow_generic::handle_outflow_fulfillment_validation);

        // Inflow validation endpoint - validates escrow deposits on connected chain for inflow intents
        // Signature is for connected chain escrow release (generated automatically by monitor)
        let validate_inflow_monitor = monitor.clone();
        let validate_inflow = warp::path("validate-inflow-escrow")
            .and(warp::post())
            .and(warp::body::json())
            .and(with_monitor(validate_inflow_monitor))
            .and_then(inflow_generic::handle_inflow_escrow_validation);

        // Combine all routes and apply rejection handler
        health
            .or(events)
            .or(approvals)
            .or(approval_by_escrow)
            .or(is_approved)
            .or(approval)
            .or(public_key)
            .or(validate_outflow)
            .or(validate_inflow)
            .with(create_cors_filter(&self.config.api.cors_origins))
            .recover(handle_rejection)
    }

    /// Public method for testing - exposes routes for integration tests
    #[allow(dead_code)] // Used by tests
    pub fn test_routes(&self) -> impl Filter<Extract = impl warp::Reply, Error = std::convert::Infallible> + Clone {
        self.create_routes()
    }
}
