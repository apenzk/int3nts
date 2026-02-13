//! Integrated GMP Relay Module
//!
//! Watches for `MessageSent` events on MVM and SVM integrated GMP endpoints
//! and delivers messages to destination chains by calling `deliver_message`.
//!
//! ## Architecture
//!
//! The relay:
//! 1. Polls MVM for `MessageSent` events from `gmp_sender` module
//! 2. Polls SVM transaction logs for `MessageSent` structured log messages
//! 3. Delivers messages to destination chain via `deliver_message` function
//!
//! ## Security
//!
//! **CRITICAL**: This relay has operator wallet keys and can deliver arbitrary messages.
//! Ensure proper key management and access controls for production use.
//! In production, this can be used directly with your own relay infrastructure,
//! or replaced by LZ's endpoint.

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use borsh::BorshSerialize;
use ed25519_dalek::SigningKey;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::collections::HashSet;
use std::process::Command;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::crypto::CryptoService;
use crate::evm_client::EvmLog;
use crate::mvm_client::MvmClient;
use crate::svm_client::SvmClient;

// Well-known Solana program IDs.
const SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([0; 32]);

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Configuration for the integrated GMP relay.
#[derive(Debug, Clone)]
pub struct NativeGmpRelayConfig {
    /// MVM hub chain RPC URL
    pub mvm_rpc_url: String,
    /// MVM intent module address (where gmp_sender is deployed)
    pub mvm_module_addr: String,
    /// MVM chain ID
    pub mvm_chain_id: u32,
    /// MVM connected chain RPC URL (optional, for MVM connected chain)
    pub mvm_connected_rpc_url: Option<String>,
    /// MVM connected chain module address (optional)
    pub mvm_connected_module_addr: Option<String>,
    /// MVM connected chain ID (optional)
    pub mvm_connected_chain_id: Option<u32>,
    /// SVM RPC URL (optional, for SVM connected chain)
    pub svm_rpc_url: Option<String>,
    /// SVM integrated GMP endpoint program ID (optional)
    pub svm_gmp_program_id: Option<String>,
    /// SVM intent escrow program ID (optional, for routing IntentRequirements)
    pub svm_escrow_program_id: Option<String>,
    /// SVM outflow validator program ID (optional, for routing IntentRequirements)
    pub svm_outflow_program_id: Option<String>,
    /// SVM chain ID (optional)
    pub svm_chain_id: Option<u32>,
    /// EVM RPC URL (optional, for EVM connected chain)
    pub evm_rpc_url: Option<String>,
    /// EVM GMP endpoint contract address (IntentGmp)
    pub evm_gmp_endpoint_addr: Option<String>,
    /// EVM chain ID (optional)
    pub evm_chain_id: Option<u32>,
    /// EVM relay address (the `from` address for eth_sendTransaction, must be authorized relay in IntentGmp)
    pub evm_relay_address: Option<String>,
    /// Polling interval in milliseconds
    pub polling_interval_ms: u64,
    /// Relay operator private key (base64 encoded Ed25519)
    pub operator_private_key: String,
}

impl NativeGmpRelayConfig {
    /// Create relay config from main config.
    pub fn from_config(config: &Config) -> Result<Self> {
        let operator_private_key = config.integrated_gmp.get_private_key()?;

        // Extract MVM connected chain config if present
        let (mvm_connected_rpc_url, mvm_connected_module_addr, mvm_connected_chain_id) =
            if let Some(ref mvm_config) = config.connected_chain_mvm {
                (
                    Some(mvm_config.rpc_url.clone()),
                    Some(mvm_config.intent_module_addr.clone()),
                    Some(mvm_config.chain_id as u32),
                )
            } else {
                (None, None, None)
            };

        // Extract SVM connected chain config if present
        let (svm_rpc_url, svm_gmp_program_id, svm_escrow_program_id, svm_outflow_program_id, svm_chain_id) =
            if let Some(ref svm_config) = config.connected_chain_svm {
                (
                    Some(svm_config.rpc_url.clone()),
                    svm_config.gmp_endpoint_program_id.clone(),
                    Some(svm_config.escrow_program_id.clone()),
                    Some(svm_config.outflow_program_id.clone()),
                    Some(svm_config.chain_id as u32),
                )
            } else {
                (None, None, None, None, None)
            };

        // Extract EVM connected chain config if present
        let (evm_rpc_url, evm_gmp_endpoint_addr, evm_chain_id, evm_relay_address) =
            if let Some(ref evm_config) = config.connected_chain_evm {
                (
                    Some(evm_config.rpc_url.clone()),
                    evm_config.gmp_endpoint_addr.clone(),
                    Some(evm_config.chain_id as u32),
                    Some(evm_config.approver_evm_pubkey_hash.clone()),
                )
            } else {
                (None, None, None, None)
            };

        Ok(Self {
            mvm_rpc_url: config.hub_chain.rpc_url.clone(),
            mvm_module_addr: config.hub_chain.intent_module_addr.clone(),
            mvm_chain_id: config.hub_chain.chain_id as u32,
            mvm_connected_rpc_url,
            mvm_connected_module_addr,
            mvm_connected_chain_id,
            svm_rpc_url,
            svm_gmp_program_id,
            svm_escrow_program_id,
            svm_outflow_program_id,
            svm_chain_id,
            evm_rpc_url,
            evm_gmp_endpoint_addr,
            evm_chain_id,
            evm_relay_address,
            polling_interval_ms: config.integrated_gmp.polling_interval_ms,
            operator_private_key,
        })
    }
}

// ============================================================================
// MESSAGE STRUCTURES
// ============================================================================

/// Represents a GMP message to be relayed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmpMessage {
    /// Source chain ID
    pub src_chain_id: u32,
    /// Remote GMP endpoint address (32 bytes, hex encoded with 0x prefix)
    pub remote_gmp_endpoint_addr: String,
    /// Destination chain ID
    pub dst_chain_id: u32,
    /// Destination address (32 bytes, hex encoded with 0x prefix)
    pub dst_addr: String,
    /// Message payload (hex encoded with 0x prefix)
    pub payload: String,
    /// Nonce for ordering/replay protection
    pub nonce: u64,
}

/// MVM MessageSent event data structure.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct MvmMessageSentEvent {
    /// Destination chain endpoint ID
    pub dst_chain_id: String,
    /// Destination address (hex array)
    pub dst_addr: Vec<String>,
    /// Message payload (hex array)
    pub payload: Vec<String>,
    /// Sender address
    pub sender: String,
    /// Sequence number
    pub nonce: String,
}

// ============================================================================
// RELAY STATE
// ============================================================================

/// Internal state for tracking processed messages.
#[derive(Debug, Default)]
struct RelayState {
    /// Processed nonces per source chain (chain_id -> set of processed nonces)
    processed_nonces: std::collections::HashMap<u32, HashSet<u64>>,
    /// Last polled nonce for MVM hub outbox (view function based)
    mvm_hub_last_nonce: u64,
    /// Last polled nonce for MVM connected chain outbox (view function based)
    mvm_connected_last_nonce: u64,
    /// SVM outbound nonce cursor per destination chain (dst_chain_id -> next nonce to process).
    /// Value N means nonces 0..N-1 have been processed; N is the next to process.
    svm_next_nonce: std::collections::HashMap<u32, u64>,
    /// Last polled EVM block number
    evm_last_block: u64,
}

// ============================================================================
// INTEGRATED GMP RELAY
// ============================================================================

/// Integrated GMP relay service that watches for MessageSent events
/// and delivers messages to destination chains.
pub struct NativeGmpRelay {
    config: NativeGmpRelayConfig,
    crypto_service: CryptoService,
    mvm_client: MvmClient,
    mvm_connected_client: Option<MvmClient>,
    svm_client: Option<SvmClient>,
    #[allow(dead_code)]
    http_client: Client,
    state: Arc<RwLock<RelayState>>,
}

