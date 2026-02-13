//! Interface tests for the outflow validator program.
//!
//! These tests verify that the interface is correctly defined and that
//! stub implementations don't panic. Full functional tests will be added
//! in Phase 2 when the implementation is complete.

use borsh::{BorshDeserialize, BorshSerialize};
use intent_outflow_validator::{
    instruction::OutflowInstruction,
    state::{ConfigAccount, IntentRequirementsAccount},
    OutflowError,
};
use solana_sdk::pubkey::Pubkey;

// ============================================================================
// TEST CONSTANTS
// ============================================================================

const DUMMY_AMOUNT: u64 = 1_000_000;
const DUMMY_EXPIRY: u64 = 1000;
const DUMMY_HUB_CHAIN_ID: u32 = 1;

// ============================================================================
// TEST HELPERS
// ============================================================================

fn dummy_intent_id() -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = 0xAA;
    id[31] = 0xBB;
    id
}

fn dummy_addr_1() -> [u8; 32] {
    let mut addr = [0u8; 32];
    addr[0] = 0x11;
    addr[31] = 0x22;
    addr
}

fn dummy_addr_2() -> [u8; 32] {
    let mut addr = [0u8; 32];
    addr[0] = 0x33;
    addr[31] = 0x44;
    addr
}

fn dummy_payload() -> Vec<u8> {
    vec![0x01, 0x02, 0x03, 0x04]
}

// ============================================================================
// INSTRUCTION SERIALIZATION TESTS
// ============================================================================

/// 1. Test: Initialize instruction serialization roundtrip
/// Verifies that Initialize instruction can be serialized and deserialized.
#[test]
fn test_initialize_instruction_roundtrip() {
    let original_endpoint = Pubkey::new_unique();
    let original_chain_id = DUMMY_HUB_CHAIN_ID;
    let original_addr = dummy_addr_1();

    let instruction = OutflowInstruction::Initialize {
        gmp_endpoint: original_endpoint,
        hub_chain_id: original_chain_id,
        hub_gmp_endpoint_addr: original_addr,
    };

    // Roundtrip: borsh could corrupt values due to field ordering, byte width,
    // or endianness bugs in derive macros. We compare against originals below.
    let serialized = instruction.try_to_vec().unwrap();
    let deserialized = OutflowInstruction::try_from_slice(&serialized).unwrap();

    match deserialized {
        OutflowInstruction::Initialize {
            gmp_endpoint: deser_endpoint,
            hub_chain_id: deser_chain_id,
            hub_gmp_endpoint_addr: deser_hub_addr,
        } => {
            assert_eq!(deser_endpoint, original_endpoint);
            assert_eq!(deser_chain_id, original_chain_id);
            assert_eq!(deser_hub_addr, original_addr);
        }
        _ => panic!("Expected Initialize instruction"),
    }
}

/// 2. Test: Receive instruction serialization roundtrip
/// Verifies that GmpReceive instruction can be serialized and deserialized.
#[test]
fn test_receive_instruction_roundtrip() {
    let original_chain_id = DUMMY_HUB_CHAIN_ID;
    let original_addr = dummy_addr_2();
    let original_payload = dummy_payload();

    let instruction = OutflowInstruction::GmpReceive {
        src_chain_id: original_chain_id,
        remote_gmp_endpoint_addr: original_addr,
        payload: original_payload.clone(),
    };

    let serialized = instruction.try_to_vec().unwrap();
    let deserialized = OutflowInstruction::try_from_slice(&serialized).unwrap();

    match deserialized {
        OutflowInstruction::GmpReceive {
            src_chain_id: deser_chain_id,
            remote_gmp_endpoint_addr: deser_addr,
            payload: deser_payload,
        } => {
            assert_eq!(deser_chain_id, original_chain_id);
            assert_eq!(deser_addr, original_addr);
            assert_eq!(deser_payload, original_payload);
        }
        _ => panic!("Expected GmpReceive instruction"),
    }
}

/// 3. Test: FulfillIntent instruction serialization roundtrip
/// Verifies that FulfillIntent instruction can be serialized and deserialized.
#[test]
fn test_fulfill_intent_instruction_roundtrip() {
    let original_intent_id = dummy_intent_id();

    let instruction = OutflowInstruction::FulfillIntent {
        intent_id: original_intent_id,
    };

    let serialized = instruction.try_to_vec().unwrap();
    let deserialized = OutflowInstruction::try_from_slice(&serialized).unwrap();

    match deserialized {
        OutflowInstruction::FulfillIntent { intent_id: deser_intent_id } => {
            assert_eq!(deser_intent_id, original_intent_id);
        }
        _ => panic!("Expected FulfillIntent instruction"),
    }
}

