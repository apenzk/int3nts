//! Instruction processor for the integrated GMP endpoint program.

use borsh::{BorshDeserialize, BorshSerialize};
#[allow(deprecated)]
use solana_program::system_instruction;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

use crate::error::GmpError;
use crate::instruction::NativeGmpInstruction;
use crate::state::{
    seeds, ConfigAccount, DeliveredMessage, MessageAccount, OutboundNonceAccount, RelayAccount,
    RoutingConfig, RemoteGmpEndpoint,
};

/// Message type constants (matches MVM's gmp_common)
const MESSAGE_TYPE_INTENT_REQUIREMENTS: u8 = 0x01;
const _MESSAGE_TYPE_ESCROW_CONFIRMATION: u8 = 0x02;
const MESSAGE_TYPE_FULFILLMENT_PROOF: u8 = 0x03;

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
        NativeGmpInstruction::SetRemoteGmpEndpointAddr {
            src_chain_id,
            addr,
        } => {
            msg!("Instruction: SetRemoteGmpEndpointAddr");
            process_set_remote_gmp_endpoint_addr(program_id, accounts, src_chain_id, addr)
        }
        NativeGmpInstruction::SetRouting {
            outflow_validator,
            intent_escrow,
        } => {
            msg!("Instruction: SetRouting");
            process_set_routing(program_id, accounts, outflow_validator, intent_escrow)
        }
        NativeGmpInstruction::Send {
            dst_chain_id,
            dst_addr,
            remote_gmp_endpoint_addr,
            payload,
        } => {
            msg!("Instruction: Send");
            process_send(program_id, accounts, dst_chain_id, dst_addr, remote_gmp_endpoint_addr, payload)
        }
        NativeGmpInstruction::DeliverMessage {
            src_chain_id,
            remote_gmp_endpoint_addr,
            payload,
        } => {
            msg!("Instruction: DeliverMessage");
            process_deliver_message(program_id, accounts, src_chain_id, remote_gmp_endpoint_addr, payload)
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

/// Set a remote GMP endpoint address for a source chain.
fn process_set_remote_gmp_endpoint_addr(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    src_chain_id: u32,
    addr: [u8; 32],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let config_account = next_account_info(account_info_iter)?;
    let remote_gmp_endpoint_account = next_account_info(account_info_iter)?;
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

    // Derive remote GMP endpoint PDA
    let chain_id_bytes = src_chain_id.to_le_bytes();
    let (remote_gmp_endpoint_pda, remote_gmp_endpoint_bump) = Pubkey::find_program_address(
        &[seeds::REMOTE_GMP_ENDPOINT_SEED, &chain_id_bytes],
        program_id,
    );

    if remote_gmp_endpoint_account.key != &remote_gmp_endpoint_pda {
        return Err(GmpError::InvalidPda.into());
    }

    // Create or update remote GMP endpoint account
    if remote_gmp_endpoint_account.data_is_empty() {
        let rent = Rent::get()?;
        let space = RemoteGmpEndpoint::SIZE;
        let lamports = rent.minimum_balance(space);

        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                remote_gmp_endpoint_account.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[
                payer.clone(),
                remote_gmp_endpoint_account.clone(),
                system_program.clone(),
            ],
            &[&[
                seeds::REMOTE_GMP_ENDPOINT_SEED,
                &chain_id_bytes,
                &[remote_gmp_endpoint_bump],
            ]],
        )?;

        let remote_gmp_endpoint =
            RemoteGmpEndpoint::new(src_chain_id, addr, remote_gmp_endpoint_bump);
        remote_gmp_endpoint.serialize(&mut &mut remote_gmp_endpoint_account.data.borrow_mut()[..])?;
    } else {
        // Update existing remote GMP endpoint
        let mut remote_gmp_endpoint =
            RemoteGmpEndpoint::try_from_slice(&remote_gmp_endpoint_account.data.borrow())
                .map_err(|_| GmpError::InvalidDiscriminator)?;
        remote_gmp_endpoint.addr = addr;
        remote_gmp_endpoint.serialize(&mut &mut remote_gmp_endpoint_account.data.borrow_mut()[..])?;
    }

    msg!(
        "Remote GMP endpoint set: chain_id={}, addr={}",
        src_chain_id,
        hex_encode(&addr)
    );
    Ok(())
}