impl NativeGmpRelay {
    /// Create a new integrated GMP relay.
    pub fn new(config: NativeGmpRelayConfig, crypto_service: CryptoService) -> Result<Self> {
        let mvm_client = MvmClient::new(&config.mvm_rpc_url)?;

        // Initialize MVM connected client if configured
        let mvm_connected_client = match &config.mvm_connected_rpc_url {
            Some(rpc_url) => {
                Some(MvmClient::new(rpc_url).context("Failed to create MVM connected client")?)
            }
            _ => None,
        };

        // Initialize SVM client if configured
        let svm_client = match (&config.svm_rpc_url, &config.svm_gmp_program_id) {
            (Some(rpc_url), Some(program_id)) => {
                Some(SvmClient::new(rpc_url, program_id).context("Failed to create SVM client")?)
            }
            _ => None,
        };

        let http_client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(15))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            config,
            crypto_service,
            mvm_client,
            mvm_connected_client,
            svm_client,
            http_client,
            state: Arc::new(RwLock::new(RelayState::default())),
        })
    }

    /// Check relay authorization on all configured destination chains at startup.
    ///
    /// Queries each chain's GMP endpoint to verify this relay operator is authorized.
    /// Fails fast if any chain reports the relay is NOT authorized.
    async fn check_authorization(&self) -> Result<()> {
        let mvm_addr = self.crypto_service.get_move_address()?;
        let evm_addr = self.crypto_service.get_ethereum_address()?;
        let svm_addr = self.crypto_service.get_solana_address();

        info!("Relay addresses: MVM={}, EVM={}, SVM={}", mvm_addr, evm_addr, svm_addr);

        // Check MVM hub
        self.check_mvm_relay_auth(
            &self.mvm_client,
            &self.config.mvm_module_addr,
            &mvm_addr,
            "MVM hub",
        )
        .await?;

        // Check MVM connected (if configured)
        if let (Some(ref client), Some(ref module_addr)) =
            (&self.mvm_connected_client, &self.config.mvm_connected_module_addr)
        {
            self.check_mvm_relay_auth(client, module_addr, &mvm_addr, "MVM connected")
                .await?;
        }

        // Check EVM (if configured)
        if let (Some(ref rpc_url), Some(ref gmp_endpoint)) =
            (&self.config.evm_rpc_url, &self.config.evm_gmp_endpoint_addr)
        {
            self.check_evm_relay_auth(rpc_url, gmp_endpoint, &evm_addr)
                .await?;
        }

        // Check SVM (if configured)
        if let (Some(ref rpc_url), Some(ref program_id_str)) =
            (&self.config.svm_rpc_url, &self.config.svm_gmp_program_id)
        {
            self.check_svm_relay_auth(rpc_url, program_id_str, &svm_addr)
                .await?;
        }

        info!("Relay authorization verified on all configured chains");
        Ok(())
    }

    /// Check if relay is authorized on an MVM chain via is_relay_authorized view function.
    async fn check_mvm_relay_auth(
        &self,
        client: &MvmClient,
        module_addr: &str,
        relay_addr: &str,
        chain_name: &str,
    ) -> Result<()> {
        let result = client
            .call_view_function(
                module_addr,
                "intent_gmp",
                "is_relay_authorized",
                vec![],
                vec![serde_json::json!(relay_addr)],
            )
            .await
            .with_context(|| format!("Failed to check relay authorization on {}", chain_name))?;

        let authorized = result
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !authorized {
            anyhow::bail!(
                "Relay {} is NOT authorized on {} (module {}). Run add_relay first.",
                relay_addr, chain_name, module_addr
            );
        }

        info!("{}: relay {} authorized", chain_name, relay_addr);
        Ok(())
    }

    /// Check if relay is authorized on EVM via isRelayAuthorized(address) view function.
    async fn check_evm_relay_auth(
        &self,
        rpc_url: &str,
        gmp_endpoint: &str,
        relay_addr: &str,
    ) -> Result<()> {
        // Function selector: keccak256("isRelayAuthorized(address)")[0..4]
        let selector = &Keccak256::digest(b"isRelayAuthorized(address)")[..4];

        // ABI-encode: selector + address padded to 32 bytes
        let addr_clean = relay_addr.strip_prefix("0x").unwrap_or(relay_addr);
        let mut calldata = Vec::with_capacity(36);
        calldata.extend_from_slice(selector);
        // Pad address (20 bytes) to 32 bytes, left-padded with zeros
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(&hex::decode(addr_clean).context("Invalid EVM address hex")?);

        let data_hex = format!("0x{}", hex::encode(&calldata));

        let result: String = self
            .evm_json_rpc(
                rpc_url,
                "eth_call",
                vec![
                    serde_json::json!({
                        "to": gmp_endpoint,
                        "data": data_hex,
                    }),
                    serde_json::json!("latest"),
                ],
            )
            .await
            .context("Failed to check relay authorization on EVM")?;

        // Result is ABI-encoded bool: 32 bytes, last byte is 0 or 1
        let clean = result.strip_prefix("0x").unwrap_or(&result);
        let authorized = clean.ends_with('1');

        if !authorized {
            anyhow::bail!(
                "Relay {} is NOT authorized on EVM (contract {}). Run addRelay first.",
                relay_addr, gmp_endpoint
            );
        }

        info!("EVM: relay {} authorized", relay_addr);
        Ok(())
    }

    /// Check if relay is authorized on SVM by reading the relay PDA account.
    async fn check_svm_relay_auth(
        &self,
        rpc_url: &str,
        program_id_str: &str,
        relay_addr: &str,
    ) -> Result<()> {
        let program_id =
            Pubkey::from_str(program_id_str).context("Invalid SVM program ID")?;
        let relay_pubkey =
            Pubkey::from_str(relay_addr).context("Invalid SVM relay address")?;

        // Derive relay PDA: seeds = [b"relay", relay_pubkey]
        let (relay_pda, _) =
            Pubkey::find_program_address(&[b"relay", relay_pubkey.as_ref()], &program_id);

        let rpc_client =
            RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());

        match rpc_client.get_account_data(&relay_pda) {
            Ok(data) => {
                // RelayAccount layout: discriminator(1) + relay(32) + is_authorized(1) + bump(1)
                // is_authorized is at offset 33
                if data.len() >= 34 && data[33] == 1 {
                    info!("SVM: relay {} authorized", relay_addr);
                    Ok(())
                } else {
                    anyhow::bail!(
                        "Relay {} is NOT authorized on SVM (program {}). Run add_relay first.",
                        relay_addr, program_id_str
                    );
                }
            }
            Err(_) => {
                anyhow::bail!(
                    "Relay PDA account not found on SVM for {} (program {}). Run add_relay first.",
                    relay_addr, program_id_str
                );
            }
        }
    }

    /// Start the relay service (blocking).
    pub async fn run(&self) -> Result<()> {
        info!(
            "Starting integrated GMP relay: MVM hub chain_id={}, polling_interval={}ms",
            self.config.mvm_chain_id, self.config.polling_interval_ms
        );

        if let Some(ref mvm_connected_chain_id) = self.config.mvm_connected_chain_id {
            info!("MVM connected chain configured: chain_id={}", mvm_connected_chain_id);
        }

        if let Some(ref svm_chain_id) = self.config.svm_chain_id {
            info!("SVM chain configured: chain_id={}", svm_chain_id);
        }

        if let Some(ref evm_chain_id) = self.config.evm_chain_id {
            info!("EVM chain configured: chain_id={}", evm_chain_id);
        }

        // Verify relay is authorized on all destination chains before starting
        self.check_authorization().await?;

        let interval = Duration::from_millis(self.config.polling_interval_ms);

        loop {
            // Poll MVM hub for MessageSent events
            if let Err(e) = self.poll_mvm_events().await {
                error!("Error polling MVM hub events: {}", e);
            }

            // Poll MVM connected chain for MessageSent events (if configured)
            if self.mvm_connected_client.is_some() {
                if let Err(e) = self.poll_mvm_connected_events().await {
                    error!("Error polling MVM connected chain events: {}", e);
                }
            }

            // Poll SVM for outbound messages (if configured)
            if self.config.svm_rpc_url.is_some() {
                if let Err(e) = self.poll_svm_events().await {
                    error!("Error polling SVM events: {}", e);
                }
            }

            // Poll EVM for MessageSent events (if configured)
            if self.config.evm_rpc_url.is_some() {
                if let Err(e) = self.poll_evm_events().await {
                    error!("Error polling EVM events: {}", e);
                }
            }

            tokio::time::sleep(interval).await;
        }
    }

    /// Poll MVM hub outbox for new messages via view functions.
    async fn poll_mvm_events(&self) -> Result<()> {
        let last_nonce = {
            self.state.read().await.mvm_hub_last_nonce
        };

        let new_last = self
            .poll_mvm_outbox(
                &self.mvm_client,
                &self.config.mvm_module_addr,
                self.config.mvm_chain_id,
                last_nonce,
                "hub",
            )
            .await?;

        if new_last > last_nonce {
            self.state.write().await.mvm_hub_last_nonce = new_last;
        }

        Ok(())
    }

    /// Poll MVM connected chain outbox for new messages via view functions.
    async fn poll_mvm_connected_events(&self) -> Result<()> {
        let Some(ref mvm_connected_client) = self.mvm_connected_client else {
            return Ok(());
        };

        let Some(mvm_connected_chain_id) = self.config.mvm_connected_chain_id else {
            return Ok(());
        };

        let Some(ref mvm_connected_module_addr) = self.config.mvm_connected_module_addr else {
            return Ok(());
        };

        let last_nonce = {
            self.state.read().await.mvm_connected_last_nonce
        };

        let new_last = self
            .poll_mvm_outbox(
                mvm_connected_client,
                mvm_connected_module_addr,
                mvm_connected_chain_id,
                last_nonce,
                "connected",
            )
            .await?;

        if new_last > last_nonce {
            self.state.write().await.mvm_connected_last_nonce = new_last;
        }

        Ok(())
    }

    /// Shared outbox polling logic for any MVM chain.
    ///
    /// Calls `gmp_sender::get_next_nonce()` to detect new messages, then
    /// reads each new message via `gmp_sender::get_message(nonce)`.
    ///
    /// Returns the new last_nonce value (highest nonce successfully delivered).
    async fn poll_mvm_outbox(
        &self,
        client: &MvmClient,
        module_addr: &str,
        src_chain_id: u32,
        last_nonce: u64,
        chain_name: &str,
    ) -> Result<u64> {
        // Call get_next_nonce() view function
        let next_nonce_result = client
            .call_view_function(
                module_addr,
                "gmp_sender",
                "get_next_nonce",
                vec![],
                vec![],
            )
            .await
            .context("Failed to call get_next_nonce")?;

        // Parse: response is [\"<number>\"] or [<number>]
        let next_nonce: u64 = next_nonce_result
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| {
                v.as_str()
                    .and_then(|s| s.parse().ok())
                    .or_else(|| v.as_u64())
            })
            .context(format!(
                "Failed to parse get_next_nonce response for MVM {} outbox: {:?}",
                chain_name, next_nonce_result
            ))?;

        // Determine start nonce: if we haven't polled yet, start from 1
        let start = if last_nonce == 0 { 1 } else { last_nonce + 1 };

        if start >= next_nonce {
            debug!(
                "MVM {} outbox: next_nonce={}, last_nonce={} (idle)",
                chain_name, next_nonce, last_nonce
            );
            return Ok(last_nonce); // No new messages
        }

        info!(
            "MVM {} outbox: processing nonces {}..{} ({} messages)",
            chain_name, start, next_nonce - 1, next_nonce - start
        );

        let mut new_last = last_nonce;

        for nonce in start..next_nonce {
            // Call get_message(nonce) view function
            let msg_result = client
                .call_view_function(
                    module_addr,
                    "gmp_sender",
                    "get_message",
                    vec![],
                    vec![serde_json::json!(nonce.to_string())],
                )
                .await;

            let msg_value = match msg_result {
                Ok(v) => v,
                Err(e) => {
                    // Message may have been cleaned up (expired). Skip it.
                    warn!(
                        "MVM {} outbox: failed to read nonce {}: {}. Skipping (may be expired).",
                        chain_name, nonce, e
                    );
                    new_last = nonce;
                    continue;
                }
            };

            // Parse view function result: [dst_chain_id, dst_addr_hex, payload_hex, sender]
            let arr = msg_value.as_array().context("get_message result is not an array")?;
            if arr.len() < 4 {
                warn!("MVM {} outbox: get_message({}) returned {} elements, expected 4", chain_name, nonce, arr.len());
                new_last = nonce;
                continue;
            }

            let dst_chain_id: u32 = arr[0]
                .as_str()
                .and_then(|s| s.parse().ok())
                .or_else(|| arr[0].as_u64().map(|n| n as u32))
                .context("Failed to parse dst_chain_id")?;

            let dst_addr_hex = parse_view_bytes(&arr[1])?;
            let payload_hex = parse_view_bytes(&arr[2])?;

            let _sender_addr = arr[3]
                .as_str()
                .unwrap_or("0x0")
                .to_string();

            let message = GmpMessage {
                src_chain_id,
                // Use the module address (where GMP contracts are deployed) as remote_gmp_endpoint_addr,
                // not the individual sender. The destination chain's intent_gmp
                // validates the source chain's module address as the remote GMP endpoint, not individual senders.
                remote_gmp_endpoint_addr: normalize_address(module_addr),
                dst_chain_id,
                dst_addr: format!("0x{}", dst_addr_hex),
                payload: format!("0x{}", payload_hex),
                nonce,
            };

            info!(
                "MVM {} outbox: nonce={}, src={}, dst_chain={}",
                chain_name, nonce, message.remote_gmp_endpoint_addr, message.dst_chain_id
            );

            if let Err(e) = self.deliver_message(&message).await {
                let err_str = format!("{:#}", e);
                // Permanent errors: skip and advance past the message
                if err_str.contains("E_UNKNOWN_REMOTE_GMP_ENDPOINT")
                    || err_str.contains("E_ALREADY_DELIVERED")
                    || err_str.contains("AlreadyDelivered")
                    || err_str.contains("Already delivered")
                    || err_str.contains("E_INTENT_NOT_FOUND")
                {
                    warn!(
                        "Permanent delivery failure for MVM {} nonce={}, skipping: {}",
                        chain_name, nonce, err_str
                    );
                    new_last = nonce;
                    continue;
                }
                error!("Failed to deliver MVM {} message nonce={}: {}", chain_name, nonce, err_str);
                // Don't advance past transient failures
                break;
            }

            new_last = nonce;
        }

        Ok(new_last)
    }

    /// Poll SVM for outbound messages using nonce-based polling.
    ///
    /// Reads the OutboundNonceAccount for each known destination chain via
    /// getAccountInfo, then reads individual MessageAccount PDAs for any new nonces.
    async fn poll_svm_events(&self) -> Result<()> {
        let Some(ref svm_client) = self.svm_client else {
            return Ok(());
        };

        let Some(svm_chain_id) = self.config.svm_chain_id else {
            return Ok(());
        };

        let gmp_program_id = Pubkey::from_str(
            self.config.svm_gmp_program_id.as_ref().unwrap(),
        )
        .context("Invalid SVM GMP program ID")?;

        // Collect destination chain IDs to poll nonces for.
        // SVM sends messages to the MVM hub (and potentially EVM).
        let mut dst_chain_ids = vec![self.config.mvm_chain_id];
        if let Some(evm_chain_id) = self.config.evm_chain_id {
            dst_chain_ids.push(evm_chain_id);
        }

        for dst_chain_id in dst_chain_ids {
            // Read the on-chain nonce counter (getAccountInfo — not rate-limited)
            let next_nonce = svm_client
                .get_outbound_nonce(&gmp_program_id, dst_chain_id)
                .await
                .context("Failed to read SVM outbound nonce")?;

            if next_nonce == 0 {
                continue; // No messages sent to this chain
            }

            // Get our cursor for this destination chain
            let start = {
                let state = self.state.read().await;
                *state.svm_next_nonce.get(&dst_chain_id).unwrap_or(&0)
            };

            if start >= next_nonce {
                continue; // No new messages
            }

            debug!(
                "SVM outbox (dst_chain={}): polling nonces {}..{} (next_nonce={})",
                dst_chain_id,
                start,
                next_nonce - 1,
                next_nonce
            );

            for nonce in start..next_nonce {
                // Read the message account (getAccountInfo — not rate-limited)
                let msg = svm_client
                    .get_message_data(&gmp_program_id, dst_chain_id, nonce)
                    .await
                    .context(format!("Failed to read SVM message nonce={}", nonce))?;

                let Some(msg) = msg else {
                    warn!(
                        "SVM outbox: message account not found for dst_chain={}, nonce={}. Skipping.",
                        dst_chain_id, nonce
                    );
                    // Advance past missing message (may have been cleaned up)
                    self.state.write().await.svm_next_nonce.insert(dst_chain_id, nonce + 1);
                    continue;
                };

                // Build GmpMessage from the on-chain data
                let message = GmpMessage {
                    src_chain_id: svm_chain_id,
                    remote_gmp_endpoint_addr: format!("0x{}", hex::encode(msg.remote_gmp_endpoint_addr)),
                    dst_chain_id: msg.dst_chain_id,
                    dst_addr: format!("0x{}", hex::encode(msg.dst_addr)),
                    payload: format!("0x{}", hex::encode(&msg.payload)),
                    nonce: msg.nonce,
                };

                info!(
                    "SVM outbox: nonce={}, src={}, dst_chain={}",
                    nonce, message.remote_gmp_endpoint_addr, message.dst_chain_id
                );

                if let Err(e) = self.deliver_message(&message).await {
                    let err_str = format!("{:#}", e);
                    // Permanent errors: skip and advance past the message
                    if err_str.contains("E_UNKNOWN_REMOTE_GMP_ENDPOINT")
                        || err_str.contains("E_ALREADY_DELIVERED")
                        || err_str.contains("E_INTENT_NOT_FOUND")
                    {
                        warn!(
                            "Permanent delivery failure for SVM nonce={}, skipping: {}",
                            nonce, err_str
                        );
                        self.state.write().await.svm_next_nonce.insert(dst_chain_id, nonce + 1);
                        continue;
                    }
                    error!("Failed to deliver SVM message nonce={}: {}", nonce, err_str);
                    // Don't advance past transient failures
                    break;
                }

                self.state.write().await.svm_next_nonce.insert(dst_chain_id, nonce + 1);
            }
        }

        Ok(())
    }

    /// Parse MVM MessageSent event into GmpMessage.
    #[allow(dead_code)]
    fn parse_mvm_message_sent(
        &self,
        event: &MvmMessageSentEvent,
        src_chain_id: u32,
    ) -> Result<GmpMessage> {
        // Parse destination chain ID
        let dst_chain_id: u32 = event.dst_chain_id.parse().context("Invalid dst_chain_id")?;

        // Parse destination address (array of hex bytes -> hex string)
        let dst_addr = bytes_array_to_hex(&event.dst_addr)?;

        // Parse payload (array of hex bytes -> hex string)
        let payload = bytes_array_to_hex(&event.payload)?;

        // Parse nonce
        let nonce: u64 = event.nonce.parse().context("Invalid nonce")?;

        // Source address is the sender (remote GMP endpoint address)
        let remote_gmp_endpoint_addr = normalize_address(&event.sender);

        Ok(GmpMessage {
            src_chain_id,
            remote_gmp_endpoint_addr,
            dst_chain_id,
            dst_addr,
            payload,
            nonce,
        })
    }

    /// Deliver a GMP message to the destination chain.
    async fn deliver_message(&self, message: &GmpMessage) -> Result<()> {
        // Determine destination chain type based on chain ID
        if message.dst_chain_id == self.config.mvm_chain_id {
            // Destination is MVM hub
            self.deliver_to_mvm_hub(message).await
        } else if Some(message.dst_chain_id) == self.config.mvm_connected_chain_id {
            // Destination is MVM connected chain
            self.deliver_to_mvm_connected(message).await
        } else if Some(message.dst_chain_id) == self.config.svm_chain_id {
            // Destination is SVM
            self.deliver_to_svm(message).await
        } else if Some(message.dst_chain_id) == self.config.evm_chain_id {
            // Destination is EVM
            self.deliver_to_evm(message).await
        } else {
            warn!(
                "Unknown destination chain ID: {}. Known chains: MVM hub={}, MVM connected={:?}, SVM={:?}, EVM={:?}",
                message.dst_chain_id, self.config.mvm_chain_id, self.config.mvm_connected_chain_id, self.config.svm_chain_id, self.config.evm_chain_id
            );
            Ok(())
        }
    }

    /// Deliver message to MVM hub chain via intent_gmp::deliver_message_entry.
    ///
    /// Uses the CLI-based transaction submission pattern (same as solver).
    async fn deliver_to_mvm_hub(&self, message: &GmpMessage) -> Result<()> {
        self.deliver_to_mvm_chain(
            message,
            &self.config.mvm_rpc_url,
            &self.config.mvm_module_addr,
            "hub",
        )
        .await
    }

    /// Deliver message to MVM connected chain via intent_gmp::deliver_message_entry.
    async fn deliver_to_mvm_connected(&self, message: &GmpMessage) -> Result<()> {
        let rpc_url = self
            .config
            .mvm_connected_rpc_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("MVM connected chain not configured"))?;
        let module_addr = self
            .config
            .mvm_connected_module_addr
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("MVM connected chain module address not configured"))?;

        self.deliver_to_mvm_chain(message, rpc_url, module_addr, "connected")
            .await
    }

    /// Deliver message to an MVM chain (hub or connected).
    async fn deliver_to_mvm_chain(
        &self,
        message: &GmpMessage,
        rpc_url: &str,
        module_addr: &str,
        chain_name: &str,
    ) -> Result<()> {
        info!(
            "Delivering message to MVM {}: dst_chain={}, nonce={}",
            chain_name, message.dst_chain_id, message.nonce
        );

        // Parse addresses and payload to hex format (strip 0x if present)
        let remote_gmp_endpoint_addr_hex = message.remote_gmp_endpoint_addr.strip_prefix("0x").unwrap_or(&message.remote_gmp_endpoint_addr);
        let payload_hex = message.payload.strip_prefix("0x").unwrap_or(&message.payload);

        // Convert base64 private key to hex for CLI
        let private_key_bytes = STANDARD
            .decode(&self.config.operator_private_key)
            .context("Failed to decode base64 private key")?;
        let private_key_hex = hex::encode(&private_key_bytes);

        // Build function ID
        let function_id = format!(
            "{}::intent_gmp::deliver_message_entry",
            module_addr
        );

        // Build CLI arguments
        // Function signature: deliver_message_entry(relay: &signer, src_chain_id: u32, remote_gmp_endpoint_addr: vector<u8>, payload: vector<u8>)
        let src_chain_id_arg = format!("u32:{}", message.src_chain_id);
        let remote_gmp_endpoint_addr_arg = format!("hex:{}", remote_gmp_endpoint_addr_hex);
        let payload_arg = format!("hex:{}", payload_hex);

        // Normalize RPC URL (strip trailing /v1 if present for CLI)
        let rpc_url_normalized = rpc_url.trim_end_matches('/').trim_end_matches("/v1");

        debug!(
            "MVM {} deliver_message CLI call: function_id={}, src_chain_id={}, nonce={}",
            chain_name, function_id, message.src_chain_id, message.nonce
        );

        // Execute CLI command (using aptos CLI for MVM)
        let output = Command::new("aptos")
            .args([
                "move",
                "run",
                "--private-key",
                &private_key_hex,
                "--url",
                rpc_url_normalized,
                "--assume-yes",
                "--function-id",
                &function_id,
                "--args",
                &src_chain_id_arg,
                &remote_gmp_endpoint_addr_arg,
                &payload_arg,
            ])
            .output()
            .context("Failed to execute aptos move run")?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        if !output.status.success() {
            // Log at debug; the caller decides severity (permanent vs transient).
            debug!(
                "MVM {} deliver_message failed: stderr={}, stdout={}",
                chain_name, stderr, stdout
            );
            anyhow::bail!(
                "aptos move run failed for deliver_message_entry on {}: stderr={}, stdout={}",
                chain_name,
                stderr,
                stdout
            );
        }

        // Extract transaction hash from output for logging
        let output_str = stdout.as_ref();
        let tx_hash = extract_transaction_hash(output_str);

        // Verify VM execution succeeded (CLI exit code 0 doesn't guarantee VM success)
        let vm_success = check_vm_status_success(output_str);
        if !vm_success {
            error!(
                "MVM {} deliver_message VM execution failed: nonce={}, tx_hash={:?}, stdout={}",
                chain_name, message.nonce, tx_hash, stdout
            );
            anyhow::bail!(
                "deliver_message_entry VM execution failed on {}: tx_hash={:?}, stdout={}",
                chain_name,
                tx_hash,
                stdout
            );
        }

        info!(
            "MVM {} deliver_message submitted successfully: nonce={}, tx_hash={:?}",
            chain_name, message.nonce, tx_hash
        );

        Ok(())
    }

    /// Deliver message to EVM chain via IntentGmp.deliverMessage().
    ///
    /// ABI-encodes the call and sends via eth_sendRawTransaction (locally signed).
    async fn deliver_to_evm(&self, message: &GmpMessage) -> Result<()> {
        let evm_rpc_url = self
            .config
            .evm_rpc_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("EVM RPC URL not configured"))?;
        let gmp_endpoint = self
            .config
            .evm_gmp_endpoint_addr
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("EVM GMP endpoint address not configured"))?;
        let relay_addr = self
            .config
            .evm_relay_address
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("EVM relay address not configured"))?;
        let chain_id = self
            .config
            .evm_chain_id
            .ok_or_else(|| anyhow::anyhow!("EVM chain ID not configured"))?;

        info!(
            "Delivering message to EVM: dst_chain={}, nonce={}",
            message.dst_chain_id, message.nonce
        );

        // Pre-check: skip if already delivered on EVM (avoids wasting gas on reverts)
        let payload_hex = message.payload.strip_prefix("0x").unwrap_or(&message.payload);
        let payload_bytes = hex::decode(payload_hex).context("Failed to hex-decode payload")?;
        if payload_bytes.len() >= 33 {
            let msg_type = payload_bytes[0];
            let intent_id = &payload_bytes[1..33];
            if self
                .evm_is_message_delivered(evm_rpc_url, gmp_endpoint, intent_id, msg_type)
                .await?
            {
                info!(
                    "EVM: message already delivered (nonce={}, msg_type=0x{:02x}), skipping",
                    message.nonce, msg_type
                );
                return Ok(());
            }
        }

        // ABI-encode deliverMessage(uint32,bytes32,bytes)
        let calldata = evm_encode_deliver_message(
            message.src_chain_id,
            &message.remote_gmp_endpoint_addr,
            &message.payload,
        )?;

        // Send locally-signed transaction via eth_sendRawTransaction
        info!("EVM: sending raw transaction to {}", evm_rpc_url);
        let tx_hash = self
            .evm_send_raw_transaction(evm_rpc_url, relay_addr, gmp_endpoint, &calldata, chain_id)
            .await?;

        // Wait for receipt and verify success
        info!("EVM: waiting for receipt for tx_hash={}", tx_hash);
        self.evm_wait_for_receipt(evm_rpc_url, &tx_hash).await?;

        info!(
            "EVM deliver_message submitted successfully: nonce={}, tx_hash={}",
            message.nonce, tx_hash
        );

        Ok(())
    }

    /// Poll EVM chain for MessageSent events from IntentGmp contract.
    async fn poll_evm_events(&self) -> Result<()> {
        let evm_rpc_url = self
            .config
            .evm_rpc_url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("EVM not configured"))?;
        let gmp_endpoint = self
            .config
            .evm_gmp_endpoint_addr
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("EVM GMP endpoint address not configured"))?;
        let evm_chain_id = self
            .config
            .evm_chain_id
            .ok_or_else(|| anyhow::anyhow!("EVM chain ID not configured"))?;

        // Get current block number
        let current_block = self.evm_get_block_number(evm_rpc_url).await?;

        // Determine start block (max 10 block range for Alchemy free tier)
        let max_range: u64 = 10;
        let last_block = { self.state.read().await.evm_last_block };
        let from_block = if last_block == 0 {
            // First poll: start from recent blocks
            current_block.saturating_sub(max_range)
        } else {
            last_block + 1
        };

        if from_block > current_block {
            return Ok(());
        }

        // Cap range to max_range blocks per query
        let to_block = from_block.saturating_add(max_range - 1).min(current_block);

        // Compute MessageSent event topic
        // MessageSent(uint32 indexed dstChainId, bytes32 dstAddr, bytes payload, uint64 nonce)
        let event_signature = evm_event_topic("MessageSent(uint32,bytes32,bytes,uint64)");

        // Query eth_getLogs
        let filter = serde_json::json!({
            "address": gmp_endpoint,
            "topics": [event_signature],
            "fromBlock": format!("0x{:x}", from_block),
            "toBlock": format!("0x{:x}", to_block),
        });

        let logs: Vec<EvmLog> = self.evm_json_rpc(evm_rpc_url, "eth_getLogs", vec![filter]).await?;

        for log in &logs {
            if let Some(message) = self.parse_evm_message_sent(log, evm_chain_id) {
                info!(
                    "Found EVM MessageSent: dst_chain={}, nonce={}",
                    message.dst_chain_id, message.nonce
                );

                // Check if already processed by nonce
                {
                    let state = self.state.read().await;
                    if let Some(processed) = state.processed_nonces.get(&evm_chain_id) {
                        if processed.contains(&message.nonce) {
                            continue;
                        }
                    }
                }

                if let Err(e) = self.deliver_message(&message).await {
                    error!("Failed to deliver EVM message nonce={}: {}", message.nonce, e);
                    continue;
                }

                // Mark nonce as processed
                {
                    let mut state = self.state.write().await;
                    state
                        .processed_nonces
                        .entry(evm_chain_id)
                        .or_default()
                        .insert(message.nonce);
                }
            }
        }

        // Update last polled block
        {
            self.state.write().await.evm_last_block = to_block;
        }

        Ok(())
    }

    /// Parse EVM MessageSent log into GmpMessage.
    ///
    /// Event: MessageSent(uint32 indexed dstChainId, bytes32 dstAddr, bytes payload, uint64 nonce)
    /// topics[0] = event signature, topics[1] = dstChainId (indexed)
    /// data = ABI-encoded(bytes32 dstAddr, bytes payload, uint64 nonce)
    fn parse_evm_message_sent(&self, log: &EvmLog, evm_chain_id: u32) -> Option<GmpMessage> {
        if log.topics.len() < 2 {
            return None;
        }

        // topics[1] = dstChainId (uint32, padded to 32 bytes)
        let dst_chain_id_hex = log.topics[1].strip_prefix("0x").unwrap_or(&log.topics[1]);
        let dst_chain_id = u32::from_str_radix(dst_chain_id_hex.trim_start_matches('0'), 16)
            .unwrap_or(0);

        // Parse non-indexed data: (bytes32 dstAddr, bytes payload, uint64 nonce)
        let data = log.data.strip_prefix("0x").unwrap_or(&log.data);

        // Minimum data: dstAddr(64) + payloadOffset(64) + nonce(64) + payloadLen(64) = 256 hex chars
        if data.len() < 256 {
            warn!("EVM MessageSent data too short: {} hex chars", data.len());
            return None;
        }

        // Word 0 (0..64): dstAddr (bytes32)
        let dst_addr = format!("0x{}", &data[0..64]);

        // Word 1 (64..128): offset to payload data (should be 96 = 0x60)
        let payload_offset_hex = &data[64..128];
        let payload_offset = usize::from_str_radix(
            payload_offset_hex.trim_start_matches('0'),
            16,
        )
        .unwrap_or(96);

        // Word 2 (128..192): nonce (uint64)
        let nonce_hex = &data[128..192];
        let nonce = u64::from_str_radix(nonce_hex.trim_start_matches('0'), 16).unwrap_or(0);

        // Payload at offset (in bytes, so offset*2 in hex chars from start of data)
        let payload_start = payload_offset * 2;
        if data.len() < payload_start + 64 {
            warn!("EVM MessageSent data too short for payload at offset {}", payload_offset);
            return None;
        }

        // Payload length
        let payload_len_hex = &data[payload_start..payload_start + 64];
        let payload_len = usize::from_str_radix(
            payload_len_hex.trim_start_matches('0'),
            16,
        )
        .unwrap_or(0);

        // Payload data
        let payload_data_start = payload_start + 64;
        let payload_data_end = payload_data_start + payload_len * 2;
        let payload = if payload_len > 0 && data.len() >= payload_data_end {
            format!("0x{}", &data[payload_data_start..payload_data_end])
        } else {
            "0x".to_string()
        };

        // Source address: use the GMP endpoint contract address (padded to 32 bytes)
        let gmp_addr = self
            .config
            .evm_gmp_endpoint_addr
            .as_ref()
            .map(|a| {
                let clean = a.strip_prefix("0x").unwrap_or(a).to_lowercase();
                format!("0x{:0>64}", clean)
            })
            .unwrap_or_else(|| "0x".to_string());

        Some(GmpMessage {
            src_chain_id: evm_chain_id,
            remote_gmp_endpoint_addr: gmp_addr,
            dst_chain_id,
            dst_addr,
            payload,
            nonce,
        })
    }

    /// Send a locally-signed EVM transaction via eth_sendRawTransaction.
    ///
    /// Builds a legacy (pre-EIP-1559) transaction, signs it with the relay's ECDSA key,
    /// and broadcasts via eth_sendRawTransaction. Works with public RPCs that don't hold keys.
    async fn evm_send_raw_transaction(
        &self,
        rpc_url: &str,
        from: &str,
        to: &str,
        calldata: &str,
        chain_id: u32,
    ) -> Result<String> {
        // 1. Fetch nonce
        let nonce_hex: String = self
            .evm_json_rpc(
                rpc_url,
                "eth_getTransactionCount",
                vec![serde_json::json!(from), serde_json::json!("pending")],
            )
            .await
            .context("eth_getTransactionCount failed")?;
        let nonce = u64::from_str_radix(
            nonce_hex.strip_prefix("0x").unwrap_or(&nonce_hex),
            16,
        )
        .context("Failed to parse nonce")?;

        // 2. Fetch gas price
        let gas_price_hex: String = self
            .evm_json_rpc(rpc_url, "eth_gasPrice", vec![])
            .await
            .context("eth_gasPrice failed")?;
        let gas_price = u64::from_str_radix(
            gas_price_hex.strip_prefix("0x").unwrap_or(&gas_price_hex),
            16,
        )
        .context("Failed to parse gas price")?;

        let gas_limit: u64 = 2_000_000;

        // Decode `to` address (20 bytes)
        let to_hex = to.strip_prefix("0x").unwrap_or(to);
        let to_bytes =
            hex::decode(to_hex).context("Failed to decode EVM 'to' address")?;

        // Decode calldata
        let calldata_hex = calldata.strip_prefix("0x").unwrap_or(calldata);
        let data_bytes =
            hex::decode(calldata_hex).context("Failed to decode EVM calldata")?;

        // 3. RLP-encode unsigned tx for EIP-155 signing:
        //    [nonce, gasPrice, gasLimit, to, value, data, chainId, 0, 0]
        let unsigned_items: Vec<Vec<u8>> = vec![
            rlp_encode_u64(nonce),
            rlp_encode_u64(gas_price),
            rlp_encode_u64(gas_limit),
            to_bytes.clone(),
            vec![], // value = 0
            data_bytes.clone(),
            rlp_encode_u64(chain_id as u64),
            vec![], // 0
            vec![], // 0
        ];
        let unsigned_rlp = rlp_encode_list(&unsigned_items);

        // 4. Keccak256 hash
        let mut hasher = Keccak256::new();
        hasher.update(&unsigned_rlp);
        let tx_hash: [u8; 32] = hasher.finalize().into();

        // 5. Sign with ECDSA key
        let (r, s, recovery_id) = self
            .crypto_service
            .sign_evm_transaction_hash(&tx_hash)
            .context("Failed to sign EVM transaction")?;

        // 6. Compute EIP-155 v value: recovery_id + chainId * 2 + 35
        let v = (recovery_id as u64) + (chain_id as u64) * 2 + 35;

        // 7. Build signed tx RLP: [nonce, gasPrice, gasLimit, to, value, data, v, r, s]
        let signed_items: Vec<Vec<u8>> = vec![
            rlp_encode_u64(nonce),
            rlp_encode_u64(gas_price),
            rlp_encode_u64(gas_limit),
            to_bytes,
            vec![], // value = 0
            data_bytes,
            rlp_encode_u64(v),
            r.to_vec(),
            s.to_vec(),
        ];
        let signed_rlp = rlp_encode_list(&signed_items);
        let raw_tx = format!("0x{}", hex::encode(&signed_rlp));

        debug!(
            "EVM raw tx: nonce={}, gas_price={}, chain_id={}, from={}",
            nonce, gas_price, chain_id, from
        );

        // 8. Send via eth_sendRawTransaction
        let tx_hash_result: String = self
            .evm_json_rpc(
                rpc_url,
                "eth_sendRawTransaction",
                vec![serde_json::json!(raw_tx)],
            )
            .await
            .context("eth_sendRawTransaction failed")?;

        Ok(tx_hash_result)
    }

    /// Wait for an EVM transaction receipt and verify success.
    async fn evm_wait_for_receipt(&self, rpc_url: &str, tx_hash: &str) -> Result<()> {
        for _ in 0..30 {
            let receipt: Option<serde_json::Value> = self
                .evm_json_rpc(
                    rpc_url,
                    "eth_getTransactionReceipt",
                    vec![serde_json::json!(tx_hash)],
                )
                .await?;

            if let Some(receipt) = receipt {
                let status = receipt
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("0x0");
                if status == "0x1" {
                    return Ok(());
                } else {
                    anyhow::bail!(
                        "EVM transaction {} failed with status: {}",
                        tx_hash,
                        status
                    );
                }
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        anyhow::bail!("Timed out waiting for EVM transaction receipt: {}", tx_hash)
    }

    /// Check if a message was already delivered on the EVM GMP endpoint.
    ///
    /// Calls `isMessageDelivered(bytes32 intentId, uint8 msgType)` view function.
    async fn evm_is_message_delivered(
        &self,
        rpc_url: &str,
        gmp_endpoint: &str,
        intent_id: &[u8],
        msg_type: u8,
    ) -> Result<bool> {
        // Function selector: keccak256("isMessageDelivered(bytes32,uint8)")[0..4]
        let selector = &Keccak256::digest(b"isMessageDelivered(bytes32,uint8)")[..4];

        // ABI-encode: selector + intentId (32 bytes) + msgType (uint8 padded to 32 bytes)
        let mut calldata = Vec::with_capacity(68);
        calldata.extend_from_slice(selector);

        // intentId as bytes32 (left-pad if needed)
        let mut intent_id_padded = [0u8; 32];
        let len = intent_id.len().min(32);
        intent_id_padded[..len].copy_from_slice(&intent_id[..len]);
        calldata.extend_from_slice(&intent_id_padded);

        // msgType as uint8 padded to 32 bytes
        let mut msg_type_padded = [0u8; 32];
        msg_type_padded[31] = msg_type;
        calldata.extend_from_slice(&msg_type_padded);

        let data_hex = format!("0x{}", hex::encode(&calldata));

        let result: String = self
            .evm_json_rpc(
                rpc_url,
                "eth_call",
                vec![
                    serde_json::json!({
                        "to": gmp_endpoint,
                        "data": data_hex,
                    }),
                    serde_json::json!("latest"),
                ],
            )
            .await
            .context("Failed to call isMessageDelivered on EVM")?;

        // Result is ABI-encoded bool: 32 bytes, last byte is 0 or 1
        let clean = result.strip_prefix("0x").unwrap_or(&result);
        Ok(clean.ends_with('1'))
    }

    /// Get the current EVM block number.
    async fn evm_get_block_number(&self, rpc_url: &str) -> Result<u64> {
        let block_hex: String = self
            .evm_json_rpc(rpc_url, "eth_blockNumber", vec![])
            .await
            .context("eth_blockNumber failed")?;

        let clean = block_hex.strip_prefix("0x").unwrap_or(&block_hex);
        u64::from_str_radix(clean, 16).context("Failed to parse block number")
    }

    /// Generic EVM JSON-RPC call helper.
    async fn evm_json_rpc<T: serde::de::DeserializeOwned>(
        &self,
        rpc_url: &str,
        method: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<T> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1,
        });

        let rpc_future = async {
            let resp = self
                .http_client
                .post(rpc_url)
                .json(&request)
                .send()
                .await
                .with_context(|| format!("Failed to send {} request to {}", method, rpc_url))?;
            resp.json::<serde_json::Value>()
                .await
                .with_context(|| format!("Failed to parse {} response from {}", method, rpc_url))
        };

        let response: serde_json::Value = tokio::time::timeout(Duration::from_secs(15), rpc_future)
            .await
            .map_err(|_| anyhow::anyhow!("Timed out after 15s waiting for {} from {}", method, rpc_url))??;

        if let Some(error) = response.get("error") {
            let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
            let message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            anyhow::bail!(
                "JSON-RPC error from {} ({}): {} (code: {})",
                rpc_url,
                method,
                message,
                code
            );
        }

        let result = response
            .get("result")
            .ok_or_else(|| anyhow::anyhow!("No result in {} response", method))?;

        serde_json::from_value(result.clone())
            .with_context(|| format!("Failed to deserialize {} result", method))
    }

    /// Deliver message to SVM chain via integrated-gmp-endpoint DeliverMessage instruction.
    ///
    /// Builds and submits a DeliverMessage transaction to the SVM integrated-gmp-endpoint program.
    /// For IntentRequirements messages (0x01), also derives and passes the outflow-validator
    /// accounts needed for GmpReceive CPI.
    async fn deliver_to_svm(&self, message: &GmpMessage) -> Result<()> {
        let Some(ref rpc_url) = self.config.svm_rpc_url else {
            return Err(anyhow::anyhow!("SVM not configured"));
        };

        let Some(ref program_id_str) = self.config.svm_gmp_program_id else {
            return Err(anyhow::anyhow!("SVM GMP program ID not configured"));
        };

        info!(
            "Delivering message to SVM: dst_chain={}, nonce={}",
            message.dst_chain_id, message.nonce
        );

        // Parse program ID (integrated-gmp-endpoint)
        let program_id = Pubkey::from_str(program_id_str)
            .context("Invalid SVM GMP program ID")?;

        // Load relay keypair from operator private key (base64 Ed25519 -> Solana keypair)
        let relay_keypair = self.load_svm_keypair()?;
        let relay_pubkey = relay_keypair.pubkey();

        // Parse remote GMP endpoint address (32 bytes)
        let remote_gmp_endpoint_addr = parse_32_byte_address(&message.remote_gmp_endpoint_addr)?;

        // Parse destination address (the receiving program on SVM - e.g., outflow-validator)
        let dst_program = parse_svm_pubkey(&message.dst_addr)?;

        // Parse payload
        let payload = hex_to_bytes(&message.payload)?;

        // Derive GMP endpoint PDAs
        let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &program_id);
        let (relay_pda, _) =
            Pubkey::find_program_address(&[b"relay", relay_pubkey.as_ref()], &program_id);
        let (remote_gmp_endpoint_pda, _) = Pubkey::find_program_address(
            &[b"remote_gmp_endpoint", &message.src_chain_id.to_le_bytes()],
            &program_id,
        );
        // Derive delivered message PDA from payload (intent_id + msg_type)
        // All GMP messages: msg_type (1 byte) + intent_id (32 bytes) at the start
        if payload.len() < 33 {
            return Err(anyhow::anyhow!("Payload too short to extract intent_id for dedup PDA"));
        }
        let msg_type = payload[0];
        let intent_id = &payload[1..33];
        let (delivered_pda, _) = Pubkey::find_program_address(
            &[b"delivered", intent_id, &[msg_type]],
            &program_id,
        );
        let (routing_pda, _) = Pubkey::find_program_address(&[b"routing"], &program_id);

        // Check if message was already delivered (delivered PDA already exists on-chain)
        let rpc_client_check = RpcClient::new_with_commitment(
            rpc_url.clone(),
            CommitmentConfig::confirmed(),
        );
        if rpc_client_check.get_account(&delivered_pda).is_ok() {
            info!(
                "SVM: message already delivered (nonce={}, msg_type=0x{:02x}), skipping",
                message.nonce, msg_type
            );
            return Ok(());
        }

        // Get outflow_validator program for destination_program_1 (required for routing IntentRequirements)
        let outflow_program = if let Some(ref outflow_id) = self.config.svm_outflow_program_id {
            Pubkey::from_str(outflow_id).context("Invalid SVM outflow program ID")?
        } else {
            // If no outflow configured, use dst_program as placeholder (routing won't be used)
            dst_program
        };

        // Get intent_escrow program for destination_program_2 (required for routing)
        let escrow_program = if let Some(ref escrow_id) = self.config.svm_escrow_program_id {
            Pubkey::from_str(escrow_id).context("Invalid SVM escrow program ID")?
        } else {
            // If no escrow configured, use dst_program as placeholder (routing won't be used)
            dst_program
        };

        // Build base accounts for DeliverMessage
        // Account order (updated for intent_id-based dedup):
        // 0. Config, 1. Relay, 2. RemoteGmpEndpoint, 3. DeliveredMessage, 4. RelaySigner, 5. Payer
        // Track if we need to create an ATA before delivering the message (for FulfillmentProof)
        // Tuple: (ata, owner, mint, token_program, associated_token_program)
        #[allow(clippy::type_complexity)]
        let mut ata_create_info: Option<(Pubkey, Pubkey, Pubkey, Pubkey, Pubkey)> = None;

        // 6. SystemProgram, 7. RoutingConfig, 8. DestProgram1, 9. DestProgram2, 10+. Remaining
        let mut accounts = vec![
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(relay_pda, false),
            AccountMeta::new_readonly(remote_gmp_endpoint_pda, false),
            AccountMeta::new(delivered_pda, false),
            AccountMeta::new_readonly(relay_pubkey, true), // signer
            AccountMeta::new(relay_pubkey, true),          // payer (signer)
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(routing_pda, false), // routing config (may not exist)
            AccountMeta::new_readonly(outflow_program, false), // destination program 1 (outflow_validator)
            AccountMeta::new_readonly(escrow_program, false), // destination program 2 (intent_escrow)
        ];

        // For IntentRequirements (0x01), add accounts for both destination programs' GmpReceive CPI.
        // The GMP endpoint routes to BOTH outflow_validator AND intent_escrow when routing is configured.
        //
        // Account layout for remaining_accounts (passed to GMP endpoint after base accounts):
        // Indices 0-4: outflow_validator's GmpReceive accounts
        // Indices 5-9: intent_escrow's GmpReceive accounts
        //
        // Each program's GmpReceive expects: requirements(w), config(r), authority(s), payer(s,w), system_program
        if !payload.is_empty() && payload[0] == 0x01 {
            // IntentRequirements format: [type(1)] [intent_id(32)] [...]
            if payload.len() >= 33 {
                let mut intent_id = [0u8; 32];
                intent_id.copy_from_slice(&payload[1..33]);

                // Derive outflow-validator PDAs
                let (outflow_requirements_pda, _) = Pubkey::find_program_address(
                    &[b"requirements", &intent_id],
                    &outflow_program,
                );
                let (outflow_config_pda, _) = Pubkey::find_program_address(
                    &[b"config"],
                    &outflow_program,
                );

                // Derive intent_escrow PDAs (escrow_program is the intent_escrow)
                let (escrow_requirements_pda, _) = Pubkey::find_program_address(
                    &[b"requirements", &intent_id],
                    &escrow_program,
                );
                let (escrow_gmp_config_pda, _) = Pubkey::find_program_address(
                    &[b"gmp_config"],
                    &escrow_program,
                );

                debug!(
                    "Adding accounts for multi-destination GmpReceive CPI: outflow_req={}, outflow_cfg={}, escrow_req={}, escrow_cfg={}",
                    outflow_requirements_pda, outflow_config_pda, escrow_requirements_pda, escrow_gmp_config_pda
                );

                // Accounts for outflow_validator's GmpReceive (indices 0-4)
                // GmpReceive expects: requirements(w), config(r), authority(s), payer(s,w), system_program
                accounts.push(AccountMeta::new(outflow_requirements_pda, false));  // 0
                accounts.push(AccountMeta::new_readonly(outflow_config_pda, false)); // 1
                accounts.push(AccountMeta::new_readonly(relay_pubkey, true));  // 2: authority (signer)
                accounts.push(AccountMeta::new(relay_pubkey, true));           // 3: payer (signer)
                accounts.push(AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)); // 4

                // Accounts for intent_escrow's GmpReceive (indices 5-9)
                // GmpReceive expects: requirements(w), gmp_config(r), authority(s), payer(s,w), system_program
                accounts.push(AccountMeta::new(escrow_requirements_pda, false));  // 5
                accounts.push(AccountMeta::new_readonly(escrow_gmp_config_pda, false)); // 6
                accounts.push(AccountMeta::new_readonly(relay_pubkey, true));  // 7: authority (signer)
                accounts.push(AccountMeta::new(relay_pubkey, true));           // 8: payer (signer)
                accounts.push(AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)); // 9
            }
        } else if !payload.is_empty() && payload[0] == 0x03 {
            // FulfillmentProof (0x03) - route to intent_escrow only
            // Payload format: [type(1)] [intent_id(32)] [solver_addr(32)] [amount(8)] [timestamp(8)]
            if payload.len() >= 65 {
                let mut intent_id = [0u8; 32];
                intent_id.copy_from_slice(&payload[1..33]);

                let mut solver_addr = [0u8; 32];
                solver_addr.copy_from_slice(&payload[33..65]);

                // Derive intent_escrow PDAs
                let (escrow_requirements_pda, _) = Pubkey::find_program_address(
                    &[b"requirements", &intent_id],
                    &escrow_program,
                );
                let (escrow_pda, _) = Pubkey::find_program_address(
                    &[b"escrow", &intent_id],
                    &escrow_program,
                );
                let (vault_pda, _) = Pubkey::find_program_address(
                    &[b"vault", &intent_id],
                    &escrow_program,
                );
                let (escrow_gmp_config_pda, _) = Pubkey::find_program_address(
                    &[b"gmp_config"],
                    &escrow_program,
                );

                // Read requirements account to get token_addr (mint)
                let rpc_client_for_read = RpcClient::new_with_commitment(
                    rpc_url.clone(),
                    CommitmentConfig::confirmed(),
                );
                let requirements_data = rpc_client_for_read
                    .get_account_data(&escrow_requirements_pda)
                    .context("Failed to read requirements account for FulfillmentProof")?;

                // Parse token_addr from StoredIntentRequirements
                // Layout: discriminator(8) + intent_id(32) + requester_addr(32) + amount_required(8) + token_addr(32)
                // token_addr starts at offset 80
                if requirements_data.len() < 112 {
                    return Err(anyhow::anyhow!(
                        "Requirements account too small: {} bytes",
                        requirements_data.len()
                    ));
                }
                let mut token_mint_bytes = [0u8; 32];
                token_mint_bytes.copy_from_slice(&requirements_data[80..112]);
                let token_mint = Pubkey::new_from_array(token_mint_bytes);

                // Derive solver's ATA manually (PDA derivation)
                // ATA = PDA([owner, TOKEN_PROGRAM_ID, mint], ASSOCIATED_TOKEN_PROGRAM_ID)
                let solver_pubkey = Pubkey::new_from_array(solver_addr);
                let token_program_id = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
                    .expect("Invalid token program ID");
                let associated_token_program_id = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
                    .expect("Invalid associated token program ID");
                let (solver_ata, _) = Pubkey::find_program_address(
                    &[
                        solver_pubkey.as_ref(),
                        token_program_id.as_ref(),
                        token_mint.as_ref(),
                    ],
                    &associated_token_program_id,
                );

                debug!(
                    "FulfillmentProof accounts: requirements={}, escrow={}, vault={}, solver_ata={}, gmp_config={}, token_mint={}",
                    escrow_requirements_pda, escrow_pda, vault_pda, solver_ata, escrow_gmp_config_pda, token_mint
                );

                // Store ATA creation info for use when building transaction
                ata_create_info = Some((solver_ata, solver_pubkey, token_mint, token_program_id, associated_token_program_id));

                // Accounts for intent_escrow's GmpReceiveFulfillmentProof
                // Expected: requirements(w), escrow(w), vault(w), solver_token(w), gmp_config(r), gmp_caller(s), token_program
                accounts.push(AccountMeta::new(escrow_requirements_pda, false));     // 0: requirements (writable)
                accounts.push(AccountMeta::new(escrow_pda, false));                  // 1: escrow (writable)
                accounts.push(AccountMeta::new(vault_pda, false));                   // 2: vault (writable)
                accounts.push(AccountMeta::new(solver_ata, false));                  // 3: solver_token (writable)
                accounts.push(AccountMeta::new_readonly(escrow_gmp_config_pda, false)); // 4: gmp_config
                accounts.push(AccountMeta::new_readonly(relay_pubkey, true));        // 5: gmp_caller (signer)
                accounts.push(AccountMeta::new_readonly(token_program_id, false));   // 6: token_program
            }
        }

        // Build DeliverMessage instruction
        let instruction_data = SvmDeliverMessageInstruction {
            src_chain_id: message.src_chain_id,
            remote_gmp_endpoint_addr,
            payload,
        };

        let deliver_instruction = Instruction {
            program_id,
            accounts,
            data: instruction_data
                .try_to_vec()
                .context("Failed to serialize DeliverMessage instruction")?,
        };

        // Build instructions list - may include ATA creation for FulfillmentProof
        let mut instructions = Vec::new();

        // If we need to create an ATA (for FulfillmentProof), add that instruction first
        if let Some((ata, owner, mint, token_program, ata_program)) = ata_create_info {
            // Build create_associated_token_account_idempotent instruction manually
            // Instruction data: [1] for idempotent create
            // Accounts: payer(s,w), ata(w), owner(r), mint(r), system_program(r), token_program(r)
            let create_ata_ix = Instruction {
                program_id: ata_program,
                accounts: vec![
                    AccountMeta::new(relay_pubkey, true),         // payer (signer, writable)
                    AccountMeta::new(ata, false),                 // associated token account (writable)
                    AccountMeta::new_readonly(owner, false),      // wallet owner
                    AccountMeta::new_readonly(mint, false),       // token mint
                    AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false), // system program
                    AccountMeta::new_readonly(token_program, false), // token program
                ],
                data: vec![1], // 1 = create_idempotent
            };
            debug!(
                "Adding create_associated_token_account_idempotent instruction: ata={}, owner={}, mint={}",
                ata, owner, mint
            );
            instructions.push(create_ata_ix);
        }

        instructions.push(deliver_instruction);

        // Create RPC client and submit transaction
        let rpc_client = RpcClient::new_with_commitment(
            rpc_url.clone(),
            CommitmentConfig::confirmed(),
        );

        let blockhash = rpc_client
            .get_latest_blockhash()
            .context("Failed to get latest blockhash")?;

        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&relay_pubkey),
            &[&relay_keypair],
            blockhash,
        );

        let signature = rpc_client
            .send_and_confirm_transaction(&transaction)
            .map_err(|e| {
                error!(
                    "SVM DeliverMessage failed: {}. Accounts: config={}, relay={}, remote_gmp_endpoint={}, delivered={}, dst_program={}",
                    e, config_pda, relay_pda, remote_gmp_endpoint_pda, delivered_pda, dst_program
                );
                e
            })
            .context("Failed to submit SVM DeliverMessage transaction")?;

        info!(
            "SVM deliver_message submitted successfully: nonce={}, signature={}",
            message.nonce, signature
        );

        Ok(())
    }

    /// Load the relay keypair for SVM from the operator private key.
    ///
    /// The operator private key is a base64-encoded Ed25519 seed (32 bytes).
    /// For Solana, we expand this to a 64-byte keypair.
    fn load_svm_keypair(&self) -> Result<Keypair> {
        let seed_bytes = STANDARD
            .decode(&self.config.operator_private_key)
            .context("Failed to decode base64 private key")?;

        if seed_bytes.len() != 32 {
            anyhow::bail!(
                "Invalid private key length: expected 32 bytes, got {}",
                seed_bytes.len()
            );
        }

        // Create Solana keypair from Ed25519 seed
        let keypair_bytes = ed25519_seed_to_keypair_bytes(&seed_bytes)?;
        let keypair = Keypair::try_from(keypair_bytes.as_slice())
            .context("Failed to create Solana keypair")?;

        Ok(keypair)
    }
}

