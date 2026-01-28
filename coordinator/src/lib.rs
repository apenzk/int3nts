//! Coordinator Service Library
//!
//! This crate provides a coordinator service that monitors blockchain events
//! and provides negotiation routing for cross-chain intents. The coordinator
//! is read-only - it does not hold private keys or perform cryptographic signing.

pub mod api;
pub mod config;
pub mod monitor;
pub mod mvm_client;
pub mod storage;
pub mod svm_client;

// Re-export storage types for tests
pub use storage::draftintents::{DraftintentStatus, DraftintentStore};

// Re-export commonly used types
pub use config::{ApiConfig, ChainConfig, Config, CoordinatorConfig, EvmChainConfig, SvmChainConfig};
pub use monitor::{ChainType, EscrowEvent, EventMonitor, FulfillmentEvent, IntentEvent};
