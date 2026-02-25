mod common;

use common::{
    create_escrow_ix, generate_intent_id, get_token_balance, program_test, read_escrow,
    setup_basic_env, setup_gmp_requirements,
};
use intent_inflow_escrow::state::seeds;
use solana_sdk::{clock::Clock, pubkey::Pubkey, signature::Signer, sysvar, transaction::Transaction};
use bincode::deserialize;

// ============================================================================
// ESCROW CREATION TESTS
// ============================================================================

/// 1. Test: Token Escrow Creation
/// Verifies that requesters can create an escrow with tokens atomically.
/// Why: Escrow creation is the first step in the intent fulfillment flow.
#[tokio::test]
async fn test_create_escrow_with_tokens() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = [1u8; 32];
    let amount = 500_000u64;

    let initial_balance = get_token_balance(&mut context, env.requester_token).await;

    let requirements_pda = setup_gmp_requirements(&mut context, &env, intent_id, amount, u64::MAX).await;
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

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);

    // Verify requester balance decreased
    let final_balance = get_token_balance(&mut context, env.requester_token).await;
    assert_eq!(final_balance, initial_balance - amount);

    // Verify vault balance increased
    let vault_balance = get_token_balance(&mut context, vault_pda).await;
    assert_eq!(vault_balance, amount);

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
}

/// 2. Test: Escrow Creation After Claim Prevention
/// Verifies that escrows cannot be created with an intent ID that was already claimed.
/// Why: Prevents duplicate escrows and ensures each intent ID maps to a single escrow state.
#[tokio::test]
async fn test_revert_if_escrow_already_claimed() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;

    let requirements_pda = setup_gmp_requirements(&mut context, &env, intent_id, amount, u64::MAX).await;
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

    // Try to create second escrow with same intent ID - should fail
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

/// 3. Test: Multiple Escrows with Different Intent IDs
/// Verifies that multiple escrows can be created for different intent IDs.
/// Why: System must support concurrent escrows.
#[tokio::test]
async fn test_support_multiple_escrows_with_different_intent_ids() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id1 = generate_intent_id();
    let intent_id2 = generate_intent_id();
    let amount1 = 400_000u64; // Amount chosen to fit within test token budget
    let amount2 = 500_000u64;

    let (vault_pda1, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id1], &env.program_id);
    let (vault_pda2, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id2], &env.program_id);

    // Create first escrow
    let requirements_pda1 = setup_gmp_requirements(&mut context, &env, intent_id1, amount1, u64::MAX).await;
    let ix1 = create_escrow_ix(
        env.program_id,
        intent_id1,
        amount1,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        requirements_pda1,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx1 = Transaction::new_signed_with_payer(
        &[ix1],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(tx1).await.unwrap();

    // Create second escrow
    let requirements_pda2 = setup_gmp_requirements(&mut context, &env, intent_id2, amount2, u64::MAX).await;
    let ix2 = create_escrow_ix(
        env.program_id,
        intent_id2,
        amount2,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        requirements_pda2,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx2 = Transaction::new_signed_with_payer(
        &[ix2],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(tx2).await.unwrap();

    // Verify both vaults have correct balances
    let vault1_balance = get_token_balance(&mut context, vault_pda1).await;
    let vault2_balance = get_token_balance(&mut context, vault_pda2).await;
    assert_eq!(vault1_balance, amount1);
    assert_eq!(vault2_balance, amount2);
}

/// 4. Test: Escrow Expiry Timestamp
/// Verifies that escrow expiry is set correctly from GMP requirements.
/// Why: Expiry must be correct for time-based cancel functionality.
#[tokio::test]
async fn test_set_correct_expiry_timestamp() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);

    // Get current time before transaction
    let clock_account = context
        .banks_client
        .get_account(sysvar::clock::id())
        .await
        .unwrap()
        .unwrap();
    let clock: Clock = deserialize(&clock_account.data).unwrap();
    let block_time = clock.unix_timestamp;

    // Set expiry to block_time + 120 seconds in GMP requirements
    let expiry = (block_time as u64) + 120;

    let requirements_pda = setup_gmp_requirements(&mut context, &env, intent_id, amount, expiry).await;
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

    // Read escrow data
    let escrow_account = context
        .banks_client
        .get_account(escrow_pda)
        .await
        .unwrap()
        .unwrap();
    let escrow = read_escrow(&escrow_account);

    // Expiry comes from GMP requirements; values up to i64::MAX are preserved exactly
    let expected_expiry = expiry as i64;

    assert_eq!(
        escrow.expiry, expected_expiry,
        "Escrow expiry should match the expiry set in GMP requirements"
    );
    assert_eq!(escrow.requester, env.requester.pubkey());
    assert_eq!(escrow.token_mint, env.mint);
    assert_eq!(escrow.amount, amount);
    assert!(!escrow.is_claimed);
}
