//! Instruction processor for the native GMP endpoint program.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::error::GmpError;
use crate::instruction::NativeGmpInstruction;
use crate::state::{
    seeds, ConfigAccount, InboundNonceAccount, OutboundNonceAccount, RelayAccount,
    TrustedRemoteAccount,
};

/// Program entrypoint processor.
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = NativeGmpInstruction::try_from_slice(instruction_data)
        .map_err(|_| GmpError::InvalidInstructionData)?;

    match instruction {
        NativeGmpInstruction::Initialize { chain_id } => {
            msg!("Instruction: Initialize");
            process_initialize(program_id, accounts, chain_id)
        }
        NativeGmpInstruction::AddRelay { relay } => {
            msg!("Instruction: AddRelay");
            process_add_relay(program_id, accounts, relay)
        }
        NativeGmpInstruction::RemoveRelay { relay } => {
            msg!("Instruction: RemoveRelay");
            process_remove_relay(program_id, accounts, relay)
        }
        NativeGmpInstruction::SetTrustedRemote {
            src_chain_id,
            trusted_addr,
        } => {
            msg!("Instruction: SetTrustedRemote");
            process_set_trusted_remote(program_id, accounts, src_chain_id, trusted_addr)
        }
        NativeGmpInstruction::Send {
            dst_chain_id,
            dst_addr,
            payload,
        } => {
            msg!("Instruction: Send");
            process_send(program_id, accounts, dst_chain_id, dst_addr, payload)
        }
        NativeGmpInstruction::DeliverMessage {
            src_chain_id,
            src_addr,
            payload,
            nonce,
        } => {
            msg!("Instruction: DeliverMessage");
            process_deliver_message(program_id, accounts, src_chain_id, src_addr, payload, nonce)
        }
    }
}

/// Initialize the GMP endpoint configuration.
fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    chain_id: u32,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let config_account = next_account_info(account_info_iter)?;
    let admin = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Verify admin is signer
    if !admin.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Derive config PDA
    let (config_pda, config_bump) =
        Pubkey::find_program_address(&[seeds::CONFIG_SEED], program_id);

    if config_account.key != &config_pda {
        return Err(GmpError::InvalidPda.into());
    }

    // Check if already initialized
    if !config_account.data_is_empty() {
        return Err(GmpError::AccountAlreadyInitialized.into());
    }

    // Create config account
    let rent = Rent::get()?;
    let space = ConfigAccount::SIZE;
    let lamports = rent.minimum_balance(space);

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            config_account.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[payer.clone(), config_account.clone(), system_program.clone()],
        &[&[seeds::CONFIG_SEED, &[config_bump]]],
    )?;

    // Initialize config data
    let config = ConfigAccount::new(*admin.key, chain_id, config_bump);
    config.serialize(&mut &mut config_account.data.borrow_mut()[..])?;

    msg!("GMP endpoint initialized: chain_id={}, admin={}", chain_id, admin.key);
    Ok(())
}

/// Add an authorized relay.
fn process_add_relay(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    relay: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let config_account = next_account_info(account_info_iter)?;
    let relay_account = next_account_info(account_info_iter)?;
    let admin = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Verify admin is signer
    if !admin.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify config
    let config = ConfigAccount::try_from_slice(&config_account.data.borrow())
        .map_err(|_| GmpError::AccountNotInitialized)?;

    if config.admin != *admin.key {
        return Err(GmpError::UnauthorizedAdmin.into());
    }

    // Derive relay PDA
    let (relay_pda, relay_bump) =
        Pubkey::find_program_address(&[seeds::RELAY_SEED, relay.as_ref()], program_id);

    if relay_account.key != &relay_pda {
        return Err(GmpError::InvalidPda.into());
    }

    // Create or update relay account
    if relay_account.data_is_empty() {
        let rent = Rent::get()?;
        let space = RelayAccount::SIZE;
        let lamports = rent.minimum_balance(space);

        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                relay_account.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[payer.clone(), relay_account.clone(), system_program.clone()],
            &[&[seeds::RELAY_SEED, relay.as_ref(), &[relay_bump]]],
        )?;

        let relay_data = RelayAccount::new(relay, relay_bump);
        relay_data.serialize(&mut &mut relay_account.data.borrow_mut()[..])?;
    } else {
        // Re-authorize existing relay
        let mut relay_data = RelayAccount::try_from_slice(&relay_account.data.borrow())
            .map_err(|_| GmpError::InvalidDiscriminator)?;
        relay_data.is_authorized = true;
        relay_data.serialize(&mut &mut relay_account.data.borrow_mut()[..])?;
    }

    msg!("Relay authorized: {}", relay);
    Ok(())
}

