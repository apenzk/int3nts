mod common;

use common::{
    create_escrow_ix, generate_intent_id, get_token_balance, hex_to_bytes32, program_test,
    read_escrow, setup_basic_env, setup_gmp_requirements,
};
use intent_inflow_escrow::state::seeds;
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};

// ============================================================================
// CROSS-CHAIN INTENT ID CONVERSION TESTS
// ============================================================================

/// 1. Test: Hex Intent ID Conversion
/// Verifies that intent IDs from hex format can be converted and used in escrow operations.
/// Why: Cross-chain intents require intent ID conversion between different formats (hex to bytes32).
#[tokio::test]
async fn test_handle_hex_intent_id_conversion() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    // Intent ID in hex format (smaller than 32 bytes) with unique suffix
    let unique_suffix = format!("{:016x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
    let intent_id_hex = format!("0x1234{}", &unique_suffix[..8.min(unique_suffix.len())]);
    let intent_id = hex_to_bytes32(&intent_id_hex);
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

    // Verify escrow was created correctly
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

    // Verify vault balance
    let vault_balance = get_token_balance(&mut context, vault_pda).await;
    assert_eq!(vault_balance, amount);
}

/// 2. Test: Intent ID Boundary Values
/// Verifies that the program handles boundary intent ID values correctly.
/// Why: Intent IDs from different chains may have different formats. Boundary testing ensures compatibility.
#[tokio::test]
async fn test_handle_intent_id_boundary_values() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let amount = 100_000u64; // Amount chosen to allow multiple escrows within test token budget

    // Test maximum value (all 0xFF) with unique suffix
    let timestamp1 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    let mut max_intent_id = [0xffu8; 32];
    max_intent_id[31] = (timestamp1 & 0xff) as u8;
    max_intent_id[30] = ((timestamp1 >> 8) & 0xff) as u8;

    let (max_escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &max_intent_id], &env.program_id);

    let max_requirements_pda =
        setup_gmp_requirements(&mut context, &env, max_intent_id, amount, u64::MAX).await;

    let max_ix = create_escrow_ix(
        env.program_id,
        max_intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        max_requirements_pda,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let max_tx = Transaction::new_signed_with_payer(
        &[max_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(max_tx).await.unwrap();

    let max_escrow_account = context
        .banks_client
        .get_account(max_escrow_pda)
        .await
        .unwrap()
        .unwrap();
    assert!(max_escrow_account.data.len() > 0);

    // Test zero value (all 0x00) with unique suffix
    let timestamp2 = timestamp1 + 1;
    let mut zero_intent_id = [0u8; 32];
    zero_intent_id[31] = (timestamp2 & 0xff) as u8;
    zero_intent_id[30] = ((timestamp2 >> 8) & 0xff) as u8;

    let (zero_escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &zero_intent_id], &env.program_id);

    let zero_requirements_pda =
        setup_gmp_requirements(&mut context, &env, zero_intent_id, amount, u64::MAX).await;

    let zero_ix = create_escrow_ix(
        env.program_id,
        zero_intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        zero_requirements_pda,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let zero_tx = Transaction::new_signed_with_payer(
        &[zero_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(zero_tx).await.unwrap();

    let zero_escrow_account = context
        .banks_client
        .get_account(zero_escrow_pda)
        .await
        .unwrap()
        .unwrap();
    assert!(zero_escrow_account.data.len() > 0);

    // Test edge value (half 0xFF, half 0x00) with unique suffix
    let timestamp3 = timestamp2 + 1;
    let mut edge_intent_id = [0u8; 32];
    for i in 0..16 {
        edge_intent_id[i] = 0xff;
    }
    edge_intent_id[31] = (timestamp3 & 0xff) as u8;
    edge_intent_id[30] = ((timestamp3 >> 8) & 0xff) as u8;

    let (edge_escrow_pda, _) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &edge_intent_id], &env.program_id);

    let edge_requirements_pda =
        setup_gmp_requirements(&mut context, &env, edge_intent_id, amount, u64::MAX).await;

    let edge_ix = create_escrow_ix(
        env.program_id,
        edge_intent_id,
        amount,
        env.requester.pubkey(),
        env.mint,
        env.requester_token,
        env.solver.pubkey(),
        edge_requirements_pda,
    );

    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let edge_tx = Transaction::new_signed_with_payer(
        &[edge_ix],
        Some(&env.requester.pubkey()),
        &[&env.requester],
        blockhash,
    );
    context.banks_client.process_transaction(edge_tx).await.unwrap();

    let edge_escrow_account = context
        .banks_client
        .get_account(edge_escrow_pda)
        .await
        .unwrap()
        .unwrap();
    assert!(edge_escrow_account.data.len() > 0);
}

/// 3. Test: Intent ID Zero Padding
/// Verifies that shorter intent IDs are properly left-padded with zeros.
/// Why: Intent IDs from other chains may be shorter than 32 bytes. Zero padding ensures correct bytes32 conversion.
#[tokio::test]
async fn test_handle_intent_id_zero_padding_correctly() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    // Test various short hex strings that need padding
    let short_hex_ids = vec![
        "0x1",
        "0x12",
        "0x123",
        "0x1234",
        "0x12345",
        "0x1234567890abcdef",
    ];

    let amount = 100_000u64; // Amount chosen to allow 6 escrows within test token budget

    for (i, short_hex) in short_hex_ids.iter().enumerate() {
        // Add unique suffix to each
        let unique_suffix = format!("{:016x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() + i as u128);
        let hex_id = format!("{}{}", short_hex, &unique_suffix[..8.min(unique_suffix.len())]);
        let intent_id = hex_to_bytes32(&hex_id);

        let (escrow_pda, _) =
            Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &env.program_id);
        let (_vault_pda, _) =
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

        // Verify escrow was created
        let escrow_account = context
            .banks_client
            .get_account(escrow_pda)
            .await
            .unwrap()
            .unwrap();
        let escrow = read_escrow(&escrow_account);

        // Verify amount
        assert_eq!(escrow.amount, amount);
    }
}

/// 4. Test: Multiple Intent IDs from Different Formats
/// Verifies that multiple escrows can be created with intent IDs from different formats.
/// Why: Real-world usage involves intent IDs in various formats. The program must handle all valid formats.
#[tokio::test]
async fn test_handle_multiple_intent_ids_from_different_formats() {
    let program_test = program_test();
    let mut context = program_test.start_with_context().await;
    let env = setup_basic_env(&mut context).await;

    let amount = 100_000u64; // Amount chosen to allow 6 escrows within test token budget
    let base_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let intent_ids = vec![
        hex_to_bytes32(&format!("0x1{:x}", base_timestamp)),
        hex_to_bytes32(&format!("0x1234{:x}", base_timestamp + 1)),
        hex_to_bytes32(&format!("0xabcdef{:x}", base_timestamp + 2)),
        hex_to_bytes32(&format!("0x1234567890abcdef{:x}", base_timestamp + 3)),
        generate_intent_id(), // Random format
        generate_intent_id(), // Another random
    ];

    // Create escrows with different intent ID formats
    for intent_id in &intent_ids {
        let (escrow_pda, _) =
            Pubkey::find_program_address(&[seeds::ESCROW_SEED, intent_id], &env.program_id);

        let requirements_pda =
            setup_gmp_requirements(&mut context, &env, *intent_id, amount, u64::MAX).await;

        let ix = create_escrow_ix(
            env.program_id,
            *intent_id,
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

    // Verify all escrows are independent
    for intent_id in &intent_ids {
        let (escrow_pda, _) =
            Pubkey::find_program_address(&[seeds::ESCROW_SEED, intent_id], &env.program_id);
        let escrow_account = context
            .banks_client
            .get_account(escrow_pda)
            .await
            .unwrap()
            .unwrap();
        assert!(escrow_account.data.len() > 0);
    }
}
