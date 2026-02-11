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
use solana_sdk::pubkey::Pubkey;
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pubkey: String,
    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    /// Read raw account data (base64-decoded) for any Solana account.
    /// Returns None if the account doesn't exist.
    pub async fn get_raw_account_data(&self, pubkey: &Pubkey) -> Result<Option<Vec<u8>>> {
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

        let data = STANDARD
            .decode(&account.data.0)
            .context("Failed to decode base64 account data")?;
        Ok(Some(data))
    }

    /// Read the outbound nonce for a destination chain from the GMP program.
    /// PDA seeds: ["nonce_out", dst_chain_id.to_le_bytes()]
    /// Returns the nonce value (next nonce to be assigned), or 0 if the account doesn't exist.
    pub async fn get_outbound_nonce(
        &self,
        gmp_program_id: &Pubkey,
        dst_chain_id: u32,
    ) -> Result<u64> {
        let chain_id_bytes = dst_chain_id.to_le_bytes();
        let (nonce_pda, _) = Pubkey::find_program_address(
            &[b"nonce_out", &chain_id_bytes],
            gmp_program_id,
        );

        let data = self.get_raw_account_data(&nonce_pda).await?;
        let Some(data) = data else {
            return Ok(0); // No nonce account = no messages sent to this chain
        };

        // OutboundNonceAccount layout: disc(1) + dst_chain_id(4) + nonce(8) + bump(1) = 14 bytes
        if data.len() < 13 {
            anyhow::bail!("OutboundNonceAccount too short: {} bytes", data.len());
        }

        let nonce = u64::from_le_bytes(
            data[5..13]
                .try_into()
                .context("Failed to parse nonce bytes")?,
        );
        Ok(nonce)
    }

    /// Read a stored outbound message from the GMP program.
    /// PDA seeds: ["message", dst_chain_id.to_le_bytes(), nonce.to_le_bytes()]
    /// Returns the parsed message, or None if the account doesn't exist.
    pub async fn get_message_data(
        &self,
        gmp_program_id: &Pubkey,
        dst_chain_id: u32,
        nonce: u64,
    ) -> Result<Option<SvmOutboundMessage>> {
        let chain_id_bytes = dst_chain_id.to_le_bytes();
        let nonce_bytes = nonce.to_le_bytes();
        let (message_pda, _) = Pubkey::find_program_address(
            &[b"message", &chain_id_bytes, &nonce_bytes],
            gmp_program_id,
        );

        let data = self.get_raw_account_data(&message_pda).await?;
        let Some(data) = data else {
            return Ok(None);
        };

        // MessageAccount layout (Borsh):
        //   disc(1) + src_chain_id(4) + dst_chain_id(4) + nonce(8) +
        //   dst_addr(32) + src_addr(32) + payload_len(4) + payload(N) + bump(1)
        if data.len() < 86 {
            anyhow::bail!("MessageAccount too short: {} bytes", data.len());
        }

        let disc = data[0];
        if disc != 7 {
            anyhow::bail!(
                "MessageAccount discriminator mismatch: expected 7, got {}",
                disc
            );
        }

        let src_chain_id =
            u32::from_le_bytes(data[1..5].try_into().context("src_chain_id")?);
        let dst_chain_id =
            u32::from_le_bytes(data[5..9].try_into().context("dst_chain_id")?);
        let msg_nonce =
            u64::from_le_bytes(data[9..17].try_into().context("nonce")?);

        let mut dst_addr = [0u8; 32];
        dst_addr.copy_from_slice(&data[17..49]);

        let mut src_addr = [0u8; 32];
        src_addr.copy_from_slice(&data[49..81]);

        let payload_len =
            u32::from_le_bytes(data[81..85].try_into().context("payload_len")?) as usize;
        if data.len() < 85 + payload_len {
            anyhow::bail!(
                "MessageAccount payload truncated: need {} bytes, have {}",
                85 + payload_len,
                data.len()
            );
        }
        let payload = data[85..85 + payload_len].to_vec();

        Ok(Some(SvmOutboundMessage {
            src_chain_id,
            dst_chain_id,
            nonce: msg_nonce,
            dst_addr,
            src_addr,
            payload,
        }))
    }
}

/// Parsed SVM outbound message from on-chain MessageAccount.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SvmOutboundMessage {
    pub src_chain_id: u32,
    pub dst_chain_id: u32,
    pub nonce: u64,
    pub dst_addr: [u8; 32],
    pub src_addr: [u8; 32],
    pub payload: Vec<u8>,
}

#[allow(dead_code)]
pub fn pubkey_to_hex(pubkey: &Pubkey) -> String {
    format!("0x{}", hex::encode(pubkey.to_bytes()))
}

fn parse_escrow_data(data_base64: &str) -> Option<EscrowAccount> {
    let data = STANDARD.decode(data_base64).ok()?;
    EscrowAccount::try_from_slice(&data).ok()
}
