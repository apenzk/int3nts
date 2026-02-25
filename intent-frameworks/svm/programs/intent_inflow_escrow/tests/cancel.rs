mod common;

use common::{
    create_cancel_ix, create_escrow_ix,
    create_gmp_receive_fulfillment_proof_ix, create_gmp_receive_requirements_ix,
    create_set_gmp_config_ix, generate_intent_id, get_token_balance, program_test, read_escrow,
    setup_basic_env, setup_gmp_requirements, setup_gmp_requirements_custom, test_program_id,
    DUMMY_HUB_CHAIN_ID, DUMMY_HUB_GMP_ENDPOINT_ADDR,
};
use gmp_common::messages::{FulfillmentProof, IntentRequirements};
use intent_inflow_escrow::state::seeds;
use solana_sdk::{
    clock::Clock,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    sysvar,
    transaction::Transaction,
};
use bincode::deserialize;

// ============================================================================
// EXPIRY TESTS
// ============================================================================

/// 1. Test: Cancellation Before Expiry Prevention
/// Verifies that admin cannot cancel escrows before expiry.
/// Why: Funds must remain locked until expiry to give solvers time to fulfill.
#[tokio::test]
async fn test_revert_if_escrow_has_not_expired_yet() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;

    // Use u64::MAX for expiry so the escrow will not be expired
    let requirements_pda =
        setup_gmp_requirements(&mut context, &env, intent_id, amount, u64::MAX).await;

    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        requirements_pda,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let create_tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(create_tx).await.unwrap();

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);

    // Admin (= requester in basic env) tries to cancel before expiry — should fail
    let cancel_ix = create_cancel_ix(
        env.program_id,
        intent_id,
        env.requester.pubkey(),
        env.requester_token,
        escrow_pda,
        vault_pda,
        env.gmp_config_pda,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let cancel_tx = Transaction::new_signed_with_payer(
        &[cancel_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );

    let result = context.banks_client.process_transaction(cancel_tx).await;
    assert!(result.is_err(), "Should have thrown an error");
}

/// 2. Test: Admin Cancellation After Expiry
/// Verifies that admin can cancel escrows after expiry and funds return to requester.
/// Why: Admin needs a way to return funds if fulfillment doesn't occur.
///
/// NOTE: Uses 1-second expiry for fast testing. Production uses 120 seconds.
#[tokio::test]
async fn test_cancel_after_expiry() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = [3u8; 32];
    let amount = 500_000u64;

    // Get current clock timestamp before setting up requirements
    let clock_account = context
        .banks_client
        .get_account(sysvar::clock::id())
        .await
        .unwrap()
        .unwrap();
    let clock: Clock = deserialize(&clock_account.data).unwrap();
    let current_time = clock.unix_timestamp;

    // Use a short expiry: current_time + 1 second
    let expiry = (current_time as u64) + 1;
    let requirements_pda =
        setup_gmp_requirements(&mut context, &env, intent_id, amount, expiry).await;

    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        requirements_pda,
    );
    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let create_tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(create_tx).await.unwrap();

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);

    // Advance the Clock sysvar to ensure the escrow has expired
    let escrow_account = context
        .banks_client
        .get_account(escrow_pda)
        .await
        .unwrap()
        .unwrap();
    let escrow = read_escrow(&escrow_account);
    let clock_account = context
        .banks_client
        .get_account(sysvar::clock::id())
        .await
        .unwrap()
        .unwrap();
    let mut clock: Clock = deserialize(&clock_account.data).unwrap();
    clock.unix_timestamp = escrow.expiry + 1;
    context.set_sysvar(&clock);

    // Admin (= requester in basic env) cancels after expiry
    let cancel_ix = create_cancel_ix(
        env.program_id,
        intent_id,
        env.requester.pubkey(),
        env.requester_token,
        escrow_pda,
        vault_pda,
        env.gmp_config_pda,
    );
    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let cancel_tx = Transaction::new_signed_with_payer(
        &[cancel_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(cancel_tx).await.unwrap();

    let vault_balance = get_token_balance(&mut context, vault_pda).await;
    let requester_balance = get_token_balance(&mut context, env.requester_token).await;
    assert_eq!(vault_balance, 0);
    assert_eq!(requester_balance, 1_000_000);

    let escrow_account = context
        .banks_client
        .get_account(escrow_pda)
        .await
        .unwrap()
        .unwrap();
    let escrow = read_escrow(&escrow_account);
    assert!(escrow.is_claimed);
    assert_eq!(escrow.amount, 0);
}

// ============================================================================
// AUTHORIZATION TESTS
// ============================================================================

/// 3. Test: Requester Cannot Cancel (only admin can)
/// Verifies that the requester cannot cancel their own escrow — only admin can.
/// Why: Security requirement - only admin can cancel expired escrows.
#[tokio::test]
async fn test_revert_if_not_admin() {
    // Custom setup where admin ≠ requester
    let pt = program_test();
    let mut context = pt.start_with_context().await;
    let payer = context.payer.insecure_clone();
    let program_id = test_program_id();
    let admin = Keypair::new();
    let requester = Keypair::new();
    let solver = Keypair::new();
    let mint_authority = Keypair::new();

    // Fund accounts
    let fund_ixs = vec![
        solana_sdk::system_instruction::transfer(&payer.pubkey(), &admin.pubkey(), 2_000_000_000),
        solana_sdk::system_instruction::transfer(&payer.pubkey(), &requester.pubkey(), 2_000_000_000),
        solana_sdk::system_instruction::transfer(&payer.pubkey(), &solver.pubkey(), 2_000_000_000),
    ];
    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let fund_tx = Transaction::new_signed_with_payer(
        &fund_ixs,
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );
    context.banks_client.process_transaction(fund_tx).await.unwrap();

    // Create mint and token accounts
    let mint = common::create_mint(&mut context, &payer, &mint_authority, 6).await;
    let requester_token =
        common::create_token_account(&mut context, &payer, mint, requester.pubkey()).await;
    common::mint_to(&mut context, &payer, mint, &mint_authority, requester_token, 1_000_000).await;

    // Initialize program
    let approver = Keypair::new();
    common::initialize_program(&mut context, &requester, program_id, approver.pubkey()).await;

    // Initialize GMP config with admin (NOT requester) as admin
    let gmp_endpoint = Pubkey::new_unique();
    let (gmp_config_pda, _) =
        Pubkey::find_program_address(&[seeds::GMP_CONFIG_SEED], &program_id);
    let set_gmp_config_ix = create_set_gmp_config_ix(
        program_id,
        gmp_config_pda,
        admin.pubkey(),
        DUMMY_HUB_CHAIN_ID,
        DUMMY_HUB_GMP_ENDPOINT_ADDR,
        gmp_endpoint,
    );
    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[set_gmp_config_ix],
        Some(&payer.pubkey()),
        &[&payer, &admin],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;

    let requirements_pda = setup_gmp_requirements_custom(
        &mut context,
        program_id,
        gmp_config_pda,
        DUMMY_HUB_CHAIN_ID,
        DUMMY_HUB_GMP_ENDPOINT_ADDR,
        intent_id,
        requester.pubkey(),
        mint,
        solver.pubkey(),
        amount,
        u64::MAX,
    )
    .await;

    let create_ix = create_escrow_ix(
        program_id,
        intent_id,
        amount,
        requester.pubkey(),
        mint,
        requester_token,
        solver.pubkey(),
        requirements_pda,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let create_tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&requester.pubkey()),
        &[&requester],
        blockhash,
    );
    context.banks_client.process_transaction(create_tx).await.unwrap();

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &program_id);

    // Requester (not admin) tries to cancel — should fail
    let cancel_ix = create_cancel_ix(
        program_id,
        intent_id,
        requester.pubkey(),
        requester_token,
        escrow_pda,
        vault_pda,
        gmp_config_pda,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let cancel_tx = Transaction::new_signed_with_payer(
        &[cancel_ix],
        Some(&requester.pubkey()),
        &[&requester],
        blockhash,
    );

    let result = context.banks_client.process_transaction(cancel_tx).await;
    assert!(result.is_err(), "Should fail - requester is not admin");
}

