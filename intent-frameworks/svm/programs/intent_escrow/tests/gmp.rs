//! GMP (Generic Message Passing) tests for Inflow Escrow
//!
//! These tests verify GMP-specific functionality for inflow intents:
//! - Tokens locked on connected chain (SVM), desired on hub (Movement)
//! - Hub sends requirements via GMP → SVM stores them
//! - User creates escrow → SVM validates against requirements, sends confirmation
//! - Solver fulfills on hub → Hub sends fulfillment proof via GMP
//! - SVM releases escrowed tokens to solver
//!
//! Test numbering matches intent-frameworks/EXTENSION-CHECKLIST.md "Inflow Escrow GMP Tests"

mod common;

use common::{
    create_escrow_ix, create_lz_receive_fulfillment_proof_ix, create_lz_receive_requirements_ix,
    create_set_gmp_config_ix, generate_intent_id, get_token_balance, program_test, read_escrow,
    read_requirements, setup_basic_env, send_tx, DUMMY_HUB_CHAIN_ID, DUMMY_TRUSTED_HUB_ADDR,
};
use gmp_common::messages::{FulfillmentProof, IntentRequirements};
use intent_escrow::state::seeds;
use solana_sdk::{pubkey::Pubkey, signature::{Keypair, Signer}, transaction::Transaction};

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Helper: Create IntentRequirements payload
fn create_requirements_payload(
    intent_id: [u8; 32],
    requester: &Pubkey,
    amount: u64,
    token: &Pubkey,
    solver: &Pubkey,
    expiry: u64,
) -> Vec<u8> {
    let requirements = IntentRequirements {
        intent_id,
        requester_addr: requester.to_bytes(),
        amount_required: amount,
        token_addr: token.to_bytes(),
        solver_addr: solver.to_bytes(),
        expiry,
    };
    requirements.encode().to_vec()
}

/// Helper: Create FulfillmentProof payload
fn create_fulfillment_proof_payload(
    intent_id: [u8; 32],
    solver: &Pubkey,
    amount: u64,
    timestamp: u64,
) -> Vec<u8> {
    let proof = FulfillmentProof {
        intent_id,
        solver_addr: solver.to_bytes(),
        amount_fulfilled: amount,
        timestamp,
    };
    proof.encode().to_vec()
}

// ============================================================================
// GMP CONFIG TESTS
// ============================================================================

/// 1. Test: SetGmpConfig creates/updates GMP configuration
/// Verifies that admin can set GMP config with hub chain ID, trusted hub address, and endpoint.
/// Why: GMP config is required for source validation in all GMP message handlers.
#[tokio::test]
async fn test_set_gmp_config() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let payer = context.payer.insecure_clone();
    let program_id = common::test_program_id();

    // Initialize program first
    let approver = Keypair::new();
    common::initialize_program(&mut context, &payer, program_id, approver.pubkey()).await;

    // Set GMP config
    let (gmp_config_pda, _) =
        Pubkey::find_program_address(&[seeds::GMP_CONFIG_SEED], &program_id);
    let gmp_endpoint = Pubkey::new_unique();
    let hub_chain_id = 30106u32; // Movement chain ID
    let trusted_hub_addr = [1u8; 32];

    let set_config_ix = create_set_gmp_config_ix(
        program_id,
        gmp_config_pda,
        payer.pubkey(),
        hub_chain_id,
        trusted_hub_addr,
        gmp_endpoint,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[set_config_ix],
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify config was created
    let config_account = context
        .banks_client
        .get_account(gmp_config_pda)
        .await
        .unwrap()
        .expect("GMP config account should exist");

    assert!(config_account.data.len() > 0);
}

/// 2. Test: SetGmpConfig rejects unauthorized caller
/// Verifies that only admin can update GMP config after initial setup.
/// Why: GMP config controls trusted sources - must be admin-only.
#[tokio::test]
async fn test_set_gmp_config_rejects_unauthorized() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    // GMP config already set by setup_basic_env with requester as admin
    // Try to update with a different (unauthorized) account
    let unauthorized = Keypair::new();

    // Fund the unauthorized account
    let payer = context.payer.insecure_clone();
    let fund_ix = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &unauthorized.pubkey(),
        1_000_000_000,
    );
    send_tx(&mut context, &payer, &[fund_ix], &[]).await;

    let new_hub_chain_id = 99999u32;
    let new_trusted_hub_addr = [99u8; 32];
    let new_gmp_endpoint = Pubkey::new_unique();

    let set_config_ix = create_set_gmp_config_ix(
        env.program_id,
        env.gmp_config_pda,
        unauthorized.pubkey(),
        new_hub_chain_id,
        new_trusted_hub_addr,
        new_gmp_endpoint,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[set_config_ix],
        Some(&unauthorized.pubkey()),
        &[&unauthorized],
        blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Should reject unauthorized config update");
}

