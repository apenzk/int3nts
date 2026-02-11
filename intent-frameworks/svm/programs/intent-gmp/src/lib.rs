//! Integrated GMP Endpoint Program (Native Solana)
//!
//! A integrated GMP endpoint that can be used for local testing, CI, or production
//! with a trusted relay or DKG-based message verification.
//!
//! ## Purpose
//!
//! This endpoint provides a standardized interface for cross-chain messaging.
//! In production, this can be replaced by LZ's endpoint or used directly
//! with your own relay infrastructure.
//!
//! ## Instructions
//!
//! - `Initialize`: Set up the endpoint with admin and chain ID
//! - `AddRelay`: Authorize a relay to deliver messages
//! - `RemoveRelay`: Deauthorize a relay
//! - `SetTrustedRemote`: Configure trusted source addresses per chain
//! - `Send`: Emit a MessageSent event for the relay to pick up
//! - `DeliverMessage`: Called by relay to deliver messages to destination
//!
//! ## Security Model
//!
//! - Admin controls relay authorization and trusted remote configuration
//! - Only authorized relays can deliver messages
//! - Messages from untrusted sources are rejected
//! - (intent_id, msg_type) deduplication prevents replay attacks

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

pub use solana_program;

// Re-export for external use
pub use error::GmpError;
pub use instruction::NativeGmpInstruction;
pub use state::{
    ConfigAccount, DeliveredMessage, MessageAccount, OutboundNonceAccount, RelayAccount,
    TrustedRemoteAccount,
};
