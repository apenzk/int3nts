//! Chain Clients Module
//!
//! This module provides clients for interacting with hub and connected chains.
//! Supports both Move VM (hub and connected MVM chains) and EVM (connected EVM chains).

pub mod hub;
pub mod connected_mvm_client;
pub mod connected_evm_client;
pub mod connected_svm_client;

// Re-export for convenience
pub use hub::{HubChainClient, IntentCreatedEvent};
pub use connected_mvm_client::ConnectedMvmClient;
pub use connected_evm_client::{normalize_evm_address, ConnectedEvmClient, EscrowCreatedEvent};
pub use connected_svm_client::{ConnectedSvmClient, EscrowAccount, EscrowEvent as SvmEscrowEvent};

