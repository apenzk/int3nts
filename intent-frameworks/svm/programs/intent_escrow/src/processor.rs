//! Instruction processing

#![allow(deprecated)] // system_instruction deprecation - will migrate when solana_system_interface is stable

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};
use spl_token::state::Account as TokenAccount;

use crate::{
    error::EscrowError,
    instruction::EscrowInstruction,
    state::{seeds, Escrow, EscrowState},
    DEFAULT_EXPIRY_DURATION,
};

pub struct Processor;

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = EscrowInstruction::try_from_slice(instruction_data)
            .map_err(|_| EscrowError::InvalidInstructionData)?;

        match instruction {
            EscrowInstruction::Initialize { approver } => {
                msg!("Instruction: Initialize");
                Self::process_initialize(program_id, accounts, approver)
            }
            EscrowInstruction::CreateEscrow {
                intent_id,
                amount,
                expiry_duration,
            } => {
                msg!("Instruction: CreateEscrow");
                Self::process_create_escrow(program_id, accounts, intent_id, amount, expiry_duration)
            }
            EscrowInstruction::Claim { intent_id, signature } => {
                msg!("Instruction: Claim - intent_id={:?}", &intent_id[..8]);
                Self::process_claim(program_id, accounts, intent_id, signature)
            }
            EscrowInstruction::Cancel { intent_id } => {
                msg!("Instruction: Cancel");
                Self::process_cancel(program_id, accounts, intent_id)
            }
        }
    }

    fn process_initialize(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        approver: Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let state_account = next_account_info(account_info_iter)?;
        let payer = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;

        // Derive state PDA
        let (state_pda, state_bump) =
            Pubkey::find_program_address(&[seeds::STATE_SEED], program_id);
        if state_pda != *state_account.key {
            return Err(EscrowError::InvalidPDA.into());
        }

        // Create state account
        let rent = Rent::get()?;
        let space = EscrowState::LEN;
        let lamports = rent.minimum_balance(space);

        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                state_account.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[payer.clone(), state_account.clone(), system_program.clone()],
            &[&[seeds::STATE_SEED, &[state_bump]]],
        )?;

        // Initialize state
        let state = EscrowState::new(approver);
        state.serialize(&mut &mut state_account.data.borrow_mut()[..])?;

        msg!("Escrow program initialized with approver: {}", approver);
        Ok(())
    }

    fn process_create_escrow(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        intent_id: [u8; 32],
        amount: u64,
        expiry_duration: Option<i64>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let escrow_account = next_account_info(account_info_iter)?;
        let requester = next_account_info(account_info_iter)?;
        let token_mint = next_account_info(account_info_iter)?;
        let requester_token_account = next_account_info(account_info_iter)?;
        let escrow_vault = next_account_info(account_info_iter)?;
        let reserved_solver = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;
        let _rent_sysvar = next_account_info(account_info_iter)?;

        // Validate inputs
        if amount == 0 {
            return Err(EscrowError::InvalidAmount.into());
        }
        if *reserved_solver.key == Pubkey::default() {
            return Err(EscrowError::InvalidSolver.into());
        }
        if !requester.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Derive escrow PDA
        let (escrow_pda, escrow_bump) =
            Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], program_id);
        if escrow_pda != *escrow_account.key {
            return Err(EscrowError::InvalidPDA.into());
        }

        // Derive vault PDA
        let (vault_pda, vault_bump) =
            Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], program_id);
        if vault_pda != *escrow_vault.key {
            return Err(EscrowError::InvalidPDA.into());
        }

        // Check if escrow already exists
        if escrow_account.data_len() > 0 {
            // Account exists, try to deserialize it
            if let Ok(existing_escrow) = Escrow::try_from_slice(&escrow_account.data.borrow()) {
                // Check if it's a valid escrow (has correct discriminator)
                if existing_escrow.discriminator == Escrow::DISCRIMINATOR {
                    return Err(EscrowError::EscrowAlreadyExists.into());
                }
            }
        }

        // Calculate expiry
        let clock = Clock::get()?;
        let duration = expiry_duration.unwrap_or(DEFAULT_EXPIRY_DURATION);
        let duration = if duration <= 0 { DEFAULT_EXPIRY_DURATION } else { duration };
        let expiry = clock.unix_timestamp + duration;

        // Create escrow account
        let rent = Rent::get()?;
        let escrow_space = Escrow::LEN;
        let escrow_lamports = rent.minimum_balance(escrow_space);

        invoke_signed(
            &system_instruction::create_account(
                requester.key,
                escrow_account.key,
                escrow_lamports,
                escrow_space as u64,
                program_id,
            ),
            &[requester.clone(), escrow_account.clone(), system_program.clone()],
            &[&[seeds::ESCROW_SEED, &intent_id, &[escrow_bump]]],
        )?;

        // Create vault token account
        let vault_space = TokenAccount::LEN;
        let vault_lamports = rent.minimum_balance(vault_space);

        invoke_signed(
            &system_instruction::create_account(
                requester.key,
                escrow_vault.key,
                vault_lamports,
                vault_space as u64,
                &spl_token::id(),
            ),
            &[requester.clone(), escrow_vault.clone(), system_program.clone()],
            &[&[seeds::VAULT_SEED, &intent_id, &[vault_bump]]],
        )?;

        // Initialize vault token account
        invoke_signed(
            &spl_token::instruction::initialize_account3(
                &spl_token::id(),
                escrow_vault.key,
                token_mint.key,
                escrow_account.key, // escrow PDA is the authority
            )?,
            &[escrow_vault.clone(), token_mint.clone()],
            &[&[seeds::VAULT_SEED, &intent_id, &[vault_bump]]],
        )?;

        // Transfer tokens to vault
        invoke(
            &spl_token::instruction::transfer(
                &spl_token::id(),
                requester_token_account.key,
                escrow_vault.key,
                requester.key,
                &[],
                amount,
            )?,
            &[
                requester_token_account.clone(),
                escrow_vault.clone(),
                requester.clone(),
                token_program.clone(),
            ],
        )?;

        // Initialize escrow state
        let escrow = Escrow::new(
            *requester.key,
            *token_mint.key,
            amount,
            expiry,
            *reserved_solver.key,
            intent_id,
            escrow_bump,
        );
        escrow.serialize(&mut &mut escrow_account.data.borrow_mut()[..])?;

        msg!(
            "Escrow created: intent_id={:?}, amount={}, expiry={}",
            &intent_id[..8],
            amount,
            expiry
        );
        Ok(())
    }

    fn process_claim(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        intent_id: [u8; 32],
        signature: [u8; 64],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let escrow_account = next_account_info(account_info_iter)?;
        let state_account = next_account_info(account_info_iter)?;
        let escrow_vault = next_account_info(account_info_iter)?;
        let solver_token_account = next_account_info(account_info_iter)?;
        let instruction_sysvar = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;

        // Deserialize accounts
        let mut escrow = Escrow::try_from_slice(&escrow_account.data.borrow())?;
        let state = EscrowState::try_from_slice(&state_account.data.borrow())?;

        // Validate escrow
        if escrow.intent_id != intent_id {
            return Err(EscrowError::EscrowDoesNotExist.into());
        }
        if escrow.is_claimed {
            return Err(EscrowError::EscrowAlreadyClaimed.into());
        }
        if escrow.amount == 0 {
            return Err(EscrowError::NoDeposit.into());
        }

        let clock = Clock::get()?;
        if clock.unix_timestamp > escrow.expiry {
            return Err(EscrowError::EscrowExpired.into());
        }

        // Verify Ed25519 signature via instruction introspection
        Self::verify_ed25519_signature(
            instruction_sysvar,
            &state.approver,
            &intent_id,
            &signature,
        )?;

        // Transfer tokens from vault to solver
        let amount = escrow.amount;
        let escrow_seeds = &[seeds::ESCROW_SEED, &intent_id[..], &[escrow.bump]];

        invoke_signed(
            &spl_token::instruction::transfer(
                &spl_token::id(),
                escrow_vault.key,
                solver_token_account.key,
                escrow_account.key,
                &[],
                amount,
            )?,
            &[
                escrow_vault.clone(),
                solver_token_account.clone(),
                escrow_account.clone(),
                token_program.clone(),
            ],
            &[escrow_seeds],
        )?;

        // Update escrow state
        escrow.is_claimed = true;
        escrow.amount = 0;
        escrow.serialize(&mut &mut escrow_account.data.borrow_mut()[..])?;

        msg!("Escrow claimed: intent_id={:?}, amount={}", &intent_id[..8], amount);
        Ok(())
    }

    fn process_cancel(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        intent_id: [u8; 32],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let escrow_account = next_account_info(account_info_iter)?;
        let requester = next_account_info(account_info_iter)?;
        let escrow_vault = next_account_info(account_info_iter)?;
        let requester_token_account = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;

        // Deserialize escrow
        let mut escrow = Escrow::try_from_slice(&escrow_account.data.borrow())?;

        // Validate
        if escrow.intent_id != intent_id {
            return Err(EscrowError::EscrowDoesNotExist.into());
        }
        if escrow.is_claimed {
            return Err(EscrowError::EscrowAlreadyClaimed.into());
        }
        if escrow.amount == 0 {
            return Err(EscrowError::NoDeposit.into());
        }
        if escrow.requester != *requester.key {
            return Err(EscrowError::UnauthorizedRequester.into());
        }
        if !requester.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let clock = Clock::get()?;
        if clock.unix_timestamp <= escrow.expiry {
            return Err(EscrowError::EscrowNotExpiredYet.into());
        }

        // Transfer tokens back to requester
        let amount = escrow.amount;
        let escrow_seeds = &[seeds::ESCROW_SEED, &intent_id[..], &[escrow.bump]];

        invoke_signed(
            &spl_token::instruction::transfer(
                &spl_token::id(),
                escrow_vault.key,
                requester_token_account.key,
                escrow_account.key,
                &[],
                amount,
            )?,
            &[
                escrow_vault.clone(),
                requester_token_account.clone(),
                escrow_account.clone(),
                token_program.clone(),
            ],
            &[escrow_seeds],
        )?;

        // Update escrow state
        escrow.is_claimed = true;
        escrow.amount = 0;
        escrow.serialize(&mut &mut escrow_account.data.borrow_mut()[..])?;

        msg!("Escrow cancelled: intent_id={:?}, amount={}", &intent_id[..8], amount);
        Ok(())
    }

    fn verify_ed25519_signature(
        instruction_sysvar: &AccountInfo,
        expected_pubkey: &Pubkey,
        expected_message: &[u8; 32],
        expected_signature: &[u8; 64],
    ) -> ProgramResult {
        // Load the Ed25519 instruction (should be at index 0)
        let ed25519_ix = solana_program::sysvar::instructions::load_instruction_at_checked(
            0,
            instruction_sysvar,
        )?;

        // Verify it's an Ed25519 instruction
        if ed25519_ix.program_id != solana_program::ed25519_program::ID {
            return Err(EscrowError::InvalidSignature.into());
        }

        // Parse Ed25519 instruction data
        let data = &ed25519_ix.data;
        if data.len() < 16 {
            return Err(EscrowError::InvalidSignature.into());
        }

        let num_signatures = data[0];
        if num_signatures < 1 {
            return Err(EscrowError::InvalidSignature.into());
        }

        // Read offsets for first signature
        let sig_offset = u16::from_le_bytes([data[2], data[3]]) as usize;
        let pubkey_offset = u16::from_le_bytes([data[6], data[7]]) as usize;
        let msg_offset = u16::from_le_bytes([data[10], data[11]]) as usize;
        let msg_size = u16::from_le_bytes([data[12], data[13]]) as usize;

        // Validate offsets
        if data.len() < sig_offset + 64
            || data.len() < pubkey_offset + 32
            || data.len() < msg_offset + msg_size
        {
            return Err(EscrowError::InvalidSignature.into());
        }

        // Extract and verify
        let instruction_signature = &data[sig_offset..sig_offset + 64];
        let instruction_pubkey = &data[pubkey_offset..pubkey_offset + 32];
        let instruction_message = &data[msg_offset..msg_offset + msg_size];

        if instruction_pubkey != expected_pubkey.to_bytes().as_slice() {
            return Err(EscrowError::UnauthorizedApprover.into());
        }
        if instruction_signature != expected_signature.as_slice() {
            return Err(EscrowError::InvalidSignature.into());
        }
        if instruction_message != expected_message.as_slice() {
            return Err(EscrowError::InvalidSignature.into());
        }

        Ok(())
    }
}
