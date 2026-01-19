#![allow(dead_code)]
#![allow(deprecated)]

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_pack::Pack;
use solana_program_test::{processor, ProgramTest, ProgramTestContext};
use solana_sdk::system_instruction;
use solana_sdk::{
    ed25519_instruction::new_ed25519_instruction_with_signature,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    sysvar,
    transaction::Transaction,
};

use intent_escrow::{
    instruction::EscrowInstruction,
    state::{seeds, Escrow, EscrowState},
};

// ============================================================================
// TEST PROGRAM ID
// ============================================================================

/// Fixed program ID for testing. Actual deployed program ID is determined by
/// the deployment keypair, not this value.
pub fn test_program_id() -> Pubkey {
    // Use a deterministic pubkey derived from a known seed for testing
    solana_sdk::pubkey!("Escrow11111111111111111111111111111111111111")
}

// ============================================================================
// TEST HARNESS HELPERS
// ============================================================================

/// Helper: Build a ProgramTest instance with intent_escrow + spl_token
pub fn program_test() -> ProgramTest {
    let program_id = test_program_id();
    let mut program_test = ProgramTest::new(
        "intent_escrow",
        program_id,
        processor!(intent_escrow::processor::Processor::process),
    );
    program_test.add_program(
        "spl_token",
        spl_token::id(),
        processor!(spl_token::processor::Processor::process),
    );
    program_test
}

/// Helper: Send a transaction with a specific payer and signers
pub async fn send_tx(
    context: &mut ProgramTestContext,
    payer: &Keypair,
    instructions: &[Instruction],
    signers: &[&Keypair],
) {
    let blockhash = context.banks_client.get_latest_blockhash().await.unwrap();
    let mut all_signers = Vec::with_capacity(signers.len() + 1);
    all_signers.push(payer);
    for signer in signers {
        if signer.pubkey() != payer.pubkey() {
            all_signers.push(*signer);
        }
    }

    let tx = Transaction::new_signed_with_payer(
        instructions,
        Some(&payer.pubkey()),
        &all_signers,
        blockhash,
    );
    context.banks_client.process_transaction(tx).await.unwrap();
}

// ============================================================================
// SPL TOKEN HELPERS
// ============================================================================

/// Helper: Create a new SPL token mint
pub async fn create_mint(
    context: &mut ProgramTestContext,
    payer: &Keypair,
    mint_authority: &Keypair,
    decimals: u8,
) -> Pubkey {
    let mint = Keypair::new();
    let rent = context.banks_client.get_rent().await.unwrap();
    let mint_rent = rent.minimum_balance(spl_token::state::Mint::LEN);

    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_rent,
        spl_token::state::Mint::LEN as u64,
        &spl_token::id(),
    );
    let init_mint_ix = spl_token::instruction::initialize_mint2(
        &spl_token::id(),
        &mint.pubkey(),
        &mint_authority.pubkey(),
        None,
        decimals,
    )
    .unwrap();

    send_tx(context, payer, &[create_mint_ix, init_mint_ix], &[&mint]).await;
    mint.pubkey()
}

/// Helper: Create an SPL token account for a given mint and owner
pub async fn create_token_account(
    context: &mut ProgramTestContext,
    payer: &Keypair,
    mint: Pubkey,
    owner: Pubkey,
) -> Pubkey {
    let token_account = Keypair::new();
    let rent = context.banks_client.get_rent().await.unwrap();
    let token_rent = rent.minimum_balance(spl_token::state::Account::LEN);

    let create_ix = system_instruction::create_account(
        &payer.pubkey(),
        &token_account.pubkey(),
        token_rent,
        spl_token::state::Account::LEN as u64,
        &spl_token::id(),
    );
    let init_ix = spl_token::instruction::initialize_account3(
        &spl_token::id(),
        &token_account.pubkey(),
        &mint,
        &owner,
    )
    .unwrap();

    send_tx(context, payer, &[create_ix, init_ix], &[&token_account]).await;
    token_account.pubkey()
}

