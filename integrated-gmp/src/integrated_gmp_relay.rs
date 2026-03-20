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
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::crypto::CryptoService;
use crate::evm_client::GmpEvmClient;
use crate::mvm_client::GmpMvmClient;
use crate::svm_client::GmpSvmClient;

// Well-known Solana program IDs.
const SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([0; 32]);

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Per-chain relay configuration for a connected MVM chain.
#[derive(Debug, Clone)]
pub struct MvmRelayChainConfig {
    /// MVM chain RPC URL
    pub rpc_url: String,
    /// MVM intent module address (where gmp_sender is deployed)
    pub module_addr: String,
    /// MVM chain ID
    pub chain_id: u32,
}

/// Per-chain relay configuration for a connected EVM chain.
#[derive(Debug, Clone)]
pub struct EvmRelayChainConfig {
    /// EVM RPC URL
    pub rpc_url: String,
    /// EVM GMP endpoint contract address (IntentGmp)
    pub gmp_endpoint_addr: Option<String>,
    /// EVM chain ID
    pub chain_id: u32,
    /// EVM relay address (the `from` address for eth_sendRawTransaction, must be authorized relay in IntentGmp)
    pub relay_address: String,
}

/// Per-chain relay configuration for a connected SVM chain.
#[derive(Debug, Clone)]
pub struct SvmRelayChainConfig {
    /// SVM RPC URL
    pub rpc_url: String,
    /// SVM integrated GMP endpoint program ID
    pub gmp_program_id: Option<String>,
    /// SVM intent escrow program ID (for routing IntentRequirements)
    pub escrow_program_id: Option<String>,
    /// SVM outflow validator program ID (for routing IntentRequirements)
    pub outflow_program_id: Option<String>,
    /// SVM chain ID
    pub chain_id: u32,
}

/// Configuration for the integrated GMP relay.
#[derive(Debug, Clone)]
pub struct NativeGmpRelayConfig {
    /// MVM hub chain RPC URL
    pub mvm_rpc_url: String,
    /// MVM intent module address (where gmp_sender is deployed)
    pub mvm_module_addr: String,
    /// MVM chain ID
    pub mvm_chain_id: u32,
    /// Connected MVM chains (each can send/receive GMP messages)
    pub mvm_chains: Vec<MvmRelayChainConfig>,
    /// Connected EVM chains (each can send/receive GMP messages)
    pub evm_chains: Vec<EvmRelayChainConfig>,
    /// Connected SVM chains (each can send/receive GMP messages)
    pub svm_chains: Vec<SvmRelayChainConfig>,
    /// Polling interval in milliseconds
    pub polling_interval_ms: u64,
    /// Relay operator private key (base64 encoded Ed25519)
    pub operator_private_key: String,
}

impl NativeGmpRelayConfig {
    /// Create relay config from main config.
    pub fn from_config(config: &Config) -> Result<Self> {
        let operator_private_key = config.integrated_gmp.get_private_key()?;

        let mvm_chains: Vec<MvmRelayChainConfig> = config
            .connected_chain_mvm
            .iter()
            .map(|mvm| MvmRelayChainConfig {
                rpc_url: mvm.rpc_url.clone(),
                module_addr: mvm.intent_module_addr.clone(),
                chain_id: mvm.chain_id as u32,
            })
            .collect();

        let evm_chains: Vec<EvmRelayChainConfig> = config
            .connected_chain_evm
            .iter()
            .map(|evm| EvmRelayChainConfig {
                rpc_url: evm.rpc_url.clone(),
                gmp_endpoint_addr: evm.gmp_endpoint_addr.clone(),
                chain_id: evm.chain_id as u32,
                relay_address: evm.approver_evm_pubkey_hash.clone(),
            })
            .collect();

        let svm_chains: Vec<SvmRelayChainConfig> = config
            .connected_chain_svm
            .iter()
            .map(|svm| SvmRelayChainConfig {
                rpc_url: svm.rpc_url.clone(),
                gmp_program_id: svm.gmp_endpoint_program_id.clone(),
                escrow_program_id: Some(svm.escrow_program_id.clone()),
                outflow_program_id: Some(svm.outflow_program_id.clone()),
                chain_id: svm.chain_id as u32,
            })
            .collect();

        Ok(Self {
            mvm_rpc_url: config.hub_chain.rpc_url.clone(),
            mvm_module_addr: config.hub_chain.intent_module_addr.clone(),
            mvm_chain_id: config.hub_chain.chain_id as u32,
            mvm_chains,
            evm_chains,
            svm_chains,
            polling_interval_ms: config.integrated_gmp.polling_interval_ms,
            operator_private_key,
        })
    }

