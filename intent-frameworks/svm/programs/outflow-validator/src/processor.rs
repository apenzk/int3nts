//! Instruction processor for the outflow validator program.
//!
//! NOTE: This is a stub implementation. Full implementation will be done in Phase 2.

use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey,
};

use crate::instruction::OutflowInstruction;

/// Program entrypoint processor.
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = OutflowInstruction::try_from_slice(instruction_data)?;

    match instruction {
        OutflowInstruction::Initialize {
            gmp_endpoint,
            hub_chain_id,
            trusted_hub_addr,
        } => {
            msg!("Instruction: Initialize");
            process_initialize(program_id, accounts, gmp_endpoint, hub_chain_id, trusted_hub_addr)
        }
        OutflowInstruction::LzReceive {
            src_chain_id,
            src_addr,
            payload,
        } => {
            msg!("Instruction: LzReceive");
            process_lz_receive(program_id, accounts, src_chain_id, src_addr, &payload)
        }
        OutflowInstruction::FulfillIntent { intent_id } => {
            msg!("Instruction: FulfillIntent");
            process_fulfill_intent(program_id, accounts, intent_id)
        }
    }
}

/// Initialize the program configuration.
/// STUB: Returns Ok(()) for now.
fn process_initialize(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _gmp_endpoint: Pubkey,
    _hub_chain_id: u32,
    _trusted_hub_addr: [u8; 32],
) -> ProgramResult {
    msg!("Initialize stub - not yet implemented");
    Ok(())
}

/// Receive intent requirements via GMP.
/// STUB: Returns Ok(()) for now.
fn process_lz_receive(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _src_chain_id: u32,
    _src_addr: [u8; 32],
    _payload: &[u8],
) -> ProgramResult {
    msg!("LzReceive stub - not yet implemented");
    // TODO (Phase 2):
    // 1. Verify source chain and address match trusted hub
    // 2. Decode IntentRequirements from payload
    // 3. Check if requirements already exist (idempotency)
    // 4. If not, create requirements PDA account
    Ok(())
}

/// Fulfill an intent by transferring tokens.
/// STUB: Returns Ok(()) for now.
fn process_fulfill_intent(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _intent_id: [u8; 32],
) -> ProgramResult {
    msg!("FulfillIntent stub - not yet implemented");
    // TODO (Phase 2):
    // 1. Load requirements account
    // 2. Verify solver is authorized
    // 3. Verify intent not already fulfilled
    // 4. Verify intent not expired
    // 5. Transfer tokens from solver to recipient
    // 6. Mark intent as fulfilled
    // 7. Emit FulfillmentSucceeded event
    // 8. Send FulfillmentProof GMP message to hub
    Ok(())
}