// ============================================================================
// STATE SERIALIZATION TESTS
// ============================================================================

/// 4. Test: IntentRequirementsAccount serialization roundtrip
/// Verifies that requirements account state can be serialized and deserialized.
#[test]
fn test_intent_requirements_account_roundtrip() {
    let original_intent_id = dummy_intent_id();
    let original_recipient = Pubkey::new_unique();
    let original_amount = DUMMY_AMOUNT;
    let original_token = Pubkey::new_unique();
    let original_solver = Pubkey::new_unique();
    let original_expiry = DUMMY_EXPIRY;
    let original_bump = 255u8;

    let account = IntentRequirementsAccount::new(
        original_intent_id,
        original_recipient,
        original_amount,
        original_token,
        original_solver,
        original_expiry,
        original_bump,
    );

    let serialized = account.try_to_vec().unwrap();
    let deserialized = IntentRequirementsAccount::try_from_slice(&serialized).unwrap();

    assert_eq!(deserialized.intent_id, original_intent_id);
    assert_eq!(deserialized.recipient_addr, original_recipient);
    assert_eq!(deserialized.amount_required, original_amount);
    assert_eq!(deserialized.token_mint, original_token);
    assert_eq!(deserialized.authorized_solver, original_solver);
    assert_eq!(deserialized.expiry, original_expiry);
    assert!(!deserialized.fulfilled);
    assert_eq!(deserialized.bump, original_bump);
}

/// 5. Test: ConfigAccount serialization roundtrip
/// Verifies that config account state can be serialized and deserialized.
#[test]
fn test_config_account_roundtrip() {
    let original_admin = Pubkey::new_unique();
    let original_endpoint = Pubkey::new_unique();
    let original_chain_id = DUMMY_HUB_CHAIN_ID;
    let original_hub_addr = dummy_addr_1();
    let original_bump = 254u8;

    let account = ConfigAccount::new(
        original_admin,
        original_endpoint,
        original_chain_id,
        original_hub_addr,
        original_bump,
    );

    let serialized = account.try_to_vec().unwrap();
    let deserialized = ConfigAccount::try_from_slice(&serialized).unwrap();

    assert_eq!(deserialized.admin, original_admin);
    assert_eq!(deserialized.gmp_endpoint, original_endpoint);
    assert_eq!(deserialized.hub_chain_id, original_chain_id);
    assert_eq!(deserialized.hub_gmp_endpoint_addr, original_hub_addr);
    assert_eq!(deserialized.bump, original_bump);
}

// ============================================================================
// ERROR CONVERSION TESTS
// ============================================================================

/// 6. Test: Error to ProgramError conversion
/// Verifies that OutflowError can be converted to ProgramError.
#[test]
fn test_error_conversion() {
    use solana_program::program_error::ProgramError;

    let error: ProgramError = OutflowError::UnauthorizedSolver.into();
    match error {
        ProgramError::Custom(code) => {
            assert_eq!(code, OutflowError::UnauthorizedSolver as u32);
        }
        _ => panic!("Expected Custom error"),
    }
}

/// 7. Test: All error variants have unique codes
/// Verifies that each error variant maps to a unique error code.
#[test]
fn test_error_codes_unique() {
    let errors = [
        OutflowError::InvalidGmpMessage,
        OutflowError::RequirementsNotFound,
        OutflowError::RequirementsAlreadyExist,
        OutflowError::UnauthorizedSolver,
        OutflowError::AmountMismatch,
        OutflowError::TokenMismatch,
        OutflowError::RecipientMismatch,
        OutflowError::AlreadyFulfilled,
        OutflowError::IntentExpired,
        OutflowError::InvalidAccountOwner,
        OutflowError::InvalidPda,
    ];

    let codes: Vec<u32> = errors.iter().map(|e| *e as u32).collect();
    let unique_codes: std::collections::HashSet<u32> = codes.iter().cloned().collect();

    assert_eq!(
        codes.len(),
        unique_codes.len(),
        "Error codes must be unique"
    );
}