/// Remove an authorized relay.
fn process_remove_relay(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    relay: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let config_account = next_account_info(account_info_iter)?;
    let relay_account = next_account_info(account_info_iter)?;
    let admin = next_account_info(account_info_iter)?;

    // Verify admin is signer
    if !admin.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify config
    let config = ConfigAccount::try_from_slice(&config_account.data.borrow())
        .map_err(|_| GmpError::AccountNotInitialized)?;

    if config.admin != *admin.key {
        return Err(GmpError::UnauthorizedAdmin.into());
    }

    // Derive relay PDA
    let (relay_pda, _) =
        Pubkey::find_program_address(&[seeds::RELAY_SEED, relay.as_ref()], program_id);

    if relay_account.key != &relay_pda {
        return Err(GmpError::InvalidPda.into());
    }

    // Deauthorize relay
    let mut relay_data = RelayAccount::try_from_slice(&relay_account.data.borrow())
        .map_err(|_| GmpError::AccountNotInitialized)?;
    relay_data.is_authorized = false;
    relay_data.serialize(&mut &mut relay_account.data.borrow_mut()[..])?;

    msg!("Relay deauthorized: {}", relay);
    Ok(())
}

/// Set a trusted remote address for a source chain.
fn process_set_trusted_remote(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    src_chain_id: u32,
    trusted_addr: [u8; 32],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let config_account = next_account_info(account_info_iter)?;
    let trusted_remote_account = next_account_info(account_info_iter)?;
    let admin = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Verify admin is signer
    if !admin.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load and verify config
    let config = ConfigAccount::try_from_slice(&config_account.data.borrow())
        .map_err(|_| GmpError::AccountNotInitialized)?;

    if config.admin != *admin.key {
        return Err(GmpError::UnauthorizedAdmin.into());
    }

    // Derive trusted remote PDA
    let chain_id_bytes = src_chain_id.to_le_bytes();
    let (trusted_remote_pda, trusted_remote_bump) = Pubkey::find_program_address(
        &[seeds::TRUSTED_REMOTE_SEED, &chain_id_bytes],
        program_id,
    );

    if trusted_remote_account.key != &trusted_remote_pda {
        return Err(GmpError::InvalidPda.into());
    }

    // Create or update trusted remote account
    if trusted_remote_account.data_is_empty() {
        let rent = Rent::get()?;
        let space = TrustedRemoteAccount::SIZE;
        let lamports = rent.minimum_balance(space);

        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                trusted_remote_account.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[
                payer.clone(),
                trusted_remote_account.clone(),
                system_program.clone(),
            ],
            &[&[
                seeds::TRUSTED_REMOTE_SEED,
                &chain_id_bytes,
                &[trusted_remote_bump],
            ]],
        )?;

        let trusted_remote =
            TrustedRemoteAccount::new(src_chain_id, trusted_addr, trusted_remote_bump);
        trusted_remote.serialize(&mut &mut trusted_remote_account.data.borrow_mut()[..])?;
    } else {
        // Update existing trusted remote
        let mut trusted_remote =
            TrustedRemoteAccount::try_from_slice(&trusted_remote_account.data.borrow())
                .map_err(|_| GmpError::InvalidDiscriminator)?;
        trusted_remote.trusted_addr = trusted_addr;
        trusted_remote.serialize(&mut &mut trusted_remote_account.data.borrow_mut()[..])?;
    }

    msg!(
        "Trusted remote set: chain_id={}, addr={}",
        src_chain_id,
        hex_encode(&trusted_addr)
    );
    Ok(())
}

