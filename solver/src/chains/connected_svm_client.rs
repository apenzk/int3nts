//! Connected SVM Chain Client
//!
//! Client for interacting with connected SVM chains to query escrow accounts
//! and check escrow release status via GMP flow.
//!
//! ## Design: why sync query methods don't delegate to chain-clients-svm
//!
//! MVM and EVM solver clients delegate query methods (is_escrow_released,
//! get_token_balance, get_native_balance) to their shared async chain-clients
//! crate. SVM does not — these methods are sync and use `solana_client::RpcClient`
//! directly.
//!
//! The reason is the fulfillment strategy: MVM/EVM solvers shell out to external
//! CLIs (aptos CLI, Hardhat scripts) for transaction signing, so they only need
//! the async shared client. The SVM solver builds and signs transactions in-process
//! using `solana_sdk` types (Keypair, Transaction, Instruction), which requires
//! `solana_client::RpcClient` (blocking) for blockhash fetching, tx submission,
//! and confirmation. Since that blocking client is already here, the query methods
//! use it too rather than wrapping async calls in block_on().
//!
//! This is intentional, not tech debt. Wrapping async in block_on() adds
//! complexity with no functional benefit. If the Solana SDK gains a stable async
//! client, revisit.

use anyhow::{Context, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_client::client_error::{ClientError, ClientErrorKind};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_request::RpcError;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;

use chain_clients_svm::parse_intent_id;
use chain_clients_svm::SvmClient;

use crate::config::SvmChainConfig;

// Re-export shared types from chain-clients-svm
pub use chain_clients_svm::EscrowEvent;

// Well-known program IDs from Solana mainnet/devnet docs.
const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111111";
const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

/// Escrow account data (Borsh-serialized on-chain layout).
///
/// Uses solana_sdk::Pubkey for compatibility with solver transaction signing.
/// The shared chain-clients-svm crate has its own version using solana_program::Pubkey.
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

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum EscrowInstruction {
    Initialize { approver: Pubkey },
    CreateEscrow {
        intent_id: [u8; 32],
        amount: u64,
        expiry_duration: Option<i64>,
    },
    // Note: Claim variant kept for Borsh enum index compatibility (index 2)
    // GMP flow uses auto-release via FulfillmentProof delivery
    #[allow(dead_code)]
    Claim {
        intent_id: [u8; 32],
        signature: [u8; 64],
    },
    Cancel { intent_id: [u8; 32] },
}

pub struct ConnectedSvmClient {
    /// Shared SVM client for async query methods (get_escrow_events)
    svm_client: SvmClient,
    program_id: Pubkey,
    rpc_client: RpcClient,
    /// Env var name that stores the solver private key (base58) for signing SVM txs.
    /// This mirrors MVM and EVM signing:
    /// - MVM: store the CLI profile name, and the Aptos CLI loads the keypair when signing.
    /// - EVM: read the private key from an env var at call time for the Hardhat signer.
    /// Here we keep the env var name and decode the base58 key when we need to sign.
    private_key_env: String,
    /// Program ID of the integrated GMP endpoint (optional, for GMP flow)
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
        let svm_client = SvmClient::new(&config.rpc_url, &config.escrow_program_id)
            .context("Failed to create shared SVM client")?;

        let program_id = Pubkey::from_str(&config.escrow_program_id)
            .context("Invalid SVM escrow_program_id")?;

        let rpc_client = RpcClient::new_with_commitment(
            config.rpc_url.clone(),
            CommitmentConfig::confirmed(),
        );

        Ok(Self {
            svm_client,
            program_id,
            rpc_client,
            private_key_env: config.private_key_env.clone(),
            gmp_endpoint_program_id: config.gmp_endpoint_program_id.clone(),
            outflow_validator_program_id: config.outflow_validator_program_id.clone(),
        })
    }

    /// Queries SVM for escrow accounts and returns intent/escrow ids.
    ///
    /// Delegates to `SvmClient::get_escrow_events`.
    pub async fn get_escrow_events(&self) -> Result<Vec<EscrowEvent>> {
        self.svm_client.get_escrow_events().await
    }

    /// Queries the SPL token balance for an owner's associated token account.
    ///
    /// Derives the Associated Token Account (ATA) for the given owner and mint,
    /// then queries its balance via `get_token_account_balance`.
    ///
    /// # Arguments
    ///
    /// * `token_mint` - SPL token mint address (base58)
    /// * `owner` - Owner public key (base58)
    ///
    /// # Returns
    ///
    /// * `Ok(u128)` - Token balance in base units
    /// * `Err(anyhow::Error)` - Failed to query balance
    pub fn get_token_balance(&self, token_mint: &str, owner: &str) -> Result<u128> {
        let mint_pubkey = Pubkey::from_str(token_mint)
            .context("Invalid token mint address")?;
        let owner_pubkey = Pubkey::from_str(owner)
            .context("Invalid owner address")?;

        let ata = get_associated_token_address(&owner_pubkey, &mint_pubkey)?;

        let token_balance = self
            .rpc_client
            .get_token_account_balance(&ata)
            .context("Failed to get token account balance")?;

        let amount_str = &token_balance.amount;
        let balance = amount_str
            .parse::<u128>()
            .context("Failed to parse token balance amount as u128")?;

        Ok(balance)
    }

    /// Queries the native SOL balance (in lamports) for an account.
    ///
    /// Used for tracking gas token balance. Unlike `get_token_balance` which queries
    /// an SPL Associated Token Account, this queries the account's native SOL balance.
    ///
    /// # Arguments
    ///
    /// * `owner` - Account public key (base58)
    ///
    /// # Returns
    ///
    /// * `Ok(u128)` - Native SOL balance in lamports
    /// * `Err(anyhow::Error)` - Failed to query balance
    pub fn get_native_balance(&self, owner: &str) -> Result<u128> {
        let owner_pubkey = Pubkey::from_str(owner)
            .context("Invalid owner address")?;
        let balance = self
            .rpc_client
            .get_balance(&owner_pubkey)
            .context("Failed to get native SOL balance")?;
        Ok(balance as u128)
    }

    /// Checks if GMP outflow requirements have been delivered for an intent.
    ///
    /// This polls the outflow_validator's requirements PDA account to see if it exists.
    /// The requirements are created when the integrated GMP relay delivers IntentRequirements.
    ///
    /// # Arguments
    ///
    /// * `intent_id` - Intent ID (0x-prefixed hex string)
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Requirements exist (intent can be fulfilled)
    /// * `Ok(false)` - Requirements not yet delivered
    /// * `Err` - Failed to check
    pub fn has_outflow_requirements(&self, intent_id: &str) -> Result<bool> {
        let outflow_program_id_str = self.outflow_validator_program_id.as_ref()
            .context("outflow_validator_program_id not configured")?;
        let outflow_program_id = Pubkey::from_str(outflow_program_id_str)
            .context("Invalid outflow_validator_program_id")?;
        let intent_bytes = parse_intent_id(intent_id)?;

        let (requirements_pda, _) = Pubkey::find_program_address(
            &[b"requirements", &intent_bytes],
            &outflow_program_id,
        );

        match self.rpc_client.get_account(&requirements_pda) {
            Ok(_) => Ok(true),
            Err(err) if is_account_not_found(&err) => Ok(false),
            Err(err) => Err(err).context("Failed to query requirements PDA account"),
        }
    }

    /// Checks if an inflow escrow has been released (auto-released when FulfillmentProof received).
    ///
    /// Reads the escrow PDA account and checks the `is_claimed` field.
    /// With GMP auto-release, when this returns true, tokens have already been transferred to solver.
    ///
    /// # Arguments
    ///
    /// * `intent_id` - Intent ID (0x-prefixed hex string)
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Escrow has been released to solver
    /// * `Ok(false)` - Escrow not yet released
    /// * `Err` - Failed to query (escrow doesn't exist or parse error)
    pub fn is_escrow_released(&self, intent_id: &str) -> Result<bool> {
        let intent_bytes = parse_intent_id(intent_id)?;

        // Derive escrow PDA using same seeds as intent_inflow_escrow program
        let (escrow_pda, _) = Pubkey::find_program_address(
            &[b"escrow", &intent_bytes],
            &self.program_id,
        );

        let account_data = self
            .rpc_client
            .get_account_data(&escrow_pda)
            .context("Failed to fetch escrow account - escrow may not exist")?;

        let escrow = EscrowAccount::try_from_slice(&account_data)
            .context("Failed to parse escrow account data")?;

        Ok(escrow.is_claimed)
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

        // Derive token accounts (ATAs)
        let solver_token = get_associated_token_address(&solver.pubkey(), &token_mint)?;
        let recipient_token = get_associated_token_address(&recipient, &token_mint)?;

        // Derive GMP endpoint PDAs
        let (gmp_config_pda, _) = Pubkey::find_program_address(
            &[b"config"],
            &gmp_endpoint_id,
        );
        let (gmp_nonce_out_pda, _) = Pubkey::find_program_address(
            &[b"nonce_out"],
            &gmp_endpoint_id,
        );

        // Read nonce account to derive message PDA.
        // GMP Send creates a message account at ["message", nonce].
        // The nonce is the *current* value (before increment).
        let current_nonce: u64 = match self.rpc_client.get_account_data(&gmp_nonce_out_pda) {
            Ok(data) if data.len() >= 9 => {
                // OutboundNonceAccount: discriminator(1) + nonce(8) + bump(1)
                u64::from_le_bytes(data[1..9].try_into().unwrap())
            }
            _ => 0, // Account doesn't exist yet; first Send will use nonce=0
        };
        let nonce_bytes = current_nonce.to_le_bytes();
        let (message_pda, _) = Pubkey::find_program_address(
            &[b"message", &nonce_bytes],
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
                AccountMeta::new_readonly(spl_token_program_id()?, false),
                AccountMeta::new_readonly(gmp_endpoint_id, false),
                // GMP Send accounts (passed through to CPI)
                AccountMeta::new_readonly(gmp_config_pda, false),
                AccountMeta::new(gmp_nonce_out_pda, false),
                AccountMeta::new_readonly(solver.pubkey(), true), // sender
                AccountMeta::new(solver.pubkey(), true),          // payer
                AccountMeta::new_readonly(system_program_id()?, false),
                AccountMeta::new(message_pda, false),             // message account
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

        let sig = match self.rpc_client.send_and_confirm_transaction(&tx) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("FulfillIntent transaction failed: {:?}", e);
                return Err(anyhow::anyhow!("Failed to send FulfillIntent transaction: {}", e));
            }
        };

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
    let token_program = spl_token_program_id().expect("SPL token program id");
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        program_id,
    )
    .0
}

/// Returns the associated token program id as a Pubkey.
///
/// # Returns
///
/// * `Ok(Pubkey)` - Associated token program id
/// * `Err(anyhow::Error)` - Invalid program id constant
fn spl_token_program_id() -> Result<Pubkey> {
    Pubkey::from_str(SPL_TOKEN_PROGRAM_ID)
        .context("Invalid SPL token program id")
}

fn system_program_id() -> Result<Pubkey> {
    Pubkey::from_str(SYSTEM_PROGRAM_ID)
        .context("Invalid system program id")
}

fn associated_token_program_id() -> Result<Pubkey> {
    Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM_ID)
        .context("Invalid associated token program id")
}

/// Returns true if the RPC error indicates the account was not found.
fn is_account_not_found(err: &ClientError) -> bool {
    matches!(
        err.kind(),
        ClientErrorKind::RpcError(RpcError::ForUser(msg)) if msg.contains("could not find account")
    )
}

