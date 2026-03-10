//! SVM Client Implementation
//!
//! Provides a shared client for communicating with Solana nodes via JSON-RPC.
//! Used by coordinator, integrated-gmp, and solver.

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use borsh::BorshDeserialize;
use reqwest::Client;
use serde::Serialize;
use solana_program::pubkey::Pubkey;
use std::str::FromStr;
use std::time::Duration;

use crate::types::*;

// Well-known Solana program IDs
const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

// ============================================================================
// JSON-RPC REQUEST
// ============================================================================

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: u64,
}

// ============================================================================
// CLIENT
// ============================================================================

/// Client for communicating with Solana nodes via JSON-RPC
#[derive(Debug)]
pub struct SvmClient {
    client: Client,
    rpc_url: String,
    program_id: Pubkey,
}

impl SvmClient {
    /// Creates a new SVM client for the given RPC URL and escrow program ID
    pub fn new(rpc_url: &str, program_id: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .no_proxy()
            .build()
            .context("Failed to create HTTP client")?;

        let program_id = Pubkey::from_str(program_id)
            .context("Invalid SVM program_id (expected base58 string)")?;

        Ok(Self {
            client,
            rpc_url: rpc_url.to_string(),
            program_id,
        })
    }

    /// Returns the RPC URL of this client
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Returns the escrow program ID
    pub fn program_id(&self) -> Pubkey {
        self.program_id
    }

    /// Derives the escrow PDA for a given intent ID
    pub fn escrow_pda(&self, intent_id: &[u8; 32]) -> Pubkey {
        Pubkey::find_program_address(&[b"escrow", intent_id], &self.program_id).0
    }

