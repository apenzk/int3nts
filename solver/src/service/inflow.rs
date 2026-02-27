//! Inflow Fulfillment Service
//!
//! Monitors escrow deposits on connected chains and fulfills inflow intents on the hub chain.
//!
//! Flow (GMP for MVM/EVM/SVM):
//! 1. **Monitor Escrows**: Poll hub chain for `is_escrow_confirmed` (GMP EscrowConfirmation received)
//! 2. **Fulfill Intent**: Call hub chain `fulfill_inflow_intent` when escrow is confirmed
//! 3. **Wait for Auto-Release**: Poll connected chain for `is_released` (escrow auto-releases
//!    when FulfillmentProof is received via GMP - no manual release call needed)

use crate::chains::{ConnectedEvmClient, ConnectedMvmClient, ConnectedSvmClient, HubChainClient};
use crate::config::SolverConfig;
use crate::service::liquidity::LiquidityMonitor;
use crate::service::tracker::{IntentTracker, TrackedIntent};
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

/// Inflow fulfillment service that monitors escrows and fulfills intents
pub struct InflowService {
    /// Solver configuration
    config: SolverConfig,
    /// Intent tracker for tracking signed intents (shared with other services)
    tracker: Arc<IntentTracker>,
    /// Hub chain client for querying escrow confirmation state
    hub_client: HubChainClient,
    /// Optional connected MVM chain client
    mvm_client: Option<ConnectedMvmClient>,
    /// Optional connected EVM chain client
    evm_client: Option<ConnectedEvmClient>,
    /// Optional connected SVM chain client
    svm_client: Option<ConnectedSvmClient>,
    /// Liquidity monitor for releasing budget after fulfillment
    liquidity_monitor: Arc<LiquidityMonitor>,
}

/// Helper struct for matching escrow events to intents
struct EscrowMatch {
    intent_id: String,
    escrow_id: String,
}

