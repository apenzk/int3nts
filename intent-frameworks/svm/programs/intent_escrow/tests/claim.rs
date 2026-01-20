mod common;

use common::{
    create_claim_ix, create_ed25519_instruction, create_escrow_ix, generate_intent_id,
    get_token_balance, program_test, read_escrow, setup_basic_env,
};
use intent_escrow::state::seeds;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signer,
    transaction::Transaction,
};

// ============================================================================
// CLAIM TESTS
// ============================================================================

/// 1. Test: Valid Claim with Verifier Signature
/// Verifies that solvers can claim escrow funds when provided with a valid verifier signature.
/// Why: Claiming is the core fulfillment mechanism. Solvers must be able to receive funds after verifier approval.
#[tokio::test]
async fn test_claim_with_valid_verifier_signature() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = [2u8; 32];
    let amount = 500_000u64;

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

    // Sign the intent_id with verifier's keypair
    let signature = env.verifier.sign_message(&intent_id);
    let mut signature_bytes = [0u8; 64];
    signature_bytes.copy_from_slice(signature.as_ref());

    let ed25519_ix = create_ed25519_instruction(&intent_id, &signature_bytes, &env.verifier.pubkey());

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
}

/// 2. Test: Invalid Signature Rejection
/// Verifies that claims with invalid signatures are rejected with UnauthorizedVerifier error.
/// Why: Security requirement - only verifier-approved fulfillments should allow fund release.
#[tokio::test]
async fn test_revert_with_invalid_signature() {
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

    // Sign with wrong keypair (solver instead of verifier)
    let wrong_signature = env.solver.sign_message(&intent_id);
    let mut wrong_signature_bytes = [0u8; 64];
    wrong_signature_bytes.copy_from_slice(wrong_signature.as_ref());

    // Create Ed25519 instruction with wrong signer's public key
    let ed25519_ix = create_ed25519_instruction(
        &intent_id,
        &wrong_signature_bytes,
        &env.solver.pubkey(), // Wrong signer
    );

    let claim_ix = create_claim_ix(
        env.program_id,
        intent_id,
        wrong_signature_bytes,
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
}

/// 3. Test: Signature Replay Prevention
/// Verifies that a signature for one intent_id cannot be reused on a different escrow with a different intent_id.
/// Why: Signatures must be bound to specific intent_ids to prevent replay attacks across different escrows.
#[tokio::test]
async fn test_prevent_signature_replay_across_different_intent_ids() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id_a = generate_intent_id();
    let intent_id_b = generate_intent_id();
    let amount = 500_000u64; // Use smaller amount so we can create two escrows

    // Mint additional tokens to requester for the second escrow
    use common::mint_to;
    let payer = context.payer.insecure_clone();
    mint_to(
        &mut context,
        &payer,
        env.mint,
        &env.mint_authority,
        env.requester_token,
        1_000_000u64,
    )
    .await;

    // Create escrow A
    let create_ix_a = create_escrow_ix(
        env.program_id,
        intent_id_a,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        None,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let create_tx_a = Transaction::new_signed_with_payer(
        &[create_ix_a],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(create_tx_a).await.unwrap();

    // Create escrow B
    let create_ix_b = create_escrow_ix(
        env.program_id,
        intent_id_b,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        None,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let create_tx_b = Transaction::new_signed_with_payer(
        &[create_ix_b],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(create_tx_b).await.unwrap();

    let (escrow_pda_b, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id_b], &env.program_id);
    let (vault_pda_b, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id_b], &env.program_id);

    // Create a VALID signature for intent_id A (the first escrow)
    let signature_for_a = env.verifier.sign_message(&intent_id_a);
    let mut signature_bytes = [0u8; 64];
    signature_bytes.copy_from_slice(signature_for_a.as_ref());

    // Create Ed25519 instruction for intent A signature
    let ed25519_ix = create_ed25519_instruction(&intent_id_a, &signature_bytes, &env.verifier.pubkey());

    // Try to use the signature for intent_id A on escrow B (which has intent_id B)
    // This should fail because the signature is bound to intent_id A, not intent_id B
    let claim_ix = create_claim_ix(
        env.program_id,
        intent_id_b,
        signature_bytes,
        escrow_pda_b,
        env.state_pda,
        vault_pda_b,
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
}

/// 4. Test: Duplicate Claim Prevention
/// Verifies that attempting to claim an already-claimed escrow reverts.
/// Why: Prevents double-spending - each escrow can only be claimed once.
#[tokio::test]
async fn test_revert_if_escrow_already_claimed() {
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

    // First claim
    let signature = env.verifier.sign_message(&intent_id);
    let mut signature_bytes = [0u8; 64];
    signature_bytes.copy_from_slice(signature.as_ref());

    let ed25519_ix = create_ed25519_instruction(&intent_id, &signature_bytes, &env.verifier.pubkey());

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

    // Verify escrow is marked as claimed and vault is empty
    let escrow_account_after_first = context
        .banks_client
        .get_account(escrow_pda)
        .await
        .unwrap()
        .unwrap();
    let escrow_after_first = read_escrow(&escrow_account_after_first);
    assert!(escrow_after_first.is_claimed, "Escrow should be marked as claimed after first claim");
    assert_eq!(escrow_after_first.amount, 0, "Escrow amount should be 0 after first claim");
    
    let vault_balance_after_first = get_token_balance(&mut context, vault_pda).await;
    assert_eq!(vault_balance_after_first, 0, "Vault should be empty after first claim");

    // Warp to next slot to ensure clean transaction processing
    context.warp_to_slot(100).unwrap();

    // Second claim should fail - create fresh instructions
    // Note: Even though vault is empty, the is_claimed check should catch this first
    let signature2 = env.verifier.sign_message(&intent_id);
    let mut signature_bytes2 = [0u8; 64];
    signature_bytes2.copy_from_slice(signature2.as_ref());

    let ed25519_ix2 = create_ed25519_instruction(&intent_id, &signature_bytes2, &env.verifier.pubkey());

    let claim_ix2 = create_claim_ix(
        env.program_id,
        intent_id,
        signature_bytes2,
        escrow_pda,
        env.state_pda,
        vault_pda,
        env.solver_token,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let claim_tx2 = Transaction::new_signed_with_payer(
        &[ed25519_ix2, claim_ix2],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        blockhash,
    );

    let result = context.banks_client.process_transaction(claim_tx2).await;
    assert!(result.is_err(), "Should have thrown an error - escrow already claimed");
}

/// 5. Test: Non-Existent Escrow Rejection
/// Verifies that attempting to claim a non-existent escrow reverts with EscrowDoesNotExist error.
/// Why: Prevents claims on non-existent escrows and ensures proper error handling.
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

    let message = non_existent_intent_id;
    let signature = env.verifier.sign_message(&message);
    let mut signature_bytes = [0u8; 64];
    signature_bytes.copy_from_slice(signature.as_ref());

    let ed25519_ix = create_ed25519_instruction(&message, &signature_bytes, &env.verifier.pubkey());

    let claim_ix = create_claim_ix(
        env.program_id,
        non_existent_intent_id,
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
}