    /// Reads raw account data (base64-decoded) for any Solana account.
    ///
    /// Returns `None` if the account doesn't exist.
    pub async fn get_raw_account_data(&self, pubkey: &Pubkey) -> Result<Option<Vec<u8>>> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getAccountInfo".to_string(),
            params: serde_json::json!([
                pubkey.to_string(),
                { "encoding": "base64" }
            ]),
            id: 1,
        };

        let response: JsonRpcResponse<AccountInfoResult> = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to call getAccountInfo")?
            .json()
            .await
            .context("Failed to parse getAccountInfo response")?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("SVM RPC error: {}", error.message));
        }

        let Some(result) = response.result else {
            return Ok(None);
        };

        let Some(account) = result.value else {
            return Ok(None);
        };

        let data = STANDARD
            .decode(&account.data.0)
            .context("Failed to decode base64 account data")?;
        Ok(Some(data))
    }

    /// Checks if an inflow escrow has been released (is_claimed == true).
    ///
    /// Reads the escrow PDA account via getAccountInfo and parses the Borsh data.
    pub async fn is_escrow_released(&self, intent_id: &str) -> Result<bool> {
        let intent_bytes = parse_intent_id(intent_id)?;
        let escrow_pda = self.escrow_pda(&intent_bytes);

        let data = self
            .get_raw_account_data(&escrow_pda)
            .await?
            .context("Escrow account not found")?;

        let escrow = EscrowAccount::try_from_slice(&data)
            .context("Failed to parse escrow account data")?;

        Ok(escrow.is_claimed)
    }

    /// Queries the SPL token balance for an owner's associated token account.
    ///
    /// Derives the ATA for the given owner and mint, then queries
    /// getTokenAccountBalance via JSON-RPC.
    pub async fn get_token_balance(&self, token_mint: &str, owner: &str) -> Result<u128> {
        let mint_pubkey =
            Pubkey::from_str(token_mint).context("Invalid token mint address")?;
        let owner_pubkey = Pubkey::from_str(owner).context("Invalid owner address")?;

        let ata = get_associated_token_address(&owner_pubkey, &mint_pubkey);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getTokenAccountBalance".to_string(),
            params: serde_json::json!([ata.to_string()]),
            id: 1,
        };

        let response: JsonRpcResponse<TokenBalanceResult> = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to call getTokenAccountBalance")?
            .json()
            .await
            .context("Failed to parse getTokenAccountBalance response")?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("SVM RPC error: {}", error.message));
        }

        let token_balance = response
            .result
            .context("No result in getTokenAccountBalance response")?;

        let balance = token_balance
            .value
            .amount
            .parse::<u128>()
            .context("Failed to parse token balance amount as u128")?;

        Ok(balance)
    }

    /// Queries the native SOL balance (in lamports) for an account.
    pub async fn get_native_balance(&self, owner: &str) -> Result<u128> {
        let owner_pubkey = Pubkey::from_str(owner).context("Invalid owner address")?;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getBalance".to_string(),
            params: serde_json::json!([owner_pubkey.to_string()]),
            id: 1,
        };

        let response: JsonRpcResponse<BalanceResult> = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to call getBalance")?
            .json()
            .await
            .context("Failed to parse getBalance response")?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("SVM RPC error: {}", error.message));
        }

        let balance = response
            .result
            .context("No result in getBalance response")?;

        Ok(balance.value as u128)
    }

    /// Queries all escrow accounts owned by the program via getProgramAccounts.
    ///
    /// Returns parsed escrow accounts with their on-chain addresses.
    pub async fn get_all_escrows(&self) -> Result<Vec<EscrowWithPubkey>> {
        let params = serde_json::json!([
            self.program_id.to_string(),
            { "encoding": "base64" }
        ]);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getProgramAccounts".to_string(),
            params,
            id: 1,
        };

        let response: JsonRpcResponse<Vec<ProgramAccountResult>> = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to call getProgramAccounts")?
            .json()
            .await
            .context("Failed to parse getProgramAccounts response")?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("SVM RPC error: {}", error.message));
        }

        let accounts = response.result
            .context("No result in getProgramAccounts response")?;
        let mut escrows = Vec::new();

        for account in accounts {
            let pubkey = Pubkey::from_str(&account.pubkey)
                .context("Invalid pubkey in getProgramAccounts response")?;
            // getProgramAccounts returns all accounts owned by the program,
            // including non-escrow accounts (metadata PDAs, etc.). Skip accounts
            // that don't deserialize as escrows.
            if let Ok(escrow) = parse_escrow_data(&account.account.data.0) {
                escrows.push(EscrowWithPubkey { pubkey, escrow });
            }
        }

        Ok(escrows)
    }

    /// Fetches a single escrow account by intent ID.
    pub async fn get_escrow_by_intent_id(
        &self,
        intent_id: &[u8; 32],
    ) -> Result<Option<EscrowAccount>> {
        let escrow_pda = self.escrow_pda(intent_id);

        let data = self.get_raw_account_data(&escrow_pda).await?;
        let Some(data) = data else {
            return Ok(None);
        };

        let escrow = EscrowAccount::try_from_slice(&data)
            .context("Failed to parse escrow account data")?;

        Ok(Some(escrow))
    }

    /// Get recent transaction signatures for the program
    pub async fn get_signatures_for_address(&self, limit: u64) -> Result<Vec<String>> {
        let params = serde_json::json!([
            self.program_id.to_string(),
            { "limit": limit }
        ]);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getSignaturesForAddress".to_string(),
            params,
            id: 1,
        };

        let response: JsonRpcResponse<Vec<SignatureInfo>> = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to call getSignaturesForAddress")?
            .json()
            .await
            .context("Failed to parse getSignaturesForAddress response")?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("SVM RPC error: {}", error.message));
        }

        let signatures = response
            .result
            .context("No result in getSignaturesForAddress response")?
            .into_iter()
            .map(|sig_info| sig_info.signature)
            .collect();

        Ok(signatures)
    }

    /// Get transaction details including logs
    pub async fn get_transaction(&self, signature: &str) -> Result<Vec<String>> {
        let params = serde_json::json!([
            signature,
            { "encoding": "json" }
        ]);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getTransaction".to_string(),
            params,
            id: 1,
        };

        let response: JsonRpcResponse<TransactionResult> = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to call getTransaction")?
            .json()
            .await
            .context("Failed to parse getTransaction response")?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("SVM RPC error: {}", error.message));
        }

        let transaction = response
            .result
            .context("No result in getTransaction response")?;
        let logs = transaction
            .meta
            .and_then(|m| m.log_messages)
            .unwrap_or_default();

        Ok(logs)
    }

    /// Queries all escrow accounts and returns simplified escrow events.
    ///
    /// Same as get_all_escrows but returns hex-encoded intent/escrow IDs.
    pub async fn get_escrow_events(&self) -> Result<Vec<EscrowEvent>> {
        let escrows = self.get_all_escrows().await?;
        let mut events = Vec::new();

        for ew in escrows {
            let intent_id = format!("0x{}", hex::encode(ew.escrow.intent_id));
            let escrow_id = pubkey_to_hex(&ew.pubkey);
            events.push(EscrowEvent {
                intent_id,
                escrow_id,
            });
        }

        Ok(events)
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Converts a Pubkey to a 0x-prefixed hex string
pub fn pubkey_to_hex(pubkey: &Pubkey) -> String {
    format!("0x{}", hex::encode(pubkey.to_bytes()))
}

/// Parses a 0x-prefixed hex string into a Pubkey.
///
/// Move addresses strip leading zeros, so this left-pads to 64 hex chars (32 bytes).
pub fn pubkey_from_hex(value: &str) -> Result<Pubkey> {
    let stripped = value.strip_prefix("0x").unwrap_or(value);
    if stripped.len() > 64 {
        anyhow::bail!("Pubkey hex too long: {} chars", stripped.len());
    }
    let padded = format!("{:0>64}", stripped);
    let bytes = hex::decode(&padded).context("Invalid hex pubkey")?;
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(Pubkey::new_from_array(array))
}

/// Parses escrow account data from base64-encoded Borsh bytes.
pub fn parse_escrow_data(data_base64: &str) -> Result<EscrowAccount> {
    let data = STANDARD.decode(data_base64)
        .context("Failed to decode base64 escrow account data")?;
    EscrowAccount::try_from_slice(&data)
        .context("Failed to deserialize escrow account from Borsh bytes")
}

/// Parse a 0x hex intent id into a 32-byte array.
pub fn parse_intent_id(value: &str) -> Result<[u8; 32]> {
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

// ============================================================================
// ATA DERIVATION
// ============================================================================

/// Derives the associated token account (ATA) address for an owner and mint.
fn get_associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    let token_program_id =
        Pubkey::from_str(SPL_TOKEN_PROGRAM_ID).expect("SPL token program id");
    let ata_program_id =
        Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM_ID).expect("ATA program id");
    Pubkey::find_program_address(
        &[owner.as_ref(), token_program_id.as_ref(), mint.as_ref()],
        &ata_program_id,
    )
    .0
}
