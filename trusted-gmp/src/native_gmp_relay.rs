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
    /// SVM intent escrow program ID (optional, for routing IntentRequirements)
    pub svm_escrow_program_id: Option<String>,
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
        let (svm_rpc_url, svm_gmp_program_id, svm_escrow_program_id, svm_chain_id) =
            if let Some(ref svm_config) = config.connected_chain_svm {
                (
                    Some(svm_config.rpc_url.clone()),
                    svm_config.gmp_endpoint_program_id.clone(),
                    Some(svm_config.escrow_program_id.clone()),
                    Some(svm_config.chain_id as u32),
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
    /// Last polled nonce for MVM hub outbox (view function based)
    mvm_hub_last_nonce: u64,
    /// Last polled nonce for MVM connected chain outbox (view function based)
    mvm_connected_last_nonce: u64,
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
    #[allow(dead_code)]
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
            .unwrap_or(1);

        // Determine start nonce: if we haven't polled yet, start from 1
        let start = if last_nonce == 0 { 1 } else { last_nonce + 1 };

        if start >= next_nonce {
            return Ok(last_nonce); // No new messages
        }

        debug!(
            "MVM {} outbox: polling nonces {}..{} (next_nonce={})",
            chain_name, start, next_nonce - 1, next_nonce
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
                // Use the module address (where GMP contracts are deployed) as src_addr,
                // not the individual sender. The destination chain's native_gmp_endpoint
                // trusts the source chain's module address, not individual senders.
                src_addr: normalize_address(module_addr),
                dst_chain_id,
                dst_addr: format!("0x{}", dst_addr_hex),
                payload: format!("0x{}", payload_hex),
                nonce,
            };

            info!(
                "MVM {} outbox: nonce={}, src={}, dst_chain={}",
                chain_name, nonce, message.src_addr, message.dst_chain_id
            );

            if let Err(e) = self.deliver_message(&message).await {
                error!("Failed to deliver MVM {} message nonce={}: {}", chain_name, nonce, e);
                // Don't advance past failed delivery
                break;
            }

            new_last = nonce;
        }

        Ok(new_last)
    }

    /// Poll SVM for MessageSent events from native-gmp-endpoint program.
    async fn poll_svm_events(&self) -> Result<()> {
        let Some(ref svm_client) = self.svm_client else {
            return Ok(());
        };

        let Some(svm_chain_id) = self.config.svm_chain_id else {
            return Ok(());
        };

        // Query recent signatures for the GMP program.
        // NOTE: Always fetch most recent signatures (before=None) to catch new transactions.
        // The svm_processed_signatures HashSet prevents duplicate processing.
        let program_id = solana_program::pubkey::Pubkey::from_str(
            self.config.svm_gmp_program_id.as_ref().unwrap(),
        )
        .context("Invalid SVM GMP program ID")?;

        let signatures = svm_client
            .get_signatures_for_address(&program_id, Some(50), None)
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

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        if !output.status.success() {
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

    /// Deliver message to SVM chain via native-gmp-endpoint DeliverMessage instruction.
    ///
    /// Builds and submits a DeliverMessage transaction to the SVM native-gmp-endpoint program.
    /// For IntentRequirements messages (0x01), also derives and passes the outflow-validator
    /// accounts needed for LzReceive CPI.
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

        // Parse program ID (native-gmp-endpoint)
        let program_id = Pubkey::from_str(program_id_str)
            .context("Invalid SVM GMP program ID")?;

        // Load relay keypair from operator private key (base64 Ed25519 -> Solana keypair)
        let relay_keypair = self.load_svm_keypair()?;
        let relay_pubkey = relay_keypair.pubkey();

        // Parse source address (32 bytes)
        let src_addr = parse_32_byte_address(&message.src_addr)?;

        // Parse destination address (the receiving program on SVM - e.g., outflow-validator)
        let dst_program = parse_svm_pubkey(&message.dst_addr)?;

        // Parse payload
        let payload = hex_to_bytes(&message.payload)?;

        // Derive GMP endpoint PDAs
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
        let (routing_pda, _) = Pubkey::find_program_address(&[b"routing"], &program_id);

        // Get intent_escrow program for second destination (required for routing)
        let escrow_program = if let Some(ref escrow_id) = self.config.svm_escrow_program_id {
            Pubkey::from_str(escrow_id).context("Invalid SVM escrow program ID")?
        } else {
            // If no escrow configured, use dst_program as placeholder (routing won't be used)
            dst_program
        };

        // Build base accounts for DeliverMessage
        // Account order (updated for routing support):
        // 0. Config, 1. Relay, 2. TrustedRemote, 3. NonceIn, 4. RelaySigner, 5. Payer
        // Track if we need to create an ATA before delivering the message (for FulfillmentProof)
        // Tuple: (ata, owner, mint, token_program, associated_token_program)
        #[allow(clippy::type_complexity)]
        let mut ata_create_info: Option<(Pubkey, Pubkey, Pubkey, Pubkey, Pubkey)> = None;

        // 6. SystemProgram, 7. RoutingConfig, 8. DestProgram1, 9. DestProgram2, 10+. Remaining
        let mut accounts = vec![
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(relay_pda, false),
            AccountMeta::new_readonly(trusted_remote_pda, false),
            AccountMeta::new(nonce_in_pda, false),
            AccountMeta::new_readonly(relay_pubkey, true), // signer
            AccountMeta::new(relay_pubkey, true),          // payer (signer)
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(routing_pda, false), // routing config (may not exist)
            AccountMeta::new_readonly(dst_program, false), // destination program 1 (outflow_validator)
            AccountMeta::new_readonly(escrow_program, false), // destination program 2 (intent_escrow)
        ];

        // For IntentRequirements (0x01), add accounts for both destination programs' LzReceive CPI.
        // The GMP endpoint routes to BOTH outflow_validator AND intent_escrow when routing is configured.
        //
        // Account layout for remaining_accounts (passed to GMP endpoint after base accounts):
        // Indices 0-4: outflow_validator's LzReceive accounts
        // Indices 5-9: intent_escrow's LzReceive accounts
        //
        // Each program's LzReceive expects: requirements(w), config(r), authority(s), payer(s,w), system_program
        if !payload.is_empty() && payload[0] == 0x01 {
            // IntentRequirements format: [type(1)] [intent_id(32)] [...]
            if payload.len() >= 33 {
                let mut intent_id = [0u8; 32];
                intent_id.copy_from_slice(&payload[1..33]);

                // Derive outflow-validator PDAs (dst_program is the outflow-validator)
                let (outflow_requirements_pda, _) = Pubkey::find_program_address(
                    &[b"requirements", &intent_id],
                    &dst_program,
                );
                let (outflow_config_pda, _) = Pubkey::find_program_address(
                    &[b"config"],
                    &dst_program,
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
                    "Adding accounts for multi-destination LzReceive CPI: outflow_req={}, outflow_cfg={}, escrow_req={}, escrow_cfg={}",
                    outflow_requirements_pda, outflow_config_pda, escrow_requirements_pda, escrow_gmp_config_pda
                );

                // Accounts for outflow_validator's LzReceive (indices 0-4)
                // LzReceive expects: requirements(w), config(r), authority(s), payer(s,w), system_program
                accounts.push(AccountMeta::new(outflow_requirements_pda, false));  // 0
                accounts.push(AccountMeta::new_readonly(outflow_config_pda, false)); // 1
                accounts.push(AccountMeta::new_readonly(relay_pubkey, true));  // 2: authority (signer)
                accounts.push(AccountMeta::new(relay_pubkey, true));           // 3: payer (signer)
                accounts.push(AccountMeta::new_readonly(system_program::id(), false)); // 4

                // Accounts for intent_escrow's LzReceive (indices 5-9)
                // LzReceive expects: requirements(w), gmp_config(r), authority(s), payer(s,w), system_program
                accounts.push(AccountMeta::new(escrow_requirements_pda, false));  // 5
                accounts.push(AccountMeta::new_readonly(escrow_gmp_config_pda, false)); // 6
                accounts.push(AccountMeta::new_readonly(relay_pubkey, true));  // 7: authority (signer)
                accounts.push(AccountMeta::new(relay_pubkey, true));           // 8: payer (signer)
                accounts.push(AccountMeta::new_readonly(system_program::id(), false)); // 9
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

                // Accounts for intent_escrow's LzReceiveFulfillmentProof
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
            src_addr,
            payload,
            nonce: message.nonce,
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
                    AccountMeta::new_readonly(system_program::id(), false), // system program
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
                    "SVM DeliverMessage failed: {}. Accounts: config={}, relay={}, trusted_remote={}, nonce_in={}, dst_program={}",
                    e, config_pda, relay_pda, trusted_remote_pda, nonce_in_pda, dst_program
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

/// SVM DeliverMessage instruction data (matches native-gmp-endpoint program).
///
/// This is the 6th variant (index 5) in the NativeGmpInstruction enum.
#[derive(BorshSerialize)]
struct SvmDeliverMessageInstruction {
    src_chain_id: u32,
    src_addr: [u8; 32],
    payload: Vec<u8>,
    nonce: u64,
}

impl SvmDeliverMessageInstruction {
    fn try_to_vec(&self) -> Result<Vec<u8>> {
        // Instruction discriminator: DeliverMessage is variant 6 in the enum
        // (Initialize=0, AddRelay=1, RemoveRelay=2, SetTrustedRemote=3, SetRouting=4, Send=5, DeliverMessage=6)
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

