//! Generic monitor structures and EventMonitor definition
//!
//! This module contains shared event structures and the EventMonitor struct definition
//! that are used across all flow types (inflow/outflow) and chain types (Move VM/EVM).

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Normalizes an intent ID by removing leading zeros after the 0x prefix and converting to lowercase.
///
/// This ensures that intent IDs like "0x0911..." and "0x911..." are treated as the same value.
///
/// # Arguments
///
/// * `intent_id` - The intent ID to normalize (e.g., "0x0911..." or "0x911...")
///
/// # Returns
///
/// * Normalized intent ID with 0x prefix, no leading zeros, lowercase (e.g., "0x911...")
pub fn normalize_intent_id(intent_id: &str) -> String {
    let stripped = intent_id.strip_prefix("0x").unwrap_or(intent_id);
    // Remove leading zeros
    let trimmed = stripped.trim_start_matches('0');
    // If all zeros, keep at least one zero
    let hex_part = if trimmed.is_empty() { "0" } else { trimmed };
    format!("0x{}", hex_part.to_lowercase())
}

/// Normalizes an intent ID to 64 hex characters (32 bytes) by padding with leading zeros.
///
/// This ensures that intent IDs can be safely parsed as hex, even if they have an odd number
/// of hex characters or are shorter than 64 characters.
///
/// # Arguments
///
/// * `intent_id` - The intent ID to normalize (e.g., "0xabc..." or "0x0abc...")
///
/// # Returns
///
/// * Normalized intent ID with 0x prefix, padded to 64 hex characters, lowercase
pub fn normalize_intent_id_to_64_chars(intent_id: &str) -> String {
    let stripped = intent_id.strip_prefix("0x").unwrap_or(intent_id);
    format!("0x{:0>64}", stripped.to_lowercase())
}

// ============================================================================
// EVENT DATA STRUCTURES
// ============================================================================

/// Type of blockchain where an escrow or intent is located.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChainType {
    /// Move VM-based chain (e.g., Aptos)
    Mvm,
    /// EVM-compatible chain (e.g., Ethereum, Polygon, Arbitrum)
    Evm,
    /// Solana chain
    Svm,
}

/// Request-intent creation event from the hub chain.
///
/// This event is emitted when a new intent is created on the hub chain.
/// The coordinator monitors these events to track new trading opportunities
/// and validate their safety for escrow operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentEvent {
    /// Unique identifier for the intent
    pub intent_id: String,
    /// Metadata of the asset being offered
    pub offered_metadata: String,
    /// Amount of the asset being offered (u64, matching Move contract constraint)
    pub offered_amount: u64,
    /// Metadata of the desired asset
    pub desired_metadata: String,
    /// Amount of the desired asset (u64, matching Move contract constraint)
    pub desired_amount: u64,
    /// Whether the intent can be revoked by the creator
    pub revocable: bool,
    /// Address of the requester who created the intent
    pub requester_addr: String,
    /// Requester address on connected chain (for outflow intents - where solver should send tokens)
    /// None for inflow intents or if not available
    pub requester_addr_connected_chain: Option<String>,
    /// Solver address if the intent is reserved (None for unreserved intents)
    pub reserved_solver_addr: Option<String>,
    /// Connected chain ID where escrow will be created (None for regular intents)
    pub connected_chain_id: Option<u64>,
    /// Unix timestamp when the intent expires
    pub expiry_time: u64,
    /// Timestamp when the event was received
    pub timestamp: u64,
}

/// Escrow deposit event from the connected chain.
///
/// This event is emitted when a solver deposits assets into an escrow
/// on the connected chain. The trusted-gmp validates that this deposit
/// fulfills the conditions specified in the original intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscrowEvent {
    /// Unique identifier for the escrow (on connected chain)
    pub escrow_id: String,
    /// Unique identifier for the intent on hub chain (for matching)
    pub intent_id: String,
    /// Metadata of the asset being offered (what's locked in escrow)
    pub offered_metadata: String,
    /// Amount of the asset being offered (u64, matching Move contract constraint)
    pub offered_amount: u64,
    /// Metadata of the desired asset (what solver needs to provide)
    pub desired_metadata: String,
    /// Amount of the desired asset (u64, matching Move contract constraint)
    pub desired_amount: u64,
    /// Whether the escrow intent can be revoked (should always be false for security)
    pub revocable: bool,
    /// Address of the requester who created the escrow (who locked the funds)
    pub requester_addr: String,
    /// Reserved solver address if the escrow is reserved (None for unreserved escrows)
    /// For Move VM escrows: Move VM address
    /// For EVM escrows: EVM address (0x-prefixed hex string)
    pub reserved_solver_addr: Option<String>,
    /// Chain ID where this escrow is located
    /// Note: This is set by the coordinator based on which monitor discovered the event (from config),
    /// not from the event data itself, so it can be trusted for validation.
    pub chain_id: u64,
    /// Type of blockchain where this escrow is located
    /// Note: This is set by the coordinator based on which monitor discovered the event,
    /// not from the event data itself, so it can be trusted for validation.
    pub chain_type: ChainType,
    /// Unix timestamp when the escrow expires
    pub expiry_time: u64,
    /// Timestamp when the event was received
    pub timestamp: u64,
}

