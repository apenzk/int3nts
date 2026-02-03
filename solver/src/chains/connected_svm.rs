//! Connected SVM Chain Client
//!
//! Client for interacting with connected SVM chains to query escrow accounts
//! and release escrows after trusted-gmp approval.

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use borsh::{BorshDeserialize, BorshSerialize};
use reqwest::Client;
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    ed25519_instruction::new_ed25519_instruction_with_signature,
    instruction::{AccountMeta, Instruction},
    signature::{Keypair, Signer},
    sysvar,
    transaction::Transaction,
};
use solana_sdk_ids::system_program;
use std::str::FromStr;
use std::time::Duration;

use crate::config::SvmChainConfig;

// Well-known program IDs from Solana mainnet/devnet docs.
const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone)]
pub struct EscrowAccount {
    pub discriminator: [u8; 8],
    pub requester: Pubkey,
    pub token_mint: Pubkey,
    pub amount: u64,
    pub is_claimed: bool,
    pub expiry: i64,
    pub reserved_solver: Pubkey,
    pub intent_id: [u8; 32],
    pub bump: u8,
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone)]
pub struct EscrowState {
    pub discriminator: [u8; 8],
    pub approver: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum EscrowInstruction {
    Initialize { approver: Pubkey },
    CreateEscrow {
        intent_id: [u8; 32],
        amount: u64,
        expiry_duration: Option<i64>,
    },
    Claim {
        intent_id: [u8; 32],
        signature: [u8; 64],
    },
    Cancel { intent_id: [u8; 32] },
}

#[derive(Debug, Clone, Deserialize)]
struct ProgramAccountResult {
    pubkey: String,
    account: RpcAccount,
}

#[derive(Debug, Clone, Deserialize)]
struct RpcAccount {
    data: (String, String),
}

#[derive(Debug, Clone)]
pub struct EscrowEvent {
    pub intent_id: String,
    pub escrow_id: String,
}

pub struct ConnectedSvmClient {
    client: Client,
    rpc_url: String,
    program_id: Pubkey,
    rpc_client: RpcClient,
    /// Env var name that stores the solver private key (base58) for signing SVM txs.
    /// This mirrors MVM and EVM signing:
    /// - MVM: store the CLI profile name, and the Aptos CLI loads the keypair when signing.
    /// - EVM: read the private key from an env var at call time for the Hardhat signer.
    /// Here we keep the env var name and decode the base58 key when we need to sign.
    private_key_env: String,
    /// Program ID of the native GMP endpoint (optional, for GMP flow)
    gmp_endpoint_program_id: Option<String>,
    /// Program ID of the outflow validator (optional, for GMP flow)
    outflow_validator_program_id: Option<String>,
}

impl ConnectedSvmClient {
    /// Creates a new connected SVM client.
    ///
    /// # Arguments
    ///
    /// * `config` - Connected chain configuration
    ///
    /// # Returns
    ///
    /// * `Ok(ConnectedSvmClient)` - Initialized client
    /// * `Err(anyhow::Error)` - Invalid config values
    pub fn new(config: &SvmChainConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .no_proxy()
            .build()
            .context("Failed to create HTTP client")?;

        let program_id = Pubkey::from_str(&config.escrow_program_id)
            .context("Invalid SVM escrow_program_id")?;

        let rpc_client = RpcClient::new_with_commitment(
            config.rpc_url.clone(),
            CommitmentConfig::confirmed(),
        );

        Ok(Self {
            client,
            rpc_url: config.rpc_url.clone(),
            program_id,
            rpc_client,
            private_key_env: config.private_key_env.clone(),
            gmp_endpoint_program_id: config.gmp_endpoint_program_id.clone(),
            outflow_validator_program_id: config.outflow_validator_program_id.clone(),
        })
    }