// ============================================================================
// SVM INSTRUCTION TYPES
// ============================================================================

/// SVM DeliverMessage instruction data (matches integrated-gmp-endpoint program).
///
/// This is the 6th variant (index 6) in the NativeGmpInstruction enum.
/// Deduplication uses (intent_id, msg_type) from the payload — no nonce needed.
#[derive(BorshSerialize)]
struct SvmDeliverMessageInstruction {
    src_chain_id: u32,
    remote_gmp_endpoint_addr: [u8; 32],
    payload: Vec<u8>,
}

impl SvmDeliverMessageInstruction {
    fn try_to_vec(&self) -> Result<Vec<u8>> {
        // Instruction discriminator: DeliverMessage is variant 6 in the enum
        // (Initialize=0, AddRelay=1, RemoveRelay=2, SetRemoteGmpEndpointAddr=3, SetRouting=4, Send=5, DeliverMessage=6)
        let mut data = vec![6u8];
        data.extend(
            borsh::to_vec(self).context("Failed to serialize instruction data")?,
        );
        Ok(data)
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Parse Move `vector<u8>` from a view function result into a hex string (no 0x prefix).
///
/// Handles two formats returned by Aptos view functions:
/// - Hex string: `"0x3c44cddd..."` -> `"3c44cddd..."`
/// - JSON byte array: `["60", "68", ...]` -> hex encoded
pub fn parse_view_bytes(value: &serde_json::Value) -> Result<String> {
    if let Some(hex_str) = value.as_str() {
        // Hex string format: "0x..."
        Ok(hex_str.strip_prefix("0x").unwrap_or(hex_str).to_string())
    } else if let Some(arr) = value.as_array() {
        // JSON byte array format: ["60", "68", ...]
        let mut bytes = Vec::with_capacity(arr.len());
        for elem in arr {
            let byte: u8 = elem
                .as_str()
                .and_then(|s| s.parse().ok())
                .or_else(|| elem.as_u64().map(|n| n as u8))
                .context("Invalid byte in view function result")?;
            bytes.push(byte);
        }
        Ok(hex::encode(bytes))
    } else {
        anyhow::bail!("Unexpected view function bytes format: {:?}", value)
    }
}

/// Convert array of byte strings (e.g., ["60", "68"]) to hex string with 0x prefix.
#[allow(dead_code)]
pub fn bytes_array_to_hex(bytes: &[String]) -> Result<String> {
    let mut result = Vec::with_capacity(bytes.len());
    for byte_str in bytes {
        let byte: u8 = byte_str.parse().context("Invalid byte value")?;
        result.push(byte);
    }
    Ok(format!("0x{}", hex::encode(result)))
}

/// Convert hex string (with or without 0x prefix) to bytes.
pub fn hex_to_bytes(hex_str: &str) -> Result<Vec<u8>> {
    let hex_clean = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    hex::decode(hex_clean).context("Invalid hex string")
}

/// Normalize address to have 0x prefix.
pub fn normalize_address(addr: &str) -> String {
    if addr.starts_with("0x") {
        addr.to_string()
    } else {
        format!("0x{}", addr)
    }
}

/// Check if the VM execution succeeded by parsing the CLI JSON output.
///
/// The Aptos CLI returns JSON with `"success": true/false` and `"vm_status"` fields.
/// CLI exit code 0 alone doesn't guarantee VM execution success on all networks.
pub fn check_vm_status_success(output: &str) -> bool {
    // Try to parse as JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        // Check "Result"."success" (standard Aptos CLI format)
        if let Some(result) = json.get("Result") {
            if let Some(success) = result.get("success") {
                return success.as_bool().unwrap_or(false);
            }
        }
        // Check top-level "success"
        if let Some(success) = json.get("success") {
            return success.as_bool().unwrap_or(false);
        }
    }

    // If we can't parse JSON, check for string indicators
    if output.contains("\"success\": true") || output.contains("\"success\":true") {
        return true;
    }
    if output.contains("\"success\": false") || output.contains("\"success\":false") {
        return false;
    }

    // If no success field found at all, assume CLI exit code was sufficient
    // (conservative: don't break on unknown output formats)
    true
}

/// Extract transaction hash from CLI output (handles both traditional and JSON formats).
///
/// This matches the pattern used by solver's hub.rs for consistency.
pub fn extract_transaction_hash(output: &str) -> Option<String> {
    // Try JSON format first: "transaction_hash": "0x..."
    if let Some(start) = output.find("\"transaction_hash\"") {
        let after_key = &output[start..];
        if let Some(colon_pos) = after_key.find(':') {
            let value_part = &after_key[colon_pos + 1..];
            if let Some(quote_start) = value_part.find('"') {
                let after_quote = &value_part[quote_start + 1..];
                if let Some(quote_end) = after_quote.find('"') {
                    let hash = &after_quote[..quote_end];
                    if hash.starts_with("0x") {
                        return Some(hash.to_string());
                    }
                }
            }
        }
    }

    // Fall back to traditional CLI format: "Transaction hash: 0x..."
    for line in output.lines() {
        if line.contains("hash") || line.contains("Hash") {
            if let Some(hash) = line.split_whitespace().find(|s| s.starts_with("0x")) {
                return Some(hash.to_string());
            }
        }
    }

    None
}

/// Parse a hex address into a 32-byte array.
///
/// Handles Move VM addresses that may have leading zeros stripped.
/// Left-pads short addresses to ensure exactly 32 bytes.
pub fn parse_32_byte_address(addr: &str) -> Result<[u8; 32]> {
    let hex_clean = addr.strip_prefix("0x").unwrap_or(addr);
    // Left-pad to 64 hex chars (32 bytes) to handle addresses with stripped leading zeros
    let padded = format!("{:0>64}", hex_clean);
    let bytes = hex::decode(&padded).context("Invalid hex address")?;
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(array)
}

/// Parse a Solana pubkey from hex (with 0x prefix) or base58.
pub fn parse_svm_pubkey(addr: &str) -> Result<Pubkey> {
    if addr.starts_with("0x") {
        let bytes = parse_32_byte_address(addr)?;
        Ok(Pubkey::new_from_array(bytes))
    } else {
        Pubkey::from_str(addr).context("Invalid base58 pubkey")
    }
}

/// Convert a 32-byte Ed25519 seed to a 64-byte Solana keypair format.
///
/// Solana keypairs are 64 bytes: 32-byte seed + 32-byte public key.
pub fn ed25519_seed_to_keypair_bytes(seed: &[u8]) -> Result<[u8; 64]> {
    if seed.len() != 32 {
        anyhow::bail!("Invalid seed length: expected 32, got {}", seed.len());
    }

    let mut seed_array = [0u8; 32];
    seed_array.copy_from_slice(seed);

    let signing_key = SigningKey::from_bytes(&seed_array);
    let verifying_key = signing_key.verifying_key();

    let mut keypair_bytes = [0u8; 64];
    keypair_bytes[..32].copy_from_slice(&seed_array);
    keypair_bytes[32..].copy_from_slice(verifying_key.as_bytes());

    Ok(keypair_bytes)
}

// ============================================================================
// EVM ABI ENCODING HELPERS
// ============================================================================

/// Compute keccak256 event topic hash for an event signature string.
fn evm_event_topic(signature: &str) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(signature.as_bytes());
    format!("0x{}", hex::encode(hasher.finalize()))
}