    /// Find the EVM chain config for a given chain ID.
    pub fn find_evm_chain(&self, chain_id: u32) -> Option<&EvmRelayChainConfig> {
        self.evm_chains.iter().find(|c| c.chain_id == chain_id)
    }

    /// Find the MVM connected chain config for a given chain ID.
    pub fn find_mvm_chain(&self, chain_id: u32) -> Option<&MvmRelayChainConfig> {
        self.mvm_chains.iter().find(|c| c.chain_id == chain_id)
    }

    /// Find the SVM chain config for a given chain ID.
    pub fn find_svm_chain(&self, chain_id: u32) -> Option<&SvmRelayChainConfig> {
        self.svm_chains.iter().find(|c| c.chain_id == chain_id)
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
// DELIVERY RETRY CONFIGURATION
// ============================================================================

/// Maximum number of delivery attempts per message before permanently skipping
pub const MAX_DELIVERY_RETRIES: u32 = 3;

/// Initial backoff duration in seconds after first delivery failure (doubles each retry)
const INITIAL_DELIVERY_BACKOFF_SECS: u64 = 5;

// ============================================================================
// RELAY STATE
// ============================================================================

/// Tracks delivery attempts for a single message.
#[derive(Debug, Clone)]
pub struct DeliveryAttempt {
    /// Number of failed delivery attempts
    pub count: u32,
    /// Earliest time the next retry is allowed (Unix timestamp)
    pub next_retry_after: u64,
}

impl DeliveryAttempt {
    /// Check if max retries have been exhausted.
    pub fn is_exhausted(&self) -> bool {
        self.count >= MAX_DELIVERY_RETRIES
    }

    /// Check if this attempt is currently in backoff.
    pub fn is_in_backoff(&self) -> bool {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.next_retry_after > current_time
    }

    /// Record a failure, returning true if max retries are now exhausted.
    pub fn record_failure(&mut self) -> bool {
        self.count += 1;
        if self.count >= MAX_DELIVERY_RETRIES {
            return true;
        }
        // Exponential backoff: INITIAL_DELIVERY_BACKOFF_SECS * 2^(attempt-1)
        let backoff_secs = INITIAL_DELIVERY_BACKOFF_SECS * 2u64.pow(self.count - 1);
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.next_retry_after = current_time + backoff_secs;
        false
    }
}

/// Internal state for tracking processed messages.
#[derive(Debug, Default)]
struct RelayState {
    /// Processed nonces per source chain (chain_id -> set of processed nonces)
    processed_nonces: HashMap<u32, HashSet<u64>>,
    /// Last polled nonce for MVM hub outbox (view function based)
    mvm_hub_last_nonce: u64,
    /// Last polled nonce per connected MVM chain (chain_id -> last nonce)
    mvm_connected_last_nonces: HashMap<u32, u64>,
    /// Last polled nonce per connected SVM chain (chain_id -> last nonce)
    svm_last_nonces: HashMap<u32, u64>,
    /// Last polled EVM block number per chain (chain_id -> block number)
    evm_last_blocks: HashMap<u32, u64>,
    /// Per-message delivery attempt tracking: (src_chain_id, nonce) -> DeliveryAttempt
    delivery_attempts: HashMap<(u32, u64), DeliveryAttempt>,
    /// Per-chain poll failure tracking: chain_name -> DeliveryAttempt
    /// When a poll fails (RPC unreachable), the chain enters backoff before retrying.
    chain_poll_failures: HashMap<String, DeliveryAttempt>,
}

// ============================================================================
// INTEGRATED GMP RELAY
// ============================================================================

/// Integrated GMP relay service that watches for MessageSent events
/// and delivers messages to destination chains.
pub struct NativeGmpRelay {
    config: NativeGmpRelayConfig,
    crypto_service: CryptoService,
    /// Hub MVM client
    mvm_hub_client: GmpMvmClient,
    /// Connected MVM clients keyed by chain ID
    mvm_connected_clients: HashMap<u32, GmpMvmClient>,
    /// Connected EVM clients keyed by chain ID
    evm_clients: HashMap<u32, GmpEvmClient>,
    /// Connected SVM clients keyed by chain ID
    svm_clients: HashMap<u32, GmpSvmClient>,
    state: Arc<RwLock<RelayState>>,
}

impl NativeGmpRelay {
    /// Create a new integrated GMP relay.
    pub fn new(config: NativeGmpRelayConfig, crypto_service: CryptoService) -> Result<Self> {
        let mvm_hub_client = GmpMvmClient::new(
            &config.mvm_rpc_url,
            &config.mvm_module_addr,
            config.mvm_chain_id,
        )
        .context("Failed to create MVM hub client")?;

        // Initialize MVM connected clients
        let mut mvm_connected_clients = HashMap::new();
        for mvm_chain in &config.mvm_chains {
            let client = GmpMvmClient::new(&mvm_chain.rpc_url, &mvm_chain.module_addr, mvm_chain.chain_id)
                .with_context(|| format!("Failed to create MVM client for chain {}", mvm_chain.chain_id))?;
            mvm_connected_clients.insert(mvm_chain.chain_id, client);
        }

        // Initialize EVM clients
        let mut evm_clients = HashMap::new();
        for evm_chain in &config.evm_chains {
            if let Some(ref gmp_endpoint) = evm_chain.gmp_endpoint_addr {
                let client = GmpEvmClient::new(
                    &evm_chain.rpc_url,
                    gmp_endpoint,
                    evm_chain.chain_id,
                    &evm_chain.relay_address,
                )
                .with_context(|| format!("Failed to create EVM client for chain {}", evm_chain.chain_id))?;
                evm_clients.insert(evm_chain.chain_id, client);
            }
        }

        // Initialize SVM clients
        let mut svm_clients = HashMap::new();
        for svm_chain in &config.svm_chains {
            if let Some(ref program_id) = svm_chain.gmp_program_id {
                let client = GmpSvmClient::new(&svm_chain.rpc_url, program_id)
                    .with_context(|| format!("Failed to create SVM client for chain {}", svm_chain.chain_id))?;
                svm_clients.insert(svm_chain.chain_id, client);
            }
        }

        Ok(Self {
            config,
            crypto_service,
            mvm_hub_client,
            mvm_connected_clients,
            evm_clients,
            svm_clients,
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
        let authorized = self.mvm_hub_client.is_relay_authorized(&mvm_addr).await
            .context("Failed to check relay authorization on MVM hub")?;
        if !authorized {
            anyhow::bail!("Relay {} is NOT authorized on MVM hub. Run add_relay first.", mvm_addr);
        }
        info!("MVM hub: relay {} authorized", mvm_addr);

        // Check all connected MVM chains
        for (chain_id, client) in &self.mvm_connected_clients {
            let authorized = client.is_relay_authorized(&mvm_addr).await
                .with_context(|| format!("Failed to check relay authorization on MVM chain {}", chain_id))?;
            if !authorized {
                anyhow::bail!("Relay {} is NOT authorized on MVM chain {}. Run add_relay first.", mvm_addr, chain_id);
            }
            info!("MVM connected (chain_id={}): relay {} authorized", chain_id, mvm_addr);
        }

        // Check all connected EVM chains
        for (chain_id, client) in &self.evm_clients {
            let authorized = client.is_relay_authorized(&evm_addr).await
                .with_context(|| format!("Failed to check relay authorization on EVM chain {}", chain_id))?;
            if !authorized {
                anyhow::bail!(
                    "Relay {} is NOT authorized on EVM chain {} (contract {}). Run addRelay first.",
                    evm_addr, chain_id, client.gmp_endpoint_addr()
                );
            }
            info!("EVM (chain_id={}): relay {} authorized", chain_id, evm_addr);
        }

        // Check all connected SVM chains
        for svm_chain in &self.config.svm_chains {
            if let Some(ref program_id_str) = svm_chain.gmp_program_id {
                self.check_svm_relay_auth(&svm_chain.rpc_url, program_id_str, &svm_addr)
                    .await?;
            }
        }

        info!("Relay authorization verified on all configured chains");
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

    // ========================================================================
    // FAILURE TRACKING
    // ========================================================================

    /// Check if a message should be skipped (max retries exhausted or in backoff).
    /// Returns true if the message should be delivered, false if it should be skipped.
    async fn should_attempt_delivery(&self, src_chain_id: u32, nonce: u64) -> bool {
        let state = self.state.read().await;
        let key = (src_chain_id, nonce);
        if let Some(attempt) = state.delivery_attempts.get(&key) {
            if attempt.count >= MAX_DELIVERY_RETRIES {
                return false; // Already permanently skipped
            }
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if attempt.next_retry_after > current_time {
                debug!(
                    "Skipping delivery for ({}, nonce={}): in backoff (retry after {}, now {})",
                    src_chain_id, nonce, attempt.next_retry_after, current_time
                );
                return false; // In backoff period
            }
        }
        true
    }

    /// Record a transient delivery failure. Returns true if max retries now exhausted.
    async fn record_delivery_failure(&self, message: &GmpMessage, error: &str) -> bool {
        let mut state = self.state.write().await;
        let key = (message.src_chain_id, message.nonce);
        let attempt = state.delivery_attempts.entry(key).or_insert(DeliveryAttempt {
            count: 0,
            next_retry_after: 0,
        });

        let exhausted = attempt.record_failure();

        if exhausted {
            error!(
                "Message permanently failed after {} attempts: src_chain={}, nonce={}, dst_chain={}, payload_len={}. Last error: {}",
                attempt.count, message.src_chain_id, message.nonce,
                message.dst_chain_id, message.payload.len(), error
            );
        } else {
            let backoff_secs = INITIAL_DELIVERY_BACKOFF_SECS * 2u64.pow(attempt.count - 1);
            warn!(
                "Delivery attempt {}/{} failed for src_chain={}, nonce={}. Next retry after {}s. Error: {}",
                attempt.count, MAX_DELIVERY_RETRIES,
                message.src_chain_id, message.nonce, backoff_secs, error
            );
        }

        exhausted
    }

    /// Check if a chain's poll should be skipped due to recent failures.
    /// Returns true if the chain is ready to be polled.
    async fn should_poll_chain(&self, chain_name: &str) -> bool {
        let state = self.state.read().await;
        if let Some(attempt) = state.chain_poll_failures.get(chain_name) {
            if attempt.is_in_backoff() {
                debug!(
                    "Skipping {} poll: in backoff (retry after {})",
                    chain_name, attempt.next_retry_after
                );
                return false;
            }
        }
        true
    }

    /// Record a chain poll failure with backoff. Resets on next successful poll.
    async fn record_chain_poll_failure(&self, chain_name: &str, error: &str) {
        let mut state = self.state.write().await;
        let attempt = state.chain_poll_failures.entry(chain_name.to_string()).or_insert(DeliveryAttempt {
            count: 0,
            next_retry_after: 0,
        });
        attempt.record_failure();
        let backoff_secs = INITIAL_DELIVERY_BACKOFF_SECS * 2u64.pow(attempt.count.saturating_sub(1));
        warn!(
            "{} poll failed (attempt {}). Next poll after {}s. Error: {}",
            chain_name, attempt.count, backoff_secs, error
        );
    }

    /// Clear chain poll failure tracking after a successful poll.
    async fn clear_chain_poll_failure(&self, chain_name: &str) {
        let mut state = self.state.write().await;
        state.chain_poll_failures.remove(chain_name);
    }

    /// Start the relay service (blocking).
    pub async fn run(&self) -> Result<()> {
        info!(
            "Starting integrated GMP relay: MVM hub chain_id={}, polling_interval={}ms",
            self.config.mvm_chain_id, self.config.polling_interval_ms
        );

        for mvm_chain in &self.config.mvm_chains {
            info!("MVM connected chain configured: chain_id={}", mvm_chain.chain_id);
        }

        for svm_chain in &self.config.svm_chains {
            info!("SVM chain configured: chain_id={}", svm_chain.chain_id);
        }

        for evm_chain in &self.config.evm_chains {
            info!("EVM chain configured: chain_id={}", evm_chain.chain_id);
        }

        // Verify relay is authorized on all destination chains before starting
        self.check_authorization().await?;

        let interval = Duration::from_millis(self.config.polling_interval_ms);

        loop {
            // Poll MVM hub for MessageSent events
            if self.should_poll_chain("mvm_hub").await {
                match self.poll_mvm_events().await {
                    Ok(()) => self.clear_chain_poll_failure("mvm_hub").await,
                    Err(e) => self.record_chain_poll_failure("mvm_hub", &format!("{:#}", e)).await,
                }
            }

            // Poll all connected MVM chains for MessageSent events
            for mvm_chain in &self.config.mvm_chains {
                let poll_key = format!("mvm_connected_{}", mvm_chain.chain_id);
                if self.should_poll_chain(&poll_key).await {
                    match self.poll_mvm_connected_events(mvm_chain).await {
                        Ok(()) => self.clear_chain_poll_failure(&poll_key).await,
                        Err(e) => self.record_chain_poll_failure(&poll_key, &format!("{:#}", e)).await,
                    }
                }
            }

            // Poll all connected SVM chains for outbound messages
            for svm_chain in &self.config.svm_chains {
                let poll_key = format!("svm_{}", svm_chain.chain_id);
                if self.should_poll_chain(&poll_key).await {
                    match self.poll_svm_events(svm_chain).await {
                        Ok(()) => self.clear_chain_poll_failure(&poll_key).await,
                        Err(e) => self.record_chain_poll_failure(&poll_key, &format!("{:#}", e)).await,
                    }
                }
            }

            // Poll all connected EVM chains for MessageSent events
            for evm_chain in &self.config.evm_chains {
                let poll_key = format!("evm_{}", evm_chain.chain_id);
                if self.should_poll_chain(&poll_key).await {
                    match self.poll_evm_events(evm_chain).await {
                        Ok(()) => self.clear_chain_poll_failure(&poll_key).await,
                        Err(e) => self.record_chain_poll_failure(&poll_key, &format!("{:#}", e)).await,
                    }
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
            .poll_mvm_outbox(&self.mvm_hub_client, last_nonce, "hub")
            .await?;

        if new_last > last_nonce {
            self.state.write().await.mvm_hub_last_nonce = new_last;
        }

        Ok(())
    }

    /// Poll a connected MVM chain outbox for new messages via view functions.
    async fn poll_mvm_connected_events(&self, mvm_chain: &MvmRelayChainConfig) -> Result<()> {
        let client = self.mvm_connected_clients.get(&mvm_chain.chain_id)
            .ok_or_else(|| anyhow::anyhow!("No MVM client for chain {}", mvm_chain.chain_id))?;

        let last_nonce = {
            *self.state.read().await.mvm_connected_last_nonces.get(&mvm_chain.chain_id).unwrap_or(&0)
        };

        let chain_label = format!("connected({})", mvm_chain.chain_id);
        let new_last = self
            .poll_mvm_outbox(client, last_nonce, &chain_label)
            .await?;

        if new_last > last_nonce {
            self.state.write().await.mvm_connected_last_nonces.insert(mvm_chain.chain_id, new_last);
        }

        Ok(())
    }

    /// Shared outbox polling logic for any MVM chain.
    ///
    /// Uses `GmpMvmClient` to read nonces and messages from the outbox.
    /// Returns the new last_nonce value (highest nonce processed).
    async fn poll_mvm_outbox(
        &self,
        client: &GmpMvmClient,
        last_nonce: u64,
        chain_name: &str,
    ) -> Result<u64> {
        let src_chain_id = client.chain_id();
        let next_nonce = client.get_next_nonce().await?;

        let start = if last_nonce == 0 { 1 } else { last_nonce + 1 };

        if start >= next_nonce {
            debug!(
                "MVM {} outbox: next_nonce={}, last_nonce={} (idle)",
                chain_name, next_nonce, last_nonce
            );
            return Ok(last_nonce);
        }

        info!(
            "MVM {} outbox: processing nonces {}..{} ({} messages)",
            chain_name, start, next_nonce - 1, next_nonce - start
        );

        let mut new_last = last_nonce;

        for nonce in start..next_nonce {
            let message = match client.get_message(nonce).await {
                Ok(msg) => msg,
                Err(e) => {
                    warn!(
                        "MVM {} outbox: failed to read nonce {}: {}. Skipping (may be expired).",
                        chain_name, nonce, e
                    );
                    new_last = nonce;
                    continue;
                }
            };

            info!(
                "MVM {} outbox: nonce={}, src={}, dst_chain={}",
                chain_name, nonce, message.remote_gmp_endpoint_addr, message.dst_chain_id
            );

            if !self.should_attempt_delivery(src_chain_id, nonce).await {
                new_last = nonce;
                continue;
            }

            if let Err(e) = self.deliver_message(&message).await {
                let err_str = format!("{:#}", e);
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
                let exhausted = self.record_delivery_failure(&message, &err_str).await;
                if exhausted {
                    new_last = nonce;
                    continue;
                }
                new_last = nonce;
                continue;
            }

            new_last = nonce;
        }

        Ok(new_last)
    }

    /// Poll SVM for outbound messages using global nonce-based polling.
    ///
    /// Reads the single OutboundNonceAccount via getAccountInfo, then reads
    /// individual MessageAccount PDAs for any new nonces — same pattern as MVM.
    async fn poll_svm_events(&self, svm_chain: &SvmRelayChainConfig) -> Result<()> {
        let svm_client = self.svm_clients.get(&svm_chain.chain_id)
            .ok_or_else(|| anyhow::anyhow!("No SVM client for chain {}", svm_chain.chain_id))?;

        let svm_chain_id = svm_chain.chain_id;

        let gmp_program_id = Pubkey::from_str(
            svm_chain.gmp_program_id.as_ref()
                .ok_or_else(|| anyhow::anyhow!("SVM GMP program ID not configured for chain {}", svm_chain_id))?,
        )
        .context("Invalid SVM GMP program ID")?;

        // Read the global on-chain nonce counter
        let next_nonce = svm_client
            .get_outbound_nonce(&gmp_program_id)
            .await
            .context("Failed to read SVM outbound nonce")?;

        let maybe_last = {
            self.state.read().await.svm_last_nonces.get(&svm_chain_id).copied()
        };

        let start = match maybe_last {
            Some(last) => last + 1,
            None => 0, // SVM nonces start at 0
        };

        if start >= next_nonce {
            return Ok(());
        }

        info!(
            "SVM outbox (chain_id={}): processing nonces {}..{} ({} messages)",
            svm_chain_id, start, next_nonce - 1, next_nonce - start
        );

        let mut new_last = maybe_last;

        for nonce in start..next_nonce {
            let msg = svm_client
                .get_message_data(&gmp_program_id, nonce)
                .await
                .context(format!("Failed to read SVM message nonce={}", nonce))?;

            let Some(msg) = msg else {
                warn!(
                    "SVM outbox: message account not found for nonce={}. Skipping (may be cleaned up).",
                    nonce
                );
                new_last = Some(nonce);
                continue;
            };

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

            if !self.should_attempt_delivery(svm_chain_id, nonce).await {
                new_last = Some(nonce);
                continue;
            }

            if let Err(e) = self.deliver_message(&message).await {
                let err_str = format!("{:#}", e);
                if err_str.contains("E_UNKNOWN_REMOTE_GMP_ENDPOINT")
                    || err_str.contains("E_ALREADY_DELIVERED")
                    || err_str.contains("AlreadyDelivered")
                    || err_str.contains("Already delivered")
                    || err_str.contains("E_INTENT_NOT_FOUND")
                {
                    warn!(
                        "Permanent delivery failure for SVM nonce={}, skipping: {}",
                        nonce, err_str
                    );
                    new_last = Some(nonce);
                    continue;
                }
                let exhausted = self.record_delivery_failure(&message, &err_str).await;
                new_last = Some(nonce);
                if exhausted {
                    continue;
                }
                continue;
            }

            new_last = Some(nonce);
        }

        if let Some(last) = new_last {
            if maybe_last != new_last {
                self.state.write().await.svm_last_nonces.insert(svm_chain_id, last);
            }
        }

        Ok(())
    }

    /// Deliver a GMP message to the destination chain.
    async fn deliver_message(&self, message: &GmpMessage) -> Result<()> {
        let dst = message.dst_chain_id;

        // Destination is MVM hub
        if dst == self.config.mvm_chain_id {
            return self.deliver_to_mvm_hub(message).await;
        }

        // Destination is a connected MVM chain
        if let Some(mvm_chain) = self.config.find_mvm_chain(dst) {
            return self.deliver_to_mvm_connected(message, mvm_chain).await;
        }

        // Destination is a connected SVM chain
        if let Some(svm_chain) = self.config.find_svm_chain(dst) {
            return self.deliver_to_svm(message, svm_chain).await;
        }

        // Destination is a connected EVM chain
        if let Some(evm_chain) = self.config.find_evm_chain(dst) {
            return self.deliver_to_evm(message, evm_chain).await;
        }

        let known_mvm: Vec<u32> = self.config.mvm_chains.iter().map(|c| c.chain_id).collect();
        let known_evm: Vec<u32> = self.config.evm_chains.iter().map(|c| c.chain_id).collect();
        let known_svm: Vec<u32> = self.config.svm_chains.iter().map(|c| c.chain_id).collect();
        warn!(
            "Unknown destination chain ID: {}. Known chains: MVM hub={}, MVM connected={:?}, SVM={:?}, EVM={:?}",
            dst, self.config.mvm_chain_id, known_mvm, known_svm, known_evm
        );
        Ok(())
    }

    /// Deliver message to MVM hub chain via intent_gmp::deliver_message_entry.
    ///
    /// Uses the CLI-based transaction submission pattern (same as solver).
    async fn deliver_to_mvm_hub(&self, message: &GmpMessage) -> Result<()> {
        info!("Delivering message to MVM hub: dst_chain={}, nonce={}", message.dst_chain_id, message.nonce);
        self.mvm_hub_client.deliver_message(message, &self.config.operator_private_key).await
    }

    /// Deliver message to a connected MVM chain via intent_gmp::deliver_message_entry.
    async fn deliver_to_mvm_connected(&self, message: &GmpMessage, mvm_chain: &MvmRelayChainConfig) -> Result<()> {
        let client = self.mvm_connected_clients.get(&mvm_chain.chain_id)
            .ok_or_else(|| anyhow::anyhow!("No MVM client for chain {}", mvm_chain.chain_id))?;
        info!("Delivering message to MVM connected({}): dst_chain={}, nonce={}",
            mvm_chain.chain_id, message.dst_chain_id, message.nonce);
        client.deliver_message(message, &self.config.operator_private_key).await
    }

    /// Deliver message to EVM chain via IntentGmp.deliverMessage().
    async fn deliver_to_evm(&self, message: &GmpMessage, evm_chain: &EvmRelayChainConfig) -> Result<()> {
        let client = self.evm_clients.get(&evm_chain.chain_id)
            .ok_or_else(|| anyhow::anyhow!("No EVM client for chain {}", evm_chain.chain_id))?;

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
            if client.is_message_delivered(intent_id, msg_type).await? {
                info!(
                    "EVM: message already delivered (nonce={}, msg_type=0x{:02x}), skipping",
                    message.nonce, msg_type
                );
                return Ok(());
            }
        }

        let tx_hash = client
            .deliver_message(
                message.src_chain_id,
                &message.remote_gmp_endpoint_addr,
                &message.payload,
                &self.crypto_service,
            )
            .await?;

        info!("EVM: waiting for receipt for tx_hash={}", tx_hash);
        client.wait_for_receipt(&tx_hash).await?;

        info!(
            "EVM deliver_message submitted successfully: nonce={}, tx_hash={}",
            message.nonce, tx_hash
        );

        Ok(())
    }

    /// Poll an EVM chain for MessageSent events from IntentGmp contract.
    async fn poll_evm_events(&self, evm_chain: &EvmRelayChainConfig) -> Result<()> {
        let evm_chain_id = evm_chain.chain_id;
        let client = self.evm_clients.get(&evm_chain_id)
            .ok_or_else(|| anyhow::anyhow!("No EVM client for chain {}", evm_chain_id))?;

        let current_block = client.get_block_number().await?;

        // Max 10 block range for Alchemy free tier
        let max_range: u64 = 10;
        let last_block = { *self.state.read().await.evm_last_blocks.get(&evm_chain_id).unwrap_or(&0) };
        let from_block = if last_block == 0 {
            current_block.saturating_sub(max_range)
        } else {
            last_block + 1
        };

        if from_block > current_block {
            return Ok(());
        }

        let to_block = from_block.saturating_add(max_range - 1).min(current_block);

        let messages = client.poll_message_sent_events(from_block, to_block).await?;

        for message in &messages {
            info!(
                "Found EVM MessageSent: dst_chain={}, nonce={}",
                message.dst_chain_id, message.nonce
            );

            {
                let state = self.state.read().await;
                if let Some(processed) = state.processed_nonces.get(&evm_chain_id) {
                    if processed.contains(&message.nonce) {
                        continue;
                    }
                }
            }

            if !self.should_attempt_delivery(evm_chain_id, message.nonce).await {
                continue;
            }

            if let Err(e) = self.deliver_message(message).await {
                let err_str = format!("{:#}", e);
                if err_str.contains("E_UNKNOWN_REMOTE_GMP_ENDPOINT")
                    || err_str.contains("E_ALREADY_DELIVERED")
                    || err_str.contains("AlreadyDelivered")
                    || err_str.contains("Already delivered")
                    || err_str.contains("E_INTENT_NOT_FOUND")
                {
                    warn!(
                        "Permanent delivery failure for EVM nonce={}, skipping: {}",
                        message.nonce, err_str
                    );
                    let mut state = self.state.write().await;
                    state.processed_nonces.entry(evm_chain_id).or_default().insert(message.nonce);
                    continue;
                }
                self.record_delivery_failure(message, &err_str).await;
                continue;
            }

            {
                let mut state = self.state.write().await;
                state
                    .processed_nonces
                    .entry(evm_chain_id)
                    .or_default()
                    .insert(message.nonce);
            }
        }

        {
            self.state.write().await.evm_last_blocks.insert(evm_chain_id, to_block);
        }

        Ok(())
    }

    /// Deliver message to SVM chain via integrated-gmp-endpoint DeliverMessage instruction.
    ///
    /// Builds and submits a DeliverMessage transaction to the SVM integrated-gmp-endpoint program.
    /// For IntentRequirements messages (0x01), also derives and passes the outflow-validator
    /// accounts needed for GmpReceive CPI.
    async fn deliver_to_svm(&self, message: &GmpMessage, svm_chain: &SvmRelayChainConfig) -> Result<()> {
        let rpc_url = &svm_chain.rpc_url;

        let program_id_str = svm_chain.gmp_program_id.as_ref()
            .ok_or_else(|| anyhow::anyhow!("SVM GMP program ID not configured for chain {}", svm_chain.chain_id))?;

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
        let outflow_program = if let Some(ref outflow_id) = svm_chain.outflow_program_id {
            Pubkey::from_str(outflow_id).context("Invalid SVM outflow program ID")?
        } else {
            // If no outflow configured, use dst_program as placeholder (routing won't be used)
            dst_program
        };

        // Get intent_escrow program for destination_program_2 (required for routing)
        let escrow_program = if let Some(ref escrow_id) = svm_chain.escrow_program_id {
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

/// Convert hex string (with or without 0x prefix) to bytes.
pub fn hex_to_bytes(hex_str: &str) -> Result<Vec<u8>> {
    let hex_clean = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    hex::decode(hex_clean).context("Invalid hex string")
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


