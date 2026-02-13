//! Integrated GMP Service Library
//!
//! This crate provides a message relay service for cross-chain intents.
//! It watches GMP endpoint events (MessageSent) and delivers messages to destination contracts.
pub mod config;
pub mod crypto;
pub mod evm_client;
pub mod svm_client;
pub mod mvm_client;
pub mod integrated_gmp_relay;

// Re-export commonly used types
pub use config::{ApiConfig, ChainConfig, Config, EvmChainConfig, SvmChainConfig, IntegratedGmpConfig};
pub use crypto::CryptoService;
pub use integrated_gmp_relay::{NativeGmpRelay, NativeGmpRelayConfig};