/// Helper: Mint tokens to a token account
pub async fn mint_to(
    context: &mut ProgramTestContext,
    payer: &Keypair,
    mint: Pubkey,
    mint_authority: &Keypair,
    destination: Pubkey,
    amount: u64,
) {
    let ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        &mint,
        &destination,
        &mint_authority.pubkey(),
        &[],
        amount,
    )
    .unwrap();

    send_tx(context, payer, &[ix], &[mint_authority]).await;
}

/// Helper: Read SPL token account balance
pub async fn get_token_balance(
    context: &mut ProgramTestContext,
    token_account: Pubkey,
) -> u64 {
    let account = context
        .banks_client
        .get_account(token_account)
        .await
        .unwrap()
        .unwrap();
    let token_state = spl_token::state::Account::unpack(&account.data).unwrap();
    token_state.amount
}

// ============================================================================
// PROGRAM HELPERS
// ============================================================================

/// Helper: Initialize the program state with a verifier
pub async fn initialize_program(
    context: &mut ProgramTestContext,
    payer: &Keypair,
    program_id: Pubkey,
    verifier: Pubkey,
) -> Pubkey {
    let (state_pda, _state_bump) =
        Pubkey::find_program_address(&[seeds::STATE_SEED], &program_id);

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data: EscrowInstruction::Initialize { verifier }
            .try_to_vec()
            .unwrap(),
    };

    send_tx(context, payer, &[ix], &[]).await;
    state_pda
}

/// Helper: Build a CreateEscrow instruction
pub fn create_escrow_ix(
    program_id: Pubkey,
    intent_id: [u8; 32],
    amount: u64,
    requester: Pubkey,
    token_mint: Pubkey,
    requester_token: Pubkey,
    reserved_solver: Pubkey,
    expiry_duration: Option<i64>,
) -> Instruction {
    let (escrow_pda, _escrow_bump) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &program_id);
    let (vault_pda, _vault_bump) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &program_id);

    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(escrow_pda, false),
            AccountMeta::new(requester, true),
            AccountMeta::new_readonly(token_mint, false),
            AccountMeta::new(requester_token, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new_readonly(reserved_solver, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: EscrowInstruction::CreateEscrow {
            intent_id,
            amount,
            expiry_duration,
        }
        .try_to_vec()
        .unwrap(),
    }
}

/// Helper: Build a Claim instruction
pub fn create_claim_ix(
    program_id: Pubkey,
    intent_id: [u8; 32],
    signature: [u8; 64],
    escrow_pda: Pubkey,
    state_pda: Pubkey,
    vault_pda: Pubkey,
    solver_token: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(escrow_pda, false),
            AccountMeta::new_readonly(state_pda, false),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new(solver_token, false),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: EscrowInstruction::Claim {
            intent_id,
            signature,
        }
        .try_to_vec()
        .unwrap(),
    }
}

/// Helper: Build a Cancel instruction
pub fn create_cancel_ix(
    program_id: Pubkey,
    intent_id: [u8; 32],
    requester: Pubkey,
    requester_token: Pubkey,
    escrow_pda: Pubkey,
    vault_pda: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(escrow_pda, false),
            AccountMeta::new(requester, true),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new(requester_token, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: EscrowInstruction::Cancel { intent_id }
            .try_to_vec()
            .unwrap(),
    }
}

/// Helper: Read escrow state from account data
pub fn read_escrow(account: &solana_sdk::account::Account) -> Escrow {
    Escrow::try_from_slice(&account.data).unwrap()
}

/// Helper: Read global state from account data
pub fn read_state(account: &solana_sdk::account::Account) -> EscrowState {
    EscrowState::try_from_slice(&account.data).unwrap()
}

// ============================================================================
// TEST ENVIRONMENT
// ============================================================================