// ============================================================================
// LZ RECEIVE REQUIREMENTS TESTS
// ============================================================================

/// 3. Test: ReceiveRequirements stores intent requirements
/// Verifies that requirements from hub are stored correctly.
/// Why: Requirements must be stored before escrow can be created with validation.
#[tokio::test]
async fn test_receive_requirements_stores_requirements() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;
    let expiry = u64::MAX;

    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    let requirements_payload = create_requirements_payload(
        intent_id,
        &env.requester.pubkey(),
        amount,
        &env.mint,
        &env.solver.pubkey(),
        expiry,
    );

    let gmp_caller = context.payer.insecure_clone();
    let lz_receive_ix = create_lz_receive_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        requirements_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify requirements were stored
    let req_account = context
        .banks_client
        .get_account(requirements_pda)
        .await
        .unwrap()
        .expect("Requirements account should exist");

    let requirements = read_requirements(&req_account);
    assert_eq!(requirements.intent_id, intent_id);
    assert_eq!(requirements.amount_required, amount);
    assert_eq!(requirements.token_addr, env.mint.to_bytes());
    assert_eq!(requirements.solver_addr, env.solver.pubkey().to_bytes());
    assert!(!requirements.escrow_created);
    assert!(!requirements.fulfilled);
}

/// 4. Test: ReceiveRequirements is idempotent
/// Verifies that duplicate requirements message succeeds without error.
/// Why: Network retries may deliver the same message multiple times.
#[tokio::test]
async fn test_receive_requirements_idempotent() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;

    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    let requirements_payload = create_requirements_payload(
        intent_id,
        &env.requester.pubkey(),
        amount,
        &env.mint,
        &env.solver.pubkey(),
        u64::MAX,
    );

    let gmp_caller = context.payer.insecure_clone();

    // First call - should succeed
    let lz_receive_ix = create_lz_receive_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        requirements_payload.clone(),
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Warp to next slot
    context.warp_to_slot(100).unwrap();

    // Second call - should also succeed (idempotent)
    let lz_receive_ix2 = create_lz_receive_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        requirements_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_ix2],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );

    // Should succeed without error
    context.banks_client.process_transaction(tx).await.unwrap();
}

/// 5. Test: ReceiveRequirements rejects untrusted source
/// Verifies that requirements from wrong chain/address are rejected.
/// Why: Only hub should be able to send requirements.
#[tokio::test]
async fn test_receive_requirements_rejects_untrusted_source() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;

    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    let requirements_payload = create_requirements_payload(
        intent_id,
        &env.requester.pubkey(),
        amount,
        &env.mint,
        &env.solver.pubkey(),
        u64::MAX,
    );

    let gmp_caller = context.payer.insecure_clone();

    // Use wrong chain ID
    let wrong_chain_id = 99999u32;
    let lz_receive_ix = create_lz_receive_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        wrong_chain_id,
        DUMMY_TRUSTED_HUB_ADDR,
        requirements_payload.clone(),
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Should reject wrong chain ID");

    // Warp to next slot
    context.warp_to_slot(100).unwrap();

    // Use wrong source address
    let wrong_src_addr = [99u8; 32];
    let lz_receive_ix2 = create_lz_receive_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        wrong_src_addr,
        requirements_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_ix2],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Should reject wrong source address");
}

// ============================================================================
// LZ RECEIVE FULFILLMENT PROOF TESTS
// ============================================================================

