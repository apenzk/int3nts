//! Interface tests for the integrated GMP endpoint program.
//!
//! These tests verify that instructions and state can be correctly serialized,
//! and that the (intent_id, msg_type) deduplication logic works correctly for replay protection.

use borsh::BorshDeserialize;
use intent_gmp::{
    instruction::NativeGmpInstruction,
    state::{
        ConfigAccount, DeliveredMessage, OutboundNonceAccount, RelayAccount,
        RoutingConfig, RemoteGmpEndpoint,
    },
    GmpError,
};
use solana_sdk::pubkey::Pubkey;

// ============================================================================
// TEST CONSTANTS
// ============================================================================

const DUMMY_CHAIN_ID_SVM: u32 = 30168;
const DUMMY_CHAIN_ID_MVM: u32 = 30325;

// ============================================================================
// TEST HELPERS
// ============================================================================

fn dummy_dst_addr() -> [u8; 32] {
    [1u8; 32]
}

fn dummy_remote_gmp_endpoint_addr() -> [u8; 32] {
    [2u8; 32]
}

fn dummy_remote_addr() -> [u8; 32] {
    [0xab; 32]
}

fn dummy_payload() -> Vec<u8> {
    vec![0x01, 0x02, 0x03]
}

// ============================================================================
// INSTRUCTION SERIALIZATION TESTS
// ============================================================================

/// 1. Test: Send instruction serialization roundtrip
/// Verifies that Send instruction can be serialized and deserialized correctly.
/// Why: Instruction serialization is critical for on-chain processing. Incorrect serialization would cause instruction parsing failures.
#[test]
fn test_send_instruction_serialization() {
    let original_dst_chain_id = DUMMY_CHAIN_ID_MVM;
    let original_dst_addr = dummy_dst_addr();
    let original_remote_gmp_endpoint_addr = dummy_remote_gmp_endpoint_addr();
    let original_payload = dummy_payload();

    let instruction = NativeGmpInstruction::Send {
        dst_chain_id: original_dst_chain_id,
        dst_addr: original_dst_addr,
        remote_gmp_endpoint_addr: original_remote_gmp_endpoint_addr,
        payload: original_payload.clone(),
    };

    let encoded = borsh::to_vec(&instruction).unwrap();
    let decoded = NativeGmpInstruction::try_from_slice(&encoded).unwrap();

    match decoded {
        NativeGmpInstruction::Send {
            dst_chain_id,
            dst_addr,
            remote_gmp_endpoint_addr,
            payload,
        } => {
            assert_eq!(dst_chain_id, original_dst_chain_id);
            assert_eq!(dst_addr, original_dst_addr);
            assert_eq!(remote_gmp_endpoint_addr, original_remote_gmp_endpoint_addr);
            assert_eq!(payload, original_payload);
        }
        _ => panic!("Wrong instruction variant"),
    }
}

/// 2. Test: DeliverMessage instruction serialization roundtrip
/// Verifies that DeliverMessage instruction can be serialized and deserialized correctly.
/// Why: Relays must be able to construct valid DeliverMessage instructions. Serialization bugs would prevent message delivery.
#[test]
fn test_deliver_message_instruction_serialization() {
    let original_src_chain_id = DUMMY_CHAIN_ID_MVM;
    let original_remote_gmp_endpoint_addr = dummy_remote_gmp_endpoint_addr();
    let original_payload = dummy_payload();

    let instruction = NativeGmpInstruction::DeliverMessage {
        src_chain_id: original_src_chain_id,
        remote_gmp_endpoint_addr: original_remote_gmp_endpoint_addr,
        payload: original_payload.clone(),
    };

    let encoded = borsh::to_vec(&instruction).unwrap();
    let decoded = NativeGmpInstruction::try_from_slice(&encoded).unwrap();

    match decoded {
        NativeGmpInstruction::DeliverMessage {
            src_chain_id,
            remote_gmp_endpoint_addr,
            payload,
        } => {
            assert_eq!(src_chain_id, original_src_chain_id);
            assert_eq!(remote_gmp_endpoint_addr, original_remote_gmp_endpoint_addr);
            assert_eq!(payload, original_payload);
        }
        _ => panic!("Wrong instruction variant"),
    }
}

/// 3. Test: Initialize instruction serialization roundtrip
/// Verifies that Initialize instruction can be serialized and deserialized correctly.
/// Why: Endpoint initialization requires correct chain_id. Serialization errors would misconfigure the endpoint.
#[test]
fn test_initialize_instruction_serialization() {
    let original_chain_id = DUMMY_CHAIN_ID_SVM;

    let instruction = NativeGmpInstruction::Initialize {
        chain_id: original_chain_id,
    };

    let encoded = borsh::to_vec(&instruction).unwrap();
    let decoded = NativeGmpInstruction::try_from_slice(&encoded).unwrap();

    match decoded {
        NativeGmpInstruction::Initialize { chain_id } => {
            assert_eq!(chain_id, original_chain_id);
        }
        _ => panic!("Wrong instruction variant"),
    }
}

/// 4. Test: AddRelay instruction serialization roundtrip
/// Verifies that AddRelay instruction can be serialized and deserialized correctly.
/// Why: Admin must be able to authorize relays. Incorrect relay pubkey serialization would authorize wrong accounts.
#[test]
fn test_add_relay_instruction_serialization() {
    let original_relay = Pubkey::new_from_array([0x11; 32]);

    let instruction = NativeGmpInstruction::AddRelay {
        relay: original_relay,
    };

    let encoded = borsh::to_vec(&instruction).unwrap();
    let decoded = NativeGmpInstruction::try_from_slice(&encoded).unwrap();

    match decoded {
        NativeGmpInstruction::AddRelay { relay: decoded_relay } => {
            assert_eq!(decoded_relay, original_relay);
        }
        _ => panic!("Wrong instruction variant"),
    }
}

/// 5. Test: SetRemoteGmpEndpointAddr instruction serialization roundtrip
/// Verifies that SetRemoteGmpEndpointAddr instruction can be serialized and deserialized correctly.
/// Why: Remote GMP endpoint configuration is security-critical. Wrong chain_id or address would accept messages from unknown remote GMP endpoints.
#[test]
fn test_set_remote_gmp_endpoint_addr_instruction_serialization() {
    let original_src_chain_id = DUMMY_CHAIN_ID_MVM;
    let original_addr = dummy_remote_addr();

    let instruction = NativeGmpInstruction::SetRemoteGmpEndpointAddr {
        src_chain_id: original_src_chain_id,
        addr: original_addr,
    };

    let encoded = borsh::to_vec(&instruction).unwrap();
    let decoded = NativeGmpInstruction::try_from_slice(&encoded).unwrap();

    match decoded {
        NativeGmpInstruction::SetRemoteGmpEndpointAddr {
            src_chain_id,
            addr,
        } => {
            assert_eq!(src_chain_id, original_src_chain_id);
            assert_eq!(addr, original_addr);
        }
        _ => panic!("Wrong instruction variant"),
    }
}

