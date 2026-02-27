//! Outflow Fulfillment Service
//!
//! Executes fulfillments on connected chains via the integrated GMP flow.
//!
//! GMP Flow (all chains: MVM, EVM, SVM):
//! 1. Hub creates intent → sends IntentRequirements via GMP to connected chain
//! 2. Integrated GMP relay delivers requirements to connected chain's outflow_validator
//! 3. Solver calls `outflow_validator::fulfill_intent` on connected chain
//! 4. outflow_validator transfers tokens and sends FulfillmentProof via GMP
//! 5. Integrated GMP relay delivers FulfillmentProof to hub
//! 6. Solver calls fulfill_outflow_intent on hub to claim locked tokens

use crate::chains::{ConnectedEvmClient, ConnectedMvmClient, ConnectedSvmClient, HubChainClient};
use crate::config::SolverConfig;
use crate::service::liquidity::LiquidityMonitor;
use crate::service::tracker::{IntentTracker, TrackedIntent};
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

/// Outflow fulfillment service that executes transfers and fulfills intents
pub struct OutflowService {
    /// Solver configuration
    config: SolverConfig,
    /// Intent tracker for tracking signed intents (shared with other services)
    tracker: Arc<IntentTracker>,
    /// Optional connected MVM chain client
    mvm_client: Option<ConnectedMvmClient>,
    /// Optional connected EVM chain client
    evm_client: Option<ConnectedEvmClient>,
    /// Optional connected SVM chain client
    svm_client: Option<ConnectedSvmClient>,
    /// Liquidity monitor for releasing budget after fulfillment
    liquidity_monitor: Arc<LiquidityMonitor>,
}

impl OutflowService {
    /// Creates a new outflow fulfillment service
    ///
    /// # Arguments
    ///
    /// * `config` - Solver configuration
    /// * `tracker` - Shared intent tracker instance
    ///
    /// # Returns
    ///
    /// * `Ok(OutflowService)` - Successfully created service
    /// * `Err(anyhow::Error)` - Failed to create service
    pub fn new(
        config: SolverConfig,
        tracker: Arc<IntentTracker>,
        liquidity_monitor: Arc<LiquidityMonitor>,
    ) -> Result<Self> {
        // Create connected chain clients for all configured chains
        let mvm_client = config.get_mvm_config()
            .map(|cfg| ConnectedMvmClient::new(cfg))
            .transpose()?;

        let evm_client = config.get_evm_config()
            .map(|cfg| ConnectedEvmClient::new(cfg))
            .transpose()?;

        let svm_client = config.get_svm_config()
            .map(|cfg| ConnectedSvmClient::new(cfg))
            .transpose()?;

        Ok(Self {
            config,
            tracker,
            mvm_client,
            evm_client,
            svm_client,
            liquidity_monitor,
        })
    }
    
    /// Gets the chain ID for a connected chain type
    fn get_chain_id(&self, chain_type: &str) -> Option<u64> {
        match chain_type {
            "mvm" => self.config.get_mvm_config().map(|c| c.chain_id),
            "evm" => self.config.get_evm_config().map(|c| c.chain_id),
            "svm" => self.config.get_svm_config().map(|c| c.chain_id),
            _ => None,
        }
    }
    
    /// Determines which connected chain to use for an outflow intent
    /// Returns ("mvm"|"evm"|"svm", chain_id) or None if no matching chain
    fn get_target_chain_for_intent(&self, intent: &TrackedIntent) -> Option<(&'static str, u64)> {
        let desired_chain_id = intent.draft_data.desired_chain_id;
        
        if let Some(chain_id) = self.get_chain_id("mvm") {
            if chain_id == desired_chain_id {
                return Some(("mvm", chain_id));
            }
        }
        if let Some(chain_id) = self.get_chain_id("evm") {
            if chain_id == desired_chain_id {
                return Some(("evm", chain_id));
            }
        }
        if let Some(chain_id) = self.get_chain_id("svm") {
            if chain_id == desired_chain_id {
                return Some(("svm", chain_id));
            }
        }
        None
    }

