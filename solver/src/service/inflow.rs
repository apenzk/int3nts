//! Inflow Fulfillment Service
//!
//! Monitors escrow deposits on connected chains and fulfills inflow intents on the hub chain.
//!
//! Flow (GMP for MVM):
//! 1. **Monitor Escrows**: Poll hub chain for `is_escrow_confirmed` (GMP EscrowConfirmation received)
//! 2. **Fulfill Intent**: Call hub chain `fulfill_inflow_intent` when escrow is confirmed
//! 3. **Release Escrow**: Poll connected chain for `is_fulfilled` (GMP FulfillmentProof received),
//!    then call `release_gmp_escrow` on connected chain
//!
//! Flow (EVM/SVM):
//! 1. **Monitor Escrows**: Poll connected chain for escrow creation events
//! 2. **Fulfill Intent**: Call hub chain `fulfill_inflow_intent` when escrow is detected
//! 3. **Release Escrow**: Poll trusted-gmp for approval signature, then release escrow on connected chain

use crate::chains::{ConnectedEvmClient, ConnectedMvmClient, ConnectedSvmClient, HubChainClient};
use crate::config::SolverConfig;
use crate::coordinator_gmp_client::CoordinatorGmpClient;
use crate::service::tracker::{IntentTracker, TrackedIntent};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
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
    /// Trusted GMP base URL for approval polling (used by EVM/SVM)
    trusted_gmp_url: String,
    /// Optional connected MVM chain client
    mvm_client: Option<ConnectedMvmClient>,
    /// Optional connected EVM chain client
    evm_client: Option<ConnectedEvmClient>,
    /// Optional connected SVM chain client
    svm_client: Option<ConnectedSvmClient>,
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
    pub fn new(config: SolverConfig, tracker: Arc<IntentTracker>) -> Result<Self> {
        let trusted_gmp_url = config.service.trusted_gmp_url.clone();
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
            trusted_gmp_url,
            mvm_client,
            evm_client,
            svm_client,
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

    /// Determines which connected chain to use for an inflow intent (based on offered_chain_id)
    /// Returns ("mvm"|"evm"|"svm", chain_id) or None if no matching chain
    fn get_source_chain_for_intent(&self, intent: &TrackedIntent) -> Option<(&'static str, u64)> {
        let offered_chain_id = intent.draft_data.offered_chain_id;

        if let Some(chain_id) = self.get_chain_id("mvm") {
            if chain_id == offered_chain_id {
                return Some(("mvm", chain_id));
            }
        }
        if let Some(chain_id) = self.get_chain_id("evm") {
            if chain_id == offered_chain_id {
                return Some(("evm", chain_id));
            }
        }
        if let Some(chain_id) = self.get_chain_id("svm") {
            if chain_id == offered_chain_id {
                return Some(("svm", chain_id));
            }
        }
        None
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
                    // Look back 200 blocks (~7 minutes on Base, same as trusted-gmp)
                    let from_block = if current_block > 200 {
                        current_block - 200
                    } else {
                        0
                    };

                    info!(
                        "Querying EVM chain for escrow events (from_block={}, current_block={})",
                        from_block, current_block
                    );
                    match client.get_escrow_events(Some(from_block), None).await {
                        Ok(events) => {
                            if !events.is_empty() {
                                info!("Found {} EVM escrow events", events.len());
                            }
                            evm_svm_escrow_events.extend(events.into_iter().map(|e| {
                                EscrowMatch {
                                    intent_id: e.intent_id,
                                    escrow_id: e.escrow_addr,
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

    /// Releases an escrow on the connected chain after fulfillment.
    ///
    /// For MVM (GMP flow):
    ///   1. Polls connected chain `is_fulfilled` until FulfillmentProof is received via GMP
    ///   2. Calls `inflow_escrow_gmp::release_escrow` (no signature needed)
    ///
    /// For EVM/SVM (trusted-gmp flow):
    ///   1. Polls trusted-gmp for approval signature
    ///   2. Calls escrow claim function with signature
    ///
    /// # Arguments
    ///
    /// * `intent` - Tracked intent with matching escrow
    /// * `escrow_id` - Escrow identifier (intent_id for MVM, contract address for EVM/SVM)
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction hash
    /// * `Err(anyhow::Error)` - Failed to release escrow
    pub async fn release_escrow(
        &self,
        intent: &TrackedIntent,
        escrow_id: &str,
    ) -> Result<String> {
        let (chain_type, _) = self
            .get_source_chain_for_intent(intent)
            .context("No configured connected chain matches intent's offered_chain_id")?;

        // MVM uses GMP flow: poll connected chain for fulfillment, then release directly
        if chain_type == "mvm" {
            return self.release_mvm_gmp_escrow(intent).await;
        }

        // EVM/SVM use trusted-gmp approval flow
        let trusted_gmp_url = self.trusted_gmp_url.clone();
        let intent_id_normalized = normalize_intent_id(&intent.intent_id);
        let poll_interval = Duration::from_secs(2);
        let expiry_time = intent.expiry_time;

        let approval = loop {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if current_time >= expiry_time {
                anyhow::bail!(
                    "Escrow expired while waiting for approval (expiry: {})",
                    expiry_time
                );
            }

            let approvals = tokio::task::spawn_blocking({
                let trusted_gmp_url = trusted_gmp_url.clone();
                move || {
                    let client = CoordinatorGmpClient::new(&trusted_gmp_url);
                    client.get_approvals()
                }
            })
            .await
            .context("Failed to spawn blocking task")?
            .context("Failed to get approvals")?;

            // Find approval matching this intent_id
            if let Some(approval) = approvals.iter().find(|approval| {
                let approval_intent_id_normalized = normalize_intent_id(&approval.intent_id);
                approval_intent_id_normalized == intent_id_normalized
            }) {
                info!("Found approval for intent {}", intent.intent_id);
                break approval.clone();
            }

            // Approval not found yet, wait and retry
            tokio::time::sleep(poll_interval).await;
        };

        // Decode base64 signature to bytes
        let signature_bytes = STANDARD
            .decode(&approval.signature)
            .context("Failed to decode base64 signature")?;

        match chain_type {
            "evm" => {
                let client = self.evm_client.as_ref().context("EVM client not configured")?;
                client
                    .claim_escrow(escrow_id, &intent.intent_id, &signature_bytes)
                    .await
            }
            "svm" => {
                let client = self.svm_client.as_ref().context("SVM client not configured")?;
                client
                    .claim_escrow(escrow_id, &intent.intent_id, &signature_bytes)
                    .await
            }
            _ => anyhow::bail!("Unknown chain type: {}", chain_type),
        }
    }

    /// Releases an MVM inflow escrow via GMP flow.
    ///
    /// Polls the connected chain `inflow_escrow_gmp::is_fulfilled` until the hub's
    /// FulfillmentProof GMP message is received, then calls `release_escrow` on the
    /// connected chain to transfer locked tokens to the solver.
    async fn release_mvm_gmp_escrow(&self, intent: &TrackedIntent) -> Result<String> {
        let client = self
            .mvm_client
            .as_ref()
            .context("MVM client not configured")?;
        let poll_interval = Duration::from_secs(2);
        let expiry_time = intent.expiry_time;

        // Poll connected chain until FulfillmentProof is received via GMP
        loop {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if current_time >= expiry_time {
                anyhow::bail!(
                    "Escrow expired while waiting for fulfillment proof (expiry: {})",
                    expiry_time
                );
            }

            match client.is_escrow_fulfilled(&intent.intent_id).await {
                Ok(true) => {
                    info!(
                        "FulfillmentProof received on connected chain for intent {}, releasing escrow",
                        intent.intent_id
                    );
                    break;
                }
                Ok(false) => {
                    // Not yet fulfilled, wait and retry
                }
                Err(e) => {
                    return Err(e.context(format!(
                        "Failed to check escrow fulfillment for intent {}",
                        intent.intent_id
                    )));
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        // Release escrow - no signature needed in GMP flow
        // offered_token is the token escrowed on the connected chain for inflow
        client.release_gmp_escrow(&intent.intent_id, &intent.draft_data.offered_token)
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
                            }
                            Err(e) => {
                                error!(
                                    "Failed to fulfill inflow intent {}: {}",
                                    intent.intent_id, e
                                );
                                continue;
                            }
                        }

                        // Release escrow after a delay (wait for GMP/trusted-gmp processing)
                        tokio::time::sleep(Duration::from_secs(2)).await;

                        match self.release_escrow(&intent, &escrow_id).await {
                            Ok(tx_hash) => {
                                info!(
                                    "Released escrow {} for intent {}: {}",
                                    escrow_id, intent.intent_id, tx_hash
                                );
                            }
                            Err(e) => {
                                error!(
                                    "Failed to release escrow {} for intent {}: {}",
                                    escrow_id, intent.intent_id, e
                                );
                            }
                        }
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