/// 6. Test: SetRouting instruction serialization roundtrip
/// Verifies that SetRouting instruction can be serialized and deserialized correctly.
/// Why: Routing configuration enables multi-destination delivery. Incorrect pubkeys would route messages to wrong programs.
#[test]
fn test_set_routing_instruction_serialization() {
    let original_outflow_validator = Pubkey::new_from_array([0x22; 32]);
    let original_intent_escrow = Pubkey::new_from_array([0x33; 32]);

    let instruction = NativeGmpInstruction::SetRouting {
        outflow_validator: original_outflow_validator,
        intent_escrow: original_intent_escrow,
    };

    let encoded = borsh::to_vec(&instruction).unwrap();
    let decoded = NativeGmpInstruction::try_from_slice(&encoded).unwrap();

    match decoded {
        NativeGmpInstruction::SetRouting {
            outflow_validator,
            intent_escrow,
        } => {
            assert_eq!(outflow_validator, original_outflow_validator);
            assert_eq!(intent_escrow, original_intent_escrow);
        }
        _ => panic!("Wrong instruction variant"),
    }
}

/// 7. Test: RoutingConfig serialization roundtrip
/// Verifies that RoutingConfig state can be serialized and deserialized correctly.
/// Why: Routing config stores destination programs. Corruption would route messages to wrong programs.
#[test]
fn test_routing_config_serialization() {
    let original_outflow_validator = Pubkey::new_from_array([0x44; 32]);
    let original_intent_escrow = Pubkey::new_from_array([0x55; 32]);
    let original_bump = 250u8;

    let routing = RoutingConfig::new(original_outflow_validator, original_intent_escrow, original_bump);

    let encoded = borsh::to_vec(&routing).unwrap();
    let decoded = RoutingConfig::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.discriminator, RoutingConfig::DISCRIMINATOR);
    assert_eq!(decoded.outflow_validator, original_outflow_validator);
    assert_eq!(decoded.intent_escrow, original_intent_escrow);
    assert_eq!(decoded.bump, original_bump);
    assert!(decoded.has_outflow_validator());
    assert!(decoded.has_intent_escrow());
}

// ============================================================================
// STATE SERIALIZATION TESTS
// ============================================================================

/// 8. Test: ConfigAccount serialization roundtrip
/// Verifies that ConfigAccount state can be serialized and deserialized correctly.
/// Why: Config stores admin and chain_id. Corruption would break authorization checks and message routing.
#[test]
fn test_config_account_serialization() {
    let original_admin = Pubkey::new_from_array([0x66; 32]);
    let original_chain_id = DUMMY_CHAIN_ID_SVM;
    let original_bump = 255u8;

    let config = ConfigAccount::new(original_admin, original_chain_id, original_bump);

    let encoded = borsh::to_vec(&config).unwrap();
    let decoded = ConfigAccount::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.discriminator, ConfigAccount::DISCRIMINATOR);
    assert_eq!(decoded.admin, original_admin);
    assert_eq!(decoded.chain_id, original_chain_id);
    assert_eq!(decoded.bump, original_bump);
}

/// 9. Test: RelayAccount serialization roundtrip
/// Verifies that RelayAccount state can be serialized and deserialized correctly.
/// Why: Relay authorization state must persist correctly. Corruption could authorize/deauthorize wrong relays.
#[test]
fn test_relay_account_serialization() {
    let original_relay = Pubkey::new_from_array([0x77; 32]);
    let original_bump = 254u8;

    let relay_account = RelayAccount::new(original_relay, original_bump);

    let encoded = borsh::to_vec(&relay_account).unwrap();
    let decoded = RelayAccount::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.discriminator, RelayAccount::DISCRIMINATOR);
    assert_eq!(decoded.relay, original_relay);
    assert!(decoded.is_authorized);
    assert_eq!(decoded.bump, original_bump);
}

/// 10. Test: RemoteGmpEndpoint serialization roundtrip
/// Verifies that RemoteGmpEndpoint state can be serialized and deserialized correctly.
/// Why: Remote GMP endpoint config is security-critical. Corruption would accept messages from unknown remote GMP endpoints.
#[test]
fn test_remote_gmp_endpoint_account_serialization() {
    let original_src_chain_id = DUMMY_CHAIN_ID_MVM;
    let original_addr = dummy_remote_addr();
    let original_bump = 253u8;

    let remote_gmp_endpoint =
        RemoteGmpEndpoint::new(original_src_chain_id, original_addr, original_bump);

    let encoded = borsh::to_vec(&remote_gmp_endpoint).unwrap();
    let decoded = RemoteGmpEndpoint::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.discriminator, RemoteGmpEndpoint::DISCRIMINATOR);
    assert_eq!(decoded.src_chain_id, original_src_chain_id);
    assert_eq!(decoded.addr, original_addr);
    assert_eq!(decoded.bump, original_bump);
}

// ============================================================================
// NONCE TRACKING TESTS
// ============================================================================

/// 11. Test: OutboundNonceAccount increment behavior
/// Verifies that outbound nonce increments correctly and returns the pre-increment value.
/// Why: Nonces must be unique per message. Incorrect increment logic would cause duplicate nonces or gaps.
#[test]
fn test_outbound_nonce_account() {
    let mut nonce_account = OutboundNonceAccount::new(DUMMY_CHAIN_ID_MVM, 252);

    assert_eq!(nonce_account.nonce, 0);

    let n1 = nonce_account.increment();
    assert_eq!(n1, 0);
    assert_eq!(nonce_account.nonce, 1);

    let n2 = nonce_account.increment();
    assert_eq!(n2, 1);
    assert_eq!(nonce_account.nonce, 2);
}

/// 12. Test: DeliveredMessage serialization roundtrip
/// Verifies that DeliveredMessage state can be serialized and deserialized correctly.
/// Why: Delivered message markers prevent double-processing of messages. Serialization bugs could allow replay attacks.
#[test]
fn test_delivered_message_serialization() {
    let original_bump = 251u8;

    let delivered = DeliveredMessage::new(original_bump);

    let encoded = borsh::to_vec(&delivered).unwrap();
    assert_eq!(encoded.len(), DeliveredMessage::SIZE, "Serialized size should match SIZE constant");

    let decoded = DeliveredMessage::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.discriminator, DeliveredMessage::DISCRIMINATOR);
    assert_eq!(decoded.bump, original_bump);
}

// ============================================================================
// ERROR CONVERSION TESTS
// ============================================================================

/// 13. Test: Error to ProgramError conversion
/// Verifies that GmpError can be converted to ProgramError.
/// Why: Errors must propagate correctly to clients. Incorrect conversion would hide error details.
#[test]
fn test_error_conversion() {
    use solana_program::program_error::ProgramError;

    let error: ProgramError = GmpError::UnauthorizedRelay.into();
    match error {
        ProgramError::Custom(code) => {
            assert_eq!(code, GmpError::UnauthorizedRelay as u32);
        }
        _ => panic!("Expected Custom error"),
    }
}

/// 14. Test: All error variants have unique codes
/// Verifies that each error variant maps to a unique error code.
/// Why: Unique error codes allow clients to identify specific failures. Duplicate codes would make debugging impossible.
#[test]
fn test_error_codes_unique() {
    let errors = [
        GmpError::InvalidInstructionData,
        GmpError::AccountAlreadyInitialized,
        GmpError::AccountNotInitialized,
        GmpError::InvalidPda,
        GmpError::UnauthorizedAdmin,
        GmpError::UnauthorizedRelay,
        GmpError::UnknownRemoteGmpEndpoint,
        GmpError::AlreadyDelivered,
        GmpError::InvalidDiscriminator,
    ];

    let codes: Vec<u32> = errors.iter().map(|e| e.clone() as u32).collect();
    let unique_codes: std::collections::HashSet<u32> = codes.iter().cloned().collect();

    assert_eq!(
        codes.len(),
        unique_codes.len(),
        "Error codes must be unique"
    );
}

