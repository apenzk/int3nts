mod common;

use common::{
    create_cancel_ix, create_escrow_ix, create_mint, create_token_account, generate_intent_id,
    initialize_program, mint_to, program_test, read_escrow, send_tx, setup_basic_env,
};
use intent_escrow::state::seeds;
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};

/// 1. Test: Zero Amount Rejection
/// Verifies that createEscrow reverts when amount is zero.
/// Why: Zero-amount escrows are meaningless and could cause accounting issues.
#[tokio::test]
async fn test_reject_zero_amount() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 0u64;

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

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Should have thrown an error");
}

/// 2. Test: Insufficient Allowance Rejection
/// Verifies that createEscrow reverts when token allowance is insufficient.
/// Why: Token transfers require explicit approval. Insufficient allowance must be rejected to prevent failed transfers.
/// We mint tokens to ensure the requester has balance, then approve less than needed to test specifically the allowance check, not the balance check.
///
/// NOTE: N/A for SVM - SPL tokens don't use approve/allowance pattern
// EVM: intent-frameworks/evm/test/error-conditions.test.js - "Should revert with insufficient ERC20 allowance"

/// 3. Test: Maximum Value Edge Case
/// Verifies that createEscrow handles maximum u64 values correctly.
/// Why: Edge case testing ensures the program doesn't overflow or fail on boundary values.
#[tokio::test]
async fn test_handle_maximum_u64_value_in_create_escrow() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let payer = context.payer.insecure_clone();
    let program_id = common::test_program_id();

    // Create fresh accounts for this test to avoid overflow from existing balances
    let requester = solana_sdk::signature::Keypair::new();
    let solver = solana_sdk::signature::Keypair::new();
    let approver = solana_sdk::signature::Keypair::new();
    let mint_authority = solana_sdk::signature::Keypair::new();

    // Fund requester
    let fund_ix = solana_sdk::system_instruction::transfer(
        &payer.pubkey(),
        &requester.pubkey(),
        2_000_000_000,
    );
    send_tx(&mut context, &payer, &[fund_ix], &[]).await;

    // Create fresh mint and token accounts
    let mint = create_mint(&mut context, &payer, &mint_authority, 6).await;
    let requester_token = create_token_account(&mut context, &payer, mint, requester.pubkey()).await;

    // Initialize program with fresh approver
    initialize_program(&mut context, &requester, program_id, approver.pubkey()).await;

    let intent_id = generate_intent_id();
    let max_amount = u64::MAX;

    // Mint maximum amount directly to fresh token account (no prior balance to overflow)
    mint_to(
        &mut context,
        &payer,
        mint,
        &mint_authority,
        requester_token,
        max_amount,
    )
    .await;

    let (escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &program_id);

    let ix = create_escrow_ix(
        program_id,
        intent_id,
        max_amount,
        requester.pubkey(),
        mint,
        requester_token,
        solver.pubkey(),
        None,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&requester.pubkey()),
        &[&requester],
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();

    // Verify escrow was created with max amount
    let escrow_account = context
        .banks_client
        .get_account(escrow_pda)
        .await
        .unwrap()
        .unwrap();
    let escrow = read_escrow(&escrow_account);
    assert_eq!(escrow.amount, max_amount);
}

/// 4. Test: Native Currency Escrow Creation with address(0)
/// Verifies that createEscrow accepts address(0) for native currency deposits.
/// Why: Native currency deposits use address(0) as a convention to distinguish from token deposits.
///
/// NOTE: N/A for SVM - No native currency escrow equivalent - all escrows use SPL tokens
// EVM: intent-frameworks/evm/test/error-conditions.test.js - "Should allow ETH escrow creation with address(0)"

/// 5. Test: Native Currency Amount Mismatch Rejection
/// Verifies that createEscrow reverts when msg.value doesn't match amount for native currency deposits.
/// Why: Prevents accidental underpayment or overpayment, ensuring exact amount matching.
///
/// NOTE: N/A for SVM - No native currency deposits - no msg.value equivalent
// EVM: intent-frameworks/evm/test/error-conditions.test.js - "Should revert with ETH amount mismatch"

/// 6. Test: Native Currency Not Accepted for Token Escrow
/// Verifies that createEscrow reverts when native currency is sent with a token address.
/// Why: Prevents confusion between native currency and token deposits. Token escrows should not accept native currency.
///
/// NOTE: N/A for SVM - No native currency/token distinction - all escrows use SPL tokens
// EVM: intent-frameworks/evm/test/error-conditions.test.js - "Should revert when ETH sent with token address"

/// 7. Test: Invalid Signature Length Rejection
/// Verifies that claim reverts with invalid signature length.
/// Why: Signatures must have the correct length. Invalid lengths indicate malformed signatures.
///
/// NOTE: N/A for SVM - Signature validation handled by Ed25519Program, not the escrow program
// EVM: intent-frameworks/evm/test/error-conditions.test.js - "Should revert with invalid signature length"

/// 8. Test: Non-Existent Escrow Cancellation Rejection
/// Verifies that cancel reverts with EscrowDoesNotExist for non-existent escrows.
/// Why: Prevents cancellation of non-existent escrows and ensures proper error handling.
#[tokio::test]
async fn test_revert_cancel_on_non_existent_escrow() {
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
    let tx = Transaction::new_signed_with_payer(
        &[cancel_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );

    let result = context.banks_client.process_transaction(tx).await;
    assert!(result.is_err(), "Should have thrown EscrowDoesNotExist error");
}

/// 9. Test: Zero Solver Address Rejection
/// Verifies that escrows cannot be created with zero/default solver address.
/// Why: A valid solver must be specified for claims.
#[tokio::test]
async fn test_reject_zero_solver_address() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;

    let ix = create_escrow_ix(
        env.program_id,
        intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        Pubkey::default(), // Zero address
        None,
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

/// 10. Test: Duplicate Intent ID Rejection
/// Verifies that escrows with duplicate intent IDs are rejected.
/// Why: Each intent ID must map to exactly one escrow.
#[tokio::test]
async fn test_reject_duplicate_intent_id() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000u64;

    // Create first escrow
    let ix1 = create_escrow_ix(
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
        None,
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

/// 11. Test: Insufficient Token Balance Rejection
/// Verifies that escrow creation fails if requester has insufficient tokens.
/// Why: Cannot deposit more tokens than available.
#[tokio::test]
async fn test_reject_if_requester_has_insufficient_balance() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let intent_id = generate_intent_id();
    let amount = 1_000_000_000_000u64; // More than minted

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

    let result = context.banks_client.process_transaction(tx).await;
    // Token transfer error
    assert!(result.is_err(), "Should have thrown an error");
}
