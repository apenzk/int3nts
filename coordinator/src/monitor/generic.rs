//! Generic monitor structures and EventMonitor definition
//!
//! This module contains shared event structures and the EventMonitor struct definition
//! that are used across all flow types (inflow/outflow) and chain types (Move VM/EVM).

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;

// Re-export normalize functions from chain-clients-common
pub use chain_clients_common::{normalize_intent_id, normalize_intent_id_to_64_chars};

// ============================================================================
// EVENT DATA STRUCTURES
// ============================================================================

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

/// Event monitor that watches the hub chain for relevant events.
///
/// This monitor runs continuously, polling the hub chain for new events and
/// caching them for API access. The coordinator monitor is read-only - it does
/// not perform validation or generate signatures.
///
/// Connected chain escrow monitoring is handled independently by the
/// integrated-gmp and solver services.
#[derive(Clone)]
pub struct EventMonitor {
    /// Service configuration
    pub config: Arc<Config>,
    /// HTTP client for hub chain communication
    #[allow(dead_code)]
    pub hub_client: reqwest::Client,
    /// In-memory cache of recent intent events
    ///
    /// **WARNING**: This field is public ONLY for unit testing purposes.
    /// It should not be accessed directly in production code.
    #[doc(hidden)]
    pub event_cache: Arc<RwLock<Vec<IntentEvent>>>,
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

        Ok(Self {
            config: Arc::new(config.clone()),
            hub_client,
            event_cache: Arc::new(RwLock::new(Vec::new())),
            fulfillment_cache: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Starts the event monitoring process for the hub chain.
    ///
    /// This function runs the hub chain monitoring loop for intent and
    /// fulfillment events. Connected chain escrow monitoring is handled
    /// independently by the integrated-gmp and solver services.
    ///
    /// The function blocks until the monitor completes (which should be never
    /// in normal operation, as it runs an infinite loop).
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Monitoring started successfully
    /// * `Err(anyhow::Error)` - Failed to start monitoring
    pub async fn start_monitoring(&self) -> anyhow::Result<()> {
        use super::outflow_generic;
        use tracing::info;

        info!("Starting hub chain event monitoring");

        outflow_generic::monitor_hub_chain(self).await?;

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
