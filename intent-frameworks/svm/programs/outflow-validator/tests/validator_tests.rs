//! Integration tests for the outflow validator program.
//!
//! These tests verify the full functionality of the outflow validator including:
//! - Program initialization
//! - Receiving intent requirements via GMP (lz_receive)
//! - Fulfilling intents with token transfers
//! - Sending fulfillment proofs via GMP

use borsh::{BorshDeserialize, BorshSerialize};
use gmp_common::messages::IntentRequirements;
use native_gmp_endpoint::{
    instruction::NativeGmpInstruction,
    state::seeds as gmp_seeds,
};
use outflow_validator::{
    instruction::OutflowInstruction,
    seeds,
    state::{ConfigAccount, IntentRequirementsAccount},
};
use solana_program_test::{processor, ProgramTest};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};

// ============================================================================
// TEST CONSTANTS
// ============================================================================

const HUB_CHAIN_ID: u32 = 30325; // Movement chain ID
const SVM_CHAIN_ID: u32 = 30168; // Solana chain ID
const FAR_FUTURE_EXPIRY: u64 = 9999999999; // Far future timestamp for non-expiry tests

/// Deterministic program ID for the outflow validator in tests.
/// Uses a simple sequential pattern (0x01..0x20) for easy identification in logs.
fn outflow_program_id() -> Pubkey {
    Pubkey::new_from_array([
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E,
        0x1F, 0x20,
    ])
}

/// Deterministic GMP endpoint program ID for tests.
/// Uses a sequential pattern (0x21..0x40) distinct from outflow_program_id.
fn gmp_endpoint_id() -> Pubkey {
    Pubkey::new_from_array([
        0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D, 0x2E, 0x2F,
        0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E,
        0x3F, 0x40,
    ])
}

/// Deterministic trusted hub address (32 bytes) for GMP message verification.
/// Non-zero first/last bytes make it easy to spot in hex dumps.
fn trusted_hub_addr() -> [u8; 32] {
    let mut addr = [0u8; 32];
    addr[0] = 0xAA;
    addr[31] = 0xBB;
    addr
}

/// Deterministic intent ID for test cases.
/// Non-zero first/last bytes distinguish it from other test addresses.
fn test_intent_id() -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = 0x11;
    id[31] = 0x22;
    id
}

// ============================================================================
// TEST HELPERS
// ============================================================================

/// Creates a ProgramTest instance configured with the outflow validator.
/// Uses native processor (not BPF) for faster test execution.
fn program_test() -> ProgramTest {
    let program_id = outflow_program_id();
    let mut pt = ProgramTest::new(
        "outflow_validator",
        program_id,
        processor!(outflow_validator::processor::process_instruction),
    );
    pt.prefer_bpf(false);
    pt
}

/// Builds an Initialize instruction with the correct account layout.
/// Derives the config PDA and sets up admin as signer.
fn create_initialize_ix(
    program_id: Pubkey,
    admin: Pubkey,
    gmp_endpoint: Pubkey,
    hub_chain_id: u32,
    trusted_hub_addr: [u8; 32],
) -> solana_sdk::instruction::Instruction {
    let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], &program_id);

    let instruction = OutflowInstruction::Initialize {
        gmp_endpoint,
        hub_chain_id,
        trusted_hub_addr,
    };

    solana_sdk::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_sdk::instruction::AccountMeta::new(config_pda, false),
            solana_sdk::instruction::AccountMeta::new(admin, true),
            solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Builds an LzReceive instruction simulating a GMP message delivery.
/// Derives both config and requirements PDAs from the intent_id.
fn create_lz_receive_ix(
    program_id: Pubkey,
    payer: Pubkey,
    src_chain_id: u32,
    src_addr: [u8; 32],
    payload: Vec<u8>,
    intent_id: [u8; 32],
) -> solana_sdk::instruction::Instruction {
    let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], &program_id);
    let (requirements_pda, _) = Pubkey::find_program_address(
        &[seeds::REQUIREMENTS_SEED, &intent_id],
        &program_id,
    );

    let instruction = OutflowInstruction::LzReceive {
        src_chain_id,
        src_addr,
        payload,
    };

    solana_sdk::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_sdk::instruction::AccountMeta::new(requirements_pda, false),
            solana_sdk::instruction::AccountMeta::new_readonly(config_pda, false),
            solana_sdk::instruction::AccountMeta::new_readonly(payer, true), // authority
            solana_sdk::instruction::AccountMeta::new(payer, true),          // payer
            solana_sdk::instruction::AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Sends a transaction and waits for confirmation.
