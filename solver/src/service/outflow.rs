//! Outflow Fulfillment Service
//!
//! Executes fulfillments on connected chains via the GMP flow.
//!
//! GMP Flow (MVM connected chain - Commit 11):
//! 1. Hub creates intent → sends IntentRequirements via GMP to connected chain
//! 2. Native GMP relay delivers requirements to connected chain's outflow_validator
//! 3. Solver calls `outflow_validator::fulfill_intent` on connected chain
//! 4. outflow_validator transfers tokens and sends FulfillmentProof via GMP
//! 5. Native GMP relay delivers FulfillmentProof to hub
//! 6. Hub auto-releases tokens to solver
//!
//! EVM/SVM: Will be updated to GMP flow in Commits 12-13.

use crate::chains::{ConnectedEvmClient, ConnectedMvmClient, ConnectedSvmClient, HubChainClient};
use crate::config::SolverConfig;
use crate::service::tracker::{IntentTracker, TrackedIntent};
use crate::coordinator_gmp_client::{ValidateOutflowFulfillmentRequest, CoordinatorGmpClient};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
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
    /// and executes fulfillments on the connected chain.
    ///
    /// For MVM chains, uses the GMP flow (outflow_validator::fulfill_intent).
    /// For EVM/SVM chains, uses direct transfer (will be updated to GMP in Commits 12-13).
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(TrackedIntent, String, bool)>)` - List of (intent, transaction_hash, uses_gmp) tuples
    /// * `Err(anyhow::Error)` - Failed to execute transfers
    pub async fn poll_and_execute_transfers(&self) -> Result<Vec<(TrackedIntent, String, bool)>> {
        // Get pending outflow intents (Created state, offered_chain_id == hub_chain_id)
        let pending_intents = self
            .tracker
            .get_intents_ready_for_fulfillment(Some(false))
            .await;

        if pending_intents.is_empty() {
            return Ok(Vec::new());
        }

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

            if let Err(e) = self.tracker.mark_outflow_attempted(&intent.intent_id).await {
                error!(
                    "Failed to mark outflow intent {} as attempted: {}",
                    intent.intent_id, e
                );
                continue;
            }

            // Execute fulfillment on connected chain
            let (tx_hash, uses_gmp) = match self.execute_connected_transfer(&intent, &requester_addr_connected_chain).await {
                Ok(result) => result,
                Err(e) => {
                    error!("Failed to execute fulfillment for intent {}: {}", intent.intent_id, e);
                    continue;
                }
            };

            if uses_gmp {
                info!("Executed GMP outflow fulfillment for intent {}: tx_hash={}", intent.intent_id, tx_hash);
            } else {
                info!("Executed outflow transfer for intent {}: tx_hash={}", intent.intent_id, tx_hash);
            }
            executed_transfers.push((intent, tx_hash, uses_gmp));
        }

        Ok(executed_transfers)
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
    /// * `Ok((String, bool))` - (transaction_hash, uses_gmp_flow)
    /// * `Err(anyhow::Error)` - Failed to execute transfer
    async fn execute_connected_transfer(
        &self,
        intent: &TrackedIntent,
        recipient: &str,
    ) -> Result<(String, bool)> {
        let desired_token = &intent.draft_data.desired_token;
        let desired_amount = intent.draft_data.desired_amount;

        // Determine target chain based on intent's desired_chain_id
        let (chain_type, _) = self.get_target_chain_for_intent(intent)
            .context("No configured connected chain matches intent's desired_chain_id")?;

        match chain_type {
            "mvm" => {
                // GMP Flow: Call outflow_validator::fulfill_intent
                // The outflow_validator will:
                // 1. Validate solver is authorized
                // 2. Transfer tokens from solver to recipient
                // 3. Send FulfillmentProof via GMP to hub
                // The hub will auto-release tokens when it receives the proof
                let client = self.mvm_client.as_ref()
                    .context("MVM client not configured")?;
                let tx_hash = client.fulfill_outflow_via_gmp(&intent.intent_id, desired_token)?;
                Ok((tx_hash, true)) // true = uses GMP flow
            }
            "evm" => {
                // TODO(Commit 12): Update to GMP flow
                let client = self.evm_client.as_ref()
                    .context("EVM client not configured")?;
                let tx_hash = client.transfer_with_intent_id(desired_token, recipient, desired_amount, &intent.intent_id).await?;
                Ok((tx_hash, false)) // false = not using GMP yet
            }
            "svm" => {
                // GMP Flow: Call outflow_validator::fulfill_intent
                // The outflow_validator will:
                // 1. Validate solver is authorized
                // 2. Transfer tokens from solver to recipient
                // 3. Send FulfillmentProof via GMP to hub
                // The hub will auto-release tokens when it receives the proof
                let client = self.svm_client.as_ref()
                    .context("SVM client not configured")?;
                let tx_hash = client.fulfill_outflow_via_gmp(&intent.intent_id, desired_token).await?;
                Ok((tx_hash, true)) // true = uses GMP flow
            }
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

    /// Gets trusted-gmp approval for an outflow fulfillment transaction
    ///
    /// # Arguments
    ///
    /// * `transaction_hash` - Transaction hash on connected chain
    /// * `chain_type` - Chain type: "mvm" or "evm"
    /// * `intent_id` - Intent ID
    ///
    /// # Returns
    ///
    /// * `Ok(ApprovalSignature)` - Trusted-gmp approval signature
    /// * `Err(anyhow::Error)` - Failed to get approval
    pub async fn get_trusted_gmp_approval(
        &self,
        transaction_hash: &str,
        chain_type: &str,
        intent_id: &str,
    ) -> Result<Vec<u8>> {
        let request = ValidateOutflowFulfillmentRequest {
            transaction_hash: transaction_hash.to_string(),
            chain_type: chain_type.to_string(),
            intent_id: Some(intent_id.to_string()),
        };

        // Call trusted-gmp API for validation (blocking call)
        let response = tokio::task::spawn_blocking({
            let base_url = self.config.service.trusted_gmp_url.clone();
            let request = request.clone();
            move || {
                let client = CoordinatorGmpClient::new(&base_url);
                client.validate_outflow_fulfillment(&request)
            }
        })
        .await
        .context("Failed to spawn blocking task for trusted-gmp approval")??;

        if !response.validation.valid {
            anyhow::bail!(
                "Trusted-gmp validation failed: {}",
                response.validation.message
            );
        }

        let signature = response
            .approval_signature
            .context("Missing approval signature in valid response")?;

        // Decode base64 signature to bytes
        let signature_bytes = STANDARD
            .decode(&signature.signature)
            .context("Failed to decode base64 signature")?;

        Ok(signature_bytes)
    }

    /// Fulfills an outflow intent on the hub chain with trusted-gmp approval
    ///
    /// # Arguments
    ///
    /// * `intent` - Tracked intent to fulfill
    /// * `approval_signature_bytes` - Trusted-gmp's Ed25519 signature as bytes (on-chain approval address)
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction hash
    /// * `Err(anyhow::Error)` - Failed to fulfill intent
    pub async fn fulfill_outflow_intent(
        &self,
        intent: &TrackedIntent,
        approval_signature_bytes: &[u8],
    ) -> Result<String> {
        let intent_addr = intent
            .intent_addr
            .as_ref()
            .context("Intent address not set (intent not created on-chain)")?;

        // Execute fulfillment (blocking call)
        tokio::task::spawn_blocking({
            let intent_addr = intent_addr.clone();
            let signature = approval_signature_bytes.to_vec();
            let hub_config = self.config.hub_chain.clone();
            move || {
                let hub_client = HubChainClient::new(&hub_config)?;
                hub_client.fulfill_outflow_intent(&intent_addr, &signature)
            }
        })
        .await
        .context("Failed to spawn blocking task for hub fulfillment")?
    }

    /// Main service loop that continuously processes outflow intents
    ///
    /// This loop:
    /// 1. Polls for pending outflow intents and executes fulfillments
    /// 2. For GMP flow (MVM): Marks as fulfilled (hub release happens via GMP)
    /// 3. For EVM/SVM (pending GMP update): Gets trusted-gmp approval and fulfills hub intent
    ///
    /// # Arguments
    ///
    /// * `polling_interval` - Interval between polling cycles
    pub async fn run(&self, polling_interval: Duration) {
        info!("Outflow fulfillment service started");

        loop {
            match self.poll_and_execute_transfers().await {
                Ok(executed_transfers) => {
                    for (intent, tx_hash, uses_gmp) in executed_transfers {
                        if uses_gmp {
                            // GMP Flow: The outflow_validator sent FulfillmentProof via GMP.
                            // The native GMP relay will deliver it to the hub, which auto-releases tokens.
                            // We just need to mark the intent as fulfilled in our tracker.
                            info!(
                                "Successfully fulfilled outflow intent {} via GMP: fulfill_tx={}",
                                intent.intent_id, tx_hash
                            );
                            info!(
                                "Hub will auto-release tokens when FulfillmentProof is delivered via GMP"
                            );
                            if let Err(e) = self.tracker.mark_fulfilled(&intent.draft_id).await {
                                error!("Failed to mark intent {} as fulfilled: {}", intent.draft_id, e);
                            }
                            continue;
                        }

                        // EVM/SVM (pending GMP update): Get trusted-gmp approval and fulfill on hub
                        let chain_type = match self.get_target_chain_for_intent(&intent) {
                            Some((ct, _)) => ct,
                            None => {
                                error!("No matching connected chain for intent {}", intent.intent_id);
                                continue;
                            }
                        };

                        match self.get_trusted_gmp_approval(&tx_hash, chain_type, &intent.intent_id).await {
                            Ok(signature_bytes) => {
                                // Fulfill hub intent
                                match self.fulfill_outflow_intent(&intent, &signature_bytes).await {
                                    Ok(fulfill_tx_hash) => {
                                        info!(
                                            "Successfully fulfilled outflow intent {}: fulfill_tx={}",
                                            intent.intent_id, fulfill_tx_hash
                                        );
                                        // Mark intent as fulfilled
                                        if let Err(e) = self.tracker.mark_fulfilled(&intent.draft_id).await {
                                            error!("Failed to mark intent {} as fulfilled: {}", intent.draft_id, e);
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to fulfill outflow intent {}: {}", intent.intent_id, e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to get trusted-gmp approval for intent {}: {}", intent.intent_id, e);
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