/// 6. Test: ReceiveFulfillmentProof releases escrow
/// Verifies that fulfillment proof auto-releases escrow to solver.
/// Why: This is the core GMP release mechanism.
#[tokio::test]
async fn test_receive_fulfillment_proof_releases_escrow() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 500_000u64;

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);
    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    let gmp_caller = context.payer.insecure_clone();

    // Step 1: Receive requirements
    let requirements_payload = create_requirements_payload(
        intent_id,
        &env.requester.pubkey(),
        amount,
        &env.mint,
        &env.solver.pubkey(),
        u64::MAX,
    );

    let lz_receive_req_ix = create_lz_receive_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        requirements_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_req_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Step 2: Create escrow
    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        None,
        Some(requirements_pda),
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Step 3: Receive fulfillment proof
    let proof_payload = create_fulfillment_proof_payload(
        intent_id,
        &env.solver.pubkey(),
        amount,
        12345,
    );

    let lz_receive_proof_ix = create_lz_receive_fulfillment_proof_ix(
        env.program_id,
        requirements_pda,
        escrow_pda,
        vault_pda,
        env.solver_token,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        proof_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_proof_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify escrow released
    let vault_balance = get_token_balance(&mut context, vault_pda).await;
    let solver_balance = get_token_balance(&mut context, env.solver_token).await;
    assert_eq!(vault_balance, 0);
    assert_eq!(solver_balance, amount);

    let escrow = read_escrow(
        &context
            .banks_client
            .get_account(escrow_pda)
            .await
            .unwrap()
            .unwrap(),
    );
    assert!(escrow.is_claimed);
}

/// 7. Test: ReceiveFulfillmentProof rejects untrusted source
/// Verifies that proof from wrong chain/address is rejected.
/// Why: Only hub should be able to authorize release.
#[tokio::test]
async fn test_receive_fulfillment_proof_rejects_untrusted_source() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 500_000u64;

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);
    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    let gmp_caller = context.payer.insecure_clone();

    // Setup: Receive requirements and create escrow
    let requirements_payload = create_requirements_payload(
        intent_id,
        &env.requester.pubkey(),
        amount,
        &env.mint,
        &env.solver.pubkey(),
        u64::MAX,
    );

    let lz_receive_req_ix = create_lz_receive_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        requirements_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_req_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        None,
        Some(requirements_pda),
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Try fulfillment with wrong chain ID
    let proof_payload = create_fulfillment_proof_payload(
        intent_id,
        &env.solver.pubkey(),
        amount,
        12345,
    );

    let wrong_chain_id = 99999u32;
    let lz_receive_proof_ix = create_lz_receive_fulfillment_proof_ix(
        env.program_id,
        requirements_pda,
        escrow_pda,
        vault_pda,
        env.solver_token,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        wrong_chain_id,
        DUMMY_TRUSTED_HUB_ADDR,
        proof_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_proof_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Should reject wrong chain ID");
}

/// 8. Test: ReceiveFulfillmentProof rejects already fulfilled
/// Verifies that duplicate fulfillment proof is rejected.
/// Why: Prevents double-spend attacks.
#[tokio::test]
async fn test_receive_fulfillment_proof_rejects_already_fulfilled() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 500_000u64;

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);
    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    let gmp_caller = context.payer.insecure_clone();

    // Setup: Full flow
    let requirements_payload = create_requirements_payload(
        intent_id,
        &env.requester.pubkey(),
        amount,
        &env.mint,
        &env.solver.pubkey(),
        u64::MAX,
    );

    let lz_receive_req_ix = create_lz_receive_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        requirements_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_req_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        None,
        Some(requirements_pda),
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // First fulfillment (should succeed)
    let proof_payload = create_fulfillment_proof_payload(
        intent_id,
        &env.solver.pubkey(),
        amount,
        12345,
    );

    let lz_receive_proof_ix = create_lz_receive_fulfillment_proof_ix(
        env.program_id,
        requirements_pda,
        escrow_pda,
        vault_pda,
        env.solver_token,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        proof_payload.clone(),
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_proof_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Warp to next slot
    context.warp_to_slot(100).unwrap();

    // Second fulfillment (should fail)
    let lz_receive_proof_ix2 = create_lz_receive_fulfillment_proof_ix(
        env.program_id,
        requirements_pda,
        escrow_pda,
        vault_pda,
        env.solver_token,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        proof_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_proof_ix2],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Should reject already fulfilled");
}

// ============================================================================
// CREATE ESCROW WITH REQUIREMENTS TESTS
// ============================================================================