/// ABI-encode a call to `deliverMessage(uint32,bytes32,bytes)`.
///
/// Returns the full calldata (selector + encoded params) as a hex string with 0x prefix.
fn evm_encode_deliver_message(
    src_chain_id: u32,
    remote_gmp_endpoint_addr: &str,
    payload: &str,
) -> Result<String> {
    // Function selector: keccak256("deliverMessage(uint32,bytes32,bytes)")[0:4]
    let mut hasher = Keccak256::new();
    hasher.update(b"deliverMessage(uint32,bytes32,bytes)");
    let hash = hasher.finalize();
    let selector = &hash[..4];

    // Parse remote_gmp_endpoint_addr to 32 bytes
    let remote_gmp_endpoint_addr_bytes = parse_32_byte_address(remote_gmp_endpoint_addr)?;

    // Parse payload to bytes
    let payload_bytes = hex_to_bytes(payload)?;

    // ABI encoding: (uint32, bytes32, bytes)
    // Word 0: srcChainId (uint32, left-padded to 32 bytes)
    // Word 1: remoteGmpEndpointAddr (bytes32)
    // Word 2: offset to payload (= 96 = 3 * 32, after the 3 head words)
    // Word 3: payload length
    // Word 4+: payload data (right-padded to 32-byte boundary)
    let mut data = Vec::new();

    // Selector
    data.extend_from_slice(selector);

    // Word 0: srcChainId
    let mut word = [0u8; 32];
    word[28..32].copy_from_slice(&src_chain_id.to_be_bytes());
    data.extend_from_slice(&word);

    // Word 1: remoteGmpEndpointAddr
    data.extend_from_slice(&remote_gmp_endpoint_addr_bytes);

    // Word 2: offset to payload (96 = 0x60, after 3 head words)
    let mut word = [0u8; 32];
    word[31] = 96;
    data.extend_from_slice(&word);

    // Dynamic section: payload
    // Length
    let mut word = [0u8; 32];
    let payload_len = payload_bytes.len() as u64;
    word[24..32].copy_from_slice(&payload_len.to_be_bytes());
    data.extend_from_slice(&word);

    // Payload data (right-padded to 32-byte boundary)
    data.extend_from_slice(&payload_bytes);
    let padding = (32 - (payload_bytes.len() % 32)) % 32;
    data.extend(std::iter::repeat(0u8).take(padding));

    Ok(format!("0x{}", hex::encode(data)))
}