impl InflowService {
    /// Creates a new inflow fulfillment service
    ///
    /// # Arguments
    ///
    /// * `config` - Solver configuration
    /// * `tracker` - Shared intent tracker instance
    ///
    /// # Returns
    ///
    /// * `Ok(InflowService)` - Successfully created service
    /// * `Err(anyhow::Error)` - Failed to create service
    pub fn new(
        config: SolverConfig,
        tracker: Arc<IntentTracker>,
        liquidity_monitor: Arc<LiquidityMonitor>,
    ) -> Result<Self> {
        let hub_client = HubChainClient::new(&config.hub_chain)?;

        // Create connected chain clients for all configured chains
        let mvm_client = config
            .get_mvm_config()
            .map(|cfg| ConnectedMvmClient::new(cfg))
            .transpose()?;

        let evm_client = config
            .get_evm_config()
            .map(|cfg| ConnectedEvmClient::new(cfg))
            .transpose()?;

        let svm_client = config
            .get_svm_config()
            .map(|cfg| ConnectedSvmClient::new(cfg))
            .transpose()?;

        Ok(Self {
            config,
            tracker,
            hub_client,
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

    /// Polls for confirmed escrows matching tracked inflow intents.
    ///
    /// For MVM: checks hub chain `gmp_intent_state::is_escrow_confirmed` (GMP flow).
    /// For EVM/SVM: queries connected chain for escrow creation events.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(TrackedIntent, String)>)` - List of (intent, escrow_id) pairs with confirmed escrows
    /// * `Err(anyhow::Error)` - Failed to poll escrows
    pub async fn poll_for_escrows(&self) -> Result<Vec<(TrackedIntent, String)>> {
        // Get pending inflow intents (Created state, desired_chain_id == hub_chain_id)
        let pending_intents = self
            .tracker
            .get_intents_ready_for_fulfillment(Some(true))
            .await;

        if pending_intents.is_empty() {
            // Debug: check if there are any Created intents at all
            let all_created = self.tracker.get_intents_ready_for_fulfillment(None).await;
            if !all_created.is_empty() {
                for intent in &all_created {
                    let hub_chain_id = self.config.hub_chain.chain_id;
                    let is_inflow = intent.draft_data.desired_chain_id == hub_chain_id;
                    info!(
                        "Inflow poll: Intent {} in Created state: is_inflow={}, offered_chain={}, desired_chain={}",
                        intent.intent_id, is_inflow,
                        intent.draft_data.offered_chain_id, intent.draft_data.desired_chain_id
                    );
                }
            }
            return Ok(Vec::new());
        }

        let mut matched_intents = Vec::new();

        // Check MVM intents via hub chain is_escrow_confirmed (GMP flow)
        if self.mvm_client.is_some() {
            let mvm_chain_id = self.get_chain_id("mvm");
            for intent in &pending_intents {
                if Some(intent.draft_data.offered_chain_id) != mvm_chain_id {
                    continue;
                }
                match self.hub_client.is_escrow_confirmed(&intent.intent_id).await {
                    Ok(true) => {
                        info!(
                            "Escrow confirmed on hub for MVM inflow intent {}",
                            intent.intent_id
                        );
                        // Use intent_id as escrow_id for GMP flow
                        matched_intents.push((intent.clone(), intent.intent_id.clone()));
                    }
                    Ok(false) => {
                        // Not yet confirmed, skip
                    }
                    Err(e) => {
                        error!(
                            "Failed to check escrow confirmation for intent {}: {}",
                            intent.intent_id, e
                        );
                        return Err(e.context(format!(
                            "Failed to check escrow confirmation for intent {}",
                            intent.intent_id
                        )));
                    }
                }
            }
        }

        // Query EVM chain for escrow events if configured
        let mut evm_svm_escrow_events: Vec<EscrowMatch> = Vec::new();

        if let Some(client) = &self.evm_client {
            match client.get_block_number().await {
                Ok(current_block) => {
                    // Alchemy free tier limits eth_getLogs to 10-block range; cap to 9
                    let from_block = if current_block > 9 {
                        current_block - 9
                    } else {
                        0
                    };

                    info!(
                        "Querying EVM chain for escrow events (from_block={}, current_block={})",
                        from_block, current_block
                    );
                    match client.get_escrow_events(Some(from_block), Some(current_block)).await {
                        Ok(events) => {
                            if !events.is_empty() {
                                info!("Found {} EVM escrow events", events.len());
                            }
                            evm_svm_escrow_events.extend(events.into_iter().map(|e| {
                                EscrowMatch {
                                    intent_id: e.intent_id,
                                    escrow_id: e.escrow_id,
                                }
                            }));
                        }
                        Err(e) => {
                            error!("Failed to query EVM escrow events: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to get EVM block number: {}", e);
                }
            }
        }

        // Query SVM chain for escrow events if configured
        if let Some(client) = &self.svm_client {
            match client.get_escrow_events().await {
                Ok(events) => {
                    if !events.is_empty() {
                        info!("Found {} SVM escrow events", events.len());
                    }
                    evm_svm_escrow_events.extend(events.into_iter().map(|e| EscrowMatch {
                        intent_id: e.intent_id,
                        escrow_id: e.escrow_id,
                    }));
                }
                Err(e) => {
                    error!("Failed to query SVM escrow events: {}", e);
                }
            }
        }

        // Match EVM/SVM escrow events to pending intents by intent_id
        if !evm_svm_escrow_events.is_empty() {
            info!(
                "Matching {} pending intents against {} EVM/SVM escrow events",
                pending_intents.len(),
                evm_svm_escrow_events.len()
            );
            for intent in &pending_intents {
                let intent_id_normalized = normalize_intent_id(&intent.intent_id);
                for escrow in evm_svm_escrow_events.iter() {
                    let escrow_intent_id_normalized = normalize_intent_id(&escrow.intent_id);
                    if escrow_intent_id_normalized == intent_id_normalized {
                        info!(
                            "Match found: intent {} matches escrow {}",
                            intent.intent_id, escrow.escrow_id
                        );
                        matched_intents.push((intent.clone(), escrow.escrow_id.clone()));
                        break;
                    }
                }
            }
        }

        if !matched_intents.is_empty() {
            info!("Matched {} intents with escrows", matched_intents.len());
        }

        Ok(matched_intents)
    }

    /// Fulfills an inflow intent on the hub chain
    ///
    /// Calls `fulfill_inflow_intent` on the hub chain to provide tokens
    /// to the requester. This should be called after detecting a matching escrow
    /// on the connected chain.
    ///
    /// # Arguments
    ///
    /// * `intent` - Tracked intent to fulfill
    /// * `payment_amount` - Amount of tokens to provide (should match desired_amount)
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction hash
    /// * `Err(anyhow::Error)` - Failed to fulfill intent
    pub fn fulfill_inflow_intent(
        &self,
        intent: &TrackedIntent,
        payment_amount: u64,
    ) -> Result<String> {
        let intent_addr = intent
            .intent_addr
            .as_ref()
            .context("Intent address not set (intent not yet created on-chain)")?;

        self.hub_client
            .fulfill_inflow_intent(intent_addr, payment_amount)
    }

    /// Runs the inflow fulfillment service loop
    ///
    /// This function continuously:
    /// 1. Polls for escrows matching tracked inflow intents
    /// 2. Fulfills intents on hub chain when escrows are detected
    /// 3. Releases escrows after getting fulfillment confirmation
    ///
    /// The loop runs at the configured polling interval.
    pub async fn run(&self) -> Result<()> {
        let polling_interval = Duration::from_millis(self.config.service.polling_interval_ms);
        info!(
            "Inflow fulfillment service started (polling every {:?})",
            polling_interval
        );

        loop {
            match self.poll_for_escrows().await {
                Ok(intents_with_escrows) => {
                    for (intent, escrow_id) in intents_with_escrows {
                        info!(
                            "Found escrow {} for inflow intent: {}",
                            escrow_id, intent.intent_id
                        );

                        // Fulfill intent on hub chain
                        match self
                            .fulfill_inflow_intent(&intent, intent.draft_data.desired_amount)
                        {
                            Ok(tx_hash) => {
                                info!(
                                    "Successfully fulfilled inflow intent {} on hub chain: {}",
                                    intent.intent_id, tx_hash
                                );
                                // Mark intent as fulfilled IMMEDIATELY after successful fulfillment
                                // This prevents retrying fulfillment on next poll
                                if let Err(e) =
                                    self.tracker.mark_fulfilled(&intent.draft_id).await
                                {
                                    warn!("Failed to mark intent as fulfilled: {}", e);
                                }
                                // Release liquidity budget for this draft
                                self.liquidity_monitor.release(&intent.draft_id).await;
                            }
                            Err(e) => {
                                let msg = e.to_string();
                                if msg.contains("E_ESCROW_NOT_CONFIRMED") {
                                    warn!(
                                        "Inflow intent {} not yet confirmed on hub (will retry): {}",
                                        intent.intent_id, e
                                    );
                                } else {
                                    error!(
                                        "Failed to fulfill inflow intent {}: {}",
                                        intent.intent_id, e
                                    );
                                }
                                continue;
                            }
                        }

                        // GMP auto-release: tokens are transferred to solver automatically when
                        // FulfillmentProof arrives on connected chain. No action needed from solver.
                        // The solver can immediately move on to the next intent.
                    }
                }
                Err(e) => {
                    error!("Failed to poll for escrows: {}", e);
                }
            }

            tokio::time::sleep(polling_interval).await;
        }
    }
}

/// Normalize intent ID for comparison (strip 0x prefix, remove leading zeros, lowercase)
fn normalize_intent_id(intent_id: &str) -> String {
    let stripped = intent_id.strip_prefix("0x").unwrap_or(intent_id);
    // Remove leading zeros
    let trimmed = stripped.trim_start_matches('0');
    // If all zeros, keep at least one zero
    let hex_part = if trimmed.is_empty() { "0" } else { trimmed };
    format!("0x{}", hex_part.to_lowercase())
}