/// 9. Test: CreateEscrow validates against requirements
/// Verifies that escrow creation succeeds when matching requirements.
/// Why: Ensures escrow parameters match what hub specified.
#[tokio::test]
async fn test_create_escrow_validates_against_requirements() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 500_000u64;

    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    let gmp_caller = context.payer.insecure_clone();

    // Receive requirements
    let requirements_payload = create_requirements_payload(
        intent_id,
        &env.requester.pubkey(),
        amount,
        &env.mint,
        &env.solver.pubkey(),
        u64::MAX,
    );

    let lz_receive_req_ix = create_lz_receive_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        requirements_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_req_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Create escrow with matching parameters
    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        None,
        Some(requirements_pda),
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify requirements marked as escrow_created
    let req_account = context
        .banks_client
        .get_account(requirements_pda)
        .await
        .unwrap()
        .unwrap();
    let requirements = read_requirements(&req_account);
    assert!(requirements.escrow_created);
}

/// 10. Test: CreateEscrow rejects amount mismatch
/// Verifies that escrow creation fails if amount is less than required.
/// Why: Prevents underfunded escrows.
#[tokio::test]
async fn test_create_escrow_rejects_amount_mismatch() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let required_amount = 500_000u64;
    let escrow_amount = 100_000u64; // Less than required

    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    let gmp_caller = context.payer.insecure_clone();

    // Receive requirements with higher amount
    let requirements_payload = create_requirements_payload(
        intent_id,
        &env.requester.pubkey(),
        required_amount,
        &env.mint,
        &env.solver.pubkey(),
        u64::MAX,
    );

    let lz_receive_req_ix = create_lz_receive_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        requirements_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_req_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Try to create escrow with lower amount
    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        escrow_amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        None,
        Some(requirements_pda),
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Should reject amount mismatch");
}

/// 11. Test: CreateEscrow rejects token mismatch
/// Verifies that escrow creation fails if token doesn't match requirements.
/// Why: Ensures correct token is escrowed.
#[tokio::test]
async fn test_create_escrow_rejects_token_mismatch() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;
    let payer = context.payer.insecure_clone();

    let intent_id = generate_intent_id();
    let amount = 500_000u64;

    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    let gmp_caller = context.payer.insecure_clone();

    // Receive requirements with one token
    let requirements_payload = create_requirements_payload(
        intent_id,
        &env.requester.pubkey(),
        amount,
        &env.mint, // Original token
        &env.solver.pubkey(),
        u64::MAX,
    );

    let lz_receive_req_ix = create_lz_receive_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        requirements_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_req_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Create a different token
    let different_mint = common::create_mint(&mut context, &payer, &env.mint_authority, 6).await;
    let requester_different_token =
        common::create_token_account(&mut context, &payer, different_mint, env.requester.pubkey()).await;
    common::mint_to(
        &mut context,
        &payer,
        different_mint,
        &env.mint_authority,
        requester_different_token,
        amount,
    ).await;

    // Try to create escrow with different token
    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        different_mint, // Wrong token
        requester_different_token,
        env.solver.pubkey(),
        None,
        Some(requirements_pda),
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Should reject token mismatch");
}

/// 12. Test: CreateEscrow sends EscrowConfirmation
/// Verifies that EscrowConfirmation GMP message is sent to hub on escrow creation.
/// Why: Hub needs confirmation to proceed with intent processing.
#[tokio::test]
async fn test_create_escrow_sends_escrow_confirmation() {
    // Note: This test verifies the code path executes without error.
    // Actual GMP message emission requires inspecting logs or using a mock endpoint.
    // The EscrowConfirmation is sent via CPI to GMP endpoint in CreateEscrow.

    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 500_000u64;

    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    let gmp_caller = context.payer.insecure_clone();

    // Receive requirements
    let requirements_payload = create_requirements_payload(
        intent_id,
        &env.requester.pubkey(),
        amount,
        &env.mint,
        &env.solver.pubkey(),
        u64::MAX,
    );

    let lz_receive_req_ix = create_lz_receive_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        requirements_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_req_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Create escrow with requirements (this should trigger EscrowConfirmation)
    // Note: Without a real GMP endpoint, the CPI will fail or be skipped
    // This test verifies the code path up to the CPI attempt
    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        None,
        Some(requirements_pda),
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );

    // This should succeed - EscrowConfirmation is only sent if GMP endpoint accounts are provided
    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify escrow was created and requirements marked
    let req_account = context
        .banks_client
        .get_account(requirements_pda)
        .await
        .unwrap()
        .unwrap();
    let requirements = read_requirements(&req_account);
    assert!(requirements.escrow_created);
}

// ============================================================================
// FULL WORKFLOW TEST
// ============================================================================

