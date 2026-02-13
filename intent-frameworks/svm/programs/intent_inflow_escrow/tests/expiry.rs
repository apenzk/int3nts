mod common;

use common::{
    create_cancel_ix, create_escrow_ix, create_gmp_receive_fulfillment_proof_ix,
    create_gmp_receive_requirements_ix, generate_intent_id, get_token_balance, program_test,
    read_escrow, setup_basic_env, DUMMY_HUB_CHAIN_ID, DUMMY_HUB_GMP_ENDPOINT_ADDR,
};
use gmp_common::messages::{FulfillmentProof, IntentRequirements};
use intent_inflow_escrow::state::seeds;
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
        None,    // No requirements PDA
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
        None, // No requirements PDA
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

/// 3. Test: Expired Escrow Can Be Fulfilled via GMP
/// Verifies that expired escrows CAN still be fulfilled via GMP fulfillment proof.
/// Why: In GMP mode, the hub is the source of truth. If the hub sends a fulfillment proof,
/// it means the solver fulfilled the intent on the other chain, so funds should be released.
/// Local expiry is only relevant for the Cancel operation (requester reclaiming funds).
#[tokio::test]
async fn test_expired_escrow_can_be_fulfilled_via_gmp() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = [4u8; 32];
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

    // Step 2: Create escrow with short expiry (2 seconds)
    let create_ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        Some(2), // 2 second expiry
        Some(requirements_pda),
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let create_tx = Transaction::new_signed_with_payer(
        &[create_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(create_tx).await.unwrap();

    // Step 3: Advance time past expiry
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

    // Step 4: Fulfill via GMP - should succeed even though expired (hub is source of truth)
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
        env.gmp_config_pda, // PDA - must be derived, cannot be a DUMMY constant
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

    // Hub fulfillment proof is honored regardless of local expiry
    context.banks_client.process_transaction(claim_tx).await.unwrap();

    // Verify funds were released to solver
    let vault_balance = get_token_balance(&mut context, vault_pda).await;
    assert_eq!(vault_balance, 0);

    let solver_balance = get_token_balance(&mut context, env.solver_token).await;
    assert_eq!(solver_balance, amount);

    // Verify escrow marked as claimed
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