    /// Polls for pending outflow intents and executes fulfillments on connected chain
    ///
    /// This function queries the tracker for pending outflow intents (Created state, offered_chain_id == hub_chain_id)
    /// and executes fulfillments on the connected chain via GMP.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(TrackedIntent, String)>)` - List of (intent, transaction_hash) tuples
    /// * `Err(anyhow::Error)` - Failed to execute transfers
    pub async fn poll_and_execute_transfers(&self) -> Result<Vec<(TrackedIntent, String)>> {
        // Get pending outflow intents (Created state, offered_chain_id == hub_chain_id)
        let pending_intents = self
            .tracker
            .get_intents_ready_for_fulfillment(Some(false))
            .await;

        if pending_intents.is_empty() {
            // Log tracker state periodically at trace level for debugging
            let all_intents = self.tracker.get_intents_ready_for_fulfillment(None).await;
            if !all_intents.is_empty() {
                tracing::debug!(
                    "Outflow poll: 0 outflow intents found, but {} total intents in Created state (hub_chain_id={})",
                    all_intents.len(),
                    self.config.hub_chain.chain_id
                );
                for i in &all_intents {
                    tracing::debug!(
                        "  Intent {}: offered_chain_id={}, desired_chain_id={}, state={:?}",
                        i.intent_id, i.draft_data.offered_chain_id, i.draft_data.desired_chain_id, i.state
                    );
                }
            }
            return Ok(Vec::new());
        }

        info!("Found {} pending outflow intent(s)", pending_intents.len());
        let mut executed_transfers = Vec::new();

        for intent in pending_intents {
            if intent.outflow_attempted {
                warn!(
                    "Skipping outflow intent {}: transfer already attempted",
                    intent.intent_id
                );
                continue;
            }

            // Get requester_addr_connected_chain from intent
            let requester_addr_connected_chain = match self.get_requester_address_connected_chain(&intent).await {
                Ok(addr) => addr,
                Err(e) => {
                    warn!("Failed to get requester_addr_connected_chain for intent {}: {}", intent.intent_id, e);
                    continue;
                }
            };

            // Execute fulfillment on connected chain via GMP
            let tx_hash = match self.execute_connected_transfer(&intent, &requester_addr_connected_chain).await {
                Ok(hash) => hash,
                Err(e) => {
                    error!("Failed to execute fulfillment for intent {}: {}", intent.intent_id, e);
                    // Don't mark as attempted - allow retry on next poll
                    continue;
                }
            };

            // Mark as attempted only AFTER successful transfer to prevent duplicate transfers
            // but allow retries if the transfer failed (e.g., requirements not yet delivered via GMP)
            if let Err(e) = self.tracker.mark_outflow_attempted(&intent.intent_id).await {
                error!(
                    "Failed to mark outflow intent {} as attempted: {}",
                    intent.intent_id, e
                );
                // Continue anyway - transfer already succeeded
            }

            info!("Executed GMP outflow fulfillment for intent {}: tx_hash={}", intent.intent_id, tx_hash);
            executed_transfers.push((intent, tx_hash));
        }

        Ok(executed_transfers)
    }

