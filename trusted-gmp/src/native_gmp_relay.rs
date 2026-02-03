//! Native GMP Relay Module
//!
//! Watches for `MessageSent` events on MVM and SVM native GMP endpoints
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
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_sdk_ids::system_program;
use std::collections::HashSet;
use std::process::Command;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::mvm_client::MvmClient;
use crate::svm_client::SvmClient;

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Configuration for the native GMP relay.
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
    /// SVM native GMP endpoint program ID (optional)
    pub svm_gmp_program_id: Option<String>,
    /// SVM chain ID (optional)
    pub svm_chain_id: Option<u32>,
    /// Polling interval in milliseconds
    pub polling_interval_ms: u64,
    /// Relay operator private key (base64 encoded Ed25519)
    pub operator_private_key: String,
}

impl NativeGmpRelayConfig {
    /// Create relay config from main config.
    pub fn from_config(config: &Config) -> Result<Self> {
        let operator_private_key = config.trusted_gmp.get_private_key()?;

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
        let (svm_rpc_url, svm_gmp_program_id, svm_chain_id) =
            if let Some(ref svm_config) = config.connected_chain_svm {
                (
                    Some(svm_config.rpc_url.clone()),
                    Some(svm_config.escrow_program_id.clone()),
                    Some(svm_config.chain_id as u32),
                )
            } else {
                (None, None, None)
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
            svm_chain_id,
            polling_interval_ms: config.trusted_gmp.polling_interval_ms,
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
    /// Source address (32 bytes, hex encoded with 0x prefix)
    pub src_addr: String,
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
    /// Last processed MVM event sequence number
    mvm_last_seq: u64,
    /// Last processed SVM signature (for pagination)
    svm_last_signature: Option<String>,
    /// Processed SVM signatures to avoid reprocessing
    svm_processed_signatures: HashSet<String>,
}

// ============================================================================
// NATIVE GMP RELAY
// ============================================================================

/// Native GMP relay service that watches for MessageSent events
/// and delivers messages to destination chains.
pub struct NativeGmpRelay {
    config: NativeGmpRelayConfig,
    mvm_client: MvmClient,
    mvm_connected_client: Option<MvmClient>,
    svm_client: Option<SvmClient>,
    http_client: Client,
    state: Arc<RwLock<RelayState>>,
}

impl NativeGmpRelay {
    /// Create a new native GMP relay.
    pub fn new(config: NativeGmpRelayConfig) -> Result<Self> {
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
            .timeout(Duration::from_secs(30))
            .no_proxy()
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            config,
            mvm_client,
            mvm_connected_client,
            svm_client,
            http_client,
            state: Arc::new(RwLock::new(RelayState::default())),
        })
    }