/// 4. Test: Cancellation After Claim Prevention (GMP Mode)
/// Verifies that attempting to cancel an already-claimed escrow reverts.
/// Why: Once funds are claimed, they cannot be cancelled.
#[tokio::test]
async fn test_revert_if_already_claimed() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = [5u8; 32];
    let amount = 1_000_000u64;
    let src_chain_id = DUMMY_HUB_CHAIN_ID;
    let remote_gmp_endpoint_addr = DUMMY_HUB_GMP_ENDPOINT_ADDR;

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);
    let (requirements_pda, _) =
        Pubkey::find_program_address(&[seeds::REQUIREMENTS_SEED, &intent_id], &env.program_id);

    // Step 1: Receive requirements via GMP
    let requirements = IntentRequirements {
        intent_id,
        requester_addr: env.requester.pubkey().to_bytes(),
        amount_required: amount,
        token_addr: env.mint.to_bytes(),
        solver_addr: env.solver.pubkey().to_bytes(),
        expiry: u64::MAX,
    };
    let requirements_payload = requirements.encode().to_vec();

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
        requirements_pda,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let create_tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(create_tx).await.unwrap();

    // Step 3: Claim the escrow via GMP fulfillment proof
    let proof = FulfillmentProof {
        intent_id,
        solver_addr: env.solver.pubkey().to_bytes(),
        amount_fulfilled: amount,
        timestamp: 12345,
    };
    let proof_payload = proof.encode().to_vec();

    let gmp_receive_proof_ix = create_gmp_receive_fulfillment_proof_ix(
        env.program_id,
        requirements_pda,
        escrow_pda,
        vault_pda,
        env.solver_token,
        env.gmp_config_pda,
        gmp_caller.pubkey(),
        src_chain_id,
        remote_gmp_endpoint_addr,
        proof_payload,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let claim_tx = Transaction::new_signed_with_payer(
        &[gmp_receive_proof_ix],
        Some(&gmp_caller.pubkey()),
        &[&gmp_caller],
        blockhash,
    );
    context.banks_client.process_transaction(claim_tx).await.unwrap();

    // Step 4: Now try to cancel as admin - should fail because already claimed
    let cancel_ix = create_cancel_ix(
        env.program_id,
        intent_id,
        env.requester.pubkey(),
        env.requester_token,
        escrow_pda,
        vault_pda,
        env.gmp_config_pda,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let cancel_tx = Transaction::new_signed_with_payer(
        &[cancel_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );

    let result = context.banks_client.process_transaction(cancel_tx).await;
    assert!(result.is_err(), "Should fail - escrow already claimed");
}

// ============================================================================
// NON-EXISTENT ESCROW TESTS
// ============================================================================

/// 5. Test: Non-Existent Escrow Prevention
/// Verifies that canceling a non-existent escrow reverts.
/// Why: Prevents invalid operations on non-existent escrows.
#[tokio::test]
async fn test_revert_if_escrow_does_not_exist() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let non_existent_intent_id = generate_intent_id();

    let (escrow_pda, _) = Pubkey::find_program_address(
        &[seeds::ESCROW_SEED, &non_existent_intent_id],
        &env.program_id,
    );
    let (vault_pda, _) = Pubkey::find_program_address(
        &[seeds::VAULT_SEED, &non_existent_intent_id],
        &env.program_id,
    );

    let cancel_ix = create_cancel_ix(
        env.program_id,
        non_existent_intent_id,
        env.requester.pubkey(),
        env.requester_token,
        escrow_pda,
        vault_pda,
        env.gmp_config_pda,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let cancel_tx = Transaction::new_signed_with_payer(
        &[cancel_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );

    let result = context.banks_client.process_transaction(cancel_tx).await;
    assert!(result.is_err(), "Should have thrown an error");
}

// ============================================================================
// ADMIN CANCEL TESTS
// ============================================================================

/// 6. Test: Double Cancellation Prevention
/// Verifies that canceling an already-cancelled escrow reverts.
/// Why: Prevents double-refund by ensuring released escrows cannot be cancelled again.
#[tokio::test]
async fn test_revert_if_already_cancelled() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = [7u8; 32];
    let amount = 500_000u64;

    // Get current clock timestamp before setting up requirements
    let clock_account = context
        .banks_client
        .get_account(sysvar::clock::id())
        .await
        .unwrap()
        .unwrap();
    let clock: Clock = deserialize(&clock_account.data).unwrap();
    let current_time = clock.unix_timestamp;

    // Use a short expiry: current_time + 1 second
    let expiry = (current_time as u64) + 1;
    let requirements_pda =
        setup_gmp_requirements(&mut context, &env, intent_id, amount, expiry).await;

    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        requirements_pda,
    );
    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let create_tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(create_tx).await.unwrap();

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);

    // Advance clock past expiry
    let escrow_account = context
        .banks_client
        .get_account(escrow_pda)
        .await
        .unwrap()
        .unwrap();
    let escrow_data = read_escrow(&escrow_account);
    let clock_account = context
        .banks_client
        .get_account(sysvar::clock::id())
        .await
        .unwrap()
        .unwrap();
    let mut clock: Clock = deserialize(&clock_account.data).unwrap();
    clock.unix_timestamp = escrow_data.expiry + 1;
    context.set_sysvar(&clock);

    // First cancel succeeds (admin = requester in basic env)
    let cancel_ix = create_cancel_ix(
        env.program_id,
        intent_id,
        env.requester.pubkey(),
        env.requester_token,
        escrow_pda,
        vault_pda,
        env.gmp_config_pda,
    );
    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let cancel_tx = Transaction::new_signed_with_payer(
        &[cancel_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(cancel_tx).await.unwrap();

    // Second cancel should fail — escrow already released
    let cancel_ix2 = create_cancel_ix(
        env.program_id,
        intent_id,
        env.requester.pubkey(),
        env.requester_token,
        escrow_pda,
        vault_pda,
        env.gmp_config_pda,
    );
    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let cancel_tx2 = Transaction::new_signed_with_payer(
        &[cancel_ix2],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    let result = context.banks_client.process_transaction(cancel_tx2).await;
    assert!(result.is_err(), "Should fail - escrow already cancelled");
}