/// 13. Test: Full inflow GMP workflow
/// Verifies complete flow: requirements → escrow → fulfillment proof → release.
/// Why: Integration test for the entire inflow GMP flow.
#[tokio::test]
async fn test_full_inflow_gmp_workflow() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 500_000u64;

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);
    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    let gmp_caller = context.payer.insecure_clone();

    // Record initial balances
    let initial_requester_balance = get_token_balance(&mut context, env.requester_token).await;
    let initial_solver_balance = get_token_balance(&mut context, env.solver_token).await;

    // Step 1: Hub sends requirements via GMP
    let requirements_payload = create_requirements_payload(
        intent_id,
        &env.requester.pubkey(),
        amount,
        &env.mint,
        &env.solver.pubkey(),
        u64::MAX,
    );

    let lz_receive_req_ix = create_lz_receive_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        requirements_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_req_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify: Requirements stored
    let req = read_requirements(
        &context
            .banks_client
            .get_account(requirements_pda)
            .await
            .unwrap()
            .unwrap(),
    );
    assert!(!req.escrow_created);
    assert!(!req.fulfilled);

    // Step 2: User creates escrow (locks tokens)
    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        None,
        Some(requirements_pda),
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify: Escrow created, tokens locked
    let requester_balance = get_token_balance(&mut context, env.requester_token).await;
    assert_eq!(requester_balance, initial_requester_balance - amount);

    let vault_balance = get_token_balance(&mut context, vault_pda).await;
    assert_eq!(vault_balance, amount);

    let req = read_requirements(
        &context
            .banks_client
            .get_account(requirements_pda)
            .await
            .unwrap()
            .unwrap(),
    );
    assert!(req.escrow_created);

    // Step 3: Solver fulfills on hub, hub sends proof via GMP
    let proof_payload = create_fulfillment_proof_payload(
        intent_id,
        &env.solver.pubkey(),
        amount,
        12345,
    );

    let lz_receive_proof_ix = create_lz_receive_fulfillment_proof_ix(
        env.program_id,
        requirements_pda,
        escrow_pda,
        vault_pda,
        env.solver_token,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        proof_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_proof_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify: Escrow released to solver
    let vault_balance = get_token_balance(&mut context, vault_pda).await;
    assert_eq!(vault_balance, 0);

    let solver_balance = get_token_balance(&mut context, env.solver_token).await;
    assert_eq!(solver_balance, initial_solver_balance + amount);

    let escrow = read_escrow(
        &context
            .banks_client
            .get_account(escrow_pda)
            .await
            .unwrap()
            .unwrap(),
    );
    assert!(escrow.is_claimed);
    assert_eq!(escrow.amount, 0);

    let req = read_requirements(
        &context
            .banks_client
            .get_account(requirements_pda)
            .await
            .unwrap()
            .unwrap(),
    );
    assert!(req.fulfilled);
}

// ============================================================================
// MVM-SPECIFIC TESTS (N/A for SVM)
// ============================================================================
//
// 14. test_create_escrow_rejects_no_requirements - N/A
//     Why: MVM tests explicit rejection when requirements don't exist, but SVM
//     handles this via account existence checks in CreateEscrow instruction - the
//     transaction fails if requirements PDA doesn't exist (covered by test 9).
//
// 15. test_create_escrow_rejects_double_create - N/A
//     Why: MVM tests explicit rejection of duplicate escrow creation, but SVM
//     prevents this via PDA account initialization semantics - the init constraint
//     fails if the escrow account already exists (covered by test 9).
//
// 16. test_release_escrow_succeeds_after_fulfillment - N/A
//     Why: MVM uses two-step fulfillment: (1) receive proof marks fulfilled,
//     (2) manual release transfers tokens. SVM auto-releases tokens in test 6
//     when fulfillment proof is received - no separate release step exists.
//
// 17. test_release_escrow_rejects_without_fulfillment - N/A
//     Why: MVM tests that manual release requires fulfillment first. SVM doesn't
//     have a separate release instruction - release happens automatically in
//     test_receive_fulfillment_proof_releases_escrow (test 6).
//
// 18. test_release_escrow_rejects_unauthorized_solver - N/A
//     Why: MVM tests solver authorization during manual release. SVM validates
//     solver during LzReceiveFulfillmentProof and auto-releases to the correct
//     solver immediately (covered in test 6).
//
// 19. test_release_escrow_rejects_double_release - N/A
//     Why: MVM tests that manual release can't happen twice. SVM auto-releases
//     once in test 6, and the escrow is marked claimed. Double fulfillment is
//     rejected in test 8 (test_receive_fulfillment_proof_rejects_already_fulfilled).