    /// Waits for GMP IntentRequirements to arrive on the MVM connected chain,
    /// then executes `outflow_validator::fulfill_intent`.
    ///
    /// The hub sends IntentRequirements via GMP when the outflow intent is created.
    /// The integrated GMP relay delivers them to the connected chain's `outflow_validator_impl`.
    /// This function polls `has_outflow_requirements` until they arrive, then fulfills.
    async fn execute_mvm_gmp_fulfillment(
        &self,
        intent: &TrackedIntent,
    ) -> Result<String> {
        let client = self.mvm_client.as_ref()
            .context("MVM client not configured")?;
        let desired_token = &intent.draft_data.desired_token;
        let poll_interval = Duration::from_secs(2);
        let expiry_time = intent.expiry_time;

        // Poll until requirements are available on connected chain
        loop {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if current_time >= expiry_time {
                anyhow::bail!(
                    "Intent expired while waiting for GMP requirements delivery (expiry: {})",
                    expiry_time
                );
            }

            match client.has_outflow_requirements(&intent.intent_id).await {
                Ok(true) => {
                    info!(
                        "GMP requirements delivered for outflow intent {}, fulfilling on connected chain",
                        intent.intent_id
                    );
                    break;
                }
                Ok(false) => {
                    // Requirements not yet delivered, wait and retry
                }
                Err(e) => {
                    return Err(e.context(format!(
                        "Failed to check outflow requirements for intent {}",
                        intent.intent_id
                    )));
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        // Requirements are available, execute fulfillment
        client.fulfill_outflow_via_gmp(&intent.intent_id, desired_token)
    }

    /// Waits for GMP IntentRequirements to arrive on the SVM connected chain,
    /// then executes `outflow_validator::fulfill_intent`.
    ///
    /// Same pattern as `execute_mvm_gmp_fulfillment` but for SVM connected chains.
    async fn execute_svm_gmp_fulfillment(
        &self,
        intent: &TrackedIntent,
    ) -> Result<String> {
        let client = self.svm_client.as_ref()
            .context("SVM client not configured")?;
        let desired_token = &intent.draft_data.desired_token;
        let poll_interval = Duration::from_secs(2);
        let expiry_time = intent.expiry_time;

        // Poll until requirements are available on connected chain
        loop {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if current_time >= expiry_time {
                anyhow::bail!(
                    "Intent expired while waiting for GMP requirements delivery (expiry: {})",
                    expiry_time
                );
            }

            match client.has_outflow_requirements(&intent.intent_id) {
                Ok(true) => {
                    info!(
                        "GMP requirements delivered for outflow intent {}, fulfilling on SVM connected chain",
                        intent.intent_id
                    );
                    break;
                }
                Ok(false) => {
                    // Requirements not yet delivered, wait and retry
                }
                Err(e) => {
                    return Err(e.context(format!(
                        "Failed to check outflow requirements for intent {}",
                        intent.intent_id
                    )));
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        // Requirements are available, execute fulfillment
        client.fulfill_outflow_via_gmp(&intent.intent_id, desired_token).await
    }

    /// Waits for GMP IntentRequirements to arrive on the EVM connected chain,
    /// then executes `IntentOutflowValidator.fulfillIntent`.
    ///
    /// Same pattern as `execute_mvm_gmp_fulfillment` and `execute_svm_gmp_fulfillment`.
    async fn execute_evm_gmp_fulfillment(
        &self,
        intent: &TrackedIntent,
    ) -> Result<String> {
        let client = self.evm_client.as_ref()
            .context("EVM client not configured")?;
        let desired_token = &intent.draft_data.desired_token;
        let poll_interval = Duration::from_secs(2);
        let expiry_time = intent.expiry_time;

        // Poll until requirements are available on connected chain
        loop {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if current_time >= expiry_time {
                anyhow::bail!(
                    "Intent expired while waiting for GMP requirements delivery (expiry: {})",
                    expiry_time
                );
            }

            match client.has_outflow_requirements(&intent.intent_id).await {
                Ok(true) => {
                    info!(
                        "GMP requirements delivered for outflow intent {}, fulfilling on EVM connected chain",
                        intent.intent_id
                    );
                    break;
                }
                Ok(false) => {
                    // Requirements not yet delivered, wait and retry
                }
                Err(e) => {
                    return Err(e.context(format!(
                        "Failed to check outflow requirements for intent {}",
                        intent.intent_id
                    )));
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        // Requirements are available, execute fulfillment
        client.fulfill_outflow_via_gmp(&intent.intent_id, desired_token)
    }

    /// Waits for FulfillmentProof to be delivered to the hub via GMP, then calls
    /// `fulfill_outflow_intent` on the hub to claim locked tokens.
    ///
    /// After the solver fulfills on the connected chain, the connected chain sends a
    /// FulfillmentProof via GMP. The integrated GMP relay delivers it to the hub. Once the
    /// hub records the proof, the solver can call `fulfill_outflow_intent` to claim tokens.
    async fn wait_for_proof_and_fulfill_hub(
        &self,
        intent: &TrackedIntent,
    ) -> Result<String> {
        let intent_addr = intent.intent_addr.as_ref()
            .context("Intent address not set (intent not created on-chain)")?;
        let poll_interval = Duration::from_secs(2);
        let expiry_time = intent.expiry_time;

        // Poll until FulfillmentProof is received on hub
        loop {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if current_time >= expiry_time {
                anyhow::bail!(
                    "Intent expired while waiting for FulfillmentProof delivery to hub (expiry: {})",
                    expiry_time
                );
            }

            let hub_client = HubChainClient::new(&self.config.hub_chain)?;
            match hub_client.is_fulfillment_proof_received(&intent.intent_id).await {
                Ok(true) => {
                    info!(
                        "FulfillmentProof received on hub for outflow intent {}, claiming tokens",
                        intent.intent_id
                    );
                    break;
                }
                Ok(false) => {
                    // Proof not yet delivered, wait and retry
                }
                Err(e) => {
                    return Err(e.context(format!(
                        "Failed to check FulfillmentProof status for intent {}",
                        intent.intent_id
                    )));
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        // FulfillmentProof received, call fulfill_outflow_intent on hub
        let intent_addr_clone = intent_addr.clone();
        let hub_config = self.config.hub_chain.clone();
        tokio::task::spawn_blocking(move || {
            let hub_client = HubChainClient::new(&hub_config)?;
            hub_client.fulfill_outflow_intent_gmp(&intent_addr_clone)
        })
        .await
        .context("Failed to spawn blocking task for hub GMP fulfillment")?
    }

    /// Executes outflow fulfillment on the connected chain.
    ///
    /// For MVM chains, uses the GMP flow via outflow_validator::fulfill_intent.
    /// For EVM/SVM chains, uses direct transfer (will be updated to GMP in Commits 12-13).
    ///
    /// # Arguments
    ///
    /// * `intent` - Tracked intent to execute transfer for
    /// * `recipient` - Recipient address on connected chain (requester_addr_connected_chain)
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction hash from connected chain fulfillment
    /// * `Err(anyhow::Error)` - Failed to execute transfer
    async fn execute_connected_transfer(
        &self,
        intent: &TrackedIntent,
        _recipient: &str,
    ) -> Result<String> {
        // Determine target chain based on intent's desired_chain_id
        let (chain_type, _) = self.get_target_chain_for_intent(intent)
            .context("No configured connected chain matches intent's desired_chain_id")?;

        match chain_type {
            "mvm" => self.execute_mvm_gmp_fulfillment(intent).await,
            "evm" => self.execute_evm_gmp_fulfillment(intent).await,
            "svm" => self.execute_svm_gmp_fulfillment(intent).await,
            _ => anyhow::bail!("Unknown chain type: {}", chain_type),
        }
    }

    /// Gets the requester's address on the connected chain from the intent object
    ///
    /// # Arguments
    ///
    /// * `intent` - Tracked intent
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Requester address on connected chain
    /// * `Err(anyhow::Error)` - Failed to query intent object
    async fn get_requester_address_connected_chain(&self, intent: &TrackedIntent) -> Result<String> {
        // Get from tracked intent (set when on-chain event was detected)
        intent.requester_addr_connected_chain.clone()
            .context("requester_addr_connected_chain not set. This may happen if the intent is inflow (not outflow) or the event data didn't include this field.")
    }

    /// Main service loop that continuously processes outflow intents
    ///
    /// This loop:
    /// 1. Polls for pending outflow intents and executes fulfillments on connected chain
    /// 2. Waits for FulfillmentProof delivery via GMP, then claims tokens on hub
    ///
    /// All chains (MVM, EVM, SVM) use the integrated GMP flow.
    ///
    /// # Arguments
    ///
    /// * `polling_interval` - Interval between polling cycles
    pub async fn run(&self, polling_interval: Duration) {
        info!("Outflow fulfillment service started");

        loop {
            match self.poll_and_execute_transfers().await {
                Ok(executed_transfers) => {
                    for (intent, tx_hash) in executed_transfers {
                        // All chains use GMP: wait for FulfillmentProof, then claim on hub
                        info!(
                            "Connected chain fulfillment complete for outflow intent {} (tx={}), waiting for FulfillmentProof delivery to hub",
                            intent.intent_id, tx_hash
                        );
                        match self.wait_for_proof_and_fulfill_hub(&intent).await {
                            Ok(hub_tx_hash) => {
                                info!(
                                    "Successfully fulfilled outflow intent {} on hub: hub_tx={}",
                                    intent.intent_id, hub_tx_hash
                                );
                                if let Err(e) = self.tracker.mark_fulfilled(&intent.draft_id).await {
                                    error!("Failed to mark intent {} as fulfilled: {}", intent.draft_id, e);
                                }
                                // Release liquidity budget for this draft
                                self.liquidity_monitor.release(&intent.draft_id).await;
                            }
                            Err(e) => {
                                error!(
                                    "Failed to complete hub fulfillment for outflow intent {}: {}",
                                    intent.intent_id, e
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to poll and execute transfers: {}", e);
                }
            }

            tokio::time::sleep(polling_interval).await;
        }
    }
}

