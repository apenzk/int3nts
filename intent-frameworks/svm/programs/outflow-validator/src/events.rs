//! Event definitions for the outflow validator program.
//!
//! Events are emitted via solana_program::msg! and can be parsed from transaction logs.

use solana_program::{msg, pubkey::Pubkey};

/// Emitted when an intent is successfully fulfilled.
pub fn emit_fulfillment_succeeded(
    intent_id: &[u8; 32],
    solver: &Pubkey,
    recipient: &Pubkey,
    amount: u64,
    token_mint: &Pubkey,
) {
    msg!(
        "FulfillmentSucceeded: intent_id={}, solver={}, recipient={}, amount={}, token={}",
        hex::encode(intent_id),
        solver,
        recipient,
        amount,
        token_mint
    );
}

/// Emitted when an intent fulfillment fails.
pub fn emit_fulfillment_failed(intent_id: &[u8; 32], solver: &Pubkey, reason: &str) {
    msg!(
        "FulfillmentFailed: intent_id={}, solver={}, reason={}",
        hex::encode(intent_id),
        solver,
        reason
    );
}

/// Emitted when intent requirements are received via GMP.
pub fn emit_requirements_received(intent_id: &[u8; 32], src_chain_id: u32) {
    msg!(
        "RequirementsReceived: intent_id={}, src_chain_id={}",
        hex::encode(intent_id),
        src_chain_id
    );
}

/// Emitted when requirements already exist (idempotent duplicate).
pub fn emit_requirements_duplicate(intent_id: &[u8; 32]) {
    msg!(
        "RequirementsDuplicate: intent_id={} (ignored)",
        hex::encode(intent_id)
    );
}