/// Test environment with common accounts and SPL token setup
pub struct TestEnv {
    pub program_id: Pubkey,
    pub requester: Keypair,
    pub solver: Keypair,
    pub verifier: Keypair,
    pub mint_authority: Keypair,
    pub mint: Pubkey,
    pub requester_token: Pubkey,
    pub solver_token: Pubkey,
    pub state_pda: Pubkey,
}

/// Helper: Create a baseline environment used by most tests
pub async fn setup_basic_env(context: &mut ProgramTestContext) -> TestEnv {
    let payer = context.payer.insecure_clone();
    let payer_pubkey = payer.pubkey();
    let program_id = test_program_id();
    let requester = Keypair::new();
    let solver = Keypair::new();
    let verifier = Keypair::new();
    let mint_authority = Keypair::new();

    // Fund requester and solver
    let fund_ix =
        system_instruction::transfer(&payer_pubkey, &requester.pubkey(), 2_000_000_000);
    let fund_ix = fund_ix;
    let fund_ix2 =
        system_instruction::transfer(&payer_pubkey, &solver.pubkey(), 2_000_000_000);
    send_tx(context, &payer, &[fund_ix, fund_ix2], &[]).await;

    // Create mint and token accounts
    let mint = create_mint(context, &payer, &mint_authority, 6).await;
    let requester_token =
        create_token_account(context, &payer, mint, requester.pubkey()).await;
    let solver_token =
        create_token_account(context, &payer, mint, solver.pubkey()).await;

    // Mint tokens to requester
    mint_to(
        context,
        &payer,
        mint,
        &mint_authority,
        requester_token,
        1_000_000,
    )
    .await;

    // Initialize program
    let state_pda = initialize_program(context, &requester, program_id, verifier.pubkey()).await;

    TestEnv {
        program_id,
        requester,
        solver,
        verifier,
        mint_authority,
        mint,
        requester_token,
        solver_token,
        state_pda,
    }
}

// ============================================================================
// ED25519 SIGNATURE HELPERS
// ============================================================================

/// Helper: Create an Ed25519 verify instruction for signature verification
/// The signature must be created by signing the message with the verifier's keypair
pub fn create_ed25519_instruction(
    message: &[u8],
    signature: &[u8; 64],
    public_key: &Pubkey,
) -> Instruction {
    new_ed25519_instruction_with_signature(message, signature, &public_key.to_bytes())
}

// ============================================================================
// INTENT ID HELPERS
// ============================================================================

/// Helper: Generate a random 32-byte intent ID
pub fn generate_intent_id() -> [u8; 32] {
    use rand::RngCore;
    let mut id = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut id);
    id
}

/// Helper: Convert a hex string to a 32-byte array
/// Useful for cross-chain intent ID compatibility
/// Supports hex strings with or without 0x prefix
/// Left-pads with zeros if hex string is shorter than 32 bytes
/// Handles odd-length hex strings by prepending a '0'
pub fn hex_to_bytes32(hex_string: &str) -> [u8; 32] {
    let hex = hex_string.strip_prefix("0x").unwrap_or(hex_string);
    // Ensure even length by prepending '0' if needed
    let hex = if hex.len() % 2 == 1 {
        format!("0{}", hex)
    } else {
        hex.to_string()
    };
    let mut bytes = [0u8; 32];
    match hex::decode(&hex) {
        Ok(hex_bytes) => {
            let start = 32usize.saturating_sub(hex_bytes.len());
            if start < 32 {
                bytes[start..].copy_from_slice(&hex_bytes);
            }
        }
        Err(_) => {
            // If hex decode fails, panic with helpful message
            panic!("Invalid hex string: {}", hex_string);
        }
    }
    bytes
}

// ============================================================================
// ERROR CHECKING HELPERS
// ============================================================================

// Note: Error code checking helper removed - use result.is_err() for now
// Specific error code checking can be added later if needed by inspecting error messages
