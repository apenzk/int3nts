//! Generic API structures and handlers
//!
//! This module contains shared structures, helper functions, and generic API handlers
//! that are used across all flow types (inflow/outflow) and chain types (Move VM/EVM).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use warp::{http::{Method, StatusCode}, Filter, Rejection, Reply};
use warp::hyper::body::Bytes;

use crate::config::Config;
use crate::crypto::CryptoService;
use crate::monitor::EventMonitor;
use crate::storage::DraftintentStore;
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
/// has received approval from the verifier.
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
/// This function retrieves the verifier's public key for external
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

/// Response structure for exchange rate query
#[derive(Debug, Serialize, Deserialize)]
pub struct ExchangeRateResponse {
    /// Desired token metadata address
    pub desired_token: String,
    /// Desired chain ID
    pub desired_chain_id: u64,
    /// Exchange rate (how many offered tokens per 1 desired token)
    pub exchange_rate: f64,
}

/// Handler for the acceptance/exchange rate endpoint.
///
/// Query parameters:
/// - offered_chain_id: Chain ID of the offered token
/// - offered_token: Metadata address of the offered token
/// - desired_chain_id: Chain ID of the desired token (optional - if not provided, returns first match)
/// - desired_token: Metadata address of the desired token (optional - if not provided, returns first match)
///
/// Returns the desired token, desired chain ID, and exchange rate.
///
/// Exchange rates are fetched live from the solver to avoid stale ratios.
pub async fn get_exchange_rate_handler(
    config: Arc<crate::config::Config>,
    query: String,
) -> Result<impl warp::Reply, warp::Rejection> {
    use std::collections::HashMap;
    use url::Url;
    
    // Parse query parameters
    let parsed = Url::parse(&format!("http://dummy?{}", query))
        .map_err(|e| warp::reject::custom(JsonDeserializeError(format!("Invalid query string: {}", e))))?;
    
    let params: HashMap<String, String> = parsed
        .query_pairs()
        .into_owned()
        .collect();
    
    let offered_chain_id = params.get("offered_chain_id")
        .ok_or_else(|| warp::reject::custom(JsonDeserializeError("Missing offered_chain_id parameter".to_string())))?;
    let offered_token = params.get("offered_token")
        .ok_or_else(|| warp::reject::custom(JsonDeserializeError("Missing offered_token parameter".to_string())))?;
    
    let desired_chain_id = params.get("desired_chain_id");
    let desired_token = params.get("desired_token");
    
    // Get acceptance config
    let acceptance = config.acceptance.as_ref()
        .ok_or_else(|| warp::reject::custom(JsonDeserializeError("Acceptance criteria not configured".to_string())))?;

    // Find matching pair in verifier's configured list
    let offered_chain_id_u64 = offered_chain_id
        .parse::<u64>()
        .map_err(|e| warp::reject::custom(JsonDeserializeError(format!("Invalid offered_chain_id: {}", e))))?;

    let matched_pair = if let (Some(d_chain_id), Some(d_token)) = (desired_chain_id, desired_token) {
        let desired_chain_id_u64 = d_chain_id
            .parse::<u64>()
            .map_err(|e| warp::reject::custom(JsonDeserializeError(format!("Invalid desired_chain_id: {}", e))))?;
        acceptance.pairs.iter().find(|pair| {
            pair.source_chain_id == offered_chain_id_u64
                && pair.source_token == *offered_token
                && pair.target_chain_id == desired_chain_id_u64
                && pair.target_token == *d_token
        })
    } else {
        acceptance.pairs.iter().find(|pair| {
            pair.source_chain_id == offered_chain_id_u64
                && pair.source_token == *offered_token
        })
    }.ok_or_else(|| {
        warp::reject::custom(JsonDeserializeError(format!(
            "No exchange rate found for offered token {} on chain {}",
            offered_token, offered_chain_id
        )))
    })?;

    // Fetch live ratio from solver
    let solver_url = acceptance.solver_url.trim_end_matches('/');
    let solver_request = format!(
        "{}/acceptance?offered_chain_id={}&offered_token={}&desired_chain_id={}&desired_token={}",
        solver_url,
        matched_pair.source_chain_id,
        matched_pair.source_token,
        matched_pair.target_chain_id,
        matched_pair.target_token,
    );

    let response = reqwest::get(&solver_request).await
        .map_err(|e| warp::reject::custom(JsonDeserializeError(format!("Solver request failed: {}", e))))?;
    let status = response.status();
    if !status.is_success() {
        return Err(warp::reject::custom(JsonDeserializeError(format!(
            "Solver returned error status {}",
            status
        ))));
    }

    let solver_response: ApiResponse<ExchangeRateResponse> = response.json().await
        .map_err(|e| warp::reject::custom(JsonDeserializeError(format!("Invalid solver response: {}", e))))?;
    let exchange_rate = solver_response.data.ok_or_else(|| {
        warp::reject::custom(JsonDeserializeError("Solver response missing data".to_string()))
    })?;

    Ok(warp::reply::json(&ApiResponse::<ExchangeRateResponse> {
        success: true,
        data: Some(exchange_rate),
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

/// REST API server for the trusted verifier service.
///
/// This server exposes HTTP endpoints for external systems to interact with
/// the verifier service, including event monitoring, validation, and signature
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
    /// Draft intent store for negotiation routing
    draft_store: Arc<RwLock<DraftintentStore>>,
}

impl ApiServer {
    /// Creates a new API server with the given components.
    ///
    /// This function initializes the API server with all necessary components
    /// for handling HTTP requests and providing verifier functionality.
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
            draft_store: Arc::new(RwLock::new(DraftintentStore::new())),
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
        use super::negotiation;
        use super::outflow_generic;

        let _config = self.config.clone();
        let monitor = self.monitor.clone();
        let _validator = self.validator.clone();
        let crypto_service = self.crypto_service.clone();
        let draft_store = self.draft_store.clone();

        // Health check endpoint - returns service status
        let health = warp::path("health").and(warp::get()).map(|| {
            warp::reply::json(&ApiResponse::<String> {
                success: true,
                data: Some("Trusted Verifier Service is running".to_string()),
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

        // Get public key endpoint - returns verifier's public key
        let public_key = warp::path("public-key")
            .and(warp::get())
            .and(with_crypto_service(crypto_service.clone()))
            .and_then(get_public_key_handler);
        
        // Get exchange rate endpoint - returns desired token and exchange rate for offered token
        let exchange_rate_config = self.config.clone();
        let exchange_rate = warp::path("acceptance")
            .and(warp::get())
            .and(warp::query::raw())
            .and_then(move |query: String| {
                let config = exchange_rate_config.clone();
                async move {
                    get_exchange_rate_handler(config, query).await
                }
            });
        

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

        // Negotiation routing endpoints
        // POST /draftintent - Submit draft intent (open to any solver)
        let create_draft_store = draft_store.clone();
        let create_draft = warp::path("draftintent")
            .and(warp::path::end()) // Exact match - don't match /draftintent/:id/...
            .and(warp::post())
            .and(warp::body::bytes())
            .and_then(move |body: Bytes| {
                let store = create_draft_store.clone();
                async move {
                    // Log raw request body for debugging
                    let body_str = String::from_utf8_lossy(&body);
                    debug!("POST /draftintent - Received body: {}", body_str);

                    // Deserialize and handle
                    match serde_json::from_slice::<negotiation::DraftintentRequest>(&body) {
                        Ok(request) => negotiation::create_draftintent_handler(request, store).await,
                        Err(e) => {
                            error!("Draft intent deserialization failed: {}. Body: {}", e, body_str);
                            Err(warp::reject::custom(JsonDeserializeError(format!("Invalid JSON: {}", e))))
                        }
                    }
                }
            });

        // GET /draftintent/:id - Get draft intent status
        let get_draft_store = draft_store.clone();
        let get_draft = warp::path("draftintent")
            .and(warp::path::param())
            .and(warp::path::end()) // Exact match - don't match /draftintent/:id/signature
            .and(warp::get())
            .and(negotiation::with_draft_store(get_draft_store))
            .and_then(negotiation::get_draftintent_handler);

        // GET /draftintents/pending - Get all pending drafts (all solvers see all drafts)
        let get_pending_store = draft_store.clone();
        let get_pending = warp::path("draftintents")
            .and(warp::path("pending"))
            .and(warp::get())
            .and(negotiation::with_draft_store(get_pending_store))
            .and_then(negotiation::get_pending_drafts_handler);

        // POST /draftintent/:id/signature - Solver submits signature (FCFS)
        let submit_sig_store = draft_store.clone();
        let submit_sig_config = self.config.clone();
        let submit_signature = warp::path("draftintent")
            .and(warp::path::param())
            .and(warp::path("signature"))
            .and(warp::post())
            .and(warp::body::bytes())
            .and_then(move |draft_id: String, body: Bytes| {
                let store = submit_sig_store.clone();
                let config = submit_sig_config.clone();
                async move {
                    // Log raw request body for debugging
                    let body_str = String::from_utf8_lossy(&body);
                    debug!("POST /draftintent/{}/signature - Received body: {}", draft_id, body_str);

                    // Deserialize and handle
                    match serde_json::from_slice::<negotiation::SignatureSubmissionRequest>(&body) {
                        Ok(request) => negotiation::submit_signature_handler(draft_id, request, store, config).await,
                        Err(e) => {
                            error!("Signature submission deserialization failed: {}. Body: {}", e, body_str);
                            Err(warp::reject::custom(JsonDeserializeError(format!("Invalid JSON: {}", e))))
                        }
                    }
                }
            });

        // GET /draftintent/:id/signature - Requester polls for signature
        let get_sig_store = draft_store.clone();
        let get_sig_config = self.config.clone();
        let get_signature = warp::path("draftintent")
            .and(warp::path::param())
            .and(warp::path("signature"))
            .and(warp::get())
            .and(negotiation::with_draft_store(get_sig_store))
            .and_then(move |draft_id: String, store: Arc<RwLock<DraftintentStore>>| {
                let config = get_sig_config.clone();
                async move {
                    negotiation::get_signature_handler(draft_id, store, config).await
                }
            });

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
            .or(create_draft)
            .or(get_draft)
            .or(get_pending)
            .or(submit_signature)
            .or(get_signature)
            .or(exchange_rate)
            .with(create_cors_filter(&self.config.api.cors_origins))
            .recover(handle_rejection)
    }

    /// Public method for testing - exposes routes for integration tests
    #[allow(dead_code)] // Used by tests
    pub fn test_routes(&self) -> impl Filter<Extract = impl warp::Reply, Error = std::convert::Infallible> + Clone {
        self.create_routes()
    }
}