/// Returns Ok on success, Err on any failure (signature, simulation, etc).
async fn send_tx(
    context: &mut solana_program_test::ProgramTestContext,
    payer: &Keypair,
    instructions: &[solana_sdk::instruction::Instruction],
    additional_signers: &[&Keypair],
) -> Result<(), solana_program_test::BanksClientError> {
    let blockhash = context.banks_client.get_latest_blockhash().await?;
    let mut signers = vec![payer];
    signers.extend(additional_signers);
    let tx = Transaction::new_signed_with_payer(
        instructions,
        Some(&payer.pubkey()),
        &signers,
        blockhash,
    );
    context.banks_client.process_transaction(tx).await
}

/// Reads and deserializes an account's data into the specified type.
/// Panics if the account doesn't exist or deserialization fails.
async fn read_account<T: BorshDeserialize>(
    context: &mut solana_program_test::ProgramTestContext,
    pubkey: Pubkey,
) -> T {
    let account = context
        .banks_client
        .get_account(pubkey)
        .await
        .unwrap()
        .unwrap();
    T::try_from_slice(&account.data).unwrap()
}

/// Creates a ProgramTest instance with outflow validator and SPL token.
/// Required for fulfill_intent tests that involve token transfers.
fn program_test_with_spl() -> ProgramTest {
    let program_id = outflow_program_id();
    let mut pt = ProgramTest::new(
        "outflow_validator",
        program_id,
        processor!(outflow_validator::processor::process_instruction),
    );
    pt.add_program(
        "spl_token",
        spl_token::id(),
        processor!(spl_token::processor::Processor::process),
    );
    pt.prefer_bpf(false);
    pt
}

/// Creates an SPL token mint.
async fn create_mint(
    context: &mut solana_program_test::ProgramTestContext,
    payer: &Keypair,
    mint_authority: &Pubkey,
    decimals: u8,
) -> Pubkey {
    use solana_program::program_pack::Pack;
    let mint = Keypair::new();
    let rent = context.banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);

    let create_mint_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_rent,
        spl_token::state::Mint::LEN as u64,
        &spl_token::id(),
    );
    let init_mint_ix = spl_token::instruction::initialize_mint2(
        &spl_token::id(),
        &mint.pubkey(),
        mint_authority,
        None,
        decimals,
    )
    .unwrap();

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_mint_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &[payer, &mint],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();
    mint.pubkey()
}

/// Creates an SPL token account.
async fn create_token_account(
    context: &mut solana_program_test::ProgramTestContext,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Pubkey {
    use solana_program::program_pack::Pack;
    let token_account = Keypair::new();
    let rent = context.banks_client.get_rent().await.unwrap();
    let token_rent = rent.minimum_balance(spl_token::state::Account::LEN);

    let create_ix = solana_sdk::system_instruction::create_account(
        &payer.pubkey(),
        &token_account.pubkey(),
        token_rent,
        spl_token::state::Account::LEN as u64,
        &spl_token::id(),
    );
    let init_ix = spl_token::instruction::initialize_account3(
        &spl_token::id(),
        &token_account.pubkey(),
        mint,
        owner,
    )
    .unwrap();

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ix, init_ix],
        Some(&payer.pubkey()),
        &[payer, &token_account],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();
    token_account.pubkey()
}

/// Mints tokens to a token account.
async fn mint_tokens(
    context: &mut solana_program_test::ProgramTestContext,
    payer: &Keypair,
    mint: &Pubkey,
    mint_authority: &Keypair,
    destination: &Pubkey,
    amount: u64,
) {
    let mint_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        mint,
        destination,
        &mint_authority.pubkey(),
        &[],
        amount,
    )
    .unwrap();

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[mint_ix],
        Some(&payer.pubkey()),
        &[payer, mint_authority],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();
}

