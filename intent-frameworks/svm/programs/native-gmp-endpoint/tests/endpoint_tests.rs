//! Interface tests for the native GMP endpoint program.
//!
//! These tests verify that instructions and state can be correctly serialized,
//! and that the nonce tracking logic works correctly for replay protection.

use borsh::BorshDeserialize;
use native_gmp_endpoint::{
    instruction::NativeGmpInstruction,
    state::{
        ConfigAccount, InboundNonceAccount, OutboundNonceAccount, RelayAccount,
        TrustedRemoteAccount,
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

fn dummy_src_addr() -> [u8; 32] {
    [2u8; 32]
}

fn dummy_trusted_addr() -> [u8; 32] {
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
    let original_payload = dummy_payload();

    let instruction = NativeGmpInstruction::Send {
        dst_chain_id: original_dst_chain_id,
        dst_addr: original_dst_addr,
        payload: original_payload.clone(),
    };

    let encoded = borsh::to_vec(&instruction).unwrap();
    let decoded = NativeGmpInstruction::try_from_slice(&encoded).unwrap();

    match decoded {
        NativeGmpInstruction::Send {
            dst_chain_id,
            dst_addr,
            payload,
        } => {
            assert_eq!(dst_chain_id, original_dst_chain_id);
            assert_eq!(dst_addr, original_dst_addr);
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
    let original_src_addr = dummy_src_addr();
    let original_payload = dummy_payload();
    let original_nonce = 42u64;

    let instruction = NativeGmpInstruction::DeliverMessage {
        src_chain_id: original_src_chain_id,
        src_addr: original_src_addr,
        payload: original_payload.clone(),
        nonce: original_nonce,
    };

    let encoded = borsh::to_vec(&instruction).unwrap();
    let decoded = NativeGmpInstruction::try_from_slice(&encoded).unwrap();

    match decoded {
        NativeGmpInstruction::DeliverMessage {
            src_chain_id,
            src_addr,
            payload,
            nonce,
        } => {
            assert_eq!(src_chain_id, original_src_chain_id);
            assert_eq!(src_addr, original_src_addr);
            assert_eq!(payload, original_payload);
            assert_eq!(nonce, original_nonce);
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
    let original_relay = Pubkey::new_unique();

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

/// 5. Test: SetTrustedRemote instruction serialization roundtrip
/// Verifies that SetTrustedRemote instruction can be serialized and deserialized correctly.
/// Why: Trusted remote configuration is security-critical. Wrong chain_id or address would accept messages from untrusted sources.
#[test]
fn test_set_trusted_remote_instruction_serialization() {
    let original_src_chain_id = DUMMY_CHAIN_ID_MVM;
    let original_trusted_addr = dummy_trusted_addr();

    let instruction = NativeGmpInstruction::SetTrustedRemote {
        src_chain_id: original_src_chain_id,
        trusted_addr: original_trusted_addr,
    };

    let encoded = borsh::to_vec(&instruction).unwrap();
    let decoded = NativeGmpInstruction::try_from_slice(&encoded).unwrap();

    match decoded {
        NativeGmpInstruction::SetTrustedRemote {
            src_chain_id,
            trusted_addr,
        } => {
            assert_eq!(src_chain_id, original_src_chain_id);
            assert_eq!(trusted_addr, original_trusted_addr);
        }
        _ => panic!("Wrong instruction variant"),
    }
}

// ============================================================================
// STATE SERIALIZATION TESTS
// ============================================================================

/// 6. Test: ConfigAccount serialization roundtrip
/// Verifies that ConfigAccount state can be serialized and deserialized correctly.
/// Why: Config stores admin and chain_id. Corruption would break authorization checks and message routing.
#[test]
fn test_config_account_serialization() {
    let original_admin = Pubkey::new_unique();
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

/// 7. Test: RelayAccount serialization roundtrip
/// Verifies that RelayAccount state can be serialized and deserialized correctly.
/// Why: Relay authorization state must persist correctly. Corruption could authorize/deauthorize wrong relays.
#[test]
fn test_relay_account_serialization() {
    let original_relay = Pubkey::new_unique();
    let original_bump = 254u8;

    let relay_account = RelayAccount::new(original_relay, original_bump);

    let encoded = borsh::to_vec(&relay_account).unwrap();
    let decoded = RelayAccount::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.discriminator, RelayAccount::DISCRIMINATOR);
    assert_eq!(decoded.relay, original_relay);
    assert!(decoded.is_authorized);
    assert_eq!(decoded.bump, original_bump);
}

/// 8. Test: TrustedRemoteAccount serialization roundtrip
/// Verifies that TrustedRemoteAccount state can be serialized and deserialized correctly.
/// Why: Trusted remote config is security-critical. Corruption would accept messages from untrusted sources.
#[test]
fn test_trusted_remote_account_serialization() {
    let original_src_chain_id = DUMMY_CHAIN_ID_MVM;
    let original_trusted_addr = dummy_trusted_addr();
    let original_bump = 253u8;

    let trusted_remote =
        TrustedRemoteAccount::new(original_src_chain_id, original_trusted_addr, original_bump);

    let encoded = borsh::to_vec(&trusted_remote).unwrap();
    let decoded = TrustedRemoteAccount::try_from_slice(&encoded).unwrap();

    assert_eq!(decoded.discriminator, TrustedRemoteAccount::DISCRIMINATOR);
    assert_eq!(decoded.src_chain_id, original_src_chain_id);
    assert_eq!(decoded.trusted_addr, original_trusted_addr);
    assert_eq!(decoded.bump, original_bump);
}

// ============================================================================
// NONCE TRACKING TESTS
// ============================================================================

/// 9. Test: OutboundNonceAccount increment behavior
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

/// 10. Test: InboundNonceAccount replay detection
/// Verifies that replay detection correctly identifies previously processed nonces.
/// Why: Replay protection prevents double-processing of messages. Bugs here would allow replay attacks.
#[test]
fn test_inbound_nonce_account_replay_detection() {
    let mut nonce_account = InboundNonceAccount::new(DUMMY_CHAIN_ID_MVM, 251);

    assert_eq!(nonce_account.last_nonce, 0);

    // Nonce 1 is not a replay (first message)
    assert!(!nonce_account.is_replay(1));
    nonce_account.update_nonce(1);
    assert_eq!(nonce_account.last_nonce, 1);

    // Nonce 1 is now a replay
    assert!(nonce_account.is_replay(1));

    // Nonce 0 is a replay
    assert!(nonce_account.is_replay(0));

    // Nonce 2 is not a replay
    assert!(!nonce_account.is_replay(2));
    nonce_account.update_nonce(2);

    // Skip to nonce 5 (gaps allowed)
    assert!(!nonce_account.is_replay(5));
    nonce_account.update_nonce(5);
    assert_eq!(nonce_account.last_nonce, 5);

    // Nonce 3 and 4 are now replays (even though never seen)
    assert!(nonce_account.is_replay(3));
    assert!(nonce_account.is_replay(4));
}

// ============================================================================
// ERROR CONVERSION TESTS
// ============================================================================

/// 11. Test: Error to ProgramError conversion
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

/// 12. Test: All error variants have unique codes
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
        GmpError::UntrustedRemote,
        GmpError::ReplayDetected,
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
    use native_gmp_endpoint::{
        instruction::NativeGmpInstruction,
        state::{seeds, ConfigAccount, OutboundNonceAccount, RelayAccount, TrustedRemoteAccount, InboundNonceAccount},
    };
    use solana_program::instruction::{AccountMeta, Instruction};
    use solana_program_test::{processor, ProgramTest, ProgramTestContext};
    use solana_sdk::{
        account::Account,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        system_program,
        transaction::Transaction,
    };

    // Test constants
    const CHAIN_ID_SVM: u32 = 30168;
    const CHAIN_ID_MVM: u32 = 30325;

    /// Fixed program ID for testing
    fn gmp_program_id() -> Pubkey {
        solana_sdk::pubkey!("GmpEnd1111111111111111111111111111111111111")
    }

    /// Mock receiver program ID
    fn mock_receiver_id() -> Pubkey {
        solana_sdk::pubkey!("MockRcv111111111111111111111111111111111111")
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

    /// Build ProgramTest with native-gmp-endpoint and mock receiver
    fn program_test() -> ProgramTest {
        let mut pt = ProgramTest::new(
            "native_gmp_endpoint",
            gmp_program_id(),
            processor!(native_gmp_endpoint::processor::process_instruction),
        );
        // Add mock receiver for DeliverMessage CPI tests
        pt.add_program("mock_receiver", mock_receiver_id(), processor!(mock_receiver_process));
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

    /// Helper: create SetTrustedRemote instruction
    fn create_set_trusted_remote_ix(
        program_id: Pubkey,
        admin: Pubkey,
        payer: Pubkey,
        src_chain_id: u32,
        trusted_addr: [u8; 32],
    ) -> Instruction {
        let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], &program_id);
        let chain_id_bytes = src_chain_id.to_le_bytes();
        let (trusted_remote_pda, _) = Pubkey::find_program_address(
            &[seeds::TRUSTED_REMOTE_SEED, &chain_id_bytes],
            &program_id,
        );
        Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new_readonly(config_pda, false),
                AccountMeta::new(trusted_remote_pda, false),
                AccountMeta::new_readonly(admin, true),
                AccountMeta::new(payer, true),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: NativeGmpInstruction::SetTrustedRemote { src_chain_id, trusted_addr }.try_to_vec().unwrap(),
        }
    }

    /// Helper: create Send instruction
    fn create_send_ix(
        program_id: Pubkey,
        sender: Pubkey,
        payer: Pubkey,
        dst_chain_id: u32,
        dst_addr: [u8; 32],
        payload: Vec<u8>,
    ) -> Instruction {
        let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], &program_id);
        let chain_id_bytes = dst_chain_id.to_le_bytes();
        let (nonce_pda, _) = Pubkey::find_program_address(
            &[seeds::NONCE_OUT_SEED, &chain_id_bytes],
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
            ],
            data: NativeGmpInstruction::Send { dst_chain_id, dst_addr, payload }.try_to_vec().unwrap(),
        }
    }

    /// Helper: create DeliverMessage instruction
    fn create_deliver_message_ix(
        program_id: Pubkey,
        relay: Pubkey,
        payer: Pubkey,
        destination_program: Pubkey,
        src_chain_id: u32,
        src_addr: [u8; 32],
        payload: Vec<u8>,
        nonce: u64,
    ) -> Instruction {
        let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], &program_id);
        let (relay_pda, _) = Pubkey::find_program_address(&[seeds::RELAY_SEED, relay.as_ref()], &program_id);
        let chain_id_bytes = src_chain_id.to_le_bytes();
        let (trusted_remote_pda, _) = Pubkey::find_program_address(
            &[seeds::TRUSTED_REMOTE_SEED, &chain_id_bytes],
            &program_id,
        );
        let (nonce_pda, _) = Pubkey::find_program_address(
            &[seeds::NONCE_IN_SEED, &chain_id_bytes],
            &program_id,
        );
        Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new_readonly(config_pda, false),
                AccountMeta::new_readonly(relay_pda, false),
                AccountMeta::new_readonly(trusted_remote_pda, false),
                AccountMeta::new(nonce_pda, false),
                AccountMeta::new_readonly(relay, true),
                AccountMeta::new(payer, true),
                AccountMeta::new_readonly(destination_program, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: NativeGmpInstruction::DeliverMessage { src_chain_id, src_addr, payload, nonce }.try_to_vec().unwrap(),
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

    /// 13. Test: Send instruction updates nonce state
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
        let payload1 = vec![0x01, 0x02, 0x03];
        let send_ix = create_send_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, dst_addr, payload1);
        send_tx(&mut context, &admin, &[send_ix], &[]).await.unwrap();

        // Verify nonce account created with nonce = 1 (after first send)
        let chain_id_bytes = CHAIN_ID_MVM.to_le_bytes();
        let (nonce_pda, _) = Pubkey::find_program_address(&[seeds::NONCE_OUT_SEED, &chain_id_bytes], &program_id);
        let nonce_account: OutboundNonceAccount = read_account(&mut context, nonce_pda).await;
        assert_eq!(nonce_account.nonce, 1, "Nonce should be 1 after first send (got {})", nonce_account.nonce);
        assert_eq!(nonce_account.dst_chain_id, CHAIN_ID_MVM);

        // Warp to a new slot to ensure transaction uniqueness in test framework
        context.warp_to_slot(100).unwrap();

        // Send second message (different payload for unique transaction)
        let payload2 = vec![0x04, 0x05, 0x06];
        let send_ix2 = create_send_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, dst_addr, payload2);
        send_tx(&mut context, &admin, &[send_ix2], &[]).await.unwrap();

        // Verify nonce incremented
        let nonce_account: OutboundNonceAccount = read_account(&mut context, nonce_pda).await;
        assert_eq!(nonce_account.nonce, 2, "Nonce should be 2 after second send (got {})", nonce_account.nonce);
    }

    /// 14. Test: DeliverMessage calls receiver's handler via CPI
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

        // Set trusted remote
        let src_addr = [0x11; 32];
        let set_trusted_ix = create_set_trusted_remote_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, src_addr);
        send_tx(&mut context, &admin, &[set_trusted_ix], &[]).await.unwrap();

        // Deliver message (nonce = 1)
        let payload = vec![0x01, 0x02, 0x03];
        let deliver_ix = create_deliver_message_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            src_addr,
            payload,
            1, // nonce
        );
        // This should succeed - the mock receiver accepts any instruction
        send_tx(&mut context, &relay, &[deliver_ix], &[]).await.unwrap();

        // Verify inbound nonce updated
        let chain_id_bytes = CHAIN_ID_MVM.to_le_bytes();
        let (nonce_pda, _) = Pubkey::find_program_address(&[seeds::NONCE_IN_SEED, &chain_id_bytes], &program_id);
        let nonce_account: InboundNonceAccount = read_account(&mut context, nonce_pda).await;
        assert_eq!(nonce_account.last_nonce, 1, "Last nonce should be 1 after first delivery");

        // Deliver another message (nonce = 2) - should succeed
        let payload2 = vec![0x04, 0x05];
        let deliver_ix2 = create_deliver_message_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            src_addr,
            payload2,
            2, // nonce
        );
        send_tx(&mut context, &relay, &[deliver_ix2], &[]).await.unwrap();

        let nonce_account: InboundNonceAccount = read_account(&mut context, nonce_pda).await;
        assert_eq!(nonce_account.last_nonce, 2, "Last nonce should be 2 after second delivery");
    }

    /// 15. Test: DeliverMessage rejects replay (duplicate nonce)
    /// Verifies that replay protection works correctly.
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

        // Initialize, add relay, set trusted remote
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        let add_relay_ix = create_add_relay_ix(program_id, admin.pubkey(), admin.pubkey(), relay.pubkey());
        let src_addr = [0x22; 32];
        let set_trusted_ix = create_set_trusted_remote_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, src_addr);
        send_tx(&mut context, &admin, &[init_ix, add_relay_ix, set_trusted_ix], &[]).await.unwrap();

        // Deliver first message (nonce = 1)
        let payload1 = vec![0x01];
        let deliver_ix = create_deliver_message_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            src_addr,
            payload1,
            1,
        );
        send_tx(&mut context, &relay, &[deliver_ix], &[]).await.unwrap();

        // Warp to a new slot to ensure transaction uniqueness in test framework
        context.warp_to_slot(100).unwrap();

        // Try to replay same nonce with different payload - should fail
        let payload2 = vec![0x02]; // Different payload to ensure unique transaction
        let deliver_replay = create_deliver_message_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            src_addr,
            payload2,
            1, // same nonce - should be rejected as replay
        );
        let result = send_tx(&mut context, &relay, &[deliver_replay], &[]).await;
        assert!(result.is_err(), "Replay should be rejected");
    }

    /// 16. Test: Unauthorized relay rejected
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

        // Initialize and set trusted remote, but do NOT add relay
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        let src_addr = [0x33; 32];
        let set_trusted_ix = create_set_trusted_remote_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, src_addr);
        send_tx(&mut context, &admin, &[init_ix, set_trusted_ix], &[]).await.unwrap();

        // Try to deliver message with unauthorized relay - should fail
        let payload = vec![0x01, 0x02];
        let deliver_ix = create_deliver_message_ix(
            program_id,
            unauthorized_relay.pubkey(), // not authorized
            unauthorized_relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            src_addr,
            payload,
            1, // nonce
        );
        let result = send_tx(&mut context, &unauthorized_relay, &[deliver_ix], &[]).await;
        assert!(result.is_err(), "Unauthorized relay should be rejected");
    }

    /// 17. Test: Authorized relay succeeds
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

        // Initialize, add relay, set trusted remote
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        let add_relay_ix = create_add_relay_ix(program_id, admin.pubkey(), admin.pubkey(), authorized_relay.pubkey());
        let src_addr = [0x44; 32];
        let set_trusted_ix = create_set_trusted_remote_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, src_addr);
        send_tx(&mut context, &admin, &[init_ix, add_relay_ix, set_trusted_ix], &[]).await.unwrap();

        // Verify relay is authorized by successfully delivering a message
        let payload = vec![0x01, 0x02];
        let deliver_ix = create_deliver_message_ix(
            program_id,
            authorized_relay.pubkey(), // explicitly authorized
            authorized_relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            src_addr,
            payload,
            1, // nonce
        );
        send_tx(&mut context, &authorized_relay, &[deliver_ix], &[]).await.unwrap();

        // Verify inbound nonce was updated (proves message was delivered)
        let chain_id_bytes = CHAIN_ID_MVM.to_le_bytes();
        let (nonce_pda, _) = Pubkey::find_program_address(&[seeds::NONCE_IN_SEED, &chain_id_bytes], &program_id);
        let nonce_account: InboundNonceAccount = read_account(&mut context, nonce_pda).await;
        assert_eq!(nonce_account.last_nonce, 1, "Message should have been delivered");
    }

    /// 18. Test: Untrusted remote address rejected
    /// Verifies that messages from non-trusted source addresses are rejected.
    /// Why: Trusted remote verification prevents spoofed cross-chain messages.
    #[tokio::test]
    async fn test_deliver_message_rejects_untrusted_remote() {
        let pt = program_test();
        let mut context = pt.start_with_context().await;
        let admin = context.payer.insecure_clone();
        let relay = Keypair::new();
        let program_id = gmp_program_id();

        // Fund relay
        let fund_ix = solana_sdk::system_instruction::transfer(&admin.pubkey(), &relay.pubkey(), 1_000_000_000);
        send_tx(&mut context, &admin, &[fund_ix], &[]).await.unwrap();

        // Initialize, add relay, set trusted remote
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        let add_relay_ix = create_add_relay_ix(program_id, admin.pubkey(), admin.pubkey(), relay.pubkey());
        let trusted_addr = [0x55; 32];
        let set_trusted_ix = create_set_trusted_remote_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, trusted_addr);
        send_tx(&mut context, &admin, &[init_ix, add_relay_ix, set_trusted_ix], &[]).await.unwrap();

        // Try to deliver message from untrusted address
        let untrusted_addr = [0xFF; 32]; // different from trusted_addr
        let payload = vec![0x01, 0x02];
        let deliver_ix = create_deliver_message_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            untrusted_addr, // not the trusted address
            payload,
            1, // nonce
        );
        let result = send_tx(&mut context, &relay, &[deliver_ix], &[]).await;
        assert!(result.is_err(), "Untrusted remote should be rejected");
    }

    /// 19. Test: No trusted remote configured
    /// Verifies that messages fail when no trusted remote is configured for the source chain.
    /// Why: Missing configuration must be caught early to prevent security holes.
    #[tokio::test]
    async fn test_deliver_message_rejects_no_trusted_remote() {
        let pt = program_test();
        let mut context = pt.start_with_context().await;
        let admin = context.payer.insecure_clone();
        let relay = Keypair::new();
        let program_id = gmp_program_id();

        // Fund relay
        let fund_ix = solana_sdk::system_instruction::transfer(&admin.pubkey(), &relay.pubkey(), 1_000_000_000);
        send_tx(&mut context, &admin, &[fund_ix], &[]).await.unwrap();

        // Initialize and add relay, but do NOT set trusted remote
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        let add_relay_ix = create_add_relay_ix(program_id, admin.pubkey(), admin.pubkey(), relay.pubkey());
        send_tx(&mut context, &admin, &[init_ix, add_relay_ix], &[]).await.unwrap();

        // Try to deliver message - should fail because no trusted remote is configured
        let src_addr = [0x66; 32];
        let payload = vec![0x01, 0x02];
        let deliver_ix = create_deliver_message_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM, // no trusted remote configured for this chain
            src_addr,
            payload,
            1, // nonce
        );
        let result = send_tx(&mut context, &relay, &[deliver_ix], &[]).await;
        assert!(result.is_err(), "Message from chain with no trusted remote should be rejected");
    }

    /// 20. Test: Non-admin cannot set trusted remote
    /// Verifies that only the admin can configure trusted remote addresses.
    /// Why: Admin-only access prevents unauthorized trust configuration changes.
    #[tokio::test]
    async fn test_set_trusted_remote_unauthorized() {
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

        // Non-admin tries to set trusted remote - should fail
        let trusted_addr = [0x77; 32];
        let set_trusted_ix = create_set_trusted_remote_ix(program_id, non_admin.pubkey(), non_admin.pubkey(), CHAIN_ID_MVM, trusted_addr);
        let result = send_tx(&mut context, &non_admin, &[set_trusted_ix], &[]).await;
        assert!(result.is_err(), "Non-admin should not be able to set trusted remote");
    }

    /// 21. Test: Lower nonce rejected
    /// Verifies that delivering a message with a nonce lower than the last processed fails.
    /// Why: Strictly increasing nonces prevent out-of-order message processing attacks.
    #[tokio::test]
    async fn test_deliver_message_rejects_lower_nonce() {
        let pt = program_test();
        let mut context = pt.start_with_context().await;
        let admin = context.payer.insecure_clone();
        let relay = Keypair::new();
        let program_id = gmp_program_id();

        // Fund relay
        let fund_ix = solana_sdk::system_instruction::transfer(&admin.pubkey(), &relay.pubkey(), 1_000_000_000);
        send_tx(&mut context, &admin, &[fund_ix], &[]).await.unwrap();

        // Initialize, add relay, set trusted remote
        let init_ix = create_initialize_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_SVM);
        let add_relay_ix = create_add_relay_ix(program_id, admin.pubkey(), admin.pubkey(), relay.pubkey());
        let src_addr = [0x88; 32];
        let set_trusted_ix = create_set_trusted_remote_ix(program_id, admin.pubkey(), admin.pubkey(), CHAIN_ID_MVM, src_addr);
        send_tx(&mut context, &admin, &[init_ix, add_relay_ix, set_trusted_ix], &[]).await.unwrap();

        // Deliver message with nonce = 5
        let payload1 = vec![0x01];
        let deliver_ix = create_deliver_message_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            src_addr,
            payload1,
            5, // nonce
        );
        send_tx(&mut context, &relay, &[deliver_ix], &[]).await.unwrap();

        // Warp to a new slot to ensure transaction uniqueness in test framework
        context.warp_to_slot(100).unwrap();

        // Try to deliver with lower nonce = 3 - should fail
        let payload2 = vec![0x02];
        let deliver_ix_lower = create_deliver_message_ix(
            program_id,
            relay.pubkey(),
            relay.pubkey(),
            mock_receiver_id(),
            CHAIN_ID_MVM,
            src_addr,
            payload2,
            3, // lower than 5 - should fail
        );
        let result = send_tx(&mut context, &relay, &[deliver_ix_lower], &[]).await;
        assert!(result.is_err(), "Lower nonce should be rejected");
    }
}
