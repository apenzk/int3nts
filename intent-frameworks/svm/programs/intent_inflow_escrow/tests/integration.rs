mod common;

use common::{
    create_cancel_ix, create_escrow_ix, create_gmp_receive_fulfillment_proof_ix,
    create_gmp_receive_requirements_ix, create_mint, create_token_account, generate_intent_id,
    get_token_balance, initialize_program, mint_to, program_test, read_escrow, send_tx,
    setup_basic_env, DUMMY_HUB_CHAIN_ID, DUMMY_HUB_GMP_ENDPOINT_ADDR,
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
// INTEGRATION TESTS
// ============================================================================

/// 1. Test: Complete Deposit to Claim Workflow (GMP Mode)
/// Verifies the full GMP workflow from requirements → escrow → fulfillment proof.
/// Why: Integration test ensures all components work together correctly in the happy path.
#[tokio::test]
async fn test_complete_full_deposit_to_claim_workflow() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = [6u8; 32];
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

    // Verify escrow created
    let vault_balance_after_create = get_token_balance(&mut context, vault_pda).await;
    assert_eq!(vault_balance_after_create, amount);

    // Step 3: Receive fulfillment proof via GMP (auto-releases escrow)
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
    context.banks_client.process_transaction(claim_tx).await.unwrap();

    // Step 4: Verify final state
    let solver_balance = get_token_balance(&mut context, env.solver_token).await;
    assert_eq!(solver_balance, amount);

    let vault_balance_after_claim = get_token_balance(&mut context, vault_pda).await;
    assert_eq!(vault_balance_after_claim, 0);

    let escrow_account = context
        .banks_client
        .get_account(escrow_pda)
        .await
        .unwrap()
        .unwrap();
    let escrow = read_escrow(&escrow_account);
    assert!(escrow.is_claimed);
}

