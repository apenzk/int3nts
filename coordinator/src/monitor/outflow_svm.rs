//! Outflow SVM-specific monitoring functions
//!
//! Monitors connected Solana chains for IntentRequirementsReceived logs
//! from the outflow validator program.

use crate::monitor::generic::EventMonitor;
use crate::svm_client::SvmClient;
use anyhow::{Context, Result};

/// Poll connected SVM chain for IntentRequirementsReceived logs.
///
/// When requirements are received, mark the corresponding intent as ready.
///
/// # Arguments
///
/// * `monitor` - Event monitor instance
///
/// # Returns
///
/// * `Ok(usize)` - Number of new requirements received logs processed
/// * `Err(anyhow::Error)` - Failed to poll logs
pub async fn poll_svm_requirements_received(monitor: &EventMonitor) -> Result<usize> {
    let connected_chain_svm = monitor
        .config
        .connected_chain_svm
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No connected SVM chain configured"))?;

    // Create SVM client - use escrow_program_id as the program to monitor
    // In production, this might be a separate outflow_validator_program_id
    let client = SvmClient::new(
        &connected_chain_svm.rpc_url,
        &connected_chain_svm.escrow_program_id,
    )
    .context("Failed to create SVM client")?;

    // Query recent transaction signatures
    let signatures = client
        .get_signatures_for_address(100)
        .await
        .context("Failed to get transaction signatures")?;

    let mut count = 0;

    // Process each transaction (skip ones that fail — old txs get pruned on devnet)
    for signature in signatures {
        let logs = match client.get_transaction(&signature).await {
            Ok(logs) => logs,
            Err(_) => continue,
        };

        // Look for IntentRequirementsReceived log messages
        for log in logs {
            if log.contains("IntentRequirementsReceived:") {
                // Parse: "IntentRequirementsReceived: intent_id=abc123, src_chain_id=1"
                if let Some(intent_id) = extract_intent_id_from_log(&log) {
                    monitor.mark_intent_ready(&intent_id).await;
                    count += 1;

                    tracing::debug!(
                        "SVM IntentRequirementsReceived: intent_id={}",
                        intent_id
                    );
                }
            }
        }
    }

    Ok(count)
}

/// Extract intent_id from SVM log message
///
/// Example log format: "IntentRequirementsReceived: intent_id=0a1b2c3d..., src_chain_id=1"
fn extract_intent_id_from_log(log: &str) -> Option<String> {
    // Find "intent_id=" and extract the hex string after it
    let intent_id_prefix = "intent_id=";
    let start = log.find(intent_id_prefix)? + intent_id_prefix.len();
    let remaining = &log[start..];

    // Extract until comma or end of string
    let end = remaining
        .find(',')
        .or_else(|| remaining.find(' '))
        .unwrap_or(remaining.len());

    let intent_id_hex = &remaining[..end];

    // Add 0x prefix if not present
    if intent_id_hex.starts_with("0x") {
        Some(intent_id_hex.to_string())
    } else {
        Some(format!("0x{}", intent_id_hex))
    }
}
