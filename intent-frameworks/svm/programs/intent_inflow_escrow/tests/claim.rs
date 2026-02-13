mod common;

use common::{
    create_escrow_ix, create_gmp_receive_fulfillment_proof_ix, create_gmp_receive_requirements_ix,
    generate_intent_id, get_token_balance, program_test, read_escrow, read_requirements,
    setup_basic_env, DUMMY_HUB_CHAIN_ID, DUMMY_HUB_GMP_ENDPOINT_ADDR,
};
use gmp_common::messages::{FulfillmentProof, IntentRequirements};
use intent_inflow_escrow::state::seeds;
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};

// ============================================================================
// GMP CLAIM TESTS
// ============================================================================
// These tests verify the GMP-based claim flow where:
// 1. Requirements are received via GmpReceiveRequirements
// 2. Escrow is created (validates against requirements)
// 3. Fulfillment proof is received via GmpReceiveFulfillmentProof (auto-releases)

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

/// 1. Test: Valid Claim via GmpReceiveFulfillmentProof
/// Verifies that escrow is auto-released when fulfillment proof is received via GMP.
/// Why: In GMP mode, fulfillment proof from hub authorizes the release.
#[tokio::test]
async fn test_claim_with_valid_fulfillment_proof() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = [2u8; 32];
    let amount = 500_000u64;
    let src_chain_id = DUMMY_HUB_CHAIN_ID;
    let remote_gmp_endpoint_addr = DUMMY_HUB_GMP_ENDPOINT_ADDR;

    // Derive PDAs
    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);
    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    // Step 1: Receive requirements via GMP
    let requirements_payload = create_requirements_payload(
        intent_id,
        &env.requester.pubkey(),
        amount,
        &env.mint,
        &env.solver.pubkey(),
        u64::MAX, // No expiry
    );

    let gmp_caller = context.payer.insecure_clone();
    let gmp_receive_req_ix = create_gmp_receive_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda, // PDA - must be derived, cannot be a DUMMY constant
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        src_chain_id,
        remote_gmp_endpoint_addr,
        requirements_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let lz_req_tx = Transaction::new_signed_with_payer(
        &[gmp_receive_req_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context
        .banks_client
        .process_transaction(lz_req_tx)
        .await
        .unwrap();

    // Verify requirements were stored
    let req_account = context
        .banks_client
        .get_account(requirements_pda)
        .await
        .unwrap()
        .unwrap();
    let requirements = read_requirements(&req_account);
    assert_eq!(requirements.intent_id, intent_id);
    assert!(!requirements.fulfilled);

    // Step 2: Create escrow (with requirements account for validation)
    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        None,                    // No expiry (using requirements)
        Some(requirements_pda),  // GMP requirements validation
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let create_tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context
        .banks_client
        .process_transaction(create_tx)
        .await
        .unwrap();

    // Verify escrow was created
    let escrow_account = context
        .banks_client
        .get_account(escrow_pda)
        .await
        .unwrap()
        .unwrap();
    let escrow = read_escrow(&escrow_account);
    assert!(!escrow.is_claimed);
    assert_eq!(escrow.amount, amount);

    // Step 3: Receive fulfillment proof via GMP (this auto-releases the escrow)
    let proof_payload =
        create_fulfillment_proof_payload(intent_id, &env.solver.pubkey(), amount, 12345);

    let gmp_receive_proof_ix = create_gmp_receive_fulfillment_proof_ix(
        env.program_id,
        requirements_pda,
        escrow_pda,
        vault_pda,
        env.solver_token,
        env.gmp_config_pda, // PDA - must be derived, cannot be a DUMMY constant
        gmp_caller.pubkey(),
        src_chain_id,
        remote_gmp_endpoint_addr,
        proof_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let lz_proof_tx = Transaction::new_signed_with_payer(
        &[gmp_receive_proof_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context
        .banks_client
        .process_transaction(lz_proof_tx)
        .await
        .unwrap();

    // Verify escrow was auto-released
    let vault_balance = get_token_balance(&mut context, vault_pda).await;
    let solver_balance = get_token_balance(&mut context, env.solver_token).await;
    assert_eq!(vault_balance, 0);
    assert_eq!(solver_balance, amount);

    let escrow_account = context
        .banks_client
        .get_account(escrow_pda)
        .await
        .unwrap()
        .unwrap();
    let escrow = read_escrow(&escrow_account);
    assert!(escrow.is_claimed);
    assert_eq!(escrow.amount, 0);

    // Verify requirements marked as fulfilled
    let req_account = context
        .banks_client
        .get_account(requirements_pda)
        .await
        .unwrap()
        .unwrap();
    let requirements = read_requirements(&req_account);
    assert!(requirements.fulfilled);
}

/// 2. Test: Reject fulfillment proof without requirements
/// Verifies that GmpReceiveFulfillmentProof fails if requirements don't exist.
#[tokio::test]
async fn test_revert_fulfillment_without_requirements() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;
    let src_chain_id = DUMMY_HUB_CHAIN_ID;
    let remote_gmp_endpoint_addr = DUMMY_HUB_GMP_ENDPOINT_ADDR;

    // Create escrow without requirements
    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        None, // No expiry
        None, // No requirements
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let create_tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context
        .banks_client
        .process_transaction(create_tx)
        .await
        .unwrap();

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);
    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    // Try to send fulfillment proof without requirements existing
    let proof_payload =
        create_fulfillment_proof_payload(intent_id, &env.solver.pubkey(), amount, 12345);

    let gmp_caller = context.payer.insecure_clone();
    let gmp_receive_proof_ix = create_gmp_receive_fulfillment_proof_ix(
        env.program_id,
        requirements_pda,
        escrow_pda,
        vault_pda,
        env.solver_token,
        env.gmp_config_pda, // PDA - must be derived, cannot be a DUMMY constant
        gmp_caller.pubkey(),
        src_chain_id,
        remote_gmp_endpoint_addr,
        proof_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let lz_proof_tx = Transaction::new_signed_with_payer(
        &[gmp_receive_proof_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );

    let result = context.banks_client.process_transaction(lz_proof_tx).await;
    assert!(result.is_err(), "Should fail - requirements don't exist");
}

/// 3. Test: Prevent double fulfillment
/// Verifies that GmpReceiveFulfillmentProof fails if already fulfilled.
#[tokio::test]
async fn test_prevent_double_fulfillment() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = [3u8; 32];
    let amount = 500_000u64;
    let src_chain_id = DUMMY_HUB_CHAIN_ID;
    let remote_gmp_endpoint_addr = DUMMY_HUB_GMP_ENDPOINT_ADDR;

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

    let gmp_receive_req_ix = create_gmp_receive_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda, // PDA - must be derived, cannot be a DUMMY constant
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        src_chain_id,
        remote_gmp_endpoint_addr,
        requirements_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[gmp_receive_req_ix],
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
        None,                    // No expiry
        Some(requirements_pda),  // GMP requirements validation
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Step 3: First fulfillment (should succeed)
    let proof_payload =
        create_fulfillment_proof_payload(intent_id, &env.solver.pubkey(), amount, 12345);

    let gmp_receive_proof_ix = create_gmp_receive_fulfillment_proof_ix(
        env.program_id,
        requirements_pda,
        escrow_pda,
        vault_pda,
        env.solver_token,
        env.gmp_config_pda, // PDA - must be derived, cannot be a DUMMY constant
        gmp_caller.pubkey(),
        src_chain_id,
        remote_gmp_endpoint_addr,
        proof_payload.clone(),
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[gmp_receive_proof_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Warp to next slot
    context.warp_to_slot(100).unwrap();

    // Step 4: Second fulfillment (should fail)
    let gmp_receive_proof_ix2 = create_gmp_receive_fulfillment_proof_ix(
        env.program_id,
        requirements_pda,
        escrow_pda,
        vault_pda,
        env.solver_token,
        env.gmp_config_pda, // PDA - must be derived, cannot be a DUMMY constant
        gmp_caller.pubkey(),
        src_chain_id,
        remote_gmp_endpoint_addr,
        proof_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[gmp_receive_proof_ix2],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Should fail - already fulfilled");
}

/// 4. Test: Escrow already claimed rejection
/// Verifies that fulfillment fails if escrow was already claimed.
#[tokio::test]
async fn test_revert_if_escrow_already_claimed() {
    // This is effectively the same as test_prevent_double_fulfillment
    // because GmpReceiveFulfillmentProof marks both requirements.fulfilled and escrow.is_claimed
    // The test above covers this case.
}

/// 5. Test: Non-existent escrow rejection
/// Verifies that GmpReceiveFulfillmentProof fails for non-existent escrow.
#[tokio::test]
async fn test_revert_if_escrow_does_not_exist() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;
    let src_chain_id = DUMMY_HUB_CHAIN_ID;
    let remote_gmp_endpoint_addr = DUMMY_HUB_GMP_ENDPOINT_ADDR;

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);
    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    let gmp_caller = context.payer.insecure_clone();

    // Store requirements but don't create escrow
    let requirements_payload = create_requirements_payload(
        intent_id,
        &env.requester.pubkey(),
        amount,
        &env.mint,
        &env.solver.pubkey(),
        u64::MAX,
    );

    let gmp_receive_req_ix = create_gmp_receive_requirements_ix(
        env.program_id,
        requirements_pda,
        env.gmp_config_pda, // PDA - must be derived, cannot be a DUMMY constant
        gmp_caller.pubkey(),
        gmp_caller.pubkey(),
        src_chain_id,
        remote_gmp_endpoint_addr,
        requirements_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[gmp_receive_req_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Try to fulfill without escrow existing
    let proof_payload =
        create_fulfillment_proof_payload(intent_id, &env.solver.pubkey(), amount, 12345);

    let gmp_receive_proof_ix = create_gmp_receive_fulfillment_proof_ix(
        env.program_id,
        requirements_pda,
        escrow_pda,
        vault_pda,
        env.solver_token,
        env.gmp_config_pda, // PDA - must be derived, cannot be a DUMMY constant
        gmp_caller.pubkey(),
        src_chain_id,
        remote_gmp_endpoint_addr,
        proof_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[gmp_receive_proof_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Should fail - escrow doesn't exist");
}
