//! Outflow Validator Program (Native Solana)
//!
//! This program validates and executes outflow intent fulfillments on Solana.
//! It receives intent requirements from the hub via GMP, validates solver
//! fulfillments, and sends fulfillment proofs back to the hub.
//!
//! ## Flow
//!
//! 1. Hub creates intent → sends IntentRequirements via GMP
//! 2. This program receives requirements via `gmp_receive` (stores in PDA)
//! 3. Authorized solver calls `fulfill_intent`:
//!    - Tokens pulled from solver to recipient
//!    - FulfillmentProof sent back to hub via GMP
//! 4. Hub receives proof → releases escrowed funds to solver

pub mod error;
pub mod events;
pub mod instruction;
pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

pub use solana_program;

// Re-export for external use
pub use error::OutflowError;
pub use instruction::OutflowInstruction;
pub use state::{seeds, ConfigAccount, IntentRequirementsAccount};
