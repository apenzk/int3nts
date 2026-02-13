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
        None, // No requirements PDA
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
        None, // Default expiry
        None, // No requirements PDA
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
        None, // Default expiry
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

    // Step 4: Now try to cancel - should fail because already claimed
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
