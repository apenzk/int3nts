//! Outflow Move VM-specific monitoring functions
//!
//! Monitors connected Move VM chains for IntentRequirementsReceived events
//! from the outflow validator. When requirements are received, marks the
//! corresponding intent as ready for fulfillment.

use crate::monitor::generic::EventMonitor;
use chain_clients_mvm::MvmClient;
use anyhow::{Context, Result};
use serde::Deserialize;

/// IntentRequirementsReceived event from the outflow validator.
///
/// This event is emitted when intent requirements are delivered via GMP
/// to the connected chain's outflow validator.
#[derive(Debug, Clone, Deserialize)]
struct IntentRequirementsReceived {
    /// Intent ID (hex string with 0x prefix)
    intent_id: String,
    /// Source chain ID (hub chain)
    #[allow(dead_code)]
    src_chain_id: String,
    /// Requester address on hub chain
    #[allow(dead_code)]
    requester_addr: String,
    /// Amount required for fulfillment
    #[allow(dead_code)]
    amount_required: String,
    /// Token address on connected chain
    #[allow(dead_code)]
    token_addr: String,
    /// Solver address (who should fulfill)
    #[allow(dead_code)]
    solver_addr: String,
    /// Expiry timestamp
    #[allow(dead_code)]
    expiry: String,
}

/// Poll connected Move VM chain for IntentRequirementsReceived events.
///
/// When requirements are received, mark the corresponding intent as ready.
///
/// # Arguments
///
/// * `monitor` - Event monitor instance
///
/// # Returns
///
/// * `Ok(usize)` - Number of new requirements received events processed
/// * `Err(anyhow::Error)` - Failed to poll events
pub async fn poll_mvm_requirements_received(monitor: &EventMonitor) -> Result<usize> {
    let connected_chain_mvm = monitor
        .config
        .connected_chain_mvm
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No connected Move VM chain configured"))?;

    let client = MvmClient::new(&connected_chain_mvm.rpc_url)?;

    // TODO: Query IntentRequirementsReceived events from outflow validator module
    // The outflow validator address is not currently in the config - need to add it
    // For now, query the intent_module_addr which may contain outflow validator events
    let outflow_validator_addr = &connected_chain_mvm.intent_module_addr;

    // Strip 0x prefix if present
    let outflow_validator_addr_normalized = outflow_validator_addr
        .strip_prefix("0x")
        .unwrap_or(outflow_validator_addr);

    // Get module events for the outflow validator
    let events_response = client
        .get_account_events(
            outflow_validator_addr_normalized,
            None,
            None,
            Some(100),
        )
        .await
        .context("Failed to query IntentRequirementsReceived events")?;

    let mut count = 0;

    // Parse events and mark intents as ready
    for event in events_response {
        // Extract event type and data
        let event_type = &event.r#type;

        if event_type.contains("IntentRequirementsReceived") {
            let data: IntentRequirementsReceived =
                serde_json::from_value(event.data.clone())
                    .context("Failed to parse IntentRequirementsReceived event")?;

            // Mark intent as ready
            monitor.mark_intent_ready(&data.intent_id).await;
            count += 1;
        }
    }

    Ok(count)
}