/// Fulfillment event from the hub chain.
///
/// This event is emitted when a intent is fulfilled by a solver.
/// The coordinator monitors these events to track when hub intents are completed,
/// which triggers the approval workflow for escrow release on the connected chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FulfillmentEvent {
    /// Unique identifier for the intent that was fulfilled
    pub intent_id: String,
    /// Address of the intent that was fulfilled
    pub intent_addr: String,
    /// Address of the solver who fulfilled the intent
    pub solver_hub_addr: String,
    /// Metadata of the asset provided by the solver
    pub provided_metadata: String,
    /// Amount of the asset provided by the solver (u64, matching Move contract constraint)
    pub provided_amount: u64,
    /// Unix timestamp when the intent was fulfilled
    pub timestamp: u64,
}

// ============================================================================
// EVENT MONITOR STRUCTURE
// ============================================================================

/// Event monitor that watches both hub and connected chains for relevant events.
///
/// This monitor runs continuously, polling both chains for new events and
/// caching them for API access. The coordinator monitor is read-only - it does
/// not perform validation or generate signatures.
#[derive(Clone)]
pub struct EventMonitor {
    /// Service configuration
    pub config: Arc<Config>,
    /// HTTP client for hub chain communication
    #[allow(dead_code)]
    pub hub_client: reqwest::Client,
    /// HTTP client for connected chain communication
    #[allow(dead_code)]
    pub connected_client: reqwest::Client,
    /// In-memory cache of recent intent events
    ///
    /// **WARNING**: This field is public ONLY for unit testing purposes.
    /// It should not be accessed directly in production code.
    #[doc(hidden)]
    pub event_cache: Arc<RwLock<Vec<IntentEvent>>>,
    /// In-memory cache of recent escrow events
    ///
    /// **WARNING**: This field is public ONLY for unit testing purposes.
    /// It should not be accessed directly in production code.
    #[doc(hidden)]
    pub escrow_cache: Arc<RwLock<Vec<EscrowEvent>>>,
    /// In-memory cache of fulfillment events
    ///
    /// **WARNING**: This field is public ONLY for unit testing purposes.
    /// It should not be accessed directly in production code.
    #[doc(hidden)]
    pub fulfillment_cache: Arc<RwLock<Vec<FulfillmentEvent>>>,
}

