//! Inflow-specific monitor helpers (chain-agnostic)
//!
//! This module handles connected-chain escrow monitoring for inflow intents.
//! Inflow intents have tokens locked on the connected chain (in escrow)
//! and request tokens on the hub chain.
//!
//! The coordinator only monitors and caches events - it does not perform
//! validation or generate approval signatures. That is handled by the
//! Trusted GMP service.

use anyhow::Result;
use tracing::{error, info, warn};

use super::generic::{EscrowEvent, EventMonitor};
use super::inflow_mvm;

// ============================================================================
// CONNECTED CHAIN MONITORING
// ============================================================================

/// Monitors the connected Move VM chain for escrow deposit events.
///
/// This function runs in an infinite loop, polling the connected Move VM chain
/// for escrow deposit events and caching them for API access.
///
/// # Arguments
///
/// * `monitor` - The event monitor instance
///
/// # Returns
///
/// * `Ok(())` - Monitoring started successfully (runs indefinitely)
/// * `Err(anyhow::Error)` - Failed to start monitoring
///
/// # Behavior
///
/// Returns early if no connected Move VM chain is configured.
pub async fn monitor_connected_chain(monitor: &EventMonitor) -> Result<()> {
    let connected_chain_mvm = match &monitor.config.connected_chain_mvm {
        Some(chain) => chain,
        None => {
            info!("No connected Move VM chain configured, skipping connected chain monitoring");
            return Ok(());
        }
    };

    info!(
        "Starting connected Move VM chain monitoring for escrow events on {}",
        connected_chain_mvm.name
    );

    loop {
        match inflow_mvm::poll_mvm_escrow_events(monitor).await {
            Ok(events) => {
                for event in events {
                    // Cache the escrow event (deduplicate by escrow_id + chain_id)
                    let is_new_event = {
                        let escrow_id = event.escrow_id.clone();
                        let chain_id = event.chain_id;
                        let mut escrow_cache = monitor.escrow_cache.write().await;
                        if !escrow_cache.iter().any(|cached| {
                            cached.escrow_id == escrow_id && cached.chain_id == chain_id
                        }) {
                            escrow_cache.push(event.clone());
                            true
                        } else {
                            false
                        }
                    };

                    // Log new events
                    if is_new_event {
                        info!("Received new MVM escrow: escrow_id={}, intent_id={}, amount={}",
                            event.escrow_id, event.intent_id, event.offered_amount);
                    }
                }
            }
            Err(e) => {
                error!("Error polling connected events: {}", e);
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(
            monitor.config.coordinator.polling_interval_ms,
        ))
        .await;
    }
}

/// Monitors the connected EVM chain for escrow initialization events.
///
/// NOTE: EVM monitoring is temporarily disabled in coordinator.
/// EVM escrow monitoring will be re-added with a read-only EVM client.
/// For now, EVM monitoring is handled by the Trusted GMP service.
///
/// # Returns
///
/// * `Ok(())` - Returns immediately (EVM monitoring disabled)
pub async fn monitor_evm_chain(monitor: &EventMonitor) -> Result<()> {
    if monitor.config.connected_chain_evm.is_some() {
        warn!("EVM chain monitoring is temporarily disabled in coordinator. EVM escrows will be monitored by Trusted GMP.");
    } else {
        info!("No connected EVM chain configured, skipping EVM chain monitoring");
    }
    Ok(())
}

/// Monitors the connected SVM chain for escrow accounts.
///
/// This function runs in an infinite loop, polling the connected SVM chain
/// for escrow accounts and caching them for API access.
///
/// # Behavior
///
/// Returns early if no connected SVM chain is configured.
pub async fn monitor_svm_chain(monitor: &EventMonitor) -> Result<()> {
    let connected_chain_svm = match &monitor.config.connected_chain_svm {
        Some(chain) => chain,
        None => {
            info!("No connected SVM chain configured, skipping SVM chain monitoring");
            return Ok(());
        }
    };

    info!(
        "Starting connected SVM chain monitoring for escrow accounts on {}",
        connected_chain_svm.name
    );

    loop {
        match crate::monitor::inflow_svm::poll_svm_escrow_events(&monitor.config).await {
            Ok(events) => {
                for event in events {
                    let is_new_event = {
                        let escrow_id = event.escrow_id.clone();
                        let chain_id = event.chain_id;
                        let mut escrow_cache = monitor.escrow_cache.write().await;
                        if !escrow_cache.iter().any(|cached| {
                            cached.escrow_id == escrow_id && cached.chain_id == chain_id
                        }) {
                            escrow_cache.push(event.clone());
                            true
                        } else {
                            false
                        }
                    };

                    if is_new_event {
                        info!(
                            "Received new SVM escrow: escrow_id={}, intent_id={}, amount={}",
                            event.escrow_id, event.intent_id, event.offered_amount
                        );
                    }
                }
            }
            Err(e) => {
                error!("Error polling SVM escrow accounts: {}", e);
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(
            monitor.config.coordinator.polling_interval_ms,
        ))
        .await;
    }
}

// ============================================================================
// EVENT POLLING
// ============================================================================

/// Polls connected chains for new escrow events.
///
/// This function queries connected chains (Move VM and/or EVM) for escrow initialization
/// events. It handles both Move VM and EVM chains if configured, aggregating events
/// from all connected chains into a single vector.
///
/// # Arguments
///
/// * `monitor` - The event monitor instance
///
/// # Returns
///
/// * `Ok(Vec<EscrowEvent>)` - List of new escrow events from all connected chains
/// * `Err(anyhow::Error)` - Failed to poll events from one or more chains
///
/// # Behavior
///
/// If polling fails for one chain, the function continues to poll other chains
/// and returns events from successfully polled chains. Errors are logged but
/// do not cause the function to fail.
#[allow(dead_code)]
pub async fn poll_connected_events(monitor: &EventMonitor) -> Result<Vec<EscrowEvent>> {
    let mut escrow_events = Vec::new();

    if let Some(_) = &monitor.config.connected_chain_mvm {
        match inflow_mvm::poll_mvm_escrow_events(monitor).await {
            Ok(mut events) => {
                escrow_events.append(&mut events);
            }
            Err(e) => {
                error!("Failed to poll Move VM escrow events: {}", e);
            }
        }
    }

    // Note: EVM polling temporarily disabled in coordinator
    // EVM escrow monitoring handled by Trusted GMP service

    Ok(escrow_events)
}

// ============================================================================
// CACHE ACCESS
// ============================================================================

/// Returns a copy of all cached escrow events.
///
/// This function provides access to the escrow event cache for API endpoints
/// and external monitoring systems. The cache contains all escrow events that
/// have been observed on connected chains (both Move VM and EVM).
///
/// # Arguments
///
/// * `monitor` - The event monitor instance
///
/// # Returns
///
/// A vector containing all cached escrow events from all connected chains
pub async fn get_cached_escrow_events(monitor: &EventMonitor) -> Vec<EscrowEvent> {
    monitor.escrow_cache.read().await.clone()
}
