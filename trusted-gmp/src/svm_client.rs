//! Solana SVM RPC Client Module
//!
//! This module provides a minimal client for querying SVM escrow accounts via
//! Solana JSON-RPC. It supports fetching escrow PDAs and parsing account data
//! using Borsh.

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use borsh::{BorshDeserialize, BorshSerialize};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use std::str::FromStr;
use std::time::Duration;

// ============================================================================
// ACCOUNT STRUCTURES
// ============================================================================

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

#[derive(Debug, Clone)]
pub struct EscrowWithPubkey {
    pub pubkey: Pubkey,
    pub escrow: EscrowAccount,
}

// ============================================================================
// JSON-RPC TYPES
// ============================================================================

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse<T> {
    result: Option<T>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct ProgramAccountResult {
    pubkey: String,
    account: RpcAccount,
}

#[derive(Debug, Deserialize)]
struct RpcAccount {
    data: (String, String),
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AccountInfoResult {
    value: Option<RpcAccount>,
}

// ============================================================================
// CLIENT
// ============================================================================

pub struct SvmClient {
    client: Client,
    rpc_url: String,
    program_id: Pubkey,
}

impl SvmClient {
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

    #[allow(dead_code)]
    pub fn program_id(&self) -> Pubkey {
        self.program_id
    }

    #[allow(dead_code)]
    pub fn escrow_pda(&self, intent_id: &[u8; 32]) -> Pubkey {
        Pubkey::find_program_address(&[b"escrow", intent_id], &self.program_id).0
    }

    #[allow(dead_code)]
    pub async fn get_escrow_by_intent_id(&self, intent_id: &[u8; 32]) -> Result<Option<EscrowAccount>> {
        let escrow_pda = self.escrow_pda(intent_id);
        let result = self.get_account_info(&escrow_pda).await?;
        Ok(result.map(|account| account.escrow))
    }

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

        let accounts = response.result.unwrap_or_default();
        let mut escrows = Vec::new();

        for account in accounts {
            let pubkey = Pubkey::from_str(&account.pubkey)
                .context("Invalid pubkey in getProgramAccounts response")?;
            if let Some(escrow) = parse_escrow_data(&account.account.data.0) {
                escrows.push(EscrowWithPubkey { pubkey, escrow });
            }
        }

        Ok(escrows)
    }

    #[allow(dead_code)]
    async fn get_account_info(&self, pubkey: &Pubkey) -> Result<Option<EscrowWithPubkey>> {
        let params = serde_json::json!([
            pubkey.to_string(),
            { "encoding": "base64" }
        ]);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "getAccountInfo".to_string(),
            params,
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

        let escrow = parse_escrow_data(&account.data.0)
            .context("Failed to parse escrow account data")?;

        Ok(Some(EscrowWithPubkey {
            pubkey: *pubkey,
            escrow,
        }))
    }
}

pub fn pubkey_to_hex(pubkey: &Pubkey) -> String {
    format!("0x{}", hex::encode(pubkey.to_bytes()))
}

fn parse_escrow_data(data_base64: &str) -> Option<EscrowAccount> {
    let data = STANDARD.decode(data_base64).ok()?;
    EscrowAccount::try_from_slice(&data).ok()
}
