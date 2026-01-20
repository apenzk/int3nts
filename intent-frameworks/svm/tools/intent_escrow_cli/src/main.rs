use borsh::{BorshDeserialize, BorshSerialize};
use intent_escrow::{
    instruction::EscrowInstruction,
    state::{seeds, Escrow, EscrowState},
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    ed25519_instruction::new_ed25519_instruction_with_signature,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signer},
    sysvar,
    transaction::Transaction,
};
use solana_program::program_pack::Pack;
use spl_token::state::Account as TokenAccount;
use std::{collections::HashMap, env, error::Error, str::FromStr};

// ============================================================================
// CLI ENTRYPOINT
// ============================================================================

fn main() {
    if let Err(error) = run() {
        eprintln!("[intent_escrow_cli] Error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_usage();
        return Ok(());
    }

    let command = args[0].as_str();
    let options = parse_options(&args[1..])?;

    let rpc_url = options
        .get("rpc")
        .cloned()
        .unwrap_or_else(|| "http://localhost:8899".to_string());
    let client = RpcClient::new(rpc_url);

    // Commands that don't require program-id
    if command == "get-token-balance" {
        return handle_get_token_balance(&client, &options);
    }

    // All other commands require program-id
    let program_id = match options.get("program-id") {
        Some(value) => parse_pubkey(value)?,
        None => {
            eprintln!("Error: --program-id is required for '{}'", command);
            print_usage();
            std::process::exit(1);
        }
    };

    match command {
        "initialize" => handle_initialize(&client, &options, program_id),
        "create-escrow" => handle_create_escrow(&client, &options, program_id),
        "claim" => handle_claim(&client, &options, program_id),
        "cancel" => handle_cancel(&client, &options, program_id),
        "get-escrow" => handle_get_escrow(&client, &options, program_id),
        _ => {
            print_usage();
            Ok(())
        }
    }
}

// ============================================================================
// COMMAND HANDLERS
// ============================================================================

fn handle_initialize(
    client: &RpcClient,
    options: &HashMap<String, String>,
    program_id: Pubkey,
) -> Result<(), Box<dyn Error>> {
    let payer = read_keypair(options, "payer")?;
    let verifier = parse_pubkey(required_option(options, "verifier")?)?;

    let (state_pda, _state_bump) =
        Pubkey::find_program_address(&[seeds::STATE_SEED], &program_id);

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(state_pda, false),
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data: EscrowInstruction::Initialize { verifier }.try_to_vec()?,
    };

    let signature = send_tx(client, &[ix], &payer, &[])?;
    println!("Initialize signature: {signature}");
    println!("State PDA: {state_pda}");
    Ok(())
}

fn handle_create_escrow(
    client: &RpcClient,
    options: &HashMap<String, String>,
    program_id: Pubkey,
) -> Result<(), Box<dyn Error>> {
    let payer = read_keypair(options, "payer")?;
    let requester = read_keypair(options, "requester")?;

    let token_mint = parse_pubkey(required_option(options, "token-mint")?)?;
    let requester_token = parse_pubkey(required_option(options, "requester-token")?)?;
    let solver = parse_pubkey(required_option(options, "solver")?)?;
    let intent_id = parse_intent_id(required_option(options, "intent-id")?)?;
    let amount = parse_u64(required_option(options, "amount")?)?;
    let expiry = options
        .get("expiry")
        .map(|value| parse_i64(value))
        .transpose()?;

    let create_ix = build_create_escrow_ix(
        program_id,
        intent_id,
        amount,
        requester.pubkey(),
        token_mint,
        requester_token,
        solver,
        expiry,
    )?;

    let signature = send_tx(client, &[create_ix], &payer, &[&requester])?;
    let (escrow_pda, _) = Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &program_id);
    let (vault_pda, _) = Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &program_id);

    println!("Create escrow signature: {signature}");
    println!("Escrow PDA: {escrow_pda}");
    println!("Vault PDA: {vault_pda}");
    Ok(())
}