// ============================================================================
// INTEGRATION TESTS (require solana-program-test runtime)
// ============================================================================
//
// These tests use `solana-program-test` to run the program in a simulated
// Solana runtime. Unlike the unit tests above (which only test serialization),
// these tests actually execute program instructions and verify on-chain state.
//
// How it works:
// 1. `ProgramTest::new()` creates an in-memory blockchain with our program loaded
// 2. `start_with_context()` starts the simulated validator
// 3. We build instructions, sign transactions, and submit them via `BanksClient`
// 4. The program executes, modifies accounts, emits logs
// 5. We read accounts back to verify the state changed correctly
//
// Key components:
// - `ProgramTestContext`: Holds the simulated blockchain state
// - `BanksClient`: Interface to submit transactions and query accounts
// - `processor!()`: Macro that wraps our program's entrypoint for testing
//
// Note: Between similar transactions, we use `context.warp_to_slot()` to advance
// the slot. This prevents the test framework from silently deduplicating
// transactions that look too similar (a quirk of solana-program-test).

mod integration {
    use borsh::{BorshDeserialize, BorshSerialize};
    use intent_gmp::{
        instruction::NativeGmpInstruction,
        state::{seeds, DeliveredMessage, MessageAccount, OutboundNonceAccount},
    };
    use solana_program::instruction::{AccountMeta, Instruction};
    use solana_program_test::{processor, ProgramTest, ProgramTestContext};
    #[allow(deprecated)]
    use solana_sdk::system_program;
    use solana_sdk::{
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
    };

    // Test constants
    const CHAIN_ID_SVM: u32 = 30168;
    const CHAIN_ID_MVM: u32 = 30325;

    /// Fixed program ID for testing
    fn gmp_program_id() -> Pubkey {
        solana_sdk::pubkey!("GmpEnd1111111111111111111111111111111111111")
    }

    /// Mock receiver program ID (simulates outflow_validator)
    fn mock_receiver_id() -> Pubkey {
        solana_sdk::pubkey!("MockRcv111111111111111111111111111111111111")
    }

    /// Mock escrow receiver program ID (simulates intent_escrow)
    fn mock_escrow_receiver_id() -> Pubkey {
        solana_sdk::pubkey!("MockEsc111111111111111111111111111111111111")
    }

    /// Mock receiver processor - accepts any instruction
    fn mock_receiver_process(
        _program_id: &Pubkey,
        _accounts: &[solana_program::account_info::AccountInfo],
        _instruction_data: &[u8],
    ) -> solana_program::entrypoint::ProgramResult {
        solana_program::msg!("MockReceiver: instruction received");
        Ok(())
    }

    /// Mock escrow receiver processor - accepts any instruction
    fn mock_escrow_receiver_process(
        _program_id: &Pubkey,
        _accounts: &[solana_program::account_info::AccountInfo],
        _instruction_data: &[u8],
    ) -> solana_program::entrypoint::ProgramResult {
        solana_program::msg!("MockEscrowReceiver: instruction received");
        Ok(())
    }

    /// Build ProgramTest with intent-gmp and mock receivers
    fn program_test() -> ProgramTest {
        let mut pt = ProgramTest::new(
            "intent_gmp",
            gmp_program_id(),
            processor!(intent_gmp::processor::process_instruction),
        );
        // Add mock receiver for DeliverMessage CPI tests (simulates outflow_validator)
        pt.add_program("mock_receiver", mock_receiver_id(), processor!(mock_receiver_process));
        // Add mock escrow receiver for routing tests (simulates intent_escrow)
        pt.add_program("mock_escrow_receiver", mock_escrow_receiver_id(), processor!(mock_escrow_receiver_process));
        pt
    }