/// Builds a FulfillIntent instruction with the correct account layout.
fn create_fulfill_intent_ix(
    program_id: Pubkey,
    solver: Pubkey,
    solver_token_account: Pubkey,
    recipient_token_account: Pubkey,
    token_mint: Pubkey,
    gmp_endpoint: Pubkey,
    intent_id: [u8; 32],
) -> solana_sdk::instruction::Instruction {
    let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], &program_id);
    let (requirements_pda, _) = Pubkey::find_program_address(
        &[seeds::REQUIREMENTS_SEED, &intent_id],
        &program_id,
    );

    let instruction = OutflowInstruction::FulfillIntent { intent_id };

    solana_sdk::instruction::Instruction {
        program_id,
        accounts: vec![
            solana_sdk::instruction::AccountMeta::new(requirements_pda, false),
            solana_sdk::instruction::AccountMeta::new_readonly(config_pda, false),
            solana_sdk::instruction::AccountMeta::new_readonly(solver, true),
            solana_sdk::instruction::AccountMeta::new(solver_token_account, false),
            solana_sdk::instruction::AccountMeta::new(recipient_token_account, false),
            solana_sdk::instruction::AccountMeta::new_readonly(token_mint, false),
            solana_sdk::instruction::AccountMeta::new_readonly(spl_token::id(), false),
            solana_sdk::instruction::AccountMeta::new_readonly(gmp_endpoint, false),
            // GMP endpoint accounts would go here for the CPI
        ],
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Helper to set up requirements via lz_receive.
async fn setup_requirements(
    context: &mut solana_program_test::ProgramTestContext,
    admin: &Keypair,
    program_id: Pubkey,
    intent_id: [u8; 32],
    recipient: Pubkey,
    token_mint: Pubkey,
    authorized_solver: Pubkey,
    amount: u64,
    expiry: u64,
) {
    // Initialize first
    let init_ix = create_initialize_ix(
        program_id,
        admin.pubkey(),
        gmp_endpoint_id(),
        HUB_CHAIN_ID,
        trusted_hub_addr(),
    );
    send_tx(context, admin, &[init_ix], &[]).await.unwrap();

    // Send lz_receive to store requirements
    let requirements = IntentRequirements {
        intent_id,
        requester_addr: recipient.to_bytes(),
        amount_required: amount,
        token_addr: token_mint.to_bytes(),
        solver_addr: authorized_solver.to_bytes(),
        expiry,
    };
    let payload = requirements.encode().to_vec();

    let lz_receive_ix = create_lz_receive_ix(
        program_id,
        admin.pubkey(),
        HUB_CHAIN_ID,
        trusted_hub_addr(),
        payload,
        intent_id,
    );
    send_tx(context, admin, &[lz_receive_ix], &[]).await.unwrap();
}

/// Creates a ProgramTest instance with outflow validator, SPL token, and native GMP endpoint.
/// Required for happy path tests that need the full GMP CPI flow.
fn program_test_with_spl_and_gmp() -> ProgramTest {
    let program_id = outflow_program_id();
    let mut pt = ProgramTest::new(
        "outflow_validator",
        program_id,
        processor!(outflow_validator::processor::process_instruction),
    );
    pt.add_program(
        "spl_token",
        spl_token::id(),
        processor!(spl_token::processor::Processor::process),
    );
    pt.add_program(
        "native_gmp_endpoint",
        gmp_endpoint_id(),
        processor!(native_gmp_endpoint::processor::process_instruction),
    );
    pt.prefer_bpf(false);
    pt
}

/// Initialize the native GMP endpoint.
async fn initialize_gmp_endpoint(
    context: &mut solana_program_test::ProgramTestContext,
    admin: &Keypair,
    chain_id: u32,
) {
    let gmp_program = gmp_endpoint_id();
    let (config_pda, _) = Pubkey::find_program_address(&[gmp_seeds::CONFIG_SEED], &gmp_program);

    let init_ix = solana_sdk::instruction::Instruction {
        program_id: gmp_program,
        accounts: vec![
            solana_sdk::instruction::AccountMeta::new(config_pda, false),
            solana_sdk::instruction::AccountMeta::new_readonly(admin.pubkey(), true),
            solana_sdk::instruction::AccountMeta::new(admin.pubkey(), true),
            solana_sdk::instruction::AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: NativeGmpInstruction::Initialize { chain_id }.try_to_vec().unwrap(),
    };
    send_tx(context, admin, &[init_ix], &[]).await.unwrap();
}

/// Builds a FulfillIntent instruction with full GMP accounts for happy path testing.
fn create_fulfill_intent_ix_with_gmp(
    program_id: Pubkey,
    solver: Pubkey,
    solver_token_account: Pubkey,
    recipient_token_account: Pubkey,
    token_mint: Pubkey,
    gmp_endpoint: Pubkey,
    intent_id: [u8; 32],
    payer: Pubkey,
    hub_chain_id: u32,
) -> solana_sdk::instruction::Instruction {
    let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], &program_id);
    let (requirements_pda, _) = Pubkey::find_program_address(
        &[seeds::REQUIREMENTS_SEED, &intent_id],
        &program_id,
    );

    // GMP endpoint accounts for Send CPI
    let (gmp_config_pda, _) = Pubkey::find_program_address(&[gmp_seeds::CONFIG_SEED], &gmp_endpoint);
    let (nonce_out_pda, _) = Pubkey::find_program_address(
        &[gmp_seeds::NONCE_OUT_SEED, &hub_chain_id.to_le_bytes()],
        &gmp_endpoint,
    );

    let instruction = OutflowInstruction::FulfillIntent { intent_id };

    solana_sdk::instruction::Instruction {
        program_id,
        accounts: vec![
            // Outflow validator accounts
            solana_sdk::instruction::AccountMeta::new(requirements_pda, false),
            solana_sdk::instruction::AccountMeta::new_readonly(config_pda, false),
            solana_sdk::instruction::AccountMeta::new_readonly(solver, true),
            solana_sdk::instruction::AccountMeta::new(solver_token_account, false),
            solana_sdk::instruction::AccountMeta::new(recipient_token_account, false),
            solana_sdk::instruction::AccountMeta::new_readonly(token_mint, false),
            solana_sdk::instruction::AccountMeta::new_readonly(spl_token::id(), false),
            solana_sdk::instruction::AccountMeta::new_readonly(gmp_endpoint, false),
            // GMP Send accounts (passed as remaining accounts for CPI)
            solana_sdk::instruction::AccountMeta::new_readonly(gmp_config_pda, false),
            solana_sdk::instruction::AccountMeta::new(nonce_out_pda, false),
            solana_sdk::instruction::AccountMeta::new_readonly(solver, true), // sender for CPI
            solana_sdk::instruction::AccountMeta::new(payer, true), // payer
            solana_sdk::instruction::AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Gets the token balance for an account.
async fn get_token_balance(
    context: &mut solana_program_test::ProgramTestContext,
    token_account: Pubkey,
) -> u64 {
    use solana_program::program_pack::Pack;
    let account = context.banks_client.get_account(token_account).await.unwrap().unwrap();
    let token_data = spl_token::state::Account::unpack(&account.data).unwrap();
    token_data.amount
}

// ============================================================================
// INITIALIZATION TESTS
// ============================================================================

/// 1. Test: Initialize creates config account
/// Verifies that Initialize creates the config PDA with correct values.
/// Why: Proper initialization is required before any other operations.
#[tokio::test]
async fn test_initialize_creates_config() {
    let pt = program_test();
    let mut context = pt.start_with_context().await;
    let admin = context.payer.insecure_clone();
    let program_id = outflow_program_id();

    let init_ix = create_initialize_ix(
        program_id,
        admin.pubkey(),
        gmp_endpoint_id(),
        HUB_CHAIN_ID,
        trusted_hub_addr(),
    );
    send_tx(&mut context, &admin, &[init_ix], &[]).await.unwrap();

    // Verify config was created
    let (config_pda, _) = Pubkey::find_program_address(&[seeds::CONFIG_SEED], &program_id);
    let config: ConfigAccount = read_account(&mut context, config_pda).await;

    assert_eq!(config.admin, admin.pubkey());
    assert_eq!(config.gmp_endpoint, gmp_endpoint_id());
    assert_eq!(config.hub_chain_id, HUB_CHAIN_ID);
    assert_eq!(config.trusted_hub_addr, trusted_hub_addr());
}

/// 2. Test: Initialize fails if already initialized
/// Verifies that double initialization is rejected.
/// Why: Config must only be set once to prevent admin takeover.
#[tokio::test]
async fn test_initialize_rejects_double_init() {
    let pt = program_test();
    let mut context = pt.start_with_context().await;
    let admin = context.payer.insecure_clone();
    let program_id = outflow_program_id();

    let init_ix = create_initialize_ix(
        program_id,
        admin.pubkey(),
        gmp_endpoint_id(),
        HUB_CHAIN_ID,
        trusted_hub_addr(),
    );

    // First init succeeds
    send_tx(&mut context, &admin, &[init_ix.clone()], &[]).await.unwrap();

    // Warp to a new slot to ensure transaction uniqueness in test framework
    context.warp_to_slot(100).unwrap();

    // Second init fails
    let result = send_tx(&mut context, &admin, &[init_ix], &[]).await;
    assert!(result.is_err(), "Double init should fail");
}

// ============================================================================
// LZ_RECEIVE TESTS
// ============================================================================

/// 3. Test: Receive stores intent requirements
/// Verifies that receiving a GMP message creates the requirements PDA.
/// Why: Intent requirements must be stored for solvers to fulfill.
#[tokio::test]
async fn test_receive_stores_requirements() {
    let pt = program_test();
    let mut context = pt.start_with_context().await;
    let admin = context.payer.insecure_clone();
    let program_id = outflow_program_id();

    // Initialize first
    let init_ix = create_initialize_ix(
        program_id,
        admin.pubkey(),
        gmp_endpoint_id(),
        HUB_CHAIN_ID,
        trusted_hub_addr(),
    );
    send_tx(&mut context, &admin, &[init_ix], &[]).await.unwrap();

    // Create intent requirements payload
    let intent_id = test_intent_id();
    let requirements = IntentRequirements {
        intent_id,
        requester_addr: admin.pubkey().to_bytes(),
        amount_required: 1_000_000,
        token_addr: Pubkey::new_unique().to_bytes(),
        solver_addr: [0u8; 32], // any solver allowed
        expiry: FAR_FUTURE_EXPIRY,
    };
    let payload = requirements.encode().to_vec();

    // Send lz_receive
    let lz_receive_ix = create_lz_receive_ix(
        program_id,
        admin.pubkey(),
        HUB_CHAIN_ID,
        trusted_hub_addr(),
        payload,
        intent_id,
    );
    send_tx(&mut context, &admin, &[lz_receive_ix], &[]).await.unwrap();

    // Verify requirements were stored
    let (requirements_pda, _) = Pubkey::find_program_address(
        &[seeds::REQUIREMENTS_SEED, &intent_id],
        &program_id,
    );
    let stored: IntentRequirementsAccount = read_account(&mut context, requirements_pda).await;

    assert_eq!(stored.intent_id, intent_id);
    assert_eq!(stored.amount_required, 1_000_000);
    assert!(!stored.fulfilled);
}

/// 4. Test: Receive is idempotent
/// Verifies that duplicate messages don't fail or overwrite.
/// Why: Network retries must not corrupt state or cause failures.
#[tokio::test]
async fn test_receive_idempotent() {
    let pt = program_test();
    let mut context = pt.start_with_context().await;
    let admin = context.payer.insecure_clone();
    let program_id = outflow_program_id();

    // Initialize first
    let init_ix = create_initialize_ix(
        program_id,
        admin.pubkey(),
        gmp_endpoint_id(),
        HUB_CHAIN_ID,
        trusted_hub_addr(),
    );
    send_tx(&mut context, &admin, &[init_ix], &[]).await.unwrap();

    // Create intent requirements payload
    let intent_id = test_intent_id();
    let requirements = IntentRequirements {
        intent_id,
        requester_addr: admin.pubkey().to_bytes(),
        amount_required: 1_000_000,
        token_addr: Pubkey::new_unique().to_bytes(),
        solver_addr: [0u8; 32],
        expiry: FAR_FUTURE_EXPIRY,
    };
    let payload = requirements.encode().to_vec();

    // First receive succeeds
    let lz_receive_ix = create_lz_receive_ix(
        program_id,
        admin.pubkey(),
        HUB_CHAIN_ID,
        trusted_hub_addr(),
        payload.clone(),
        intent_id,
    );
    send_tx(&mut context, &admin, &[lz_receive_ix.clone()], &[]).await.unwrap();

    // Second receive also succeeds (idempotent)
    send_tx(&mut context, &admin, &[lz_receive_ix], &[]).await.unwrap();
}

/// 5. Test: Receive rejects untrusted source
/// Verifies that messages from wrong chain/address are rejected.
/// Why: Only the trusted hub can send intent requirements.
#[tokio::test]
async fn test_receive_rejects_untrusted_source() {
    let pt = program_test();
    let mut context = pt.start_with_context().await;
    let admin = context.payer.insecure_clone();
    let program_id = outflow_program_id();

    // Initialize first
    let init_ix = create_initialize_ix(
        program_id,
        admin.pubkey(),
        gmp_endpoint_id(),
        HUB_CHAIN_ID,
        trusted_hub_addr(),
    );
    send_tx(&mut context, &admin, &[init_ix], &[]).await.unwrap();

    // Create intent requirements payload
    let intent_id = test_intent_id();
    let requirements = IntentRequirements {
        intent_id,
        requester_addr: admin.pubkey().to_bytes(),
        amount_required: 1_000_000,
        token_addr: Pubkey::new_unique().to_bytes(),
        solver_addr: [0u8; 32],
        expiry: FAR_FUTURE_EXPIRY,
    };
    let payload = requirements.encode().to_vec();

    // Try with wrong chain ID
    let lz_receive_ix = create_lz_receive_ix(
        program_id,
        admin.pubkey(),
        12345, // wrong chain ID
        trusted_hub_addr(),
        payload.clone(),
        intent_id,
    );
    let result = send_tx(&mut context, &admin, &[lz_receive_ix], &[]).await;
    assert!(result.is_err(), "Wrong chain ID should be rejected");

    // Try with wrong source address
    let wrong_addr = [0xFFu8; 32];
    let lz_receive_ix = create_lz_receive_ix(
        program_id,
        admin.pubkey(),
        HUB_CHAIN_ID,
        wrong_addr, // wrong address
        payload,
        intent_id,
    );
    let result = send_tx(&mut context, &admin, &[lz_receive_ix], &[]).await;
    assert!(result.is_err(), "Wrong source address should be rejected");
}

/// 6. Test: Receive rejects invalid payload
/// Verifies that malformed GMP messages are rejected.
/// Why: Prevents processing of corrupted or malicious messages.
#[tokio::test]
async fn test_receive_rejects_invalid_payload() {
    let pt = program_test();
    let mut context = pt.start_with_context().await;
    let admin = context.payer.insecure_clone();
    let program_id = outflow_program_id();

    // Initialize first
    let init_ix = create_initialize_ix(
        program_id,
        admin.pubkey(),
        gmp_endpoint_id(),
        HUB_CHAIN_ID,
        trusted_hub_addr(),
    );
    send_tx(&mut context, &admin, &[init_ix], &[]).await.unwrap();

    // Invalid payload (too short)
    let invalid_payload = vec![0x01, 0x02, 0x03];
    let intent_id = test_intent_id();

    let lz_receive_ix = create_lz_receive_ix(
        program_id,
        admin.pubkey(),
        HUB_CHAIN_ID,
        trusted_hub_addr(),
        invalid_payload,
        intent_id,
    );
    let result = send_tx(&mut context, &admin, &[lz_receive_ix], &[]).await;
    assert!(result.is_err(), "Invalid payload should be rejected");
}

// ============================================================================
// FULFILL_INTENT TESTS
// ============================================================================

/// 7. Test: FulfillIntent rejects already fulfilled intent
/// Verifies that double fulfillment is rejected.
/// Why: Prevents solver from claiming payment twice.
#[tokio::test]
async fn test_fulfill_intent_rejects_already_fulfilled() {
    let pt = program_test_with_spl();
    let mut context = pt.start_with_context().await;
    let admin = context.payer.insecure_clone();
    let program_id = outflow_program_id();
    let solver = Keypair::new();
    let intent_id = test_intent_id();

    // Create mint and token accounts
    let mint = create_mint(&mut context, &admin, &admin.pubkey(), 6).await;
    let solver_token = create_token_account(&mut context, &admin, &mint, &solver.pubkey()).await;
    let recipient_token = create_token_account(&mut context, &admin, &mint, &admin.pubkey()).await;

    // Mint tokens to solver
    mint_tokens(&mut context, &admin, &mint, &admin, &solver_token, 1_000_000).await;

    // Setup requirements with zero-address solver (any solver allowed)
    setup_requirements(
        &mut context,
        &admin,
        program_id,
        intent_id,
        admin.pubkey(),
        mint,
        Pubkey::default(), // any solver
        500_000,
        FAR_FUTURE_EXPIRY,
    ).await;

    // Manually mark as fulfilled by modifying the account data directly
    let (requirements_pda, _) = Pubkey::find_program_address(
        &[seeds::REQUIREMENTS_SEED, &intent_id],
        &program_id,
    );
    let mut req_account = context.banks_client.get_account(requirements_pda).await.unwrap().unwrap();
    let mut requirements = IntentRequirementsAccount::try_from_slice(&req_account.data).unwrap();
    requirements.fulfilled = true;
    requirements.serialize(&mut &mut req_account.data[..]).unwrap();
    context.set_account(&requirements_pda, &req_account.into());

    // Try to fulfill - should fail
    let fulfill_ix = create_fulfill_intent_ix(
        program_id,
        solver.pubkey(),
        solver_token,
        recipient_token,
        mint,
        gmp_endpoint_id(),
        intent_id,
    );
    let result = send_tx(&mut context, &admin, &[fulfill_ix], &[&solver]).await;
    assert!(result.is_err(), "Already fulfilled intent should be rejected");
}

/// 8. Test: FulfillIntent rejects expired intent
/// Verifies that expired intents cannot be fulfilled.
/// Why: Protects solver from fulfilling intents user no longer wants.
#[tokio::test]
async fn test_fulfill_intent_rejects_expired() {
    let pt = program_test_with_spl();
    let mut context = pt.start_with_context().await;
    let admin = context.payer.insecure_clone();
    let program_id = outflow_program_id();
    let solver = Keypair::new();
    let intent_id = test_intent_id();

    // Create mint and token accounts
    let mint = create_mint(&mut context, &admin, &admin.pubkey(), 6).await;
    let solver_token = create_token_account(&mut context, &admin, &mint, &solver.pubkey()).await;
    let recipient_token = create_token_account(&mut context, &admin, &mint, &admin.pubkey()).await;
    mint_tokens(&mut context, &admin, &mint, &admin, &solver_token, 1_000_000).await;

    // Setup requirements with expiry in the past (1 = very old timestamp)
    setup_requirements(
        &mut context,
        &admin,
        program_id,
        intent_id,
        admin.pubkey(),
        mint,
        Pubkey::default(),
        500_000,
        1, // Expired
    ).await;

    // Try to fulfill - should fail due to expiry
    let fulfill_ix = create_fulfill_intent_ix(
        program_id,
        solver.pubkey(),
        solver_token,
        recipient_token,
        mint,
        gmp_endpoint_id(),
        intent_id,
    );
    let result = send_tx(&mut context, &admin, &[fulfill_ix], &[&solver]).await;
    assert!(result.is_err(), "Expired intent should be rejected");
}

/// 9. Test: FulfillIntent rejects unauthorized solver
/// Verifies that only the authorized solver can fulfill.
/// Why: Ensures intent creator's solver preference is respected.
#[tokio::test]
async fn test_fulfill_intent_rejects_unauthorized_solver() {
    let pt = program_test_with_spl();
    let mut context = pt.start_with_context().await;
    let admin = context.payer.insecure_clone();
    let program_id = outflow_program_id();
    let authorized_solver = Keypair::new();
    let unauthorized_solver = Keypair::new();
    let intent_id = test_intent_id();

    // Create mint and token accounts
    let mint = create_mint(&mut context, &admin, &admin.pubkey(), 6).await;
    let solver_token = create_token_account(&mut context, &admin, &mint, &unauthorized_solver.pubkey()).await;
    let recipient_token = create_token_account(&mut context, &admin, &mint, &admin.pubkey()).await;
    mint_tokens(&mut context, &admin, &mint, &admin, &solver_token, 1_000_000).await;

    // Setup requirements with specific authorized solver
    setup_requirements(
        &mut context,
        &admin,
        program_id,
        intent_id,
        admin.pubkey(),
        mint,
        authorized_solver.pubkey(), // Only this solver allowed
        500_000,
        FAR_FUTURE_EXPIRY,
    ).await;

    // Try to fulfill with unauthorized solver - should fail
    let fulfill_ix = create_fulfill_intent_ix(
        program_id,
        unauthorized_solver.pubkey(),
        solver_token,
        recipient_token,
        mint,
        gmp_endpoint_id(),
        intent_id,
    );
    let result = send_tx(&mut context, &admin, &[fulfill_ix], &[&unauthorized_solver]).await;
    assert!(result.is_err(), "Unauthorized solver should be rejected");
}

/// 10. Test: FulfillIntent rejects token mismatch
/// Verifies that wrong token mint is rejected.
/// Why: Prevents solver from fulfilling with different token.
#[tokio::test]
async fn test_fulfill_intent_rejects_token_mismatch() {
    let pt = program_test_with_spl();
    let mut context = pt.start_with_context().await;
    let admin = context.payer.insecure_clone();
    let program_id = outflow_program_id();
    let solver = Keypair::new();
    let intent_id = test_intent_id();

    // Create two different mints
    let required_mint = create_mint(&mut context, &admin, &admin.pubkey(), 6).await;
    let wrong_mint = create_mint(&mut context, &admin, &admin.pubkey(), 6).await;

    // Create token accounts with wrong mint
    let solver_token = create_token_account(&mut context, &admin, &wrong_mint, &solver.pubkey()).await;
    let recipient_token = create_token_account(&mut context, &admin, &wrong_mint, &admin.pubkey()).await;
    mint_tokens(&mut context, &admin, &wrong_mint, &admin, &solver_token, 1_000_000).await;

    // Setup requirements expecting required_mint
    setup_requirements(
        &mut context,
        &admin,
        program_id,
        intent_id,
        admin.pubkey(),
        required_mint, // Expects this mint
        Pubkey::default(),
        500_000,
        FAR_FUTURE_EXPIRY,
    ).await;

    // Try to fulfill with wrong mint - should fail
    let fulfill_ix = create_fulfill_intent_ix(
        program_id,
        solver.pubkey(),
        solver_token,
        recipient_token,
        wrong_mint, // Wrong mint!
        gmp_endpoint_id(),
        intent_id,
    );
    let result = send_tx(&mut context, &admin, &[fulfill_ix], &[&solver]).await;
    assert!(result.is_err(), "Token mismatch should be rejected");
}

/// 11. Test: FulfillIntent rejects non-existent requirements
/// Verifies that fulfilling unknown intent_id fails.
/// Why: Prevents fulfillment of intents that were never created.
#[tokio::test]
async fn test_fulfill_intent_rejects_requirements_not_found() {
    let pt = program_test_with_spl();
    let mut context = pt.start_with_context().await;
    let admin = context.payer.insecure_clone();
    let program_id = outflow_program_id();
    let solver = Keypair::new();

    // Use an intent_id that was never created
    let unknown_intent_id = [0xFFu8; 32];

    // Create mint and token accounts
    let mint = create_mint(&mut context, &admin, &admin.pubkey(), 6).await;
    let solver_token = create_token_account(&mut context, &admin, &mint, &solver.pubkey()).await;
    let recipient_token = create_token_account(&mut context, &admin, &mint, &admin.pubkey()).await;
    mint_tokens(&mut context, &admin, &mint, &admin, &solver_token, 1_000_000).await;

    // Initialize config but don't create requirements
    let init_ix = create_initialize_ix(
        program_id,
        admin.pubkey(),
        gmp_endpoint_id(),
        HUB_CHAIN_ID,
        trusted_hub_addr(),
    );
    send_tx(&mut context, &admin, &[init_ix], &[]).await.unwrap();

    // Try to fulfill non-existent intent - should fail
    let fulfill_ix = create_fulfill_intent_ix(
        program_id,
        solver.pubkey(),
        solver_token,
        recipient_token,
        mint,
        gmp_endpoint_id(),
        unknown_intent_id,
    );
    let result = send_tx(&mut context, &admin, &[fulfill_ix], &[&solver]).await;
    assert!(result.is_err(), "Non-existent requirements should be rejected");
}

/// 12. Test: FulfillIntent rejects recipient mismatch
/// Verifies that sending tokens to wrong recipient's account fails.
/// Why: Prevents solver from redirecting funds to unauthorized recipient.
#[tokio::test]
async fn test_fulfill_intent_rejects_recipient_mismatch() {
    let pt = program_test_with_spl();
    let mut context = pt.start_with_context().await;
    let admin = context.payer.insecure_clone();
    let program_id = outflow_program_id();
    let solver = Keypair::new();
    let intent_id = test_intent_id();
    let intended_recipient = Keypair::new(); // This is who should receive tokens

    // Create mint
    let mint = create_mint(&mut context, &admin, &admin.pubkey(), 6).await;

    // Create solver's token account
    let solver_token = create_token_account(&mut context, &admin, &mint, &solver.pubkey()).await;

    // Create token account owned by WRONG recipient (admin instead of intended_recipient)
    let wrong_recipient_token = create_token_account(&mut context, &admin, &mint, &admin.pubkey()).await;

    // Mint tokens to solver
    mint_tokens(&mut context, &admin, &mint, &admin, &solver_token, 1_000_000).await;

    // Setup requirements expecting intended_recipient
    setup_requirements(
        &mut context,
        &admin,
        program_id,
        intent_id,
        intended_recipient.pubkey(), // Intended recipient
        mint,
        Pubkey::default(),
        500_000,
        FAR_FUTURE_EXPIRY,
    ).await;

    // Try to fulfill with wrong recipient's token account - should fail
    let fulfill_ix = create_fulfill_intent_ix(
        program_id,
        solver.pubkey(),
        solver_token,
        wrong_recipient_token, // Token account owned by admin, not intended_recipient!
        mint,
        gmp_endpoint_id(),
        intent_id,
    );
    let result = send_tx(&mut context, &admin, &[fulfill_ix], &[&solver]).await;
    assert!(result.is_err(), "Recipient mismatch should be rejected");
}

/// 13. Test: FulfillIntent succeeds with valid inputs
/// Verifies the happy path: tokens transferred, state updated, GMP message sent.
/// Why: Ensures the core fulfillment flow works end-to-end.
#[tokio::test]
async fn test_fulfill_intent_succeeds() {
    let pt = program_test_with_spl_and_gmp();
    let mut context = pt.start_with_context().await;
    let admin = context.payer.insecure_clone();
    let program_id = outflow_program_id();
    let solver = Keypair::new();
    let intent_id = test_intent_id();
    let fulfillment_amount = 500_000u64;

    // Initialize GMP endpoint first (chain_id = local SVM chain)
    initialize_gmp_endpoint(&mut context, &admin, SVM_CHAIN_ID).await;

    // Create mint and token accounts
    let mint = create_mint(&mut context, &admin, &admin.pubkey(), 6).await;
    let solver_token = create_token_account(&mut context, &admin, &mint, &solver.pubkey()).await;
    let recipient_token = create_token_account(&mut context, &admin, &mint, &admin.pubkey()).await;

    // Mint tokens to solver (more than needed)
    mint_tokens(&mut context, &admin, &mint, &admin, &solver_token, 1_000_000).await;

    // Verify initial balances
    assert_eq!(get_token_balance(&mut context, solver_token).await, 1_000_000);
    assert_eq!(get_token_balance(&mut context, recipient_token).await, 0);

    // Setup requirements with zero-address solver (any solver allowed)
    setup_requirements(
        &mut context,
        &admin,
        program_id,
        intent_id,
        admin.pubkey(),
        mint,
        Pubkey::default(), // any solver
        fulfillment_amount,
        FAR_FUTURE_EXPIRY,
    ).await;

    // Fulfill the intent
    let fulfill_ix = create_fulfill_intent_ix_with_gmp(
        program_id,
        solver.pubkey(),
        solver_token,
        recipient_token,
        mint,
        gmp_endpoint_id(),
        intent_id,
        admin.pubkey(),
        HUB_CHAIN_ID,
    );
    send_tx(&mut context, &admin, &[fulfill_ix], &[&solver]).await.unwrap();

    // Verify token balances changed
    assert_eq!(
        get_token_balance(&mut context, solver_token).await,
        1_000_000 - fulfillment_amount,
        "Solver balance should decrease"
    );
    assert_eq!(
        get_token_balance(&mut context, recipient_token).await,
        fulfillment_amount,
        "Recipient balance should increase"
    );

    // Verify requirements marked as fulfilled
    let (requirements_pda, _) = Pubkey::find_program_address(
        &[seeds::REQUIREMENTS_SEED, &intent_id],
        &program_id,
    );
    let stored: IntentRequirementsAccount = read_account(&mut context, requirements_pda).await;
    assert!(stored.fulfilled, "Requirements should be marked fulfilled");
}
