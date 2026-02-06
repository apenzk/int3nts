//! Coordinator / Trusted-GMP API Client
//!
//! HTTP client for communicating with the coordinator (drafts, negotiation) and
//! trusted-gmp (validation, approval signatures). Same API response format for both.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ============================================================================
// API RESPONSE WRAPPER
// ============================================================================

/// Standardized response structure from coordinator and trusted-gmp APIs.
///
/// Both services return this format:
/// ```json
/// {
///   "success": true|false,
///   "data": <payload>|null,
///   "error": <message>|null
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Whether the request was successful
    pub success: bool,
    /// Response data (if successful)
    pub data: Option<T>,
    /// Error message (if failed)
    pub error: Option<String>,
}

// ============================================================================
// DRAFT-INTENT STRUCTURES
// ============================================================================

/// Pending draftintent from coordinator API.
///
/// Matches the response format from GET /draftintents/pending.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingDraft {
    /// Unique identifier for the draft
    pub draft_id: String,
    /// Address of the requester who submitted the draft
    pub requester_addr: String,
    /// Draft data (JSON object - matches Draftintent structure from Move)
    pub draft_data: serde_json::Value,
    /// Timestamp when draft was created (Unix timestamp)
    pub timestamp: u64,
    /// Expiry time (Unix timestamp)
    pub expiry_time: u64,
}

/// Request structure for submitting a signature for a draftintent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureSubmission {
    /// Hub solver address submitting the signature
    pub solver_hub_addr: String,
    /// Signature in hex format (Ed25519, 64 bytes = 128 hex characters)
    pub signature: String,
    /// Public key of the solver (hex format)
    pub public_key: String,
}

/// Response structure for signature submission.
#[derive(Debug, Clone, Deserialize)]
pub struct SignatureSubmissionResponse {
    /// Unique identifier for the draft
    pub draft_id: String,
    /// Current status of the draft
    pub status: String,
}

/// Response structure for signature retrieval.
#[derive(Debug, Clone, Deserialize)]
pub struct SignatureResponse {
    /// Signature in hex format
    pub signature: String,
    /// Hub solver address of the signer (first signer)
    pub solver_hub_addr: String,
    /// Timestamp when signature was received
    pub timestamp: u64,
}

// ============================================================================
// COORDINATOR CLIENT
// ============================================================================

/// HTTP client for communicating with the coordinator (drafts) and trusted-gmp (validation/approval).
///
/// Uses blocking HTTP requests (reqwest blocking client).
/// All methods return `Result` with appropriate error context.
pub struct CoordinatorGmpClient {
    /// Base URL of coordinator (drafts) or trusted-gmp (validation/approval), e.g. "http://127.0.0.1:3333" or "http://127.0.0.1:3334"
    base_url: String,
    /// HTTP client instance
    client: reqwest::blocking::Client,
}

impl CoordinatorGmpClient {
    /// Create a new API client for coordinator or trusted-gmp.
    ///
    /// # Arguments
    ///
    /// * `base_url` - Base URL of coordinator (3333) or trusted-gmp (3334)
    ///
    /// # Returns
    ///
    /// * `CoordinatorGmpClient` - New client instance
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .no_proxy() // Avoid macOS system-configuration issues in tests
            .build()
            .expect("Failed to create HTTP client");

        Self {
            base_url: base_url.into(),
            client,
        }
    }

    /// Poll for pending draftintents.
    ///
    /// Returns all pending drafts (all solvers see all drafts).
    /// This is a polling endpoint - solvers call this regularly to discover new drafts.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<PendingDraft>)` - List of pending drafts
    /// * `Err(anyhow::Error)` - Failed to fetch drafts
    pub fn poll_pending_drafts(&self) -> Result<Vec<PendingDraft>> {
        let url = format!("{}/draftintents/pending", self.base_url);

        let response: ApiResponse<Vec<PendingDraft>> = self
            .client
            .get(&url)
            .send()
            .context("Failed to send GET /draftintents/pending request")?
            .json()
            .context("Failed to parse GET /draftintents/pending response")?;

        if !response.success {
            return Err(anyhow::anyhow!(
                "Coordinator/Trusted-GMP API error: {}",
                response.error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        Ok(response.data.unwrap_or_default())
    }

    /// Submit a signature for a draftintent.
    ///
    /// The solver submits its signature to the coordinator. The coordinator implements FCFS logic:
    /// the first signature wins, and later signatures are rejected with 409 Conflict.
    /// This method handles the 409 Conflict response and converts it to an appropriate error.
    ///
    /// # Arguments
    ///
    /// * `draft_id` - The draft ID to sign
    /// * `submission` - The signature submission data
    ///
    /// # Returns
    ///
    /// * `Ok(SignatureSubmissionResponse)` - Signature accepted (200 OK - solver was first)
    /// * `Err(anyhow::Error)` - Failed to submit signature (may be 409 Conflict if draft already signed by another solver)
    pub fn submit_signature(
        &self,
        draft_id: &str,
        submission: &SignatureSubmission,
    ) -> Result<SignatureSubmissionResponse> {
        let url = format!("{}/draftintent/{}/signature", self.base_url, draft_id);

        let http_response = self
            .client
            .post(&url)
            .json(submission)
            .send()
            .context("Failed to send POST /draftintent/:id/signature request")?;

        let status = http_response.status();
        let response: ApiResponse<SignatureSubmissionResponse> = http_response
            .json()
            .context("Failed to parse POST /draftintent/:id/signature response")?;

        if !response.success {
            if status == reqwest::StatusCode::CONFLICT {
                return Err(anyhow::anyhow!(
                    "Draft already signed by another solver (FCFS): {}",
                    response.error.unwrap_or_else(|| "Unknown error".to_string())
                ));
            }

            return Err(anyhow::anyhow!(
                "Coordinator/Trusted-GMP API error: {}",
                response.error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        Ok(response.data.context("Missing data in successful response")?)
    }

}