    /// Queries SVM for escrow accounts and returns intent/escrow ids.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<EscrowEvent>)` - Escrow events with intent ids
    /// * `Err(anyhow::Error)` - RPC or parsing failure
    pub async fn get_escrow_events(&self) -> Result<Vec<EscrowEvent>> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getProgramAccounts",
            "params": [
                self.program_id.to_string(),
                { "encoding": "base64" }
            ]
        });

        let response: serde_json::Value = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to call getProgramAccounts")?
            .json()
            .await
            .context("Failed to parse getProgramAccounts response")?;

        if let Some(error) = response.get("error") {
            return Err(anyhow::anyhow!("SVM RPC error: {}", error));
        }

        let result = response
            .get("result")
            .and_then(|r| r.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid getProgramAccounts response"))?;

        let mut events = Vec::new();
        for entry in result {
            let account: ProgramAccountResult = serde_json::from_value(entry.clone())
                .context("Failed to parse program account entry")?;
            if let Some(escrow) = parse_escrow_data(&account.account.data.0) {
                let intent_id = format!("0x{}", hex::encode(escrow.intent_id));
                let escrow_id = pubkey_to_hex(&account.pubkey)?;
                events.push(EscrowEvent { intent_id, escrow_id });
            }
        }

        Ok(events)
    }

    /// Claims an escrow using a trusted-gmp signature (not yet implemented).
    ///
    /// # Arguments
    ///
    /// * `escrow_id` - Escrow PDA address
    /// * `intent_id` - Intent id
    /// * `signature` - Approver (Trusted GMP) signature bytes
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction signature
    /// * `Err(anyhow::Error)` - Unimplemented or RPC failure
    pub async fn claim_escrow(
        &self,
        escrow_id: &str,
        intent_id: &str,
        signature: &[u8],
    ) -> Result<String> {
        let intent_bytes = parse_intent_id(intent_id)?;
        let signature_bytes = parse_signature(signature)?;
        let escrow_pubkey = pubkey_from_hex(escrow_id)?;

        let escrow_account = self
            .rpc_client
            .get_account(&escrow_pubkey)
            .context("Failed to fetch escrow account")?;
        let escrow = EscrowAccount::try_from_slice(&escrow_account.data)
            .context("Failed to parse escrow account")?;

        let state_pda =
            Pubkey::find_program_address(&[b"state"], &self.program_id).0;
        let vault_pda =
            Pubkey::find_program_address(&[b"vault", &intent_bytes], &self.program_id).0;

        let solver_token =
            get_associated_token_address(&escrow.reserved_solver, &escrow.token_mint)?;

        let payer = self.load_solver_keypair()?;
        if self.rpc_client.get_account(&solver_token).is_err() {
            let create_ata_ix = create_associated_token_account_instruction(
                &payer.pubkey(),
                &escrow.reserved_solver,
                &escrow.token_mint,
            )?;
            let blockhash = self
                .rpc_client
                .get_latest_blockhash()
                .context("Failed to get latest blockhash")?;
            let tx = Transaction::new_signed_with_payer(
                &[create_ata_ix],
                Some(&payer.pubkey()),
                &[&payer],
                blockhash,
            );
            self.rpc_client
                .send_and_confirm_transaction(&tx)
                .context("Failed to create solver ATA")?;
        }

        let state_account = self
            .rpc_client
            .get_account(&state_pda)
            .context("Failed to fetch state account")?;
        let state = EscrowState::try_from_slice(&state_account.data)
            .context("Failed to parse state account")?;

        let ed25519_ix = new_ed25519_instruction_with_signature(
            &intent_bytes,
            &signature_bytes,
            &state.approver.to_bytes(),
        );

        let claim_ix = Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(escrow_pubkey, false),
                AccountMeta::new_readonly(state_pda, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new(solver_token, false),
                AccountMeta::new_readonly(sysvar::instructions::id(), false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
            data: EscrowInstruction::Claim {
                intent_id: intent_bytes,
                signature: signature_bytes,
            }
            .try_to_vec()
            .context("Failed to serialize claim instruction")?,
        };

        let blockhash = self
            .rpc_client
            .get_latest_blockhash()
            .context("Failed to get latest blockhash")?;
        let tx = Transaction::new_signed_with_payer(
            &[ed25519_ix, claim_ix],
            Some(&payer.pubkey()),
            &[&payer],
            blockhash,
        );
        let sig = self
            .rpc_client
            .send_and_confirm_transaction(&tx)
            .context("Failed to send claim transaction")?;
        Ok(sig.to_string())
    }

    /// Fulfills an outflow intent via the GMP flow on SVM.
    ///
    /// Builds and submits the `outflow_validator::FulfillIntent` instruction which:
    /// 1. Validates the solver is authorized and requirements exist
    /// 2. Transfers tokens from solver to recipient
    /// 3. Sends FulfillmentProof back to hub via GMP
    ///
    /// The hub will automatically release tokens when it receives the FulfillmentProof.
    ///
    /// # Arguments
    ///
    /// * `intent_id` - 32-byte intent identifier (0x-prefixed hex)
    /// * `_token_mint` - SPL token mint (currently unused, stored in requirements)
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction signature
    /// * `Err(anyhow::Error)` - Failed to fulfill intent
    pub async fn fulfill_outflow_via_gmp(
        &self,
        intent_id: &str,
        _token_mint: &str,
    ) -> Result<String> {
        // Check that GMP config is available
        let outflow_program_id_str = self.outflow_validator_program_id.as_ref()
            .context("outflow_validator_program_id not configured for SVM GMP flow")?;
        let gmp_endpoint_id_str = self.gmp_endpoint_program_id.as_ref()
            .context("gmp_endpoint_program_id not configured for SVM GMP flow")?;

        let outflow_program_id = Pubkey::from_str(outflow_program_id_str)
            .context("Invalid outflow_validator_program_id")?;
        let gmp_endpoint_id = Pubkey::from_str(gmp_endpoint_id_str)
            .context("Invalid gmp_endpoint_program_id")?;

        let intent_bytes = parse_intent_id(intent_id)?;

        tracing::info!(
            "Calling outflow_validator::fulfill_intent - intent_id: {}, outflow_program: {}",
            intent_id, outflow_program_id
        );

        let solver = self.load_solver_keypair()?;

        // Derive outflow_validator PDAs
        let (requirements_pda, _) = Pubkey::find_program_address(
            &[b"requirements", &intent_bytes],
            &outflow_program_id,
        );
        let (config_pda, _) = Pubkey::find_program_address(
            &[b"config"],
            &outflow_program_id,
        );

        // Read requirements account to get token_mint and recipient
        let requirements_data = self.rpc_client
            .get_account_data(&requirements_pda)
            .context("Failed to fetch requirements account - intent may not exist on connected chain")?;

        // Parse requirements: discriminator(1) + intent_id(32) + recipient(32) + amount(8) + token_mint(32) + solver(32) + expiry(8) + fulfilled(1) + bump(1)
        if requirements_data.len() < 147 {
            anyhow::bail!("Requirements account data too short");
        }
        let recipient = Pubkey::try_from(&requirements_data[33..65])
            .context("Failed to parse recipient from requirements")?;
        let token_mint = Pubkey::try_from(&requirements_data[73..105])
            .context("Failed to parse token_mint from requirements")?;

        // Read config to get hub_chain_id for GMP nonce derivation
        let config_data = self.rpc_client
            .get_account_data(&config_pda)
            .context("Failed to fetch outflow config account")?;
        
        // Parse config: discriminator(1) + admin(32) + gmp_endpoint(32) + hub_chain_id(4) + trusted_hub_addr(32) + bump(1)
        if config_data.len() < 102 {
            anyhow::bail!("Config account data too short");
        }
        let hub_chain_id = u32::from_le_bytes(
            config_data[65..69].try_into().context("Failed to parse hub_chain_id")?
        );

        // Derive token accounts (ATAs)
        let solver_token = get_associated_token_address(&solver.pubkey(), &token_mint)?;
        let recipient_token = get_associated_token_address(&recipient, &token_mint)?;

        // Derive GMP endpoint PDAs
        let (gmp_config_pda, _) = Pubkey::find_program_address(
            &[b"config"],
            &gmp_endpoint_id,
        );
        let (gmp_nonce_out_pda, _) = Pubkey::find_program_address(
            &[b"nonce_out", &hub_chain_id.to_le_bytes()],
            &gmp_endpoint_id,
        );

        tracing::info!(
            "Building FulfillIntent tx: solver={}, recipient={}, token_mint={}, amount=from_requirements",
            solver.pubkey(), recipient, token_mint
        );

        // Build FulfillIntent instruction
        // Instruction data: variant(1) + intent_id(32)
        let mut instruction_data = vec![2u8]; // FulfillIntent variant index
        instruction_data.extend_from_slice(&intent_bytes);

        let fulfill_ix = Instruction {
            program_id: outflow_program_id,
            accounts: vec![
                // FulfillIntent accounts
                AccountMeta::new(requirements_pda, false),
                AccountMeta::new_readonly(config_pda, false),
                AccountMeta::new_readonly(solver.pubkey(), true),
                AccountMeta::new(solver_token, false),
                AccountMeta::new(recipient_token, false),
                AccountMeta::new_readonly(token_mint, false),
                AccountMeta::new_readonly(spl_token::id(), false),
                AccountMeta::new_readonly(gmp_endpoint_id, false),
                // GMP Send accounts (passed through to CPI)
                AccountMeta::new_readonly(gmp_config_pda, false),
                AccountMeta::new(gmp_nonce_out_pda, false),
                AccountMeta::new_readonly(solver.pubkey(), true), // sender
                AccountMeta::new(solver.pubkey(), true),          // payer
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: instruction_data,
        };

        let blockhash = self.rpc_client
            .get_latest_blockhash()
            .context("Failed to get latest blockhash")?;

        let tx = Transaction::new_signed_with_payer(
            &[fulfill_ix],
            Some(&solver.pubkey()),
            &[&solver],
            blockhash,
        );

        let sig = self.rpc_client
            .send_and_confirm_transaction(&tx)
            .context("Failed to send FulfillIntent transaction")?;

        Ok(sig.to_string())
    }
}

impl ConnectedSvmClient {
    /// Loads the solver keypair from the env var (base58 private key).
    ///
    /// Mirrors EVM/MVM approach: private key is stored directly as a string
    /// (base58 for SVM, hex for EVM/MVM), not as a file path.
    ///
    /// # Returns
    ///
    /// * `Ok(Keypair)` - Loaded keypair
    /// * `Err(anyhow::Error)` - Missing env var or invalid private key
    fn load_solver_keypair(&self) -> Result<Keypair> {
        let private_key_b58 = std::env::var(&self.private_key_env).with_context(|| {
            format!(
                "Missing solver private key env var: {}",
                self.private_key_env
            )
        })?;
        keypair_from_base58(&private_key_b58)
            .context("Failed to decode solver private key from base58")
    }
}

/// Decodes a base58 private key string into a Keypair.
///
/// Solana private keys are 64 bytes (seed + public key) encoded as base58.
///
/// # Arguments
///
/// * `b58` - Base58-encoded private key string
///
/// # Returns
///
/// * `Ok(Keypair)` - Decoded keypair
/// * `Err(anyhow::Error)` - Invalid base58 or wrong length
fn keypair_from_base58(b58: &str) -> Result<Keypair> {
    let bytes = bs58::decode(b58)
        .into_vec()
        .context("Invalid base58 encoding")?;
    Keypair::try_from(bytes.as_slice())
        .map_err(|e| anyhow::anyhow!("Invalid keypair bytes: {}", e))
}

/// Converts a base58 pubkey string into a 0x-prefixed hex string.
///
/// # Arguments
///
/// * `pubkey_str` - Base58-encoded pubkey
///
/// # Returns
///
/// * `Ok(String)` - 0x-prefixed hex string
/// * `Err(anyhow::Error)` - Invalid pubkey string
fn pubkey_to_hex(pubkey_str: &str) -> Result<String> {
    let pubkey = Pubkey::from_str(pubkey_str)
        .context("Invalid pubkey string")?;
    Ok(format!("0x{}", hex::encode(pubkey.to_bytes())))
}

/// Parses a 0x-prefixed hex pubkey into a Pubkey.
/// Move addresses strip leading zeros, so we left-pad to 64 hex chars (32 bytes).
fn pubkey_from_hex(value: &str) -> Result<Pubkey> {
    let stripped = value.strip_prefix("0x").unwrap_or(value);
    if stripped.len() > 64 {
        anyhow::bail!("Pubkey hex too long: {} chars", stripped.len());
    }
    // Left-pad to 64 hex chars to recover leading zero bytes stripped by Move
    let padded = format!("{:0>64}", stripped);
    let bytes = hex::decode(&padded).context("Invalid hex pubkey")?;
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(Pubkey::new_from_array(array))
}

/// Parse a 0x hex intent id into a 32-byte array.
fn parse_intent_id(value: &str) -> Result<[u8; 32]> {
    let stripped = value.strip_prefix("0x").unwrap_or(value);
    if stripped.len() > 64 {
        anyhow::bail!("Intent id too long");
    }
    let padded = format!("{:0>64}", stripped);
    let bytes = hex::decode(padded).context("Invalid intent id hex")?;
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn parse_signature(signature: &[u8]) -> Result<[u8; 64]> {
    if signature.len() != 64 {
        anyhow::bail!("Invalid signature length: {}", signature.len());
    }
    let mut out = [0u8; 64];
    out.copy_from_slice(signature);
    Ok(out)
}

/// Parses escrow account data from base64-encoded bytes.
///
/// # Arguments
///
/// * `data_base64` - Base64-encoded account data
///
/// # Returns
///
/// * `Some(EscrowAccount)` - Parsed escrow account
/// * `None` - Invalid or unparsable data
fn parse_escrow_data(data_base64: &str) -> Option<EscrowAccount> {
    let data = STANDARD.decode(data_base64).ok()?;
    EscrowAccount::try_from_slice(&data).ok()
}

/// Derives the associated token account (ATA) for an owner and mint.
///
/// # Arguments
///
/// * `owner` - Token account owner
/// * `mint` - SPL token mint
///
/// # Returns
///
/// * `Ok(Pubkey)` - Derived ATA address
/// * `Err(anyhow::Error)` - Invalid associated token program id
fn get_associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Result<Pubkey> {
    let program_id = associated_token_program_id()?;
    Ok(get_associated_token_address_with_program_id(
        owner,
        mint,
        &program_id,
    ))
}

/// Derives the ATA address using an explicit associated token program id.
///
/// # Arguments
///
/// * `owner` - Token account owner
/// * `mint` - SPL token mint
/// * `program_id` - Associated token program id
///
/// # Returns
///
/// * `Pubkey` - Derived ATA address
fn get_associated_token_address_with_program_id(
    owner: &Pubkey,
    mint: &Pubkey,
    program_id: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &[owner.as_ref(), spl_token::id().as_ref(), mint.as_ref()],
        program_id,
    )
    .0
}

/// Builds a CreateAssociatedTokenAccount instruction.
///
/// # Arguments
///
/// * `payer` - Fee payer
/// * `owner` - Token account owner
/// * `mint` - SPL token mint
///
/// # Returns
///
/// * `Ok(Instruction)` - ATA creation instruction
/// * `Err(anyhow::Error)` - Invalid associated token program id
fn create_associated_token_account_instruction(
    payer: &Pubkey,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Result<Instruction> {
    let program_id = associated_token_program_id()?;
    let ata = get_associated_token_address_with_program_id(owner, mint, &program_id);

    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(ata, false),
            AccountMeta::new_readonly(*owner, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: vec![],
    })
}

/// Returns the associated token program id as a Pubkey.
///
/// # Returns
///
/// * `Ok(Pubkey)` - Associated token program id
/// * `Err(anyhow::Error)` - Invalid program id constant
fn associated_token_program_id() -> Result<Pubkey> {
    Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM_ID)
        .context("Invalid associated token program id")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that associated token program id parses to a valid pubkey
    /// Why: ATA derivation depends on a correct program id
    #[test]
    fn test_associated_token_program_id() {
        let program_id = associated_token_program_id().expect("ATA program id");
        assert_eq!(program_id.to_string(), ASSOCIATED_TOKEN_PROGRAM_ID);
    }

    /// Test that escrow parsing succeeds for valid Borsh data
    /// Why: Escrow scanning relies on correctly decoding account data
    #[test]
    fn test_parse_escrow_data() {
        let escrow = EscrowAccount {
            discriminator: [7u8; 8],
            requester: Pubkey::new_from_array([1u8; 32]),
            token_mint: Pubkey::new_from_array([2u8; 32]),
            amount: 42,
            is_claimed: false,
            expiry: 123456,
            reserved_solver: Pubkey::new_from_array([3u8; 32]),
            intent_id: [4u8; 32],
            bump: 1,
        };

        let data = escrow.try_to_vec().expect("serialize escrow");
        let encoded = STANDARD.encode(data);
        let parsed = parse_escrow_data(&encoded).expect("parse escrow");
        assert_eq!(parsed.intent_id, escrow.intent_id);
        assert_eq!(parsed.amount, escrow.amount);
        assert_eq!(parsed.requester, escrow.requester);
    }

    /// Test that hex pubkey parsing handles leading zeros stripped by Move addresses
    /// Why: Move address serialization strips leading zeros, but Solana pubkeys need exactly 32 bytes
    #[test]
    fn test_pubkey_from_hex_with_leading_zeros() {
        // Full 64-char hex (32 bytes) - normal case
        let full_hex = "0x00d30e3caf2adf837ead1c43d8fca0825b70993bf75053ad7d89dc66a7e31144";
        let pk1 = pubkey_from_hex(full_hex).expect("full hex");
        
        // Stripped leading zeros (62 chars / 31 bytes) - Move address format
        let stripped_hex = "0xd30e3caf2adf837ead1c43d8fca0825b70993bf75053ad7d89dc66a7e31144";
        let pk2 = pubkey_from_hex(stripped_hex).expect("stripped hex");
        
        // Both should produce the same pubkey
        assert_eq!(pk1, pk2, "Leading zeros should be restored");
        
        // Verify the pubkey bytes start with 0x00
        assert_eq!(pk1.to_bytes()[0], 0x00, "First byte should be zero");
    }

    /// Test that hex pubkey parsing works for addresses without leading zeros
    #[test]
    fn test_pubkey_from_hex_no_leading_zeros() {
        let hex = "0xf48282d15ca5c2b19e3e619aee72648fa1e5087fe91f933cd595eeef03468141";
        let pk = pubkey_from_hex(hex).expect("parse hex");
        assert_eq!(pk.to_bytes()[0], 0xf4, "First byte should be 0xf4");
    }
}