/// Process Send instruction - emit event for relay to pick up.
fn process_send(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    dst_chain_id: u32,
    dst_addr: [u8; 32],
    payload: Vec<u8>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let config_account = next_account_info(account_info_iter)?;
    let nonce_account = next_account_info(account_info_iter)?;
    let sender = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Verify sender is signer
    if !sender.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Load config to get this chain's ID
    let config = ConfigAccount::try_from_slice(&config_account.data.borrow())
        .map_err(|_| GmpError::AccountNotInitialized)?;

    // Derive nonce PDA
    let chain_id_bytes = dst_chain_id.to_le_bytes();
    let (nonce_pda, nonce_bump) =
        Pubkey::find_program_address(&[seeds::NONCE_OUT_SEED, &chain_id_bytes], program_id);

    if nonce_account.key != &nonce_pda {
        return Err(GmpError::InvalidPda.into());
    }

    // Create nonce account if needed, then increment
    let nonce = if nonce_account.data_is_empty() {
        let rent = Rent::get()?;
        let space = OutboundNonceAccount::SIZE;
        let lamports = rent.minimum_balance(space);

        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                nonce_account.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[payer.clone(), nonce_account.clone(), system_program.clone()],
            &[&[seeds::NONCE_OUT_SEED, &chain_id_bytes, &[nonce_bump]]],
        )?;

        let mut nonce_data = OutboundNonceAccount::new(dst_chain_id, nonce_bump);
        let nonce = nonce_data.increment();
        nonce_data.serialize(&mut &mut nonce_account.data.borrow_mut()[..])?;
        nonce
    } else {
        let mut nonce_data = OutboundNonceAccount::try_from_slice(&nonce_account.data.borrow())
            .map_err(|_| GmpError::InvalidDiscriminator)?;
        let nonce = nonce_data.increment();
        nonce_data.serialize(&mut &mut nonce_account.data.borrow_mut()[..])?;
        nonce
    };

    // Emit MessageSent event for GMP relay
    // Format: structured for easy parsing by the relay
    msg!(
        "MessageSent: src_chain_id={}, dst_chain_id={}, src_addr={}, dst_addr={}, nonce={}, payload_len={}, payload_hex={}",
        config.chain_id,
        dst_chain_id,
        sender.key,
        hex_encode(&dst_addr),
        nonce,
        payload.len(),
        hex_encode(&payload)
    );

    Ok(())
}