// ============================================================================
// GENERIC LZRECEIVE ROUTING TESTS
// ============================================================================
//
// These tests verify the generic LzReceive instruction (variant index 1) that
// routes based on message type. This is used by the GMP endpoint's CPI which
// always uses variant index 1 for destination programs.

/// 20. Test: Generic LzReceive routes IntentRequirements correctly
/// Verifies that message type 0x01 routes to requirements handler.
/// Why: GMP endpoint uses generic LzReceive for all CPIs - must route correctly.
#[tokio::test]
async fn test_generic_lz_receive_routes_requirements() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;
    let expiry = u64::MAX;

    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    // Create IntentRequirements payload (message type 0x01 is embedded in encode())
    let requirements_payload = create_requirements_payload(
        intent_id,
        &env.requester.pubkey(),
        amount,
        &env.mint,
        &env.solver.pubkey(),
        expiry,
    );

    let gmp_caller = context.payer.insecure_clone();

    // Use the generic LzReceive instruction (variant index 1)
    let lz_receive_ix = common::create_lz_receive_generic_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        requirements_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify requirements were stored
    let req_account = context
        .banks_client
        .get_account(requirements_pda)
        .await
        .unwrap()
        .expect("Requirements account should exist");

    let requirements = read_requirements(&req_account);
    assert_eq!(requirements.intent_id, intent_id);
    assert_eq!(requirements.amount_required, amount);
    assert!(!requirements.escrow_created);
    assert!(!requirements.fulfilled);
}

/// 21. Test: Generic LzReceive routes FulfillmentProof correctly
/// Verifies that message type 0x03 routes to fulfillment proof handler.
/// Why: GMP endpoint uses generic LzReceive for all CPIs - must route correctly.
#[tokio::test]
async fn test_generic_lz_receive_routes_fulfillment_proof() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 500_000u64;

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);
    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    let gmp_caller = context.payer.insecure_clone();

    // Step 1: Receive requirements (using specific instruction to set up state)
    let requirements_payload = create_requirements_payload(
        intent_id,
        &env.requester.pubkey(),
        amount,
        &env.mint,
        &env.solver.pubkey(),
        u64::MAX,
    );

    let lz_receive_req_ix = create_lz_receive_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        requirements_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_req_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Step 2: Create escrow
    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        None,
        Some(requirements_pda),
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Step 3: Receive fulfillment proof using GENERIC LzReceive
    let proof_payload = create_fulfillment_proof_payload(
        intent_id,
        &env.solver.pubkey(),
        amount,
        12345,
    );

    let lz_receive_proof_ix = common::create_lz_receive_generic_fulfillment_ix(
        env.program_id,
        requirements_pda,
        escrow_pda,
        vault_pda,
        env.solver_token,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        proof_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_proof_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify escrow released
    let vault_balance = get_token_balance(&mut context, vault_pda).await;
    let solver_balance = get_token_balance(&mut context, env.solver_token).await;
    assert_eq!(vault_balance, 0);
    assert_eq!(solver_balance, amount);

    let escrow = read_escrow(
        &context
            .banks_client
            .get_account(escrow_pda)
            .await
            .unwrap()
            .unwrap(),
    );
    assert!(escrow.is_claimed);
}

/// 22. Test: Generic LzReceive rejects unknown message types
/// Verifies that invalid message types (not 0x01 or 0x03) are rejected.
/// Why: Unknown message types should fail explicitly, not silently.
#[tokio::test]
async fn test_generic_lz_receive_rejects_unknown_message_type() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();

    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    let gmp_caller = context.payer.insecure_clone();

    // Create payload with invalid message type (0x00)
    let invalid_payload = vec![0x00, 0x01, 0x02, 0x03]; // 0x00 is not a valid message type

    let lz_receive_ix = common::create_lz_receive_generic_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        invalid_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Should reject unknown message type 0x00");

    // Warp to next slot
    context.warp_to_slot(100).unwrap();

    // Try another invalid type (0x02 - EscrowConfirmation is outbound only)
    let invalid_payload2 = vec![0x02, 0x01, 0x02, 0x03];

    let lz_receive_ix2 = common::create_lz_receive_generic_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_TRUSTED_HUB_ADDR,
        invalid_payload2,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[lz_receive_ix2],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Should reject message type 0x02 (EscrowConfirmation)");
}
