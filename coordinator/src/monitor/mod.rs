//! Event Monitoring Module
//!
//! This module handles monitoring blockchain events from the hub chain.
//! It listens for intent creation and fulfillment events on the hub chain,
//! providing real-time event processing and caching.
//!
//! ## Security Requirements
//!
//! **CRITICAL**: The monitor must validate that escrow intents are **non-revocable**
//! (`revocable = false`) before allowing any cross-chain actions to proceed.

// Generic shared code
mod generic;

// Flow-specific modules (chain-agnostic)
mod outflow_generic;

// Flow + chain specific modules
mod hub_mvm;

// Re-export public types and functions
pub use generic::{
    EventMonitor, FulfillmentEvent, IntentEvent,
};

// Re-export utility functions (used in tests and API handlers)
#[allow(unused_imports)] // Used by integration tests (monitor_tests.rs)
pub use generic::normalize_intent_id;

// Re-export poll_hub_events for testing
#[doc(hidden)]
#[allow(unused_imports)] // Only used in tests
pub use outflow_generic::poll_hub_events;

// Re-export parse_amount_with_u64_limit for testing
#[doc(hidden)]
#[allow(unused_imports)] // Only used in tests
pub use hub_mvm::parse_amount_with_u64_limit;

