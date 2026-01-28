//! REST API Server Module
//!
//! This module provides a REST API server for the coordinator service,
//! exposing endpoints for monitoring events, retrieving cached data,
//! and negotiation routing for draft intents.
//!
//! ## Security Model
//!
//! The coordinator API is read-only for blockchain data and provides
//! negotiation routing. It does NOT hold private keys or generate signatures.

// Generic shared code (health, events, exchange rate, draft intent routing)
mod generic;

// Negotiation routing module (draft intent FCFS matching)
mod negotiation;

// Re-export ApiServer for convenience
pub use generic::ApiServer;
// Re-export ApiResponse for testing
#[allow(unused_imports)]
pub use generic::ApiResponse;
// Re-export negotiation validation functions for testing
#[allow(unused_imports)]
pub use negotiation::validate_signature_format;
