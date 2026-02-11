//! Outflow EVM-specific monitoring functions
//!
//! Monitors connected EVM chains for IntentRequirementsReceived events
//! from the IntentOutflowValidator contract.

use crate::monitor::generic::EventMonitor;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ============================================================================
// JSON-RPC TYPES
// ============================================================================

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse<T> {
    result: Option<T>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct LogEntry {
    data: String,
}

/// Poll connected EVM chain for IntentRequirementsReceived events.
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
pub async fn poll_evm_requirements_received(monitor: &EventMonitor) -> Result<usize> {
    let connected_chain_evm = monitor
        .config
        .connected_chain_evm
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No connected EVM chain configured"))?;

    // Create HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .no_proxy()
        .build()
        .context("Failed to create HTTP client")?;

    // IntentRequirementsReceived event signature (keccak256)
    // This is a placeholder - in production, this should be the actual event signature
    let event_signature = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

    // Get current block number first, then query last 1000 blocks
    // (public RPCs have a max block range limit, typically 50000)
    let block_request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "eth_blockNumber".to_string(),
        params: serde_json::json!([]),
        id: 1,
    };

    let block_response: JsonRpcResponse<String> = client
        .post(&connected_chain_evm.rpc_url)
        .json(&block_request)
        .send()
        .await
        .context("Failed to get block number")?
        .json()
        .await
        .context("Failed to parse block number response")?;

    let current_block = block_response
        .result
        .ok_or_else(|| anyhow::anyhow!("No block number returned"))?;
    let current_block_num =
        u64::from_str_radix(current_block.strip_prefix("0x").unwrap_or(&current_block), 16)
            .unwrap_or(0);
    // Cap to 9-block lookback so inclusive range [from, to] = 10 blocks (Alchemy free tier limit)
    let range = connected_chain_evm.event_block_range.min(9);
    let from_block = current_block_num.saturating_sub(range);
    let from_block_hex = format!("0x{:x}", from_block);
    let to_block_hex = format!("0x{:x}", current_block_num);

    // Query eth_getLogs for IntentRequirementsReceived events
    let params = serde_json::json!([{
        "address": connected_chain_evm.outflow_validator_contract_addr,
        "fromBlock": from_block_hex,
        "toBlock": to_block_hex,
        "topics": [event_signature]
    }]);

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "eth_getLogs".to_string(),
        params,
        id: 1,
    };

    let response: JsonRpcResponse<Vec<LogEntry>> = client
        .post(&connected_chain_evm.rpc_url)
        .json(&request)
        .send()
        .await
        .context("Failed to call eth_getLogs")?
        .json()
        .await
        .context("Failed to parse eth_getLogs response")?;

    if let Some(error) = response.error {
        return Err(anyhow::anyhow!("EVM RPC error: {}", error.message));
    }

    let logs = response.result.unwrap_or_default();
    let mut count = 0;

    for log in logs {
        // Parse log data to extract intent_id (first 32 bytes after 0x prefix)
        let data = log.data.strip_prefix("0x").unwrap_or(&log.data);
        if data.len() >= 64 {
            let intent_id_hex = &data[..64];
            let intent_id = format!("0x{}", intent_id_hex);

            // Mark intent as ready
            monitor.mark_intent_ready(&intent_id).await;
            count += 1;

            tracing::debug!(
                "EVM IntentRequirementsReceived: intent_id={}",
                intent_id
            );
        }
    }

    Ok(count)
}
