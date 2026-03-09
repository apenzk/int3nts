//! EVM chain client for cross-chain intent services
//!
//! Shared EVM JSON-RPC client used by coordinator, integrated-gmp, and solver.

pub mod client;
pub mod types;

pub use client::{normalize_evm_address, EvmClient};
pub use types::{
    EscrowCreatedEvent, EvmLog, EvmTransaction, JsonRpcError, JsonRpcRequest, JsonRpcResponse,
};
