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

use gmp_common::messages::{EscrowConfirmation, FulfillmentProof, IntentRequirements};

use crate::{
    error::EscrowError,
    instruction::EscrowInstruction,
    state::{seeds, Escrow, EscrowState, GmpConfig, StoredIntentRequirements},
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
            EscrowInstruction::GmpReceive {
                src_chain_id,
                remote_gmp_endpoint_addr,
                payload,
            } => {
                // Route based on message type (first byte of payload)
                let message_type = payload.first().copied().unwrap_or(0);
                msg!("Instruction: GmpReceive (message_type=0x{:02x})", message_type);
                match message_type {
                    0x01 => Self::process_gmp_receive_requirements(
                        program_id,
                        accounts,
                        src_chain_id,
                        remote_gmp_endpoint_addr,
                        payload,
                    ),
                    0x03 => Self::process_gmp_receive_fulfillment_proof(
                        program_id,
                        accounts,
                        src_chain_id,
                        remote_gmp_endpoint_addr,
                        payload,
                    ),
                    _ => {
                        msg!("Unknown GMP message type: 0x{:02x}", message_type);
                        Err(EscrowError::InvalidGmpMessage.into())
                    }
                }
            }
            EscrowInstruction::SetGmpConfig {
                hub_chain_id,
                hub_gmp_endpoint_addr,
                gmp_endpoint,
            } => {
                msg!("Instruction: SetGmpConfig");
                Self::process_set_gmp_config(
                    program_id,
                    accounts,
                    hub_chain_id,
                    hub_gmp_endpoint_addr,
                    gmp_endpoint,
                )
            }
            EscrowInstruction::CreateEscrow {
                intent_id,
                amount,
            } => {
                msg!("Instruction: CreateEscrow");
                Self::process_create_escrow(program_id, accounts, intent_id, amount)
            }
            EscrowInstruction::Claim { intent_id } => {
                msg!("Instruction: Claim - intent_id={:?}", &intent_id[..8]);
                Self::process_claim(program_id, accounts, intent_id)
            }
            EscrowInstruction::Cancel { intent_id } => {
                msg!("Instruction: Cancel");
                Self::process_cancel(program_id, accounts, intent_id)
            }
            EscrowInstruction::GmpReceiveRequirements {
                src_chain_id,
                remote_gmp_endpoint_addr,
                payload,
            } => {
                msg!("Instruction: GmpReceiveRequirements");
                Self::process_gmp_receive_requirements(
                    program_id,
                    accounts,
                    src_chain_id,
                    remote_gmp_endpoint_addr,
                    payload,
                )
            }
            EscrowInstruction::GmpReceiveFulfillmentProof {
                src_chain_id,
                remote_gmp_endpoint_addr,
                payload,
            } => {
                msg!("Instruction: GmpReceiveFulfillmentProof");
                Self::process_gmp_receive_fulfillment_proof(
                    program_id,
                    accounts,
                    src_chain_id,
                    remote_gmp_endpoint_addr,
                    payload,
                )
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
            return Err(EscrowError::InvalidPda.into());
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

    /// Set or update GMP configuration.
    fn process_set_gmp_config(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        hub_chain_id: u32,
        hub_gmp_endpoint_addr: [u8; 32],
        gmp_endpoint: Pubkey,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let gmp_config_account = next_account_info(account_info_iter)?;
        let admin = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;

        // Admin must be signer
        if !admin.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Derive GMP config PDA
        let (config_pda, config_bump) =
            Pubkey::find_program_address(&[seeds::GMP_CONFIG_SEED], program_id);
        if config_pda != *gmp_config_account.key {
            return Err(EscrowError::InvalidPda.into());
        }

        // Check if config already exists
        if gmp_config_account.data_len() > 0 {
            // Update existing config - verify admin matches
            let mut config = GmpConfig::try_from_slice(&gmp_config_account.data.borrow())
                .map_err(|_| EscrowError::AccountNotInitialized)?;

            if config.admin != *admin.key {
                return Err(EscrowError::UnauthorizedApprover.into());
            }

            // Update config
            config.hub_chain_id = hub_chain_id;
            config.hub_gmp_endpoint_addr = hub_gmp_endpoint_addr;
            config.gmp_endpoint = gmp_endpoint;
            config.serialize(&mut &mut gmp_config_account.data.borrow_mut()[..])?;

            msg!(
                "GMP config updated: hub_chain_id={}, gmp_endpoint={}",
                hub_chain_id,
                gmp_endpoint
            );
        } else {
            // Create new config account
            let rent = Rent::get()?;
            let space = GmpConfig::LEN;
            let lamports = rent.minimum_balance(space);

            invoke_signed(
                &system_instruction::create_account(
                    admin.key,
                    gmp_config_account.key,
                    lamports,
                    space as u64,
                    program_id,
                ),
                &[
                    admin.clone(),
                    gmp_config_account.clone(),
                    system_program.clone(),
                ],
                &[&[seeds::GMP_CONFIG_SEED, &[config_bump]]],
            )?;

            // Initialize config
            let config = GmpConfig::new(
                *admin.key,
                hub_chain_id,
                hub_gmp_endpoint_addr,
                gmp_endpoint,
                config_bump,
            );
            config.serialize(&mut &mut gmp_config_account.data.borrow_mut()[..])?;

            msg!(
                "GMP config initialized: hub_chain_id={}, gmp_endpoint={}",
                hub_chain_id,
                gmp_endpoint
            );
        }

        Ok(())
    }

    fn process_create_escrow(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        intent_id: [u8; 32],
        amount: u64,
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
        // Requirements account (mandatory) - validates against stored GMP requirements
        let requirements_account = next_account_info(account_info_iter)?;

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

        // Validate requirements account PDA
        let (req_pda, _) = Pubkey::find_program_address(
            &[seeds::REQUIREMENTS_SEED, &intent_id],
            program_id,
        );
        if req_pda != *requirements_account.key {
            return Err(EscrowError::InvalidPda.into());
        }

        // Load and validate stored GMP requirements
        let requirements = StoredIntentRequirements::try_from_slice(&requirements_account.data.borrow())
            .map_err(|_| EscrowError::RequirementsNotFound)?;

        if requirements.escrow_created {
            return Err(EscrowError::EscrowAlreadyCreated.into());
        }
        if amount < requirements.amount_required {
            return Err(EscrowError::AmountMismatch.into());
        }
        // Validate token - convert Pubkey to 32-byte array for comparison
        let token_bytes = token_mint.key.to_bytes();
        if token_bytes != requirements.token_addr {
            return Err(EscrowError::TokenMismatch.into());
        }

        // Derive escrow PDA
        let (escrow_pda, escrow_bump) =
            Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], program_id);
        if escrow_pda != *escrow_account.key {
            return Err(EscrowError::InvalidPda.into());
        }

        // Derive vault PDA
        let (vault_pda, vault_bump) =
            Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], program_id);
        if vault_pda != *escrow_vault.key {
            return Err(EscrowError::InvalidPda.into());
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

        // Use the hub-provided expiry directly (absolute timestamp).
        // This matches EVM and MVM behavior where the hub is the source of truth.
        // Cap at i64::MAX to avoid overflow when storing in Escrow (which uses i64 for Solana Clock compatibility).
        let clock = Clock::get()?;
        let expiry = if requirements.expiry > i64::MAX as u64 {
            i64::MAX
        } else {
            requirements.expiry as i64
        };
        if clock.unix_timestamp > expiry {
            return Err(EscrowError::IntentExpired.into());
        }

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

        // Mark requirements as having escrow created and send EscrowConfirmation
        {
            let mut requirements = requirements;
            requirements.escrow_created = true;
            requirements.serialize(&mut &mut requirements_account.data.borrow_mut()[..])?;

            // Try to send EscrowConfirmation GMP message if GMP config is available
            let gmp_config_account = next_account_info(account_info_iter).ok();
            let gmp_endpoint_program = next_account_info(account_info_iter).ok();

            if let (Some(config_account), Some(endpoint_program)) =
                (gmp_config_account, gmp_endpoint_program)
            {
                // Verify GMP config PDA
                let (config_pda, _) =
                    Pubkey::find_program_address(&[seeds::GMP_CONFIG_SEED], program_id);
                if config_pda == *config_account.key && config_account.data_len() > 0 {
                    let config = GmpConfig::try_from_slice(&config_account.data.borrow())
                        .map_err(|_| EscrowError::AccountNotInitialized)?;

                    // Verify GMP endpoint matches config
                    if endpoint_program.key == &config.gmp_endpoint {
                        // Build EscrowConfirmation message
                        let confirmation = EscrowConfirmation {
                            intent_id,
                            escrow_id: escrow_account.key.to_bytes(),
                            amount_escrowed: amount,
                            token_addr: token_mint.key.to_bytes(),
                            creator_addr: requester.key.to_bytes(),
                        };
                        let payload = confirmation.encode();

                        // Collect remaining accounts for GMP CPI
                        let gmp_accounts: Vec<AccountInfo> = account_info_iter.cloned().collect();

                        // Build Send instruction for GMP endpoint
                        // NativeGmpInstruction::Send variant index is 5 (0=Initialize, 1=AddRelay, 2=RemoveRelay, 3=SetRemoteGmpEndpointAddr, 4=SetRouting, 5=Send)
                        // Format: variant(1) + dst_chain_id(4) + dst_addr(32) + remote_gmp_endpoint_addr(32) + payload_len(4) + payload
                        let mut send_data =
                            Vec::with_capacity(1 + 4 + 32 + 32 + 4 + payload.len());
                        send_data.push(5); // Send variant index
                        send_data.extend_from_slice(&config.hub_chain_id.to_le_bytes());
                        send_data.extend_from_slice(&config.hub_gmp_endpoint_addr);
                        send_data.extend_from_slice(&endpoint_program.key.to_bytes()); // remote_gmp_endpoint_addr = GMP endpoint program ID (must match hub's remote GMP endpoint)
                        send_data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
                        send_data.extend_from_slice(&payload);

                        // Build account metas for GMP Send CPI
                        let mut account_metas = Vec::with_capacity(gmp_accounts.len());
                        for acc in &gmp_accounts {
                            if acc.is_writable {
                                account_metas.push(
                                    solana_program::instruction::AccountMeta::new(
                                        *acc.key,
                                        acc.is_signer,
                                    ),
                                );
                            } else {
                                account_metas.push(
                                    solana_program::instruction::AccountMeta::new_readonly(
                                        *acc.key,
                                        acc.is_signer,
                                    ),
                                );
                            }
                        }

                        let cpi_instruction = solana_program::instruction::Instruction {
                            program_id: *endpoint_program.key,
                            accounts: account_metas,
                            data: send_data,
                        };

                        invoke(&cpi_instruction, &gmp_accounts)?;

                        msg!(
                            "EscrowConfirmationSent: intent_id={:?}, escrow_id={}, amount={}",
                            &intent_id[..8],
                            escrow_account.key,
                            amount
                        );
                    }
                }
            }
        }

        msg!(
            "Escrow created: intent_id={:?}, amount={}, expiry={}",
            &intent_id[..8],
            amount,
            expiry
        );
        Ok(())
    }

    /// Process Claim instruction (GMP mode - no signature required).
    /// Requires that the fulfillment proof has been received via GMP.
    fn process_claim(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        intent_id: [u8; 32],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let escrow_account = next_account_info(account_info_iter)?;
        let requirements_account = next_account_info(account_info_iter)?;
        let escrow_vault = next_account_info(account_info_iter)?;
        let solver_token_account = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;

        // Validate requirements PDA
        let (req_pda, _) = Pubkey::find_program_address(
            &[seeds::REQUIREMENTS_SEED, &intent_id],
            program_id,
        );
        if req_pda != *requirements_account.key {
            return Err(EscrowError::InvalidPda.into());
        }

        // Load and validate requirements
        let requirements =
            StoredIntentRequirements::try_from_slice(&requirements_account.data.borrow())
                .map_err(|_| EscrowError::RequirementsNotFound)?;

        // GMP mode: require fulfillment proof to have been received
        if !requirements.fulfilled {
            return Err(EscrowError::AlreadyFulfilled.into()); // Not fulfilled yet
        }

        // Deserialize escrow
        let mut escrow = Escrow::try_from_slice(&escrow_account.data.borrow())?;

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
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        intent_id: [u8; 32],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let escrow_account = next_account_info(account_info_iter)?;
        let caller = next_account_info(account_info_iter)?;
        let escrow_vault = next_account_info(account_info_iter)?;
        let requester_token_account = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;
        let gmp_config_account = next_account_info(account_info_iter)?;

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
        if !caller.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Verify caller is admin (only admin can cancel expired escrows)
        let (config_pda, _) =
            Pubkey::find_program_address(&[seeds::GMP_CONFIG_SEED], program_id);
        if config_pda != *gmp_config_account.key {
            return Err(EscrowError::InvalidPda.into());
        }
        let config = GmpConfig::try_from_slice(&gmp_config_account.data.borrow())
            .map_err(|_| EscrowError::AccountNotInitialized)?;
        if config.admin != *caller.key {
            return Err(EscrowError::UnauthorizedCaller.into());
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

    /// Process GmpReceiveRequirements instruction.
    /// Stores intent requirements received via GMP from the hub.
    /// Implements idempotency: if requirements already exist, silently succeeds.
    fn process_gmp_receive_requirements(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        src_chain_id: u32,
        remote_gmp_endpoint_addr: [u8; 32],
        payload: Vec<u8>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let requirements_account = next_account_info(account_info_iter)?;
        let gmp_config_account = next_account_info(account_info_iter)?;
        let gmp_caller = next_account_info(account_info_iter)?;
        let payer = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;

        // GMP caller must be a signer (trusted relay or endpoint)
        if !gmp_caller.is_signer {
            return Err(EscrowError::UnauthorizedGmpSource.into());
        }

        // Load and validate GMP config
        let (config_pda, _) =
            Pubkey::find_program_address(&[seeds::GMP_CONFIG_SEED], program_id);
        if config_pda != *gmp_config_account.key {
            return Err(EscrowError::InvalidPda.into());
        }

        let config = GmpConfig::try_from_slice(&gmp_config_account.data.borrow())
            .map_err(|_| EscrowError::AccountNotInitialized)?;

        // Validate source chain matches hub GMP endpoint
        if src_chain_id != config.hub_chain_id {
            msg!(
                "Invalid source chain: expected {}, got {}",
                config.hub_chain_id,
                src_chain_id
            );
            return Err(EscrowError::UnauthorizedGmpSource.into());
        }

        // Validate source address matches hub GMP endpoint
        if remote_gmp_endpoint_addr != config.hub_gmp_endpoint_addr {
            msg!("Invalid source address: not hub GMP endpoint");
            return Err(EscrowError::UnauthorizedGmpSource.into());
        }

        // Decode the GMP message
        let requirements = IntentRequirements::decode(&payload)
            .map_err(|_| EscrowError::InvalidGmpMessage)?;

        // Derive requirements PDA
        let (req_pda, req_bump) = Pubkey::find_program_address(
            &[seeds::REQUIREMENTS_SEED, &requirements.intent_id],
            program_id,
        );
        if req_pda != *requirements_account.key {
            return Err(EscrowError::InvalidPda.into());
        }

        // Idempotency check: if requirements already exist, emit log and return success
        if requirements_account.data_len() > 0 {
            msg!(
                "RequirementsDuplicate: intent_id={:?} (already stored, ignoring)",
                &requirements.intent_id[..8]
            );
            return Ok(());
        }

        // Create requirements account
        let rent = Rent::get()?;
        let space = StoredIntentRequirements::LEN;
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
            &[&[seeds::REQUIREMENTS_SEED, &requirements.intent_id, &[req_bump]]],
        )?;

        // Store requirements
        let stored = StoredIntentRequirements::new(
            requirements.intent_id,
            requirements.requester_addr,
            requirements.amount_required,
            requirements.token_addr,
            requirements.solver_addr,
            requirements.expiry,
            req_bump,
        );
        stored.serialize(&mut &mut requirements_account.data.borrow_mut()[..])?;

        msg!(
            "IntentRequirementsReceived: intent_id={:?}, amount={}, src_chain_id={}",
            &requirements.intent_id[..8],
            requirements.amount_required,
            src_chain_id
        );
        Ok(())
    }

    /// Process GmpReceiveFulfillmentProof instruction.
    /// Auto-releases escrow when fulfillment proof is received from hub.
    fn process_gmp_receive_fulfillment_proof(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        src_chain_id: u32,
        remote_gmp_endpoint_addr: [u8; 32],
        payload: Vec<u8>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let requirements_account = next_account_info(account_info_iter)?;
        let escrow_account = next_account_info(account_info_iter)?;
        let escrow_vault = next_account_info(account_info_iter)?;
        let solver_token_account = next_account_info(account_info_iter)?;
        let gmp_config_account = next_account_info(account_info_iter)?;
        let gmp_caller = next_account_info(account_info_iter)?;
        let token_program = next_account_info(account_info_iter)?;

        // GMP caller must be a signer (trusted relay or endpoint)
        if !gmp_caller.is_signer {
            return Err(EscrowError::UnauthorizedGmpSource.into());
        }

        // Load and validate GMP config
        let (config_pda, _) =
            Pubkey::find_program_address(&[seeds::GMP_CONFIG_SEED], program_id);
        if config_pda != *gmp_config_account.key {
            return Err(EscrowError::InvalidPda.into());
        }

        let config = GmpConfig::try_from_slice(&gmp_config_account.data.borrow())
            .map_err(|_| EscrowError::AccountNotInitialized)?;

        // Validate source chain matches hub GMP endpoint
        if src_chain_id != config.hub_chain_id {
            msg!(
                "Invalid source chain: expected {}, got {}",
                config.hub_chain_id,
                src_chain_id
            );
            return Err(EscrowError::UnauthorizedGmpSource.into());
        }

        // Validate source address matches hub GMP endpoint
        if remote_gmp_endpoint_addr != config.hub_gmp_endpoint_addr {
            msg!("Invalid source address: not hub GMP endpoint");
            return Err(EscrowError::UnauthorizedGmpSource.into());
        }

        // Decode the GMP message
        let proof = FulfillmentProof::decode(&payload)
            .map_err(|_| EscrowError::InvalidGmpMessage)?;

        // Validate requirements account
        let (req_pda, _) = Pubkey::find_program_address(
            &[seeds::REQUIREMENTS_SEED, &proof.intent_id],
            program_id,
        );
        if req_pda != *requirements_account.key {
            return Err(EscrowError::InvalidPda.into());
        }

        let mut requirements =
            StoredIntentRequirements::try_from_slice(&requirements_account.data.borrow())
                .map_err(|_| EscrowError::RequirementsNotFound)?;

        if requirements.fulfilled {
            return Err(EscrowError::AlreadyFulfilled.into());
        }

        // Load escrow
        let mut escrow = Escrow::try_from_slice(&escrow_account.data.borrow())?;

        if escrow.intent_id != proof.intent_id {
            return Err(EscrowError::EscrowDoesNotExist.into());
        }
        if escrow.is_claimed {
            return Err(EscrowError::EscrowAlreadyClaimed.into());
        }
        if escrow.amount == 0 {
            return Err(EscrowError::NoDeposit.into());
        }

        // Transfer tokens from vault to solver
        let amount = escrow.amount;
        let escrow_seeds = &[seeds::ESCROW_SEED, &proof.intent_id[..], &[escrow.bump]];

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

        // Update states
        escrow.is_claimed = true;
        escrow.amount = 0;
        escrow.serialize(&mut &mut escrow_account.data.borrow_mut()[..])?;

        requirements.fulfilled = true;
        requirements.serialize(&mut &mut requirements_account.data.borrow_mut()[..])?;

        msg!(
            "Escrow auto-released via fulfillment proof: intent_id={:?}, amount={}",
            &proof.intent_id[..8],
            amount
        );
        Ok(())
    }
}