    /// Start the relay service (blocking).
    pub async fn run(&self) -> Result<()> {
        info!(
            "Starting native GMP relay: MVM hub chain_id={}, polling_interval={}ms",
            self.config.mvm_chain_id, self.config.polling_interval_ms
        );

        if let Some(ref mvm_connected_chain_id) = self.config.mvm_connected_chain_id {
            info!("MVM connected chain configured: chain_id={}", mvm_connected_chain_id);
        }

        if let Some(ref svm_chain_id) = self.config.svm_chain_id {
            info!("SVM chain configured: chain_id={}", svm_chain_id);
        }

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

            // Poll SVM for MessageSent events (if configured)
            if self.config.svm_rpc_url.is_some() {
                if let Err(e) = self.poll_svm_events().await {
                    error!("Error polling SVM events: {}", e);
                }
            }

            tokio::time::sleep(interval).await;
        }
    }

    /// Poll MVM for MessageSent events from gmp_sender module.
    async fn poll_mvm_events(&self) -> Result<()> {
        // Query transactions from the module account to find MessageSent events
        let events = self
            .mvm_client
            .get_account_events(&self.config.mvm_module_addr, None, None, Some(100))
            .await
            .context("Failed to query MVM events")?;

        let message_sent_type = format!(
            "{}::gmp_sender::MessageSent",
            self.config.mvm_module_addr
        );

        for event in events {
            // Filter for MessageSent events
            if !event.r#type.contains("MessageSent") && event.r#type != message_sent_type {
                continue;
            }

            // Parse event data
            let event_data: MvmMessageSentEvent = match serde_json::from_value(event.data.clone()) {
                Ok(data) => data,
                Err(e) => {
                    warn!("Failed to parse MessageSent event: {}", e);
                    continue;
                }
            };

            // Parse nonce
            let nonce: u64 = event_data.nonce.parse().unwrap_or(0);

            // Check if already processed
            {
                let state = self.state.read().await;
                if let Some(processed) = state.processed_nonces.get(&self.config.mvm_chain_id) {
                    if processed.contains(&nonce) {
                        continue;
                    }
                }
            }

            // Convert to GmpMessage
            let message = self.parse_mvm_message_sent(&event_data, self.config.mvm_chain_id)?;

            info!(
                "Found MVM MessageSent: src={}, dst_chain={}, nonce={}",
                message.src_addr, message.dst_chain_id, message.nonce
            );

            // Deliver message to destination
            if let Err(e) = self.deliver_message(&message).await {
                error!("Failed to deliver message: {}", e);
                continue;
            }

            // Mark as processed
            {
                let mut state = self.state.write().await;
                state
                    .processed_nonces
                    .entry(self.config.mvm_chain_id)
                    .or_default()
                    .insert(nonce);
            }
        }

        Ok(())
    }

    /// Poll MVM connected chain for MessageSent events from gmp_sender module.
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

        // Query transactions from the module account to find MessageSent events
        let events = mvm_connected_client
            .get_account_events(mvm_connected_module_addr, None, None, Some(100))
            .await
            .context("Failed to query MVM connected chain events")?;

        let message_sent_type = format!("{}::gmp_sender::MessageSent", mvm_connected_module_addr);

        for event in events {
            // Filter for MessageSent events
            if !event.r#type.contains("MessageSent") && event.r#type != message_sent_type {
                continue;
            }

            // Parse event data
            let event_data: MvmMessageSentEvent = match serde_json::from_value(event.data.clone()) {
                Ok(data) => data,
                Err(e) => {
                    warn!("Failed to parse MVM connected MessageSent event: {}", e);
                    continue;
                }
            };

            // Parse nonce
            let nonce: u64 = event_data.nonce.parse().unwrap_or(0);

            // Check if already processed
            {
                let state = self.state.read().await;
                if let Some(processed) = state.processed_nonces.get(&mvm_connected_chain_id) {
                    if processed.contains(&nonce) {
                        continue;
                    }
                }
            }

            // Convert to GmpMessage
            let message = self.parse_mvm_message_sent(&event_data, mvm_connected_chain_id)?;

            info!(
                "Found MVM connected MessageSent: src={}, dst_chain={}, nonce={}",
                message.src_addr, message.dst_chain_id, message.nonce
            );

            // Deliver message to destination
            if let Err(e) = self.deliver_message(&message).await {
                error!("Failed to deliver MVM connected message: {}", e);
                continue;
            }

            // Mark as processed
            {
                let mut state = self.state.write().await;
                state
                    .processed_nonces
                    .entry(mvm_connected_chain_id)
                    .or_default()
                    .insert(nonce);
            }
        }

        Ok(())
    }

    /// Poll SVM for MessageSent events from native-gmp-endpoint program.
    async fn poll_svm_events(&self) -> Result<()> {
        let Some(ref svm_client) = self.svm_client else {
            return Ok(());
        };

        let Some(svm_chain_id) = self.config.svm_chain_id else {
            return Ok(());
        };

        // Get the last processed signature for pagination
        let before_sig = {
            let state = self.state.read().await;
            state.svm_last_signature.clone()
        };

        // Query recent signatures for the GMP program
        let program_id = solana_program::pubkey::Pubkey::from_str(
            self.config.svm_gmp_program_id.as_ref().unwrap(),
        )
        .context("Invalid SVM GMP program ID")?;

        let signatures = svm_client
            .get_signatures_for_address(&program_id, Some(50), before_sig.as_deref())
            .await
            .context("Failed to get SVM signatures")?;

        if signatures.is_empty() {
            return Ok(());
        }

        debug!("Found {} SVM signatures to process", signatures.len());

        // Process signatures in reverse order (oldest first) for proper ordering
        for sig_info in signatures.iter().rev() {
            // Skip if already processed
            {
                let state = self.state.read().await;
                if state.svm_processed_signatures.contains(&sig_info.signature) {
                    continue;
                }
            }

            // Skip failed transactions
            if sig_info.err.is_some() {
                continue;
            }

            // Fetch transaction details
            let tx = match svm_client.get_transaction(&sig_info.signature).await? {
                Some(tx) => tx,
                None => continue,
            };

            // Parse logs for MessageSent events
            let logs = tx.meta.as_ref().and_then(|m| m.log_messages.as_ref());
            if let Some(logs) = logs {
                for log in logs {
                    if let Some(message) = self.parse_svm_message_sent(log, svm_chain_id) {
                        info!(
                            "Found SVM MessageSent: src={}, dst_chain={}, nonce={}",
                            message.src_addr, message.dst_chain_id, message.nonce
                        );

                        // Check if already processed by nonce
                        {
                            let state = self.state.read().await;
                            if let Some(processed) = state.processed_nonces.get(&svm_chain_id) {
                                if processed.contains(&message.nonce) {
                                    continue;
                                }
                            }
                        }

                        // Deliver message to destination
                        if let Err(e) = self.deliver_message(&message).await {
                            error!("Failed to deliver SVM message: {}", e);
                            continue;
                        }

                        // Mark nonce as processed
                        {
                            let mut state = self.state.write().await;
                            state
                                .processed_nonces
                                .entry(svm_chain_id)
                                .or_default()
                                .insert(message.nonce);
                        }
                    }
                }
            }

            // Mark signature as processed
            {
                let mut state = self.state.write().await;
                state
                    .svm_processed_signatures
                    .insert(sig_info.signature.clone());
            }
        }

        // Update last processed signature (newest one, which is first in the list)
        if let Some(newest) = signatures.first() {
            let mut state = self.state.write().await;
            state.svm_last_signature = Some(newest.signature.clone());
        }

        Ok(())
    }

    /// Parse SVM MessageSent log line into GmpMessage.
    /// Log format: "MessageSent: src_chain_id={}, dst_chain_id={}, src_addr={}, dst_addr={}, nonce={}, payload_len={}, payload_hex={}"
    fn parse_svm_message_sent(&self, log: &str, svm_chain_id: u32) -> Option<GmpMessage> {
        if !log.contains("MessageSent:") {
            return None;
        }

        // Extract the MessageSent part
        let msg_part = log.split("MessageSent:").nth(1)?.trim();

        // Parse key=value pairs
        let mut src_chain_id: Option<u32> = None;
        let mut dst_chain_id: Option<u32> = None;
        let mut src_addr: Option<String> = None;
        let mut dst_addr: Option<String> = None;
        let mut nonce: Option<u64> = None;
        let mut payload_hex: Option<String> = None;

        for part in msg_part.split(", ") {
            let mut kv = part.splitn(2, '=');
            let key = kv.next()?.trim();
            let value = kv.next()?.trim();

            match key {
                "src_chain_id" => src_chain_id = value.parse().ok(),
                "dst_chain_id" => dst_chain_id = value.parse().ok(),
                "src_addr" => src_addr = Some(value.to_string()),
                "dst_addr" => dst_addr = Some(format!("0x{}", value)),
                "nonce" => nonce = value.parse().ok(),
                "payload_hex" => payload_hex = Some(format!("0x{}", value)),
                _ => {}
            }
        }

        // Validate we have all required fields
        let src_chain_id = src_chain_id?;
        let dst_chain_id = dst_chain_id?;
        let src_addr_raw = src_addr?;
        let nonce = nonce?;
        let payload = payload_hex.unwrap_or_else(|| "0x".to_string());

        // Verify source chain matches expected SVM chain
        if src_chain_id != svm_chain_id {
            warn!(
                "SVM MessageSent src_chain_id {} doesn't match expected {}",
                src_chain_id, svm_chain_id
            );
            return None;
        }

        // Convert Solana pubkey (base58) to hex
        let src_addr_hex = match solana_program::pubkey::Pubkey::from_str(&src_addr_raw) {
            Ok(pubkey) => format!("0x{}", hex::encode(pubkey.to_bytes())),
            Err(_) => {
                warn!("Invalid Solana pubkey in MessageSent: {}", src_addr_raw);
                return None;
            }
        };

        Some(GmpMessage {
            src_chain_id,
            src_addr: src_addr_hex,
            dst_chain_id,
            dst_addr: dst_addr?,
            payload,
            nonce,
        })
    }

    /// Parse MVM MessageSent event into GmpMessage.
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

        // Source address is the sender
        let src_addr = normalize_address(&event.sender);

        Ok(GmpMessage {
            src_chain_id,
            src_addr,
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
        } else {
            warn!(
                "Unknown destination chain ID: {}. Known chains: MVM hub={}, MVM connected={:?}, SVM={:?}",
                message.dst_chain_id, self.config.mvm_chain_id, self.config.mvm_connected_chain_id, self.config.svm_chain_id
            );
            Ok(())
        }
    }

    /// Deliver message to MVM hub chain via native_gmp_endpoint::deliver_message_entry.
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

    /// Deliver message to MVM connected chain via native_gmp_endpoint::deliver_message_entry.
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
        let src_addr_hex = message.src_addr.strip_prefix("0x").unwrap_or(&message.src_addr);
        let payload_hex = message.payload.strip_prefix("0x").unwrap_or(&message.payload);

        // Convert base64 private key to hex for CLI
        let private_key_bytes = STANDARD
            .decode(&self.config.operator_private_key)
            .context("Failed to decode base64 private key")?;
        let private_key_hex = hex::encode(&private_key_bytes);

        // Build function ID
        let function_id = format!(
            "{}::native_gmp_endpoint::deliver_message_entry",
            module_addr
        );

        // Build CLI arguments
        // Function signature: deliver_message_entry(relay: &signer, src_chain_id: u32, src_addr: vector<u8>, payload: vector<u8>, nonce: u64)
        let src_chain_id_arg = format!("u32:{}", message.src_chain_id);
        let src_addr_arg = format!("hex:{}", src_addr_hex);
        let payload_arg = format!("hex:{}", payload_hex);
        let nonce_arg = format!("u64:{}", message.nonce);

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
                &src_addr_arg,
                &payload_arg,
                &nonce_arg,
            ])
            .output()
            .context("Failed to execute aptos move run")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            error!(
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
        let output_str = String::from_utf8_lossy(&output.stdout);
        let tx_hash = extract_transaction_hash(&output_str);

        info!(
            "MVM {} deliver_message submitted successfully: nonce={}, tx_hash={:?}",
            chain_name, message.nonce, tx_hash
        );

        Ok(())
    }

    /// Deliver message to SVM chain via native-gmp-endpoint DeliverMessage instruction.
    ///
    /// Builds and submits a DeliverMessage transaction to the SVM native-gmp-endpoint program.
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

        // Parse program ID
        let program_id = Pubkey::from_str(program_id_str)
            .context("Invalid SVM GMP program ID")?;

        // Load relay keypair from operator private key (base64 Ed25519 -> Solana keypair)
        let relay_keypair = self.load_svm_keypair()?;
        let relay_pubkey = relay_keypair.pubkey();

        // Parse source address (32 bytes)
        let src_addr = parse_32_byte_address(&message.src_addr)?;

        // Parse destination address (the receiving program on SVM)
        let dst_program = parse_svm_pubkey(&message.dst_addr)?;

        // Parse payload
        let payload = hex_to_bytes(&message.payload)?;

        // Derive PDAs
        let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &program_id);
        let (relay_pda, _) =
            Pubkey::find_program_address(&[b"relay", relay_pubkey.as_ref()], &program_id);
        let (trusted_remote_pda, _) = Pubkey::find_program_address(
            &[b"trusted_remote", &message.src_chain_id.to_le_bytes()],
            &program_id,
        );
        let (nonce_in_pda, _) = Pubkey::find_program_address(
            &[b"nonce_in", &message.src_chain_id.to_le_bytes()],
            &program_id,
        );

        // Build DeliverMessage instruction
        let instruction_data = SvmDeliverMessageInstruction {
            src_chain_id: message.src_chain_id,
            src_addr,
            payload,
            nonce: message.nonce,
        };

        let instruction = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new_readonly(config_pda, false),
                AccountMeta::new_readonly(relay_pda, false),
                AccountMeta::new_readonly(trusted_remote_pda, false),
                AccountMeta::new(nonce_in_pda, false),
                AccountMeta::new_readonly(relay_pubkey, true), // signer
                AccountMeta::new(relay_pubkey, true),          // payer (signer)
                AccountMeta::new_readonly(dst_program, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: instruction_data
                .try_to_vec()
                .context("Failed to serialize DeliverMessage instruction")?,
        };

        // Create RPC client and submit transaction
        let rpc_client = RpcClient::new_with_commitment(
            rpc_url.clone(),
            CommitmentConfig::confirmed(),
        );

        let blockhash = rpc_client
            .get_latest_blockhash()
            .context("Failed to get latest blockhash")?;

        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&relay_pubkey),
            &[&relay_keypair],
            blockhash,
        );

        let signature = rpc_client
            .send_and_confirm_transaction(&transaction)
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
        let keypair = Keypair::from_bytes(&ed25519_seed_to_keypair_bytes(&seed_bytes)?)
            .context("Failed to create Solana keypair")?;

        Ok(keypair)
    }
}

// ============================================================================
// SVM INSTRUCTION TYPES
// ============================================================================

/// SVM DeliverMessage instruction data (matches native-gmp-endpoint program).
///
/// This is the 4th variant (index 4) in the NativeGmpInstruction enum.
#[derive(BorshSerialize)]
struct SvmDeliverMessageInstruction {
    src_chain_id: u32,
    src_addr: [u8; 32],
    payload: Vec<u8>,
    nonce: u64,
}

impl SvmDeliverMessageInstruction {
    fn try_to_vec(&self) -> Result<Vec<u8>> {
        // Instruction discriminator: DeliverMessage is variant 4 in the enum
        let mut data = vec![4u8];
        data.extend(
            borsh::to_vec(self).context("Failed to serialize instruction data")?,
        );
        Ok(data)
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Convert array of byte strings (e.g., ["60", "68"]) to hex string with 0x prefix.
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