// ============================================================================
// RLP ENCODING HELPERS (for legacy EVM transactions)
// ============================================================================

/// Encode a u64 as big-endian bytes with no leading zeros (RLP integer format).
fn rlp_encode_u64(val: u64) -> Vec<u8> {
    if val == 0 {
        return vec![];
    }
    let bytes = val.to_be_bytes();
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(8);
    bytes[start..].to_vec()
}

/// RLP-encode a single byte-string item.
fn rlp_encode_item(data: &[u8]) -> Vec<u8> {
    if data.len() == 1 && data[0] < 0x80 {
        // Single byte below 0x80: encoded as itself
        vec![data[0]]
    } else if data.is_empty() {
        // Empty bytes: 0x80
        vec![0x80]
    } else if data.len() <= 55 {
        let mut out = vec![0x80 + data.len() as u8];
        out.extend_from_slice(data);
        out
    } else {
        let len_bytes = rlp_encode_u64(data.len() as u64);
        let mut out = vec![0xb7 + len_bytes.len() as u8];
        out.extend_from_slice(&len_bytes);
        out.extend_from_slice(data);
        out
    }
}

/// RLP-encode a list of items (each item is already raw bytes, NOT RLP-encoded).
fn rlp_encode_list(items: &[Vec<u8>]) -> Vec<u8> {
    let mut payload = Vec::new();
    for item in items {
        payload.extend(rlp_encode_item(item));
    }

    if payload.len() <= 55 {
        let mut out = vec![0xc0 + payload.len() as u8];
        out.extend(payload);
        out
    } else {
        let len_bytes = rlp_encode_u64(payload.len() as u64);
        let mut out = vec![0xf7 + len_bytes.len() as u8];
        out.extend_from_slice(&len_bytes);
        out.extend(payload);
        out
    }
}