impl EventMonitor {
    /// Creates a new event monitor with the given configuration.
    ///
    /// This function initializes HTTP clients with appropriate timeouts
    /// and prepares the event cache for use.
    ///
    /// # Arguments
    ///
    /// * `config` - Service configuration containing chain URLs and timeouts
    ///
    /// # Returns
    ///
    /// * `Ok(EventMonitor)` - Successfully created monitor
    /// * `Err(anyhow::Error)` - Failed to create monitor
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        // Create HTTP client for hub chain with configured timeout
        let hub_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(
                config.coordinator.validation_timeout_ms,
            ))
            .no_proxy() // Avoid macOS system-configuration issues in tests
            .build()?;

        // Create HTTP client for connected chain with configured timeout
        let connected_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(
                config.coordinator.validation_timeout_ms,
            ))
            .no_proxy() // Avoid macOS system-configuration issues in tests
            .build()?;

        Ok(Self {
            config: Arc::new(config.clone()),
            hub_client,
            connected_client,
            event_cache: Arc::new(RwLock::new(Vec::new())),
            escrow_cache: Arc::new(RwLock::new(Vec::new())),
            fulfillment_cache: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Starts the event monitoring process for configured chains.
    ///
    /// This function runs monitoring loops:
    /// 1. Hub chain monitoring for intent events (always)
    /// 2. Connected MVM chain monitoring for escrow events (if configured)
    /// 3. Connected EVM chain monitoring for escrow events (if configured)
    /// 4. Connected SVM chain monitoring for escrow events (if configured)
    ///
    /// The function blocks until all monitors complete (which should be never
    /// in normal operation, as they run infinite loops).
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Monitoring started successfully
    /// * `Err(anyhow::Error)` - Failed to start monitoring
    pub async fn start_monitoring(&self) -> anyhow::Result<()> {
        use super::inflow_generic;
        use super::outflow_generic;
        use tracing::info;

        info!("Starting event monitoring");

        // Start hub chain monitoring (always required) - for outflow intents
        let hub_monitor = outflow_generic::monitor_hub_chain(self);

        let has_mvm = self.config.connected_chain_mvm.is_some();
        let has_evm = self.config.connected_chain_evm.is_some();
        let has_svm = self.config.connected_chain_svm.is_some();

        if has_mvm {
            info!("Connected Move VM chain configured, starting connected chain monitoring");
        }
        if has_evm {
            info!("Connected EVM chain configured, starting EVM chain monitoring");
        }
        if has_svm {
            info!("Connected SVM chain configured, starting SVM chain monitoring");
        }

        match (has_mvm, has_evm, has_svm) {
            (true, true, true) => {
                let mvm_monitor = inflow_generic::monitor_connected_chain(self);
                let evm_monitor = inflow_generic::monitor_evm_chain(self);
                let svm_monitor = inflow_generic::monitor_svm_chain(self);
                tokio::try_join!(hub_monitor, mvm_monitor, evm_monitor, svm_monitor)?;
            }
            (true, true, false) => {
                let mvm_monitor = inflow_generic::monitor_connected_chain(self);
                let evm_monitor = inflow_generic::monitor_evm_chain(self);
                tokio::try_join!(hub_monitor, mvm_monitor, evm_monitor)?;
            }
            (true, false, true) => {
                let mvm_monitor = inflow_generic::monitor_connected_chain(self);
                let svm_monitor = inflow_generic::monitor_svm_chain(self);
                tokio::try_join!(hub_monitor, mvm_monitor, svm_monitor)?;
            }
            (false, true, true) => {
                let evm_monitor = inflow_generic::monitor_evm_chain(self);
                let svm_monitor = inflow_generic::monitor_svm_chain(self);
                tokio::try_join!(hub_monitor, evm_monitor, svm_monitor)?;
            }
            (true, false, false) => {
                let mvm_monitor = inflow_generic::monitor_connected_chain(self);
                tokio::try_join!(hub_monitor, mvm_monitor)?;
            }
            (false, true, false) => {
                let evm_monitor = inflow_generic::monitor_evm_chain(self);
                tokio::try_join!(hub_monitor, evm_monitor)?;
            }
            (false, false, true) => {
                let svm_monitor = inflow_generic::monitor_svm_chain(self);
                tokio::try_join!(hub_monitor, svm_monitor)?;
            }
            (false, false, false) => {
                info!("No connected chains configured, monitoring hub chain only");
                hub_monitor.await?;
            }
        }

        Ok(())
    }

    /// Polls the hub chain for new intent events.
    ///
    /// This function queries the hub chain's event logs for new intent
    /// creation events. Since module events are emitted in user transactions,
    /// we query known test accounts for their events.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<IntentEvent>)` - List of new intent events
    /// * `Err(anyhow::Error)` - Failed to poll events
    #[allow(dead_code)]
    pub async fn poll_hub_events(&self) -> anyhow::Result<Vec<IntentEvent>> {
        use super::outflow_generic;
        outflow_generic::poll_hub_events(self).await
    }

    /// Polls connected chains for new escrow events.
    ///
    /// This function queries connected chains (Move VM and/or EVM) for escrow initialization
    /// events. It handles both Move VM and EVM chains if configured.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<EscrowEvent>)` - List of new escrow events from all connected chains
    /// * `Err(anyhow::Error)` - Failed to poll events
    #[allow(dead_code)]
    pub async fn poll_connected_events(&self) -> anyhow::Result<Vec<EscrowEvent>> {
        use super::inflow_generic;
        inflow_generic::poll_connected_events(self).await
    }

    /// Returns a copy of all cached intent events.
    ///
    /// This function provides access to the event cache for API endpoints
    /// and external monitoring systems.
    ///
    /// # Returns
    ///
    /// A vector containing all cached intent events
    pub async fn get_cached_events(&self) -> Vec<IntentEvent> {
        use super::outflow_generic;
        outflow_generic::get_cached_events(self).await
    }

    /// Returns a copy of all cached escrow events.
    ///
    /// This function provides access to the escrow event cache for API endpoints
    /// and external monitoring systems.
    ///
    /// # Returns
    ///
    /// A vector containing all cached escrow events
    pub async fn get_cached_escrow_events(&self) -> Vec<EscrowEvent> {
        use super::inflow_generic;
        inflow_generic::get_cached_escrow_events(self).await
    }

    /// Returns a copy of all cached fulfillment events.
    ///
    /// This function provides access to the fulfillment event cache for API endpoints.
    ///
    /// # Returns
    ///
    /// A vector containing all cached fulfillment events
    pub async fn get_cached_fulfillment_events(&self) -> Vec<FulfillmentEvent> {
        use super::outflow_generic;
        outflow_generic::get_cached_fulfillment_events(self).await
    }
}
