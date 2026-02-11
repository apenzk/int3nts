//! Instruction processor for the outflow validator program.

use borsh::{BorshDeserialize, BorshSerialize};
use gmp_common::messages::{FulfillmentProof, IntentRequirements};
#[allow(deprecated)]
use solana_program::system_instruction;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

use crate::error::OutflowError;
use crate::events::{
    emit_fulfillment_succeeded, emit_requirements_duplicate, emit_requirements_received,
};
use crate::instruction::OutflowInstruction;
use crate::state::{seeds, ConfigAccount, IntentRequirementsAccount};

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
fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    gmp_endpoint: Pubkey,
    hub_chain_id: u32,
    trusted_hub_addr: [u8; 32],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let config_account = next_account_info(account_info_iter)?;
    let admin = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Verify admin is signer
    if !admin.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Derive config PDA
    let (config_pda, config_bump) =
        Pubkey::find_program_address(&[seeds::CONFIG_SEED], program_id);

    if config_account.key != &config_pda {
        return Err(OutflowError::InvalidPda.into());
    }

    // Check if already initialized
    if !config_account.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    // Create config account
    let rent = Rent::get()?;
    let space = ConfigAccount::SIZE;
    let lamports = rent.minimum_balance(space);

    invoke_signed(
        &system_instruction::create_account(
            admin.key,
            config_account.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[admin.clone(), config_account.clone(), system_program.clone()],
        &[&[seeds::CONFIG_SEED, &[config_bump]]],
    )?;

    // Initialize config data
    let config = ConfigAccount::new(*admin.key, gmp_endpoint, hub_chain_id, trusted_hub_addr, config_bump);
    config.serialize(&mut &mut config_account.data.borrow_mut()[..])?;

    msg!(
        "OutflowValidator initialized: hub_chain_id={}, gmp_endpoint={}",
        hub_chain_id,
        gmp_endpoint
    );
    Ok(())
}

/// Receive intent requirements via GMP.
///
/// This is called by the GMP endpoint when a message is delivered from the hub.
/// Implements idempotency: if requirements already exist, silently succeeds.
fn process_lz_receive(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    src_chain_id: u32,
    src_addr: [u8; 32],
    payload: &[u8],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let requirements_account = next_account_info(account_info_iter)?;
    let config_account = next_account_info(account_info_iter)?;
    let _authority = next_account_info(account_info_iter)?; // GMP endpoint or relay
    let payer = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Load and verify config
    let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], program_id);
    if config_account.key != &config_pda {
        return Err(OutflowError::InvalidPda.into());
    }

    let config = ConfigAccount::try_from_slice(&config_account.data.borrow())
        .map_err(|_| OutflowError::InvalidAccountOwner)?;

    // Verify source chain and address match trusted hub
    if src_chain_id != config.hub_chain_id {
        msg!(
            "Invalid source chain: expected {}, got {}",
            config.hub_chain_id,
            src_chain_id
        );
        return Err(OutflowError::InvalidGmpMessage.into());
    }

    if src_addr != config.trusted_hub_addr {
        msg!("Invalid source address: not trusted hub");
        return Err(OutflowError::InvalidGmpMessage.into());
    }

    // Decode IntentRequirements from payload
    let requirements = IntentRequirements::decode(payload)
        .map_err(|_| OutflowError::InvalidGmpMessage)?;

    // Derive requirements PDA
    let (requirements_pda, requirements_bump) = Pubkey::find_program_address(
        &[seeds::REQUIREMENTS_SEED, &requirements.intent_id],
        program_id,
    );

    if requirements_account.key != &requirements_pda {
        return Err(OutflowError::InvalidPda.into());
    }

    // Idempotency check: if account already exists, emit duplicate event and return success
    if !requirements_account.data_is_empty() {
        emit_requirements_duplicate(&requirements.intent_id);
        return Ok(());
    }

    // Create requirements account
    let rent = Rent::get()?;
    let space = IntentRequirementsAccount::SIZE;
    let lamports = rent.minimum_balance(space);

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            requirements_account.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[
            payer.clone(),
            requirements_account.clone(),
            system_program.clone(),
        ],
        &[&[
            seeds::REQUIREMENTS_SEED,
            &requirements.intent_id,
            &[requirements_bump],
        ]],
    )?;

    // Convert addresses from GMP format (32 bytes) to Pubkey
    let recipient_addr = Pubkey::try_from(&requirements.requester_addr[..])
        .map_err(|_| OutflowError::InvalidGmpMessage)?;
    let token_mint = Pubkey::try_from(&requirements.token_addr[..])
        .map_err(|_| OutflowError::InvalidGmpMessage)?;
    let authorized_solver = Pubkey::try_from(&requirements.solver_addr[..])
        .map_err(|_| OutflowError::InvalidGmpMessage)?;

    // Store requirements
    let requirements_data = IntentRequirementsAccount::new(
        requirements.intent_id,
        recipient_addr,
        requirements.amount_required,
        token_mint,
        authorized_solver,
        requirements.expiry,
        requirements_bump,
    );
    requirements_data.serialize(&mut &mut requirements_account.data.borrow_mut()[..])?;

    emit_requirements_received(&requirements.intent_id, src_chain_id);
    Ok(())
}

