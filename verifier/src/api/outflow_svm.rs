//! Outflow SVM-specific API handlers
//!
//! This module contains SVM-specific transaction querying and parameter extraction
//! for outflow fulfillment validation on SVM connected chains.

use crate::validator::{
    extract_svm_fulfillment_params, CrossChainValidator, FulfillmentTransactionParams,
};
use anyhow::{Context, Result};
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Queries an SVM transaction and extracts fulfillment parameters for outflow validation.
///
/// NOTE: SVM fulfillment parsing is not implemented yet.
pub async fn query_svm_fulfillment_transaction(
    transaction_hash: &str,
    validator: &CrossChainValidator,
) -> Result<(FulfillmentTransactionParams, bool), String> {
    let chain_config = validator
        .config
        .connected_chain_svm
        .as_ref()
        .ok_or_else(|| "No connected SVM chain configured".to_string())?;

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            transaction_hash,
            {
                "encoding": "jsonParsed",
                "maxSupportedTransactionVersion": 0
            }
        ]
    });

    let client = reqwest::Client::new();
    let timeout_ms = validator.config.verifier.validation_timeout_ms;
    let start = Instant::now();
    let mut response: serde_json::Value;
    loop {
        response = client
            .post(&chain_config.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to call getTransaction")
            .map_err(|e| e.to_string())?
            .json()
            .await
            .context("Failed to parse getTransaction response")
            .map_err(|e| e.to_string())?;

        if let Some(error) = response.get("error") {
            return Err(format!("SVM RPC error: {}", error));
        }

        let result = response
            .get("result")
            .ok_or_else(|| "Missing getTransaction result".to_string())?;
        if !result.is_null() {
            break;
        }

        if start.elapsed() >= Duration::from_millis(timeout_ms) {
            return Err("Transaction not found".to_string());
        }

        sleep(Duration::from_millis(500)).await;
    }

    let result = response
        .get("result")
        .ok_or_else(|| "Missing getTransaction result".to_string())?;
    let meta = result.get("meta");
    let is_success = meta
        .and_then(|m| m.get("err"))
        .map(|err| err.is_null())
        .unwrap_or(false);

    let params = extract_svm_fulfillment_params(result)
        .map_err(|e| e.to_string())?;

    Ok((params, is_success))
}