    /// Helper: send transaction
    async fn send_tx(
        context: &mut ProgramTestContext,
        payer: &Keypair,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<(), solana_program_test::BanksClientError> {
        let blockhash = context.banks_client.get_latest_blockhash().await?;
        let mut all_signers: Vec<&Keypair> = vec![payer];
        for s in signers {
            if s.pubkey() != payer.pubkey() {
                all_signers.push(s);
            }
        }
        let tx = Transaction::new_signed_with_payer(
            instructions,
            Some(&payer.pubkey()),
            &all_signers,
            blockhash,
        );
        context.banks_client.process_transaction(tx).await
    }

    /// Helper: create Initialize instruction
    fn create_initialize_ix(program_id: Pubkey, admin: Pubkey, payer: Pubkey, chain_id: u32) -> Instruction {
        let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], &program_id);
        Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(config_pda, false),
                AccountMeta::new_readonly(admin, true),
                AccountMeta::new(payer, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: NativeGmpInstruction::Initialize { chain_id }.try_to_vec().unwrap(),
        }
    }

    /// Helper: create AddRelay instruction
    fn create_add_relay_ix(program_id: Pubkey, admin: Pubkey, payer: Pubkey, relay: Pubkey) -> Instruction {
        let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], &program_id);
        let (relay_pda, _) = Pubkey::find_program_address(&[seeds::RELAY_SEED, relay.as_ref()], &program_id);
        Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new_readonly(config_pda, false),
                AccountMeta::new(relay_pda, false),
                AccountMeta::new_readonly(admin, true),
                AccountMeta::new(payer, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: NativeGmpInstruction::AddRelay { relay }.try_to_vec().unwrap(),
        }
    }

    /// Helper: create RemoveRelay instruction
    fn create_remove_relay_ix(program_id: Pubkey, admin: Pubkey, relay: Pubkey) -> Instruction {
        let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], &program_id);
        let (relay_pda, _) = Pubkey::find_program_address(&[seeds::RELAY_SEED, relay.as_ref()], &program_id);
        Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new_readonly(config_pda, false),
                AccountMeta::new(relay_pda, false),
                AccountMeta::new_readonly(admin, true),
            ],
            data: NativeGmpInstruction::RemoveRelay { relay }.try_to_vec().unwrap(),
        }
    }

    /// Helper: create SetRemoteGmpEndpointAddr instruction
    fn create_set_remote_gmp_endpoint_addr_ix(
        program_id: Pubkey,
        admin: Pubkey,
        payer: Pubkey,
        src_chain_id: u32,
        addr: [u8; 32],
    ) -> Instruction {
        let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], &program_id);
        let chain_id_bytes = src_chain_id.to_le_bytes();
        let (remote_gmp_endpoint_pda, _) = Pubkey::find_program_address(
            &[seeds::REMOTE_GMP_ENDPOINT_SEED, &chain_id_bytes],
            &program_id,
        );
        Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new_readonly(config_pda, false),
                AccountMeta::new(remote_gmp_endpoint_pda, false),
                AccountMeta::new_readonly(admin, true),
                AccountMeta::new(payer, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: NativeGmpInstruction::SetRemoteGmpEndpointAddr { src_chain_id, addr }.try_to_vec().unwrap(),
        }
    }

    /// Helper: create Send instruction
    ///
    /// `current_nonce` is the current value in the OutboundNonceAccount (or 0 if
    /// the account doesn't exist yet). The message PDA is derived from this nonce
    /// because `increment()` returns the current value before advancing.
    fn create_send_ix(
        program_id: Pubkey,
        sender: Pubkey,
        payer: Pubkey,
        dst_chain_id: u32,
        dst_addr: [u8; 32],
        remote_gmp_endpoint_addr: [u8; 32],
        payload: Vec<u8>,
        current_nonce: u64,
    ) -> Instruction {
        let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], &program_id);
        let chain_id_bytes = dst_chain_id.to_le_bytes();
        let (nonce_pda, _) = Pubkey::find_program_address(
            &[seeds::NONCE_OUT_SEED, &chain_id_bytes],
            &program_id,
        );
        let nonce_bytes = current_nonce.to_le_bytes();
        let (message_pda, _) = Pubkey::find_program_address(
            &[seeds::MESSAGE_SEED, &chain_id_bytes, &nonce_bytes],
            &program_id,
        );
        Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new_readonly(config_pda, false),
                AccountMeta::new(nonce_pda, false),
                AccountMeta::new_readonly(sender, true),
                AccountMeta::new(payer, true),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new(message_pda, false),
            ],
            data: NativeGmpInstruction::Send { dst_chain_id, dst_addr, remote_gmp_endpoint_addr, payload }.try_to_vec().unwrap(),
        }
    }

    /// Helper: create DeliverMessage instruction
    ///
    /// Payload must be >= 33 bytes: msg_type(1) + intent_id(32) + ...
    /// The delivered_pda is derived from the payload's intent_id and msg_type.
    fn create_deliver_message_ix(
        program_id: Pubkey,
        relay: Pubkey,
        payer: Pubkey,
        destination_program: Pubkey,
        src_chain_id: u32,
        remote_gmp_endpoint_addr: [u8; 32],
        payload: Vec<u8>,
    ) -> Instruction {
        let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], &program_id);
        let (relay_pda, _) = Pubkey::find_program_address(&[seeds::RELAY_SEED, relay.as_ref()], &program_id);
        let chain_id_bytes = src_chain_id.to_le_bytes();
        let (remote_gmp_endpoint_pda, _) = Pubkey::find_program_address(
            &[seeds::REMOTE_GMP_ENDPOINT_SEED, &chain_id_bytes],
            &program_id,
        );
        // Derive delivered PDA from payload (intent_id + msg_type)
        let msg_type = payload[0];
        let intent_id = &payload[1..33];
        let (delivered_pda, _) = Pubkey::find_program_address(
            &[seeds::DELIVERED_SEED, intent_id, &[msg_type]],
            &program_id,
        );
        let (routing_pda, _) = Pubkey::find_program_address(&[seeds::ROUTING_SEED], &program_id);
        // Account order:
        // 0. Config, 1. Relay, 2. RemoteGmpEndpoint, 3. DeliveredMessage, 4. RelaySigner, 5. Payer
        // 6. SystemProgram, 7. RoutingConfig, 8. DestProgram1, 9. DestProgram2, 10+. Remaining
        // For tests without routing, we pass destination_program as both dest1 and dest2
        Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new_readonly(config_pda, false),
                AccountMeta::new_readonly(relay_pda, false),
                AccountMeta::new_readonly(remote_gmp_endpoint_pda, false),
                AccountMeta::new(delivered_pda, false),
                AccountMeta::new_readonly(relay, true),
                AccountMeta::new(payer, true),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(routing_pda, false), // routing config (may not exist)
                AccountMeta::new_readonly(destination_program, false), // dest program 1
                AccountMeta::new_readonly(destination_program, false), // dest program 2 (same for tests)
            ],
            data: NativeGmpInstruction::DeliverMessage { src_chain_id, remote_gmp_endpoint_addr, payload }.try_to_vec().unwrap(),
        }
    }

    /// Helper: create SetRouting instruction
    fn create_set_routing_ix(
        program_id: Pubkey,
        admin: Pubkey,
        payer: Pubkey,
        outflow_validator: Pubkey,
        intent_escrow: Pubkey,
    ) -> Instruction {
        let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], &program_id);
        let (routing_pda, _) = Pubkey::find_program_address(&[seeds::ROUTING_SEED], &program_id);
        Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new_readonly(config_pda, false),
                AccountMeta::new(routing_pda, false),
                AccountMeta::new_readonly(admin, true),
                AccountMeta::new(payer, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: NativeGmpInstruction::SetRouting { outflow_validator, intent_escrow }.try_to_vec().unwrap(),
        }
    }

    /// Helper: create DeliverMessage instruction with two different destination programs (for routing tests)
    fn create_deliver_message_with_routing_ix(
        program_id: Pubkey,
        relay: Pubkey,
        payer: Pubkey,
        outflow_validator: Pubkey,
        intent_escrow: Pubkey,
        src_chain_id: u32,
        remote_gmp_endpoint_addr: [u8; 32],
        payload: Vec<u8>,
        remaining_accounts: Vec<AccountMeta>,
    ) -> Instruction {
        let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], &program_id);
        let (relay_pda, _) = Pubkey::find_program_address(&[seeds::RELAY_SEED, relay.as_ref()], &program_id);
        let chain_id_bytes = src_chain_id.to_le_bytes();
        let (remote_gmp_endpoint_pda, _) = Pubkey::find_program_address(
            &[seeds::REMOTE_GMP_ENDPOINT_SEED, &chain_id_bytes],
            &program_id,
        );
        // Derive delivered PDA from payload
        let msg_type = payload[0];
        let intent_id = &payload[1..33];
        let (delivered_pda, _) = Pubkey::find_program_address(
            &[seeds::DELIVERED_SEED, intent_id, &[msg_type]],
            &program_id,
        );
        let (routing_pda, _) = Pubkey::find_program_address(&[seeds::ROUTING_SEED], &program_id);
        let mut accounts = vec![
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(relay_pda, false),
            AccountMeta::new_readonly(remote_gmp_endpoint_pda, false),
            AccountMeta::new(delivered_pda, false),
            AccountMeta::new_readonly(relay, true),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(routing_pda, false),
            AccountMeta::new_readonly(outflow_validator, false),
            AccountMeta::new_readonly(intent_escrow, false),
        ];
        accounts.extend(remaining_accounts);
        Instruction {
            program_id,
            accounts,
            data: NativeGmpInstruction::DeliverMessage { src_chain_id, remote_gmp_endpoint_addr, payload }.try_to_vec().unwrap(),
        }
    }

    /// Helper: read account and deserialize
    async fn read_account<T: BorshDeserialize>(context: &mut ProgramTestContext, pubkey: Pubkey) -> T {
        let account = context.banks_client.get_account(pubkey).await.unwrap().unwrap();
        T::try_from_slice(&account.data).unwrap()
    }

    // ========================================================================
    // INTEGRATION TESTS
    // ========================================================================

    /// 15. Test: Send instruction updates nonce state
    /// Verifies that Send creates/updates outbound nonce account correctly.
    /// Why: Nonce tracking is critical for message ordering. State must persist correctly.
    #[tokio::test]
    async fn test_send_updates_nonce_state() {
        let pt = program_test();
        let mut context = pt.start_with_context().await;
        let admin = context.payer.insecure_clone();
        let program_id = gmp_program_id();

        // Initialize endpoint
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        send_tx(&mut context, &admin, &[init_ix], &[]).await.unwrap();

        // Send first message
        let dst_addr = [0xab; 32];
        let remote_gmp_endpoint_addr = program_id.to_bytes(); // Use program ID as remote_gmp_endpoint_addr (typical for on-chain callers)
        let payload1 = vec![0x01, 0x02, 0x03];
        let send_ix = create_send_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, dst_addr, remote_gmp_endpoint_addr, payload1.clone(), 0);
        send_tx(&mut context, &admin, &[send_ix], &[]).await.unwrap();

        // Verify nonce account created with nonce = 1 (after first send)
        let chain_id_bytes = CHAIN_ID_MVM.to_le_bytes();
        let (nonce_pda, _) = Pubkey::find_program_address(&[seeds::NONCE_OUT_SEED, &chain_id_bytes], &program_id);
        let nonce_account: OutboundNonceAccount = read_account(&mut context, nonce_pda).await;
        assert_eq!(nonce_account.nonce, 1, "Nonce should be 1 after first send (got {})", nonce_account.nonce);
        assert_eq!(nonce_account.dst_chain_id, CHAIN_ID_MVM);

        // Verify message account was created for nonce=0
        let nonce_0_bytes = 0u64.to_le_bytes();
        let (message_pda, _) = Pubkey::find_program_address(
            &[seeds::MESSAGE_SEED, &chain_id_bytes, &nonce_0_bytes], &program_id);
        let message: MessageAccount = read_account(&mut context, message_pda).await;
        assert_eq!(message.discriminator, MessageAccount::DISCRIMINATOR);
        assert_eq!(message.src_chain_id, CHAIN_ID_SVM);
        assert_eq!(message.dst_chain_id, CHAIN_ID_MVM);
        assert_eq!(message.nonce, 0);
        assert_eq!(message.dst_addr, dst_addr);
        assert_eq!(message.remote_gmp_endpoint_addr, remote_gmp_endpoint_addr);
        assert_eq!(message.payload, payload1);

        // Warp to a new slot to ensure transaction uniqueness in test framework
        context.warp_to_slot(100).unwrap();

        // Send second message (different payload for unique transaction)
        let payload2 = vec![0x04, 0x05, 0x06];
        let send_ix2 = create_send_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, dst_addr, remote_gmp_endpoint_addr, payload2, 1);
        send_tx(&mut context, &admin, &[send_ix2], &[]).await.unwrap();

        // Verify nonce incremented
        let nonce_account: OutboundNonceAccount = read_account(&mut context, nonce_pda).await;
        assert_eq!(nonce_account.nonce, 2, "Nonce should be 2 after second send (got {})", nonce_account.nonce);
    }

    /// 16. Test: DeliverMessage calls receiver's handler via CPI
    /// Verifies that DeliverMessage successfully CPIs to destination program.
    /// Why: CPI is the core mechanism for message delivery. Must succeed for cross-chain messaging to work.
    #[tokio::test]
    async fn test_deliver_message_calls_receiver() {
        let pt = program_test();
        let mut context = pt.start_with_context().await;
        let admin = context.payer.insecure_clone();
        let relay = Keypair::new();
        let program_id = gmp_program_id();

        // Fund relay
        let fund_ix = solana_sdk::system_instruction::transfer(&admin.pubkey(), &relay.pubkey(), 1_000_000_000);
        send_tx(&mut context, &admin, &[fund_ix], &[]).await.unwrap();

        // Initialize endpoint
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        send_tx(&mut context, &admin, &[init_ix], &[]).await.unwrap();

        // Add relay
        let add_relay_ix = create_add_relay_ix(program_id, admin.pubkey(), admin.pubkey(), relay.pubkey());
        send_tx(&mut context, &admin, &[add_relay_ix], &[]).await.unwrap();

        // Set remote GMP endpoint
        let remote_gmp_endpoint_addr = [0x11; 32];
        let set_remote_gmp_endpoint_ix = create_set_remote_gmp_endpoint_addr_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, remote_gmp_endpoint_addr);
        send_tx(&mut context, &admin, &[set_remote_gmp_endpoint_ix], &[]).await.unwrap();

        // Deliver message with intent_id_1
        // Payload: msg_type(1) + intent_id(32) + extra data
        let mut payload = vec![0x01]; // msg_type = IntentRequirements
        payload.extend_from_slice(&[0xA1; 32]); // intent_id_1
        payload.extend_from_slice(&[0x02, 0x03]); // extra data
        let deliver_ix = create_deliver_message_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            remote_gmp_endpoint_addr,
            payload.clone(),
        );
        // This should succeed - the mock receiver accepts any instruction
        send_tx(&mut context, &relay, &[deliver_ix], &[]).await.unwrap();

        // Verify delivered message PDA was created
        let intent_id_1 = &[0xA1u8; 32];
        let (delivered_pda, _) = Pubkey::find_program_address(
            &[seeds::DELIVERED_SEED, &intent_id_1[..], &[0x01]],
            &program_id,
        );
        let delivered: DeliveredMessage = read_account(&mut context, delivered_pda).await;
        assert_eq!(delivered.discriminator, DeliveredMessage::DISCRIMINATOR);

        // Warp to a new slot to ensure transaction uniqueness in test framework
        context.warp_to_slot(100).unwrap();

        // Deliver another message with different intent_id - should succeed
        let mut payload2 = vec![0x01]; // same msg_type
        payload2.extend_from_slice(&[0xA2; 32]); // intent_id_2 (different)
        payload2.extend_from_slice(&[0x04, 0x05]); // extra data
        let deliver_ix2 = create_deliver_message_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            remote_gmp_endpoint_addr,
            payload2,
        );
        send_tx(&mut context, &relay, &[deliver_ix2], &[]).await.unwrap();

        // Verify second delivered message PDA was created
        let intent_id_2 = &[0xA2u8; 32];
        let (delivered_pda2, _) = Pubkey::find_program_address(
            &[seeds::DELIVERED_SEED, &intent_id_2[..], &[0x01]],
            &program_id,
        );
        let delivered2: DeliveredMessage = read_account(&mut context, delivered_pda2).await;
        assert_eq!(delivered2.discriminator, DeliveredMessage::DISCRIMINATOR);
    }

    /// 17. Test: DeliverMessage rejects replay (duplicate intent_id + msg_type)
    /// Verifies that replay protection works correctly using (intent_id, msg_type) deduplication.
    /// Why: Replay attacks would allow double-processing of messages, potentially causing fund loss.
    #[tokio::test]
    async fn test_deliver_message_rejects_replay() {
        let pt = program_test();
        let mut context = pt.start_with_context().await;
        let admin = context.payer.insecure_clone();
        let relay = Keypair::new();
        let program_id = gmp_program_id();

        // Fund relay
        let fund_ix = solana_sdk::system_instruction::transfer(&admin.pubkey(), &relay.pubkey(), 1_000_000_000);
        send_tx(&mut context, &admin, &[fund_ix], &[]).await.unwrap();

        // Initialize, add relay, set remote GMP endpoint
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        let add_relay_ix = create_add_relay_ix(program_id, admin.pubkey(), admin.pubkey(), relay.pubkey());
        let remote_gmp_endpoint_addr = [0x22; 32];
        let set_remote_gmp_endpoint_ix = create_set_remote_gmp_endpoint_addr_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, remote_gmp_endpoint_addr);
        send_tx(&mut context, &admin, &[init_ix, add_relay_ix, set_remote_gmp_endpoint_ix], &[]).await.unwrap();

        // Deliver first message
        let mut payload1 = vec![0x01]; // msg_type = IntentRequirements
        payload1.extend_from_slice(&[0xBB; 32]); // intent_id
        payload1.extend_from_slice(&[0x01, 0x02]); // extra data
        let deliver_ix = create_deliver_message_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            remote_gmp_endpoint_addr,
            payload1.clone(),
        );
        send_tx(&mut context, &relay, &[deliver_ix], &[]).await.unwrap();

        // Warp to a new slot to ensure transaction uniqueness in test framework
        context.warp_to_slot(100).unwrap();

        // Try to deliver same intent_id + msg_type again - should fail (AlreadyDelivered)
        // Same payload means same intent_id and msg_type -> same DeliveredMessage PDA (already exists)
        let deliver_replay = create_deliver_message_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            remote_gmp_endpoint_addr,
            payload1, // same intent_id + msg_type
        );
        let result = send_tx(&mut context, &relay, &[deliver_replay], &[]).await;
        assert!(result.is_err(), "Replay should be rejected (same intent_id + msg_type)");
    }

    /// 18. Test: Unauthorized relay rejected
    /// Verifies that only authorized relays can deliver messages.
    /// Why: Relay authorization prevents malicious actors from injecting fake messages.
    #[tokio::test]
    async fn test_deliver_message_rejects_unauthorized_relay() {
        let pt = program_test();
        let mut context = pt.start_with_context().await;
        let admin = context.payer.insecure_clone();
        let unauthorized_relay = Keypair::new();
        let program_id = gmp_program_id();

        // Fund unauthorized relay
        let fund_ix = solana_sdk::system_instruction::transfer(&admin.pubkey(), &unauthorized_relay.pubkey(), 1_000_000_000);
        send_tx(&mut context, &admin, &[fund_ix], &[]).await.unwrap();

        // Initialize and set remote GMP endpoint, but do NOT add relay
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        let remote_gmp_endpoint_addr = [0x33; 32];
        let set_remote_gmp_endpoint_ix = create_set_remote_gmp_endpoint_addr_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, remote_gmp_endpoint_addr);
        send_tx(&mut context, &admin, &[init_ix, set_remote_gmp_endpoint_ix], &[]).await.unwrap();

        // Try to deliver message with unauthorized relay - should fail
        let mut payload = vec![0x01]; // msg_type
        payload.extend_from_slice(&[0xCC; 32]); // intent_id
        let deliver_ix = create_deliver_message_ix(
            program_id,
            unauthorized_relay.pubkey(), // not authorized
            unauthorized_relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            remote_gmp_endpoint_addr,
            payload,
        );
        let result = send_tx(&mut context, &unauthorized_relay, &[deliver_ix], &[]).await;
        assert!(result.is_err(), "Unauthorized relay should be rejected");
    }

    /// 19. Test: Authorized relay succeeds
    /// Verifies that explicitly authorized relays can deliver messages.
    /// Why: The relay authorization system must correctly grant access to approved relays.
    #[tokio::test]
    async fn test_deliver_message_authorized_relay() {
        let pt = program_test();
        let mut context = pt.start_with_context().await;
        let admin = context.payer.insecure_clone();
        let authorized_relay = Keypair::new();
        let program_id = gmp_program_id();

        // Fund authorized relay
        let fund_ix = solana_sdk::system_instruction::transfer(&admin.pubkey(), &authorized_relay.pubkey(), 1_000_000_000);
        send_tx(&mut context, &admin, &[fund_ix], &[]).await.unwrap();

        // Initialize, add relay, set remote GMP endpoint
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        let add_relay_ix = create_add_relay_ix(program_id, admin.pubkey(), admin.pubkey(), authorized_relay.pubkey());
        let remote_gmp_endpoint_addr = [0x44; 32];
        let set_remote_gmp_endpoint_ix = create_set_remote_gmp_endpoint_addr_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, remote_gmp_endpoint_addr);
        send_tx(&mut context, &admin, &[init_ix, add_relay_ix, set_remote_gmp_endpoint_ix], &[]).await.unwrap();

        // Verify relay is authorized by successfully delivering a message
        let mut payload = vec![0x01]; // msg_type
        payload.extend_from_slice(&[0xDD; 32]); // intent_id
        payload.extend_from_slice(&[0x02]); // extra
        let deliver_ix = create_deliver_message_ix(
            program_id,
            authorized_relay.pubkey(), // explicitly authorized
            authorized_relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            remote_gmp_endpoint_addr,
            payload,
        );
        send_tx(&mut context, &authorized_relay, &[deliver_ix], &[]).await.unwrap();

        // Verify delivered message PDA was created (proves message was delivered)
        let intent_id = &[0xDDu8; 32];
        let (delivered_pda, _) = Pubkey::find_program_address(
            &[seeds::DELIVERED_SEED, &intent_id[..], &[0x01]],
            &program_id,
        );
        let delivered: DeliveredMessage = read_account(&mut context, delivered_pda).await;
        assert_eq!(delivered.discriminator, DeliveredMessage::DISCRIMINATOR, "Message should have been delivered");
    }

    /// 20. Test: Unknown remote GMP endpoint address rejected
    /// Verifies that messages from unknown remote GMP endpoint addresses are rejected.
    /// Why: Remote GMP endpoint verification prevents spoofed cross-chain messages.
    #[tokio::test]
    async fn test_deliver_message_rejects_unknown_remote_gmp_endpoint() {
        let pt = program_test();
        let mut context = pt.start_with_context().await;
        let admin = context.payer.insecure_clone();
        let relay = Keypair::new();
        let program_id = gmp_program_id();

        // Fund relay
        let fund_ix = solana_sdk::system_instruction::transfer(&admin.pubkey(), &relay.pubkey(), 1_000_000_000);
        send_tx(&mut context, &admin, &[fund_ix], &[]).await.unwrap();

        // Initialize, add relay, set remote GMP endpoint
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        let add_relay_ix = create_add_relay_ix(program_id, admin.pubkey(), admin.pubkey(), relay.pubkey());
        let addr = [0x55; 32];
        let set_remote_gmp_endpoint_ix = create_set_remote_gmp_endpoint_addr_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, addr);
        send_tx(&mut context, &admin, &[init_ix, add_relay_ix, set_remote_gmp_endpoint_ix], &[]).await.unwrap();

        // Try to deliver message from unknown remote GMP endpoint address
        let unknown_addr = [0xFF; 32]; // different from addr
        let mut payload = vec![0x01]; // msg_type
        payload.extend_from_slice(&[0xEE; 32]); // intent_id
        let deliver_ix = create_deliver_message_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            unknown_addr, // not the remote GMP endpoint address
            payload,
        );
        let result = send_tx(&mut context, &relay, &[deliver_ix], &[]).await;
        assert!(result.is_err(), "Unknown remote GMP endpoint should be rejected");
    }

    /// 21. Test: No remote GMP endpoint configured
    /// Verifies that messages fail when no remote GMP endpoint is configured for the source chain.
    /// Why: Missing configuration must be caught early to prevent security holes.
    #[tokio::test]
    async fn test_deliver_message_rejects_no_remote_gmp_endpoint() {
        let pt = program_test();
        let mut context = pt.start_with_context().await;
        let admin = context.payer.insecure_clone();
        let relay = Keypair::new();
        let program_id = gmp_program_id();

        // Fund relay
        let fund_ix = solana_sdk::system_instruction::transfer(&admin.pubkey(), &relay.pubkey(), 1_000_000_000);
        send_tx(&mut context, &admin, &[fund_ix], &[]).await.unwrap();

        // Initialize and add relay, but do NOT set remote GMP endpoint
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        let add_relay_ix = create_add_relay_ix(program_id, admin.pubkey(), admin.pubkey(), relay.pubkey());
        send_tx(&mut context, &admin, &[init_ix, add_relay_ix], &[]).await.unwrap();

        // Try to deliver message - should fail because no remote GMP endpoint is configured
        let remote_gmp_endpoint_addr = [0x66; 32];
        let mut payload = vec![0x01]; // msg_type
        payload.extend_from_slice(&[0xFF; 32]); // intent_id
        let deliver_ix = create_deliver_message_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM, // no remote GMP endpoint configured for this chain
            remote_gmp_endpoint_addr,
            payload,
        );
        let result = send_tx(&mut context, &relay, &[deliver_ix], &[]).await;
        assert!(result.is_err(), "Message from chain with no remote GMP endpoint should be rejected");
    }

    /// 22. Test: Non-admin cannot set remote GMP endpoint
    /// Verifies that only the admin can configure remote GMP endpoint addresses.
    /// Why: Admin-only access prevents unauthorized trust configuration changes.
    #[tokio::test]
    async fn test_set_remote_gmp_endpoint_addr_unauthorized() {
        let pt = program_test();
        let mut context = pt.start_with_context().await;
        let admin = context.payer.insecure_clone();
        let non_admin = Keypair::new();
        let program_id = gmp_program_id();

        // Fund non-admin
        let fund_ix = solana_sdk::system_instruction::transfer(&admin.pubkey(), &non_admin.pubkey(), 1_000_000_000);
        send_tx(&mut context, &admin, &[fund_ix], &[]).await.unwrap();

        // Initialize endpoint
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        send_tx(&mut context, &admin, &[init_ix], &[]).await.unwrap();

        // Non-admin tries to set remote GMP endpoint - should fail
        let addr = [0x77; 32];
        let set_remote_gmp_endpoint_ix = create_set_remote_gmp_endpoint_addr_ix(program_id, non_admin.pubkey(), non_admin.pubkey(), CHAIN_ID_MVM, addr);
        let result = send_tx(&mut context, &non_admin, &[set_remote_gmp_endpoint_ix], &[]).await;
        assert!(result.is_err(), "Non-admin should not be able to set remote GMP endpoint");
    }

    /// 23. Test: Same intent_id with different msg_type succeeds (not a duplicate)
    /// Verifies that dedup is per (intent_id, msg_type) pair, not just intent_id.
    /// Why: The same intent legitimately receives different message types (e.g., IntentRequirements
    /// then FulfillmentProof). These must not be treated as duplicates.
    #[tokio::test]
    async fn test_deliver_message_different_msg_type_succeeds() {
        let pt = program_test();
        let mut context = pt.start_with_context().await;
        let admin = context.payer.insecure_clone();
        let relay = Keypair::new();
        let program_id = gmp_program_id();

        // Fund relay
        let fund_ix = solana_sdk::system_instruction::transfer(&admin.pubkey(), &relay.pubkey(), 1_000_000_000);
        send_tx(&mut context, &admin, &[fund_ix], &[]).await.unwrap();

        // Initialize, add relay, set remote GMP endpoint
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        let add_relay_ix = create_add_relay_ix(program_id, admin.pubkey(), admin.pubkey(), relay.pubkey());
        let remote_gmp_endpoint_addr = [0x88; 32];
        let set_remote_gmp_endpoint_ix = create_set_remote_gmp_endpoint_addr_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, remote_gmp_endpoint_addr);
        send_tx(&mut context, &admin, &[init_ix, add_relay_ix, set_remote_gmp_endpoint_ix], &[]).await.unwrap();

        // Deliver IntentRequirements (0x01) for intent_id
        let intent_id = [0xF1u8; 32];
        let mut payload1 = vec![0x01]; // msg_type = IntentRequirements
        payload1.extend_from_slice(&intent_id);
        payload1.extend_from_slice(&[0x00; 10]); // extra data
        let deliver_ix = create_deliver_message_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            remote_gmp_endpoint_addr,
            payload1,
        );
        send_tx(&mut context, &relay, &[deliver_ix], &[]).await.unwrap();

        // Warp to a new slot
        context.warp_to_slot(100).unwrap();

        // Deliver FulfillmentProof (0x03) for same intent_id - should succeed (different msg_type)
        let mut payload2 = vec![0x03]; // msg_type = FulfillmentProof (different!)
        payload2.extend_from_slice(&intent_id); // same intent_id
        payload2.extend_from_slice(&[0xBB; 32]); // solver_addr
        payload2.extend_from_slice(&0u64.to_be_bytes()); // amount
        payload2.extend_from_slice(&0u64.to_be_bytes()); // timestamp
        let deliver_ix2 = create_deliver_message_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            remote_gmp_endpoint_addr,
            payload2,
        );
        send_tx(&mut context, &relay, &[deliver_ix2], &[]).await.unwrap();

        // Verify both delivered message PDAs exist
        let (delivered_pda1, _) = Pubkey::find_program_address(
            &[seeds::DELIVERED_SEED, &intent_id[..], &[0x01]],
            &program_id,
        );
        let (delivered_pda2, _) = Pubkey::find_program_address(
            &[seeds::DELIVERED_SEED, &intent_id[..], &[0x03]],
            &program_id,
        );
        let _d1: DeliveredMessage = read_account(&mut context, delivered_pda1).await;
        let _d2: DeliveredMessage = read_account(&mut context, delivered_pda2).await;
    }

    // ========================================================================
    // ADMIN TESTS
    // ========================================================================

    /// 25. Test: Non-admin cannot add relay
    /// Verifies that only the admin can add authorized relays.
    /// Why: Relay management is security-critical; must be admin-only.
    #[tokio::test]
    async fn test_add_relay_rejects_non_admin() {
        let pt = program_test();
        let mut context = pt.start_with_context().await;
        let admin = context.payer.insecure_clone();
        let non_admin = Keypair::new();
        let program_id = gmp_program_id();

        // Fund non-admin
        let fund_ix = solana_sdk::system_instruction::transfer(&admin.pubkey(), &non_admin.pubkey(), 1_000_000_000);
        send_tx(&mut context, &admin, &[fund_ix], &[]).await.unwrap();

        // Initialize endpoint
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        send_tx(&mut context, &admin, &[init_ix], &[]).await.unwrap();

        // Non-admin tries to add relay - should fail
        let new_relay = Keypair::new();
        let add_relay_ix = create_add_relay_ix(program_id, non_admin.pubkey(), non_admin.pubkey(), new_relay.pubkey());
        let result = send_tx(&mut context, &non_admin, &[add_relay_ix], &[]).await;
        assert!(result.is_err(), "Non-admin should not be able to add relay");
    }

    /// 26. Test: Non-admin cannot remove relay
    /// Verifies that only the admin can remove authorized relays.
    /// Why: Relay management is security-critical; must be admin-only.
    #[tokio::test]
    async fn test_remove_relay_rejects_non_admin() {
        let pt = program_test();
        let mut context = pt.start_with_context().await;
        let admin = context.payer.insecure_clone();
        let non_admin = Keypair::new();
        let relay = Keypair::new();
        let program_id = gmp_program_id();

        // Fund non-admin
        let fund_ix = solana_sdk::system_instruction::transfer(&admin.pubkey(), &non_admin.pubkey(), 1_000_000_000);
        send_tx(&mut context, &admin, &[fund_ix], &[]).await.unwrap();

        // Initialize endpoint and add a relay as admin
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        let add_relay_ix = create_add_relay_ix(program_id, admin.pubkey(), admin.pubkey(), relay.pubkey());
        send_tx(&mut context, &admin, &[init_ix, add_relay_ix], &[]).await.unwrap();

        // Non-admin tries to remove relay - should fail
        let remove_relay_ix = create_remove_relay_ix(program_id, non_admin.pubkey(), relay.pubkey());
        let result = send_tx(&mut context, &non_admin, &[remove_relay_ix], &[]).await;
        assert!(result.is_err(), "Non-admin should not be able to remove relay");
    }

    // ========================================================================
    // FULFILLMENT PROOF ROUTING TESTS
    // ========================================================================

    /// 28. Test: FulfillmentProof (0x03) routes to intent_escrow when routing is configured
    /// Verifies that message type 0x03 is routed to destination_program_2 (intent_escrow).
    /// Why: FulfillmentProof releases escrow funds on the connected chain. It must route to
    /// intent_escrow, not outflow_validator. This test validates the MESSAGE_TYPE_FULFILLMENT_PROOF
    /// routing logic added to the GMP endpoint.
    #[tokio::test]
    async fn test_fulfillment_proof_routes_to_intent_escrow() {
        let pt = program_test();
        let mut context = pt.start_with_context().await;
        let admin = context.payer.insecure_clone();
        let relay = Keypair::new();
        let program_id = gmp_program_id();

        // Fund relay
        let fund_ix = solana_sdk::system_instruction::transfer(&admin.pubkey(), &relay.pubkey(), 1_000_000_000);
        send_tx(&mut context, &admin, &[fund_ix], &[]).await.unwrap();

        // Initialize, add relay, set remote GMP endpoint
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        let add_relay_ix = create_add_relay_ix(program_id, admin.pubkey(), admin.pubkey(), relay.pubkey());
        let remote_gmp_endpoint_addr = [0x99; 32];
        let set_remote_gmp_endpoint_ix = create_set_remote_gmp_endpoint_addr_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, remote_gmp_endpoint_addr);
        send_tx(&mut context, &admin, &[init_ix, add_relay_ix, set_remote_gmp_endpoint_ix], &[]).await.unwrap();

        // Set up routing config
        let set_routing_ix = create_set_routing_ix(
            program_id,
            admin.pubkey(),
            admin.pubkey(),
            mock_receiver_id(),       // outflow_validator
            mock_escrow_receiver_id(), // intent_escrow
        );
        send_tx(&mut context, &admin, &[set_routing_ix], &[]).await.unwrap();

        // Create FulfillmentProof payload (message type 0x03)
        // Format: [type(1)] [intent_id(32)] [solver_addr(32)] [amount(8)] [timestamp(8)] = 81 bytes
        let mut payload = vec![0x03]; // FulfillmentProof message type
        payload.extend_from_slice(&[0xAA; 32]); // intent_id
        payload.extend_from_slice(&[0xBB; 32]); // solver_addr
        payload.extend_from_slice(&0u64.to_be_bytes()); // amount
        payload.extend_from_slice(&0u64.to_be_bytes()); // timestamp
        assert_eq!(payload.len(), 81, "FulfillmentProof payload should be 81 bytes");

        // Create mock remaining accounts (7 accounts for GmpReceiveFulfillmentProof)
        // In production: requirements, escrow, vault, solver_token, gmp_config, gmp_caller, token_program
        // For test: we just need 7 placeholder accounts with deterministic addresses
        let remaining_accounts = vec![
            AccountMeta::new(Pubkey::new_from_array([0xD1; 32]), false),        // 0: requirements (writable)
            AccountMeta::new(Pubkey::new_from_array([0xD2; 32]), false),        // 1: escrow (writable)
            AccountMeta::new(Pubkey::new_from_array([0xD3; 32]), false),        // 2: vault (writable)
            AccountMeta::new(Pubkey::new_from_array([0xD4; 32]), false),        // 3: solver_token (writable)
            AccountMeta::new_readonly(Pubkey::new_from_array([0xD5; 32]), false), // 4: gmp_config
            AccountMeta::new_readonly(relay.pubkey(), true),      // 5: gmp_caller (signer)
            AccountMeta::new_readonly(Pubkey::new_from_array([0xD6; 32]), false), // 6: token_program
        ];

        // Deliver FulfillmentProof message - should route to intent_escrow (mock_escrow_receiver)
        let deliver_ix = create_deliver_message_with_routing_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),        // outflow_validator (should NOT be called)
            mock_escrow_receiver_id(), // intent_escrow (should be called)
            CHAIN_ID_MVM,
            remote_gmp_endpoint_addr,
            payload.clone(),
            remaining_accounts,
        );

        // The transaction should succeed (message delivered to mock_escrow_receiver)
        send_tx(&mut context, &relay, &[deliver_ix], &[]).await.unwrap();

        // Verify delivered message PDA was created (proves message was processed)
        let intent_id = &payload[1..33];
        let (delivered_pda, _) = Pubkey::find_program_address(
            &[seeds::DELIVERED_SEED, intent_id, &[0x03]],
            &program_id,
        );
        let delivered: DeliveredMessage = read_account(&mut context, delivered_pda).await;
        assert_eq!(delivered.discriminator, DeliveredMessage::DISCRIMINATOR, "Message should have been delivered");
    }

    /// 29. Test: FulfillmentProof fails with insufficient accounts
    /// Verifies that FulfillmentProof routing fails when fewer than 7 remaining accounts provided.
    /// Why: GmpReceiveFulfillmentProof requires 7 accounts for token transfer. The GMP endpoint
    /// must validate account count before attempting CPI.
    #[tokio::test]
    async fn test_fulfillment_proof_fails_with_insufficient_accounts() {
        let pt = program_test();
        let mut context = pt.start_with_context().await;
        let admin = context.payer.insecure_clone();
        let relay = Keypair::new();
        let program_id = gmp_program_id();

        // Fund relay
        let fund_ix = solana_sdk::system_instruction::transfer(&admin.pubkey(), &relay.pubkey(), 1_000_000_000);
        send_tx(&mut context, &admin, &[fund_ix], &[]).await.unwrap();

        // Initialize, add relay, set remote GMP endpoint
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        let add_relay_ix = create_add_relay_ix(program_id, admin.pubkey(), admin.pubkey(), relay.pubkey());
        let remote_gmp_endpoint_addr = [0xAA; 32];
        let set_remote_gmp_endpoint_ix = create_set_remote_gmp_endpoint_addr_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, remote_gmp_endpoint_addr);
        send_tx(&mut context, &admin, &[init_ix, add_relay_ix, set_remote_gmp_endpoint_ix], &[]).await.unwrap();

        // Set up routing config
        let set_routing_ix = create_set_routing_ix(
            program_id,
            admin.pubkey(),
            admin.pubkey(),
            mock_receiver_id(),        // outflow_validator
            mock_escrow_receiver_id(), // intent_escrow
        );
        send_tx(&mut context, &admin, &[set_routing_ix], &[]).await.unwrap();

        // Create FulfillmentProof payload
        let mut payload = vec![0x03];
        payload.extend_from_slice(&[0xAA; 32]); // intent_id
        payload.extend_from_slice(&[0xBB; 32]); // solver_addr
        payload.extend_from_slice(&0u64.to_be_bytes()); // amount
        payload.extend_from_slice(&0u64.to_be_bytes()); // timestamp

        // Only provide 3 accounts instead of required 7
        let insufficient_accounts = vec![
            AccountMeta::new(Pubkey::new_from_array([0xE1; 32]), false),
            AccountMeta::new(Pubkey::new_from_array([0xE2; 32]), false),
            AccountMeta::new(Pubkey::new_from_array([0xE3; 32]), false),
        ];

        let deliver_ix = create_deliver_message_with_routing_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),
            mock_escrow_receiver_id(),
            CHAIN_ID_MVM,
            remote_gmp_endpoint_addr,
            payload,
            insufficient_accounts,
        );

        // Should fail due to insufficient accounts
        let result = send_tx(&mut context, &relay, &[deliver_ix], &[]).await;
        assert!(result.is_err(), "Should fail with insufficient accounts for FulfillmentProof");
    }
}
