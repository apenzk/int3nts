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
// EXPIRY HANDLING TESTS
// ============================================================================

/// 1. Test: Expired Escrow Cancellation
/// Verifies that requesters can cancel escrows after expiry and reclaim funds.
/// Why: Requesters need a way to reclaim funds if fulfillment doesn't occur before expiry. Cancellation before expiry is blocked to ensure funds remain locked until expiry.
#[tokio::test]
async fn test_allow_requester_to_cancel_expired_escrow() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);

    // Create escrow with short expiry (2 seconds)
    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        Some(2), // 2 second expiry
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let create_tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(create_tx).await.unwrap();

    // Cancellation blocked before expiry
    let cancel_ix_early = create_cancel_ix(
        env.program_id,
        intent_id,
        env.requester.pubkey(),
        env.requester_token,
        escrow_pda,
        vault_pda,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let cancel_tx_early = Transaction::new_signed_with_payer(
        &[cancel_ix_early],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );

    let result = context.banks_client.process_transaction(cancel_tx_early).await;
    assert!(result.is_err(), "Should have thrown an error");

    // Warp to new slot to ensure clean transaction processing after the failed attempt
    context.warp_to_slot(50).unwrap();

    // Advance time past expiry
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

    // Cancellation allowed after expiry
    let initial_balance = get_token_balance(&mut context, env.requester_token).await;

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

    // Verify funds returned
    let final_balance = get_token_balance(&mut context, env.requester_token).await;
    assert_eq!(final_balance, initial_balance + amount);

    // Verify vault is empty
    let vault_balance = get_token_balance(&mut context, vault_pda).await;
    assert_eq!(vault_balance, 0);

    // Verify escrow state (isClaimed = true after cancel)
    let escrow_account = context
        .banks_client
        .get_account(escrow_pda)
        .await
        .unwrap()
        .unwrap();
    let escrow = read_escrow(&escrow_account);
    assert!(escrow.is_claimed);
}

/// 2. Test: Expiry Timestamp Validation
/// Verifies that expiry timestamp is correctly calculated and stored.
/// Why: Correct expiry calculation is critical for time-based cancellation logic.
#[tokio::test]
async fn test_verify_expiry_timestamp_is_stored_correctly() {
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

    let ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        None, // Default expiry
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

    // Verify requester
    assert_eq!(escrow.requester, env.requester.pubkey());

    // Verify token mint
    assert_eq!(escrow.token_mint, env.mint);

    // Verify amount
    assert_eq!(escrow.amount, amount);

    // Verify isClaimed = false
    assert!(!escrow.is_claimed);

    // Verify expiry
    const DEFAULT_EXPIRY_DURATION: i64 = 120;
    let expected_expiry = block_time + DEFAULT_EXPIRY_DURATION;
    assert!(
        (escrow.expiry - expected_expiry).abs() < 10,
        "Expiry should be approximately block_time + DEFAULT_EXPIRY_DURATION (10 second tolerance)"
    );
}

/// 3. Test: Expired Escrow Claim Prevention
/// Verifies that expired escrows cannot be claimed, even with valid approver signatures.
/// Why: Expired escrows should only be cancellable by the requester, not claimable by solvers.
#[tokio::test]
async fn test_prevent_claim_on_expired_escrow() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);

    // Create escrow with short expiry (2 seconds)
    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        Some(2), // 2 second expiry
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let create_tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(create_tx).await.unwrap();

    // Advance time past expiry
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

    // Claims blocked after expiry
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

    let result = context.banks_client.process_transaction(claim_tx).await;
    assert!(result.is_err(), "Should have thrown an error");

    // Verify vault still has funds
    let vault_balance = get_token_balance(&mut context, vault_pda).await;
    assert_eq!(vault_balance, amount);

    // Verify solver didn't receive funds
    let solver_balance = get_token_balance(&mut context, env.solver_token).await;
    assert_eq!(solver_balance, 0);

    // Verify escrow state unchanged
    let escrow_account = context
        .banks_client
        .get_account(escrow_pda)
        .await
        .unwrap()
        .unwrap();
    let escrow = read_escrow(&escrow_account);
    assert!(!escrow.is_claimed);
    assert_eq!(escrow.amount, amount);
}
