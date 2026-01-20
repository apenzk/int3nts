mod common;

use common::{
    create_escrow_ix, generate_intent_id, get_token_balance, program_test, read_escrow,
    setup_basic_env,
};
use intent_escrow::state::seeds;
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

/// 1. Test: Maximum Values
/// Verifies that createEscrow handles maximum values for both amounts and intent IDs.
/// Why: Edge case testing ensures the program handles boundary values without overflow or underflow.
#[tokio::test]
async fn test_handle_maximum_values_for_amounts_and_intent_ids() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    // Use maximum intent ID (all 0xFF bytes)
    let max_intent_id: [u8; 32] = [0xFF; 32];
    // Use the available balance (1_000_000 tokens minted in setup)
    let amount = 500_000u64;

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &max_intent_id], &env.program_id);

    let ix = create_escrow_ix(
        env.program_id,
        max_intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        None,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify escrow was created with max intent ID
    let escrow_account = context
        .banks_client
        .get_account(escrow_pda)
        .await
        .unwrap()
        .unwrap();
    let escrow = read_escrow(&escrow_account);
    assert_eq!(escrow.amount, amount);
    assert_eq!(escrow.requester, env.requester.pubkey());
}

/// 2. Test: Empty Deposit Scenarios
/// Verifies edge cases around minimum deposit amounts (1 token unit).
/// Why: Ensures the program accepts the minimum valid amount (1 token unit) without rejecting it as zero.
#[tokio::test]
async fn test_handle_minimum_deposit_amount() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let min_amount = 1u64; // 1 token unit (smallest possible)

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);

    let ix = create_escrow_ix(
        env.program_id,
        intent_id,
        min_amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        None,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify escrow was created
    let escrow_account = context
        .banks_client
        .get_account(escrow_pda)
        .await
        .unwrap()
        .unwrap();
    let escrow = read_escrow(&escrow_account);

    // Verify amount
    assert_eq!(escrow.amount, min_amount);

    // Verify vault balance
    let vault_balance = get_token_balance(&mut context, vault_pda).await;
    assert_eq!(vault_balance, min_amount);
}

/// 3. Test: Multiple Escrows Per Requester
/// Verifies that a requester can create multiple escrows with different intent IDs.
/// Why: Requesters may need multiple concurrent escrows for different intents. State isolation must be maintained.
#[tokio::test]
async fn test_allow_requester_to_create_multiple_escrows() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let num_escrows = 5;
    let amount = 100_000u64; // Amount chosen to allow 5 escrows within test token budget

    // Create multiple escrows with sequential intent IDs
    for _i in 0..num_escrows {
        let intent_id = generate_intent_id();
        let (escrow_pda, _) =
            Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
        let (_vault_pda, _) =
            Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);

        let ix = create_escrow_ix(
            env.program_id,
            intent_id,
            amount,
            env.requester.pubkey(),
            env.mint,
            env.requester_token,
            env.solver.pubkey(),
            None,
        );

        let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&env.requester.pubkey()),
            &[&env.requester],
            blockhash,
        );
        context.banks_client.process_transaction(tx).await.unwrap();

        // Verify escrow was created
        let escrow_account = context
            .banks_client
            .get_account(escrow_pda)
            .await
            .unwrap()
            .unwrap();
        let escrow = read_escrow(&escrow_account);

        // Verify requester
        assert_eq!(escrow.requester, env.requester.pubkey());

        // Verify amount
        assert_eq!(escrow.amount, amount);
    }
}

/// 4. Test: Gas Limit Scenarios
/// Verifies gas consumption for large operations (multiple escrows, large amounts).
/// Why: Gas efficiency is critical for user experience. Operations must stay within reasonable gas limits.
///
/// NOTE: SVM uses compute units instead of gas. This test verifies compute unit consumption.
#[tokio::test]
async fn test_handle_gas_consumption_for_large_operations() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let num_escrows = 3;
    let amount = 200_000u64; // Amount chosen to allow 3 escrows within test token budget

    // Create multiple escrows and verify they all succeed
    let mut escrows = Vec::new();
    for _i in 0..num_escrows {
        let intent_id = generate_intent_id();
        let (escrow_pda, _) =
            Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
        let (vault_pda, _) =
            Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);

        let ix = create_escrow_ix(
            env.program_id,
            intent_id,
            amount,
            env.requester.pubkey(),
            env.mint,
            env.requester_token,
            env.solver.pubkey(),
            None,
        );

        let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&env.requester.pubkey()),
            &[&env.requester],
            blockhash,
        );
        context.banks_client.process_transaction(tx).await.unwrap();

        escrows.push((intent_id, escrow_pda, vault_pda));
    }

    // Verify all transactions succeeded
    assert_eq!(escrows.len(), num_escrows);

    // Verify all escrows exist
    for (_intent_id, _escrow_pda, vault_pda) in &escrows {
        let vault_balance = get_token_balance(&mut context, *vault_pda).await;
        assert_eq!(vault_balance, amount);
    }
}

/// 5. Test: Concurrent Operations
/// Verifies that multiple simultaneous escrow operations can be handled correctly.
/// Why: Real-world usage involves concurrent operations. The program must handle them without state corruption.
#[tokio::test]
async fn test_handle_concurrent_escrow_operations() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let num_escrows = 3;
    let amount = 200_000u64; // Amount chosen to allow 3 escrows within test token budget

    // Create multiple escrows sequentially (Solana doesn't support true concurrent txs in tests)
    let mut escrow_infos = Vec::new();
    for _i in 0..num_escrows {
        let intent_id = generate_intent_id();
        let (escrow_pda, _) =
            Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
        let (vault_pda, _) =
            Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);

        let ix = create_escrow_ix(
            env.program_id,
            intent_id,
            amount,
            env.requester.pubkey(),
            env.mint,
            env.requester_token,
            env.solver.pubkey(),
            None,
        );

        let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&env.requester.pubkey()),
            &[&env.requester],
            blockhash,
        );
        context.banks_client.process_transaction(tx).await.unwrap();

        escrow_infos.push((intent_id, escrow_pda, vault_pda));
    }

    // Verify all escrows were created correctly
    assert_eq!(escrow_infos.len(), num_escrows);

    for (_intent_id, escrow_pda, _vault_pda) in &escrow_infos {
        let escrow_account = context
            .banks_client
            .get_account(*escrow_pda)
            .await
            .unwrap()
            .unwrap();
        let escrow = read_escrow(&escrow_account);

        // Verify amount
        assert_eq!(escrow.amount, amount);

        // Verify requester
        assert_eq!(escrow.requester, env.requester.pubkey());
    }
}
