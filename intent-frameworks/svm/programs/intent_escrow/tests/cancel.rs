mod common;

use common::{
    create_cancel_ix, create_claim_ix, create_ed25519_instruction, create_escrow_ix,
    generate_intent_id, get_token_balance, program_test, read_escrow, setup_basic_env,
};
use intent_escrow::state::seeds;
use solana_sdk::{
    clock::Clock,
    pubkey::Pubkey,
    signature::Signer,
    sysvar,
    transaction::Transaction,
};
use bincode::deserialize;

// ============================================================================
// EXPIRY TESTS
// ============================================================================

/// 1. Test: Cancellation Before Expiry Prevention
/// Verifies that requesters cannot cancel escrows before expiry.
/// Why: Funds must remain locked until expiry to give solvers time to fulfill.
#[tokio::test]
async fn test_revert_if_escrow_has_not_expired_yet() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;

    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        None, // Default expiry (120 seconds)
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

    let cancel_ix = create_cancel_ix(
        env.program_id,
        intent_id,
        env.requester.pubkey(),
        env.requester_token,
        escrow_pda,
        vault_pda,
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

/// 2. Test: Cancellation After Expiry
/// Verifies that requesters can cancel escrows after expiry and reclaim funds.
/// Why: Requesters need a way to reclaim funds if fulfillment doesn't occur.
///
/// NOTE: Uses 1-second expiry for fast testing. Production uses 120 seconds.
#[tokio::test]
async fn test_cancel_after_expiry() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = [3u8; 32];
    let amount = 500_000u64;

    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        Some(1), // 1 second expiry
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

    let cancel_ix = create_cancel_ix(
        env.program_id,
        intent_id,
        env.requester.pubkey(),
        env.requester_token,
        escrow_pda,
        vault_pda,
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

/// 3. Test: Unauthorized Cancellation Prevention
/// Verifies that only the requester can cancel their escrow.
/// Why: Security requirement - only the escrow creator should be able to cancel.
#[tokio::test]
async fn test_revert_if_not_requester() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;

    let create_ix = create_escrow_ix(
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

    // Try to cancel with solver (wrong requester)
    let cancel_ix = create_cancel_ix(
        env.program_id,
        intent_id,
        env.solver.pubkey(), // Wrong requester
        env.solver_token,
        escrow_pda,
        vault_pda,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let cancel_tx = Transaction::new_signed_with_payer(
        &[cancel_ix],
        Some(&env.solver.pubkey()),
        &[&env.solver],
        blockhash,
    );

    let result = context.banks_client.process_transaction(cancel_tx).await;
    assert!(result.is_err(), "Should have thrown an error");
}

/// 4. Test: Cancellation After Claim Prevention
/// Verifies that attempting to cancel an already-claimed escrow reverts.
/// Why: Once funds are claimed, they cannot be cancelled.
#[tokio::test]
async fn test_revert_if_already_claimed() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;

    let create_ix = create_escrow_ix(
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

    // Claim the escrow first
    let signature = env.approver.sign_message(&intent_id);
    let mut signature_bytes = [0u8; 64];
    signature_bytes.copy_from_slice(signature.as_ref());

    let ed25519_ix = create_ed25519_instruction(&intent_id, &signature_bytes, &env.approver.pubkey());

    let claim_ix = create_claim_ix(
        env.program_id,
        intent_id,
        signature_bytes,
        escrow_pda,
        env.state_pda,
        vault_pda,
        env.solver_token,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let claim_tx = Transaction::new_signed_with_payer(
        &[ed25519_ix, claim_ix],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        blockhash,
    );
    context.banks_client.process_transaction(claim_tx).await.unwrap();

    // Now try to cancel - should fail
    let cancel_ix = create_cancel_ix(
        env.program_id,
        intent_id,
        env.requester.pubkey(),
        env.requester_token,
        escrow_pda,
        vault_pda,
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