/// Set routing configuration for message delivery.
/// Configures which programs handle different message types (like MVM's route_message).
fn process_set_routing(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    outflow_validator: Pubkey,
    intent_escrow: Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let config_account = next_account_info(account_info_iter)?;
    let routing_account = next_account_info(account_info_iter)?;
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

    // Derive routing PDA
    let (routing_pda, routing_bump) =
        Pubkey::find_program_address(&[seeds::ROUTING_SEED], program_id);

    if routing_account.key != &routing_pda {
        return Err(GmpError::InvalidPda.into());
    }

    // Create or update routing account
    if routing_account.data_is_empty() {
        let rent = Rent::get()?;
        let space = RoutingConfig::SIZE;
        let lamports = rent.minimum_balance(space);

        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                routing_account.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[payer.clone(), routing_account.clone(), system_program.clone()],
            &[&[seeds::ROUTING_SEED, &[routing_bump]]],
        )?;

        let routing = RoutingConfig::new(outflow_validator, intent_escrow, routing_bump);
        routing.serialize(&mut &mut routing_account.data.borrow_mut()[..])?;
    } else {
        // Update existing routing config
        let mut routing = RoutingConfig::try_from_slice(&routing_account.data.borrow())
            .map_err(|_| GmpError::InvalidDiscriminator)?;
        routing.outflow_validator = outflow_validator;
        routing.intent_escrow = intent_escrow;
        routing.serialize(&mut &mut routing_account.data.borrow_mut()[..])?;
    }

    msg!(
        "Routing configured: outflow_validator={}, intent_escrow={}",
        outflow_validator,
        intent_escrow
    );
    Ok(())
}

/// Process Send instruction - store message on-chain for relay to read.
///
/// Creates a MessageAccount PDA that the relay reads via getAccountInfo,
/// eliminating the need for getSignaturesForAddress (which is rate-limited
/// on public Solana RPC endpoints).
fn process_send(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    dst_chain_id: u32,
    dst_addr: [u8; 32],
    remote_gmp_endpoint_addr: [u8; 32],
    payload: Vec<u8>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let config_account = next_account_info(account_info_iter)?;
    let nonce_account = next_account_info(account_info_iter)?;
    let sender = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let message_account = next_account_info(account_info_iter)?;

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

    // Create message account PDA to store the outbound message on-chain.
    // Relay reads this via getAccountInfo (not rate-limited) instead of
    // scanning transaction logs via getSignaturesForAddress (rate-limited).
    let nonce_bytes = nonce.to_le_bytes();
    let (message_pda, message_bump) = Pubkey::find_program_address(
        &[seeds::MESSAGE_SEED, &chain_id_bytes, &nonce_bytes],
        program_id,
    );

    if message_account.key != &message_pda {
        return Err(GmpError::InvalidPda.into());
    }

    let message_space = MessageAccount::size(payload.len());
    let rent = Rent::get()?;
    let message_lamports = rent.minimum_balance(message_space);

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            message_account.key,
            message_lamports,
            message_space as u64,
            program_id,
        ),
        &[
            payer.clone(),
            message_account.clone(),
            system_program.clone(),
        ],
        &[&[
            seeds::MESSAGE_SEED,
            &chain_id_bytes,
            &nonce_bytes,
            &[message_bump],
        ]],
    )?;

    let message_data = MessageAccount::new(
        config.chain_id,
        dst_chain_id,
        nonce,
        dst_addr,
        remote_gmp_endpoint_addr,
        payload.clone(),
        message_bump,
    );
    message_data.serialize(&mut &mut message_account.data.borrow_mut()[..])?;

    // Also emit log for backward compatibility / debugging
    let endpoint_addr_pubkey = Pubkey::new_from_array(remote_gmp_endpoint_addr);
    msg!(
        "MessageSent: src_chain_id={}, dst_chain_id={}, remote_gmp_endpoint_addr={}, dst_addr={}, nonce={}, payload_len={}, payload_hex={}",
        config.chain_id,
        dst_chain_id,
        endpoint_addr_pubkey,
        hex_encode(&dst_addr),
        nonce,
        payload.len(),
        hex_encode(&payload)
    );

    Ok(())
}

