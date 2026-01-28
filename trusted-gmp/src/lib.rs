//! Trusted GMP Service Library
//!
//! This crate provides message relay and validation services for cross-chain intents.
//! It watches mock GMP endpoint events and delivers messages to destination contracts.

pub mod api;
pub mod config;
pub mod crypto;
pub mod evm_client;
pub mod svm_client;
pub mod monitor;
pub mod mvm_client;
pub mod validator;

// Re-export commonly used types
pub use config::{ApiConfig, ChainConfig, Config, EvmChainConfig, SvmChainConfig, TrustedGmpConfig};
pub use crypto::{ApprovalSignature, CryptoService};
pub use monitor::{ChainType, EscrowEvent, EventMonitor, FulfillmentEvent, IntentEvent};
pub use validator::{CrossChainValidator, ValidationResult};