/// Fulfill an intent by transferring tokens.
fn process_fulfill_intent(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    intent_id: [u8; 32],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let requirements_account = next_account_info(account_info_iter)?;
    let config_account = next_account_info(account_info_iter)?;
    let solver = next_account_info(account_info_iter)?;
    let solver_token_account = next_account_info(account_info_iter)?;
    let recipient_token_account = next_account_info(account_info_iter)?;
    let token_mint = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;
    let gmp_endpoint_program = next_account_info(account_info_iter)?;
    // Remaining accounts are for GMP endpoint CPI
    let gmp_accounts: Vec<AccountInfo> = account_info_iter.cloned().collect();

    // Verify solver is signer
    if !solver.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify config
    let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], program_id);
    if config_account.key != &config_pda {
        return Err(OutflowError::InvalidPda.into());
    }

    let config = ConfigAccount::try_from_slice(&config_account.data.borrow())
        .map_err(|_| OutflowError::InvalidAccountOwner)?;

    // Verify GMP endpoint matches config
    if gmp_endpoint_program.key != &config.gmp_endpoint {
        msg!("Invalid GMP endpoint program");
        return Err(OutflowError::InvalidAccountOwner.into());
    }

    // Load and verify requirements
    let (requirements_pda, _) = Pubkey::find_program_address(
        &[seeds::REQUIREMENTS_SEED, &intent_id],
        program_id,
    );
    if requirements_account.key != &requirements_pda {
        return Err(OutflowError::InvalidPda.into());
    }

    let mut requirements = IntentRequirementsAccount::try_from_slice(&requirements_account.data.borrow())
        .map_err(|_| OutflowError::RequirementsNotFound)?;

    // Verify intent_id matches
    if requirements.intent_id != intent_id {
        return Err(OutflowError::RequirementsNotFound.into());
    }

    // Verify intent not already fulfilled
    if requirements.fulfilled {
        return Err(OutflowError::AlreadyFulfilled.into());
    }

    // Verify intent not expired
    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp as u64;
    if current_timestamp > requirements.expiry {
        return Err(OutflowError::IntentExpired.into());
    }

    // Verify solver is authorized (zero address = any solver allowed)
    let zero_pubkey = Pubkey::default();
    if requirements.authorized_solver != zero_pubkey && requirements.authorized_solver != *solver.key {
        return Err(OutflowError::UnauthorizedSolver.into());
    }

    // Verify token mint matches
    if token_mint.key != &requirements.token_mint {
        return Err(OutflowError::TokenMismatch.into());
    }

    // Verify recipient token account belongs to the intended recipient
    // SPL Token account layout: mint (32) | owner (32) | amount (8) | ...
    // We need to read the owner field at offset 32
    // Note: borrow is scoped to release before transfer CPI
    {
        let recipient_data = recipient_token_account.try_borrow_data()?;
        if recipient_data.len() < 64 {
            msg!("Invalid recipient token account data");
            return Err(OutflowError::InvalidAccountOwner.into());
        }
        let recipient_owner = Pubkey::try_from(&recipient_data[32..64])
            .map_err(|_| OutflowError::InvalidAccountOwner)?;
        if recipient_owner != requirements.recipient_addr {
            msg!(
                "Recipient mismatch: token account owner {} != required recipient {}",
                recipient_owner,
                requirements.recipient_addr
            );
            return Err(OutflowError::RecipientMismatch.into());
        }
    }

    // Transfer tokens from solver to recipient
    let transfer_ix = spl_token::instruction::transfer(
        token_program.key,
        solver_token_account.key,
        recipient_token_account.key,
        solver.key,
        &[],
        requirements.amount_required,
    )?;

    invoke(
        &transfer_ix,
        &[
            solver_token_account.clone(),
            recipient_token_account.clone(),
            solver.clone(),
            token_program.clone(),
        ],
    )?;

    // Mark intent as fulfilled
    requirements.fulfilled = true;
    requirements.serialize(&mut &mut requirements_account.data.borrow_mut()[..])?;

    // Emit success event
    emit_fulfillment_succeeded(
        &intent_id,
        solver.key,
        &requirements.recipient_addr,
        requirements.amount_required,
        &requirements.token_mint,
    );

    // Send FulfillmentProof GMP message to hub
    let fulfillment_proof = FulfillmentProof {
        intent_id,
        solver_addr: solver.key.to_bytes(),
        amount_fulfilled: requirements.amount_required,
        timestamp: current_timestamp,
    };
    let payload = fulfillment_proof.encode();

    // Build Send instruction for GMP endpoint
    // NativeGmpInstruction::Send variant index is 5 (0=Initialize, 1=AddRelay, 2=RemoveRelay, 3=SetTrustedRemote, 4=SetRouting, 5=Send)
    // Format: variant(1) + dst_chain_id(4) + dst_addr(32) + src_addr(32) + payload_len(4) + payload
    let mut send_data = Vec::with_capacity(1 + 4 + 32 + 32 + 4 + payload.len());
    send_data.push(5); // Send variant index
    send_data.extend_from_slice(&config.hub_chain_id.to_le_bytes());
    send_data.extend_from_slice(&config.trusted_hub_addr);
    send_data.extend_from_slice(&program_id.to_bytes()); // src_addr = outflow-validator program ID
    send_data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    send_data.extend_from_slice(&payload);

    // Build account metas for GMP Send CPI
    let mut account_metas = Vec::with_capacity(gmp_accounts.len());
    for acc in &gmp_accounts {
        if acc.is_writable {
            account_metas.push(solana_program::instruction::AccountMeta::new(*acc.key, acc.is_signer));
        } else {
            account_metas.push(solana_program::instruction::AccountMeta::new_readonly(*acc.key, acc.is_signer));
        }
    }

    let cpi_instruction = solana_program::instruction::Instruction {
        program_id: *gmp_endpoint_program.key,
        accounts: account_metas,
        data: send_data,
    };

    invoke(&cpi_instruction, &gmp_accounts)?;

    msg!("FulfillmentProof sent to hub");
    Ok(())
}