/// Process DeliverMessage instruction - verify relay and route to destination(s).
///
/// Deduplication uses (intent_id, msg_type) extracted from the payload,
/// making delivery immune to program redeployments (unlike sequential nonces).
///
/// Message routing (similar to MVM's route_message):
/// - IntentRequirements (0x01): Routes to BOTH outflow_validator AND intent_escrow (if configured)
/// - Other message types: Single destination (destination_program account)
///
/// Account layout:
/// 0. Config account (PDA: ["config"])
/// 1. Relay account (PDA: ["relay", relay_pubkey])
/// 2. Remote GMP endpoint account (PDA: ["remote_gmp_endpoint", src_chain_id])
/// 3. Delivered message account (PDA: ["delivered", intent_id, &[msg_type]])
/// 4. Relay signer
/// 5. Payer
/// 6. System program
/// 7. Routing config account (PDA: ["routing"]) - can be any account if routing not configured
/// 8. Destination program 1 (outflow_validator for routing, or single destination)
/// 9. Destination program 2 (intent_escrow for routing, or any account if not routing)
/// 10+. Remaining accounts passed to destination(s)
fn process_deliver_message(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    src_chain_id: u32,
    remote_gmp_endpoint_addr: [u8; 32],
    payload: Vec<u8>,
) -> ProgramResult {
    // Extract intent_id and msg_type from payload for dedup
    // All GMP messages: msg_type (1 byte) + intent_id (32 bytes) at the start
    if payload.len() < 33 {
        return Err(GmpError::InvalidPayload.into());
    }
    let msg_type = payload[0];
    let intent_id = &payload[1..33];

    let account_info_iter = &mut accounts.iter();
    let config_account = next_account_info(account_info_iter)?;
    let relay_account = next_account_info(account_info_iter)?;
    let remote_gmp_endpoint_account = next_account_info(account_info_iter)?;
    let delivered_account = next_account_info(account_info_iter)?;
    let relay_signer = next_account_info(account_info_iter)?;
    let payer = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let routing_account = next_account_info(account_info_iter)?;
    let destination_program_1 = next_account_info(account_info_iter)?;
    let destination_program_2 = next_account_info(account_info_iter)?;

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

    // Verify remote GMP endpoint
    let chain_id_bytes = src_chain_id.to_le_bytes();
    let (remote_gmp_endpoint_pda, _) = Pubkey::find_program_address(
        &[seeds::REMOTE_GMP_ENDPOINT_SEED, &chain_id_bytes],
        program_id,
    );

    if remote_gmp_endpoint_account.key != &remote_gmp_endpoint_pda {
        return Err(GmpError::InvalidPda.into());
    }

    let remote_gmp_endpoint = RemoteGmpEndpoint::try_from_slice(&remote_gmp_endpoint_account.data.borrow())
        .map_err(|_| GmpError::UnknownRemoteGmpEndpoint)?;

    if remote_gmp_endpoint.addr != remote_gmp_endpoint_addr {
        msg!(
            "Unknown remote GMP endpoint: expected={}, got={}",
            hex_encode(&remote_gmp_endpoint.addr),
            hex_encode(&remote_gmp_endpoint_addr)
        );
        return Err(GmpError::UnknownRemoteGmpEndpoint.into());
    }

    // Replay protection: deduplicate by (intent_id, msg_type) via DeliveredMessage PDA
    let (delivered_pda, delivered_bump) = Pubkey::find_program_address(
        &[seeds::DELIVERED_SEED, intent_id, &[msg_type]],
        program_id,
    );

    if delivered_account.key != &delivered_pda {
        return Err(GmpError::InvalidPda.into());
    }

    // If the delivered account already exists, the message was already delivered
    if !delivered_account.data_is_empty() {
        msg!(
            "Already delivered: intent_id={}, msg_type={}",
            hex_encode(intent_id),
            msg_type
        );
        return Err(GmpError::AlreadyDelivered.into());
    }

    // Create the delivered message PDA to mark this message as delivered
    let rent = Rent::get()?;
    let space = DeliveredMessage::SIZE;
    let lamports = rent.minimum_balance(space);

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            delivered_account.key,
            lamports,
            space as u64,
            program_id,
        ),
        &[payer.clone(), delivered_account.clone(), system_program.clone()],
        &[&[seeds::DELIVERED_SEED, intent_id, &[msg_type], &[delivered_bump]]],
    )?;

    let delivered_data = DeliveredMessage::new(delivered_bump);
    delivered_data.serialize(&mut &mut delivered_account.data.borrow_mut()[..])?;

    // Check message type and determine routing
    let message_type = msg_type;

    // Check if routing config exists and is valid
    let (routing_pda, _) = Pubkey::find_program_address(&[seeds::ROUTING_SEED], program_id);
    let routing_config = if routing_account.key == &routing_pda && !routing_account.data_is_empty() {
        RoutingConfig::try_from_slice(&routing_account.data.borrow()).ok()
    } else {
        None
    };

    // Collect remaining accounts for CPI
    let remaining_accounts: Vec<AccountInfo> = account_info_iter.cloned().collect();

    // Build GmpReceive instruction data
    // Format: [variant_index(1 byte)] + [src_chain_id(4)] + [remote_gmp_endpoint_addr(32)] + [payload_len(4)] + [payload]
    let mut gmp_receive_data = Vec::with_capacity(1 + 4 + 32 + 4 + payload.len());
    gmp_receive_data.push(1); // GmpReceive variant index (assuming 0=Initialize, 1=GmpReceive)
    gmp_receive_data.extend_from_slice(&src_chain_id.to_le_bytes());
    gmp_receive_data.extend_from_slice(&remote_gmp_endpoint_addr);
    gmp_receive_data.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    gmp_receive_data.extend_from_slice(&payload);

    // Route based on message type and configuration
    match (message_type, &routing_config) {
        (MESSAGE_TYPE_INTENT_REQUIREMENTS, Some(routing)) if routing.has_outflow_validator() && routing.has_intent_escrow() => {
            // IntentRequirements with routing: deliver to BOTH outflow_validator AND intent_escrow
            // Verify destination programs match routing config
            if destination_program_1.key != &routing.outflow_validator {
                msg!(
                    "Destination program 1 mismatch: expected={}, got={}",
                    routing.outflow_validator,
                    destination_program_1.key
                );
                return Err(GmpError::InvalidPda.into());
            }
            if destination_program_2.key != &routing.intent_escrow {
                msg!(
                    "Destination program 2 mismatch: expected={}, got={}",
                    routing.intent_escrow,
                    destination_program_2.key
                );
                return Err(GmpError::InvalidPda.into());
            }

            msg!(
                "MessageDelivered (routed): src_chain_id={}, remote_gmp_endpoint_addr={}, intent_id={}, payload_len={}, dest1={}, dest2={}",
                src_chain_id,
                hex_encode(&remote_gmp_endpoint_addr),
                hex_encode(intent_id),
                payload.len(),
                destination_program_1.key,
                destination_program_2.key
            );

            // Remaining accounts layout (set up by relay):
            // Indices 0-4: outflow_validator's GmpReceive accounts (requirements, config, authority, payer, system)
            // Indices 5-9: intent_escrow's GmpReceive accounts (requirements, gmp_config, authority, payer, system)
            if remaining_accounts.len() < 10 {
                msg!("Insufficient remaining accounts for multi-destination routing: need 10, got {}", remaining_accounts.len());
                return Err(GmpError::InvalidAccountCount.into());
            }

            // CPI to outflow_validator (destination_program_1) with its accounts (indices 0-4)
            let outflow_accounts = &remaining_accounts[0..5];
            msg!("Routing to outflow_validator: {} with {} accounts", destination_program_1.key, outflow_accounts.len());
            invoke_gmp_receive(destination_program_1.key, &gmp_receive_data, outflow_accounts)?;

            // CPI to intent_escrow (destination_program_2) with its accounts (indices 5-9)
            let escrow_accounts = &remaining_accounts[5..10];
            msg!("Routing to intent_escrow: {} with {} accounts", destination_program_2.key, escrow_accounts.len());
            invoke_gmp_receive(destination_program_2.key, &gmp_receive_data, escrow_accounts)?;

            msg!("Multi-destination routing succeeded");
        }
        (MESSAGE_TYPE_FULFILLMENT_PROOF, Some(routing)) if routing.has_intent_escrow() => {
            // FulfillmentProof with routing: deliver to intent_escrow only (not outflow_validator)
            // Verify destination_program_2 matches routing config
            if destination_program_2.key != &routing.intent_escrow {
                msg!(
                    "Destination program 2 mismatch for FulfillmentProof: expected={}, got={}",
                    routing.intent_escrow,
                    destination_program_2.key
                );
                return Err(GmpError::InvalidPda.into());
            }

            msg!(
                "MessageDelivered (FulfillmentProof to escrow): src_chain_id={}, remote_gmp_endpoint_addr={}, intent_id={}, payload_len={}, dest={}",
                src_chain_id,
                hex_encode(&remote_gmp_endpoint_addr),
                hex_encode(intent_id),
                payload.len(),
                destination_program_2.key
            );

            // Remaining accounts are for intent_escrow's GmpReceiveFulfillmentProof:
            // requirements(w), escrow(w), vault(w), solver_token(w), gmp_config(r), gmp_caller(s), token_program
            if remaining_accounts.len() < 7 {
                msg!("Insufficient remaining accounts for FulfillmentProof routing: need 7, got {}", remaining_accounts.len());
                return Err(GmpError::InvalidAccountCount.into());
            }

            // CPI to intent_escrow (destination_program_2) with all remaining accounts
            msg!("Routing FulfillmentProof to intent_escrow: {} with {} accounts", destination_program_2.key, remaining_accounts.len());
            invoke_gmp_receive(destination_program_2.key, &gmp_receive_data, &remaining_accounts)?;

            msg!("FulfillmentProof routing to intent_escrow succeeded");
        }
        _ => {
            // Single destination: use destination_program_1 account
            msg!(
                "MessageDelivered: src_chain_id={}, remote_gmp_endpoint_addr={}, intent_id={}, payload_len={}, destination={}",
                src_chain_id,
                hex_encode(&remote_gmp_endpoint_addr),
                hex_encode(intent_id),
                payload.len(),
                destination_program_1.key
            );

            // Pass remaining_accounts directly - destination program is invoked, not passed as account
            invoke_gmp_receive(destination_program_1.key, &gmp_receive_data, &remaining_accounts)?;

            msg!("CPI to destination program succeeded");
        }
    }

    Ok(())
}

/// Helper to invoke GmpReceive on a destination program.
fn invoke_gmp_receive(
    program_id: &Pubkey,
    gmp_receive_data: &[u8],
    accounts: &[AccountInfo],
) -> ProgramResult {
    // Build account metas for CPI
    let mut account_metas = Vec::with_capacity(accounts.len());
    for acc in accounts {
        if acc.is_writable {
            account_metas.push(solana_program::instruction::AccountMeta::new(*acc.key, acc.is_signer));
        } else {
            account_metas.push(solana_program::instruction::AccountMeta::new_readonly(*acc.key, acc.is_signer));
        }
    }

    let cpi_instruction = solana_program::instruction::Instruction {
        program_id: *program_id,
        accounts: account_metas,
        data: gmp_receive_data.to_vec(),
    };

    // Invoke the destination program
    solana_program::program::invoke(&cpi_instruction, accounts)
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