fn handle_claim(
    client: &RpcClient,
    options: &HashMap<String, String>,
    program_id: Pubkey,
) -> Result<(), Box<dyn Error>> {
    let payer = read_keypair(options, "payer")?;
    let intent_id = parse_intent_id(required_option(options, "intent-id")?)?;
    let signature = parse_signature(required_option(options, "signature")?)?;
    let solver_token = parse_pubkey(required_option(options, "solver-token")?)?;

    let (state_pda, _state_bump) =
        Pubkey::find_program_address(&[seeds::STATE_SEED], &program_id);
    let (escrow_pda, _) = Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &program_id);
    let (vault_pda, _) = Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &program_id);

    let state_account = client.get_account(&state_pda)?;
    let state = EscrowState::try_from_slice(&state_account.data)?;

    let ed25519_ix = new_ed25519_instruction_with_signature(
        &intent_id,
        &signature,
        &state.verifier.to_bytes(),
    );

    let claim_ix = build_claim_ix(
        program_id,
        intent_id,
        signature,
        escrow_pda,
        state_pda,
        vault_pda,
        solver_token,
    )?;

    let signature = send_tx(client, &[ed25519_ix, claim_ix], &payer, &[])?;
    println!("Claim signature: {signature}");
    Ok(())
}

fn handle_cancel(
    client: &RpcClient,
    options: &HashMap<String, String>,
    program_id: Pubkey,
) -> Result<(), Box<dyn Error>> {
    let payer = read_keypair(options, "payer")?;
    let requester = read_keypair(options, "requester")?;
    let requester_token = parse_pubkey(required_option(options, "requester-token")?)?;
    let intent_id = parse_intent_id(required_option(options, "intent-id")?)?;

    let cancel_ix = build_cancel_ix(
        program_id,
        intent_id,
        requester.pubkey(),
        requester_token,
    )?;

    let signature = send_tx(client, &[cancel_ix], &payer, &[&requester])?;
    println!("Cancel signature: {signature}");
    Ok(())
}

fn handle_get_escrow(
    client: &RpcClient,
    options: &HashMap<String, String>,
    program_id: Pubkey,
) -> Result<(), Box<dyn Error>> {
    let intent_id = parse_intent_id(required_option(options, "intent-id")?)?;
    let (escrow_pda, _) = Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &program_id);
    let account = client.get_account(&escrow_pda)?;
    let escrow = Escrow::try_from_slice(&account.data)?;

    println!("Escrow PDA: {escrow_pda}");
    println!("Requester: {}", escrow.requester);
    println!("Token mint: {}", escrow.token_mint);
    println!("Amount: {}", escrow.amount);
    println!("Expiry: {}", escrow.expiry);
    println!("Reserved solver: {}", escrow.reserved_solver);
    println!("Claimed: {}", escrow.is_claimed);
    Ok(())
}

fn handle_get_token_balance(
    client: &RpcClient,
    options: &HashMap<String, String>,
) -> Result<(), Box<dyn Error>> {
    let token_account = parse_pubkey(required_option(options, "token-account")?)?;
    let account = client.get_account(&token_account)?;
    let token_state = TokenAccount::unpack(&account.data)?;
    println!("Token account: {token_account}");
    println!("Balance: {}", token_state.amount);
    Ok(())
}

// ============================================================================
// INSTRUCTION BUILDERS
// ============================================================================

fn build_create_escrow_ix(
    program_id: Pubkey,
    intent_id: [u8; 32],
    amount: u64,
    requester: Pubkey,
    token_mint: Pubkey,
    requester_token: Pubkey,
    reserved_solver: Pubkey,
    expiry_duration: Option<i64>,
) -> Result<Instruction, Box<dyn Error>> {
    let (escrow_pda, _escrow_bump) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &program_id);
    let (vault_pda, _vault_bump) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &program_id);

    Ok(Instruction {
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
        .try_to_vec()?,
    })
}

fn build_claim_ix(
    program_id: Pubkey,
    intent_id: [u8; 32],
    signature: [u8; 64],
    escrow_pda: Pubkey,
    state_pda: Pubkey,
    vault_pda: Pubkey,
    solver_token: Pubkey,
) -> Result<Instruction, Box<dyn Error>> {
    Ok(Instruction {
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
        .try_to_vec()?,
    })
}

fn build_cancel_ix(
    program_id: Pubkey,
    intent_id: [u8; 32],
    requester: Pubkey,
    requester_token: Pubkey,
) -> Result<Instruction, Box<dyn Error>> {
    let (escrow_pda, _escrow_bump) =
        Pubkey::find_program_address(&[seeds::ESCROW_SEED, &intent_id], &program_id);
    let (vault_pda, _vault_bump) =
        Pubkey::find_program_address(&[seeds::VAULT_SEED, &intent_id], &program_id);

    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(escrow_pda, false),
            AccountMeta::new(requester, true),
            AccountMeta::new(vault_pda, false),
            AccountMeta::new(requester_token, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: EscrowInstruction::Cancel { intent_id }.try_to_vec()?,
    })
}

