//! Solana SVM chain client for cross-chain intent services
//!
//! Shared SVM JSON-RPC client used by coordinator, integrated-gmp, and solver.

pub mod client;
pub mod types;

pub use client::{parse_escrow_data, parse_intent_id, pubkey_from_hex, pubkey_to_hex, SvmClient};
pub use types::{EscrowAccount, EscrowEvent, EscrowWithPubkey};

// Re-export solana_program for consumers that need Pubkey
pub use solana_program;