/// 2. Test: Multi-Token Scenarios
/// Verifies that the escrow works with different SPL token types.
/// Why: The escrow must support any SPL token, not just a single token type.
#[tokio::test]
async fn test_handle_multiple_different_spl_tokens() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let payer = context.payer.insecure_clone();
    let program_id = common::test_program_id();

    // Create fresh accounts
    let requester = solana_sdk::signature::Keypair::new();
    let solver = solana_sdk::signature::Keypair::new();
    let approver = solana_sdk::signature::Keypair::new();
    let mint_authority = solana_sdk::signature::Keypair::new();

    // Fund requester
    let fund_ix = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &requester.pubkey(),
        5_000_000_000,
    );
    send_tx(&mut context, &payer, &[fund_ix], &[]).await;

    // Initialize program
    initialize_program(&mut context, &requester, program_id, approver.pubkey()).await;

    // Create 3 different token mints with different decimals
    let mint1 = create_mint(&mut context, &payer, &mint_authority, 6).await;
    let mint2 = create_mint(&mut context, &payer, &mint_authority, 9).await;
    let mint3 = create_mint(&mut context, &payer, &mint_authority, 18).await;

    // Create token accounts for requester
    let requester_token1 = create_token_account(&mut context, &payer, mint1, requester.pubkey()).await;
    let requester_token2 = create_token_account(&mut context, &payer, mint2, requester.pubkey()).await;
    let requester_token3 = create_token_account(&mut context, &payer, mint3, requester.pubkey()).await;

    // Mint different amounts to each token account
    let amount1 = 100_000u64;
    let amount2 = 200_000u64;
    let amount3 = 300_000u64;

    mint_to(&mut context, &payer, mint1, &mint_authority, requester_token1, amount1).await;
    mint_to(&mut context, &payer, mint2, &mint_authority, requester_token2, amount2).await;
    mint_to(&mut context, &payer, mint3, &mint_authority, requester_token3, amount3).await;

    // Create escrows with different tokens
    let intent_id1 = generate_intent_id();
    let intent_id2 = generate_intent_id();
    let intent_id3 = generate_intent_id();

    // Create escrow 1 with mint1
    let (escrow_pda1, _) = Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id1], &program_id);
    let (vault_pda1, _) = Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id1], &program_id);

    let ix1 = create_escrow_ix(
        program_id,
        intent_id1,
        amount1,
        requester.pubkey(),
        mint1,
        requester_token1,
        solver.pubkey(),
        None, // Default expiry
        None, // No requirements PDA
    );
    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx1 = Transaction::new_signed_with_payer(&[ix1], Some(&requester.pubkey()), &[&requester], blockhash);
    context.banks_client.process_transaction(tx1).await.unwrap();

    // Create escrow 2 with mint2
    let (escrow_pda2, _) = Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id2], &program_id);
    let (vault_pda2, _) = Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id2], &program_id);

    let ix2 = create_escrow_ix(
        program_id,
        intent_id2,
        amount2,
        requester.pubkey(),
        mint2,
        requester_token2,
        solver.pubkey(),
        None, // Default expiry
        None, // No requirements PDA
    );
    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx2 = Transaction::new_signed_with_payer(&[ix2], Some(&requester.pubkey()), &[&requester], blockhash);
    context.banks_client.process_transaction(tx2).await.unwrap();

    // Create escrow 3 with mint3
    let (escrow_pda3, _) = Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id3], &program_id);
    let (vault_pda3, _) = Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id3], &program_id);

    let ix3 = create_escrow_ix(
        program_id,
        intent_id3,
        amount3,
        requester.pubkey(),
        mint3,
        requester_token3,
        solver.pubkey(),
        None, // Default expiry
        None, // No requirements PDA
    );
    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx3 = Transaction::new_signed_with_payer(&[ix3], Some(&requester.pubkey()), &[&requester], blockhash);
    context.banks_client.process_transaction(tx3).await.unwrap();

    // Verify all escrows were created correctly
    let escrow1 = read_escrow(&context.banks_client.get_account(escrow_pda1).await.unwrap().unwrap());
    let escrow2 = read_escrow(&context.banks_client.get_account(escrow_pda2).await.unwrap().unwrap());
    let escrow3 = read_escrow(&context.banks_client.get_account(escrow_pda3).await.unwrap().unwrap());

    assert_eq!(escrow1.token_mint, mint1);
    assert_eq!(escrow1.amount, amount1);
    assert_eq!(escrow2.token_mint, mint2);
    assert_eq!(escrow2.amount, amount2);
    assert_eq!(escrow3.token_mint, mint3);
    assert_eq!(escrow3.amount, amount3);

    // Verify vault balances
    assert_eq!(get_token_balance(&mut context, vault_pda1).await, amount1);
    assert_eq!(get_token_balance(&mut context, vault_pda2).await, amount2);
    assert_eq!(get_token_balance(&mut context, vault_pda3).await, amount3);
}

/// 3. Test: Comprehensive Log Emission
/// Verifies that all program logs are emitted with correct parameters.
/// Why: Logs are critical for off-chain monitoring and debugging. Incorrect logs break integrations.
///
/// NOTE: N/A for SVM - solana-program-test does not capture msg!() output in transaction metadata.
/// The msg!() logs are emitted to stdout during test execution but cannot be programmatically
/// asserted. On a real validator, these logs would be captured and queryable via RPC.
/// The program DOES emit structured logs (visible in test output):
///   - "Instruction: CreateEscrow"
///   - "Escrow created: intent_id=..., amount=..., expiry=..."
///   - "Instruction: Claim"
///   - "Escrow claimed: intent_id=..., amount=..."
///   - "Instruction: Cancel"
///   - "Escrow cancelled: intent_id=..., amount=..."

/// 4. Test: Complete Cancellation Workflow
/// Verifies the full workflow from escrow creation through cancellation after expiry.
/// Why: Integration test ensures the cancellation flow works end-to-end after expiry.
#[tokio::test]
async fn test_complete_full_cancellation_workflow() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
    let (vault_pda, _) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &env.program_id);

    let initial_requester_balance = get_token_balance(&mut context, env.requester_token).await;

    // Step 1: Create escrow with short expiry
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

    // Step 2: Advance time past expiry
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

    // Step 3: Cancel and reclaim
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

    // Step 4: Verify final state
    let final_requester_balance = get_token_balance(&mut context, env.requester_token).await;
    assert_eq!(final_requester_balance, initial_requester_balance);

    let vault_balance = get_token_balance(&mut context, vault_pda).await;
    assert_eq!(vault_balance, 0);
}