/// Process DeliverMessage instruction - verify relay and CPI to destination.
fn process_deliver_message(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    src_chain_id: u32,
    src_addr: [u8; 32],
    payload: Vec<u8>,
    nonce: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let config_account = next_account_info(account_info_iter)?;
    let relay_account = next_account_info(account_info_iter)?;
    let trusted_remote_account = next_account_info(account_info_iter)?;
    let nonce_account = next_account_info(account_info_iter)?;
    let relay_signer = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;
    let destination_program = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Verify relay is signer
    if !relay_signer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify config account
    let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], program_id);
    if config_account.key != &config_pda {
        return Err(GmpError::InvalidPda.into());
    }

    // Load config
    let _config = ConfigAccount::try_from_slice(&config_account.data.borrow())
        .map_err(|_| GmpError::AccountNotInitialized)?;

    // Verify relay is authorized
    let (relay_pda, _) =
        Pubkey::find_program_address(&[seeds::RELAY_SEED, relay_signer.key.as_ref()], program_id);

    if relay_account.key != &relay_pda {
        return Err(GmpError::InvalidPda.into());
    }

    let relay_data = RelayAccount::try_from_slice(&relay_account.data.borrow())
        .map_err(|_| GmpError::AccountNotInitialized)?;

    if !relay_data.is_authorized {
        return Err(GmpError::UnauthorizedRelay.into());
    }

    // Verify trusted remote
    let chain_id_bytes = src_chain_id.to_le_bytes();
    let (trusted_remote_pda, _) = Pubkey::find_program_address(
        &[seeds::TRUSTED_REMOTE_SEED, &chain_id_bytes],
        program_id,
    );

    if trusted_remote_account.key != &trusted_remote_pda {
        return Err(GmpError::InvalidPda.into());
    }

    let trusted_remote = TrustedRemoteAccount::try_from_slice(&trusted_remote_account.data.borrow())
        .map_err(|_| GmpError::UntrustedRemote)?;

    if trusted_remote.trusted_addr != src_addr {
        msg!(
            "Untrusted source: expected={}, got={}",
            hex_encode(&trusted_remote.trusted_addr),
            hex_encode(&src_addr)
        );
        return Err(GmpError::UntrustedRemote.into());
    }

    // Check nonce for replay protection
    let (nonce_pda, nonce_bump) =
        Pubkey::find_program_address(&[seeds::NONCE_IN_SEED, &chain_id_bytes], program_id);

    if nonce_account.key != &nonce_pda {
        return Err(GmpError::InvalidPda.into());
    }

    // Create or check nonce account
    if nonce_account.data_is_empty() {
        // First message from this chain - create nonce account
        let rent = Rent::get()?;
        let space = InboundNonceAccount::SIZE;
        let lamports = rent.minimum_balance(space);

        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                nonce_account.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[payer.clone(), nonce_account.clone(), system_program.clone()],
            &[&[seeds::NONCE_IN_SEED, &chain_id_bytes, &[nonce_bump]]],
        )?;

        let mut nonce_data = InboundNonceAccount::new(src_chain_id, nonce_bump);
        nonce_data.update_nonce(nonce);
        nonce_data.serialize(&mut &mut nonce_account.data.borrow_mut()[..])?;
    } else {
        let mut nonce_data = InboundNonceAccount::try_from_slice(&nonce_account.data.borrow())
            .map_err(|_| GmpError::InvalidDiscriminator)?;

        if nonce_data.is_replay(nonce) {
            msg!(
                "Replay detected: nonce={}, last_nonce={}",
                nonce,
                nonce_data.last_nonce
            );
            return Err(GmpError::ReplayDetected.into());
        }

        nonce_data.update_nonce(nonce);
        nonce_data.serialize(&mut &mut nonce_account.data.borrow_mut()[..])?;
    }

    // Log the delivery
    msg!(
        "MessageDelivered: src_chain_id={}, src_addr={}, nonce={}, payload_len={}, destination={}",
        src_chain_id,
        hex_encode(&src_addr),
        nonce,
        payload.len(),
        destination_program.key
    );

    // CPI to destination program's lz_receive instruction
    // The destination program expects an LzReceive instruction with:
    // - src_chain_id, src_addr, payload
    //
    // We construct the instruction data for the destination's LzReceive variant.
    // This assumes the destination uses a compatible instruction format.
    //
    // The remaining accounts (after system_program) are passed to the destination.
    let remaining_accounts: Vec<AccountInfo> = account_info_iter.cloned().collect();

    // Build LzReceive instruction data for destination
    // Format: [variant_index(1 byte)] + [src_chain_id(4)] + [src_addr(32)] + [payload_len(4)] + [payload]
    let mut lz_receive_data = Vec::with_capacity(1 + 4 + 32 + 4 + payload.len());
    lz_receive_data.push(1); // LzReceive variant index (assuming 0=Initialize, 1=LzReceive)
    lz_receive_data.extend_from_slice(&src_chain_id.to_le_bytes());
    lz_receive_data.extend_from_slice(&src_addr);
    lz_receive_data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    lz_receive_data.extend_from_slice(&payload);

    // Build account metas for CPI
    let mut account_metas = Vec::with_capacity(remaining_accounts.len());
    for acc in &remaining_accounts {
        if acc.is_writable {
            account_metas.push(solana_program::instruction::AccountMeta::new(*acc.key, acc.is_signer));
        } else {
            account_metas.push(solana_program::instruction::AccountMeta::new_readonly(*acc.key, acc.is_signer));
        }
    }

    let cpi_instruction = solana_program::instruction::Instruction {
        program_id: *destination_program.key,
        accounts: account_metas,
        data: lz_receive_data,
    };

    // Invoke the destination program
    // Note: We don't use invoke_signed here because we're not signing as a PDA
    solana_program::program::invoke(&cpi_instruction, &remaining_accounts)?;

    msg!("CPI to destination program succeeded");
    Ok(())
}

/// Simple hex encoding for logging (no dependencies).
fn hex_encode(bytes: &[u8]) -> String {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(HEX_CHARS[(byte >> 4) as usize] as char);
        result.push(HEX_CHARS[(byte & 0x0f) as usize] as char);
    }
    result
}