// ============================================================================
// TRANSACTION HELPERS
// ============================================================================

fn send_tx(
    client: &RpcClient,
    instructions: &[Instruction],
    payer: &Keypair,
    signers: &[&Keypair],
) -> Result<solana_sdk::signature::Signature, Box<dyn Error>> {
    let blockhash = client.get_latest_blockhash()?;
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
    let signature = client.send_and_confirm_transaction(&tx)?;
    Ok(signature)
}

// ============================================================================
// OPTION PARSING
// ============================================================================

fn parse_options(args: &[String]) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let mut options = HashMap::new();
    let mut index = 0;
    while index < args.len() {
        let key = args[index]
            .strip_prefix("--")
            .ok_or("Expected option in --key format")?;
        let value = args
            .get(index + 1)
            .ok_or("Missing value for option")?
            .to_string();
        options.insert(key.to_string(), value);
        index += 2;
    }
    Ok(options)
}

fn required_option<'a>(
    options: &'a HashMap<String, String>,
    key: &str,
) -> Result<&'a str, Box<dyn Error>> {
    options
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("Missing required option: --{key}").into())
}

fn read_keypair(
    options: &HashMap<String, String>,
    key: &str,
) -> Result<Keypair, Box<dyn Error>> {
    let path = required_option(options, key)?;
    Ok(read_keypair_file(path)?)
}

// ============================================================================
// VALUE PARSING
// ============================================================================

fn parse_pubkey(value: &str) -> Result<Pubkey, Box<dyn Error>> {
    Ok(Pubkey::from_str(value)?)
}

fn parse_u64(value: &str) -> Result<u64, Box<dyn Error>> {
    Ok(value.parse::<u64>()?)
}

fn parse_i64(value: &str) -> Result<i64, Box<dyn Error>> {
    Ok(value.parse::<i64>()?)
}

fn parse_intent_id(value: &str) -> Result<[u8; 32], Box<dyn Error>> {
    Ok(hex_to_bytes32(value))
}

fn parse_signature(value: &str) -> Result<[u8; 64], Box<dyn Error>> {
    let hex = value.strip_prefix("0x").unwrap_or(value);
    let bytes = hex::decode(hex)?;
    if bytes.len() != 64 {
        return Err("Signature must be 64 bytes (128 hex chars)".into());
    }
    let mut signature = [0u8; 64];
    signature.copy_from_slice(&bytes);
    Ok(signature)
}

fn hex_to_bytes32(hex_string: &str) -> [u8; 32] {
    let hex = hex_string.strip_prefix("0x").unwrap_or(hex_string);
    let hex = if hex.len() % 2 == 1 {
        format!("0{}", hex)
    } else {
        hex.to_string()
    };
    let mut bytes = [0u8; 32];
    if let Ok(hex_bytes) = hex::decode(&hex) {
        let start = 32usize.saturating_sub(hex_bytes.len());
        if start < 32 {
            bytes[start..].copy_from_slice(&hex_bytes);
        }
    } else {
        panic!("Invalid hex string: {}", hex_string);
    }
    bytes
}

// ============================================================================
// USAGE
// ============================================================================

fn print_usage() {
    eprintln!(
        r#"SVM Intent Escrow CLI

Usage:
  intent_escrow_cli <command> --program-id <pubkey> [--option value]...

Commands:
  initialize         --program-id <pubkey> --payer <keypair> --verifier <pubkey> [--rpc <url>]
  create-escrow      --program-id <pubkey> --payer <keypair> --requester <keypair> --token-mint <pubkey>
                     --requester-token <pubkey> --solver <pubkey> --intent-id <hex> --amount <u64>
                     [--expiry <i64>] [--rpc <url>]
  claim              --program-id <pubkey> --payer <keypair> --solver-token <pubkey> --intent-id <hex>
                     --signature <hex> [--rpc <url>]
  cancel             --program-id <pubkey> --payer <keypair> --requester <keypair> --requester-token <pubkey>
                     --intent-id <hex> [--rpc <url>]
  get-escrow         --program-id <pubkey> --intent-id <hex> [--rpc <url>]
  get-token-balance  --token-account <pubkey> [--rpc <url>]
        "#
    );
}
