mod common;

use common::{
    create_escrow_ix, generate_intent_id, get_token_balance, initialize_program, program_test,
    read_escrow, read_state, setup_basic_env, setup_gmp_requirements,
};
use intent_inflow_escrow::state::seeds;
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};

// ============================================================================
// APPROVER INITIALIZATION TESTS
// ============================================================================

/// 1. Test: Approver Address Initialization
/// Verifies that the escrow is initialized with the correct approver address.
/// Why: The approver address is critical for signature validation.
#[tokio::test]
async fn test_initialize_approver_address() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let payer = context.payer.insecure_clone();
    let approver = solana_sdk::signature::Keypair::new();

    let state_pda = initialize_program(
        &mut context,
        &payer,
        common::test_program_id(),
        approver.pubkey(),
    )
    .await;

    let state_account = context
        .banks_client
        .get_account(state_pda)
        .await
        .unwrap()
        .unwrap();
    let state = read_state(&state_account);
    assert_eq!(state.approver, approver.pubkey());
}

// ============================================================================
// ESCROW CREATION TESTS
// ============================================================================

/// 2. Test: Escrow Creation
/// Verifies that requesters can create a new escrow with funds atomically.
/// Why: Escrow creation must be atomic and set expiry correctly.
#[tokio::test]
async fn test_allow_requester_to_create_escrow() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);

    let requirements_pda =
        setup_gmp_requirements(&mut context, &env, intent_id, amount, u64::MAX).await;

    let ix = create_escrow_ix(
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
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify escrow data
    let escrow_account = context
        .banks_client
        .get_account(escrow_pda)
        .await
        .unwrap()
        .unwrap();
    let escrow = read_escrow(&escrow_account);
    assert_eq!(escrow.requester, env.requester.pubkey());
    assert_eq!(escrow.amount, amount);
    assert!(!escrow.is_claimed);

    // Verify vault balance
    let vault_balance = get_token_balance(&mut context, vault_pda).await;
    assert_eq!(vault_balance, amount);
}

/// 3. Test: Duplicate Creation Prevention
/// Verifies that attempting to create an escrow with an existing intent ID reverts.
/// Why: Each intent ID must map to a single escrow.
#[tokio::test]
async fn test_revert_if_escrow_already_exists() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;

    let requirements_pda =
        setup_gmp_requirements(&mut context, &env, intent_id, amount, u64::MAX).await;

    let ix1 = create_escrow_ix(
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
    let tx1 = Transaction::new_signed_with_payer(
        &[ix1],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(tx1).await.unwrap();

    // Warp to next slot to ensure clean transaction processing
    context.warp_to_slot(100).unwrap();

    // Try to create second escrow with same intent ID
    let ix2 = create_escrow_ix(
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
    let tx2 = Transaction::new_signed_with_payer(
        &[ix2],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );

    let result = context.banks_client.process_transaction(tx2).await;
    assert!(result.is_err(), "Should have thrown an error");
}

/// 4. Test: Zero Amount Prevention
/// Verifies that escrows cannot be created with zero amount.
/// Why: Zero-amount escrows are invalid.
#[tokio::test]
async fn test_revert_if_amount_is_zero() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 0u64;

    let requirements_pda =
        setup_gmp_requirements(&mut context, &env, intent_id, 1_000_000u64, u64::MAX).await;

    let ix = create_escrow_ix(
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
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Should have thrown an error");
}
