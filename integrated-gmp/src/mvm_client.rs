//! GMP-specific MVM Client
//!
//! Wraps the shared `chain_clients_mvm::MvmClient` and adds GMP-specific methods
//! for relay authorization, outbox reading, message parsing, and message delivery.

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chain_clients_mvm::MvmClient;
use std::process::Command;
use tracing::{debug, error, info, warn};

use crate::integrated_gmp_relay::GmpMessage;

// ============================================================================
// CLIENT
// ============================================================================

pub struct GmpMvmClient {
    mvm_client: MvmClient,
    module_addr: String,
    chain_id: u32,
}

impl GmpMvmClient {
    pub fn new(rpc_url: &str, module_addr: &str, chain_id: u32) -> Result<Self> {
        let mvm_client =
            MvmClient::new(rpc_url).context("Failed to create MVM client")?;
        Ok(Self {
            mvm_client,
            module_addr: module_addr.to_string(),
            chain_id,
        })
    }

    pub fn chain_id(&self) -> u32 {
        self.chain_id
    }

    pub fn module_addr(&self) -> &str {
        &self.module_addr
    }

    pub fn mvm_client(&self) -> &MvmClient {
        &self.mvm_client
    }

    // ========================================================================
    // Authorization check
    // ========================================================================

    /// Check if a relay address is authorized on the GMP endpoint contract.
    pub async fn is_relay_authorized(&self, relay_addr: &str) -> Result<bool> {
        let result = self
            .mvm_client
            .call_view_function(
                &self.module_addr,
                "intent_gmp",
                "is_relay_authorized",
                vec![],
                vec![serde_json::json!(relay_addr)],
            )
            .await
            .context("Failed to check relay authorization")?;

        let authorized = result
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_bool())
            .ok_or_else(|| anyhow::anyhow!(
                "Unexpected response format from is_relay_authorized: {result}"
            ))?;

        Ok(authorized)
    }

    // ========================================================================
    // Outbox reading
    // ========================================================================

    /// Get the next nonce from the GMP sender outbox.
    pub async fn get_next_nonce(&self) -> Result<u64> {
        let next_nonce_result = self
            .mvm_client
            .call_view_function(
                &self.module_addr,
                "gmp_sender",
                "get_next_nonce",
                vec![],
                vec![],
            )
            .await
            .context("Failed to call get_next_nonce")?;

        next_nonce_result
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| {
                v.as_str()
                    .and_then(|s| s.parse().ok())
                    .or_else(|| v.as_u64())
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Failed to parse get_next_nonce response: {:?}",
                    next_nonce_result
                )
            })
    }

    /// Read a message from the outbox by nonce.
    pub async fn get_message(&self, nonce: u64) -> Result<GmpMessage> {
        let msg_value = self
            .mvm_client
            .call_view_function(
                &self.module_addr,
                "gmp_sender",
                "get_message",
                vec![],
                vec![serde_json::json!(nonce.to_string())],
            )
            .await
            .context("Failed to call get_message")?;

        let arr = msg_value
            .as_array()
            .context("get_message result is not an array")?;
        if arr.len() < 4 {
            anyhow::bail!(
                "get_message({}) returned {} elements, expected 4",
                nonce,
                arr.len()
            );
        }

        let dst_chain_id: u32 = arr[0]
            .as_str()
            .and_then(|s| s.parse().ok())
            .or_else(|| arr[0].as_u64().map(|n| n as u32))
            .context("Failed to parse dst_chain_id")?;

        let dst_addr_hex = parse_view_bytes(&arr[1])?;
        let payload_hex = parse_view_bytes(&arr[2])?;

        Ok(GmpMessage {
            src_chain_id: self.chain_id,
            remote_gmp_endpoint_addr: normalize_address(&self.module_addr),
            dst_chain_id,
            dst_addr: format!("0x{}", dst_addr_hex),
            payload: format!("0x{}", payload_hex),
            nonce,
        })
    }

    // ========================================================================
    // Message delivery
    // ========================================================================

    /// Deliver a GMP message to this MVM chain via aptos CLI.
    pub async fn deliver_message(
        &self,
        message: &GmpMessage,
        operator_private_key: &str,
    ) -> Result<()> {
        let remote_gmp_endpoint_addr_hex = message
            .remote_gmp_endpoint_addr
            .strip_prefix("0x")
            .unwrap_or(&message.remote_gmp_endpoint_addr);
        let payload_hex = message
            .payload
            .strip_prefix("0x")
            .unwrap_or(&message.payload);

        let private_key_bytes = STANDARD
            .decode(operator_private_key)
            .context("Failed to decode base64 private key")?;
        let private_key_hex = hex::encode(&private_key_bytes);

        let function_id = format!(
            "{}::intent_gmp::deliver_message_entry",
            self.module_addr
        );

        let src_chain_id_arg = format!("u32:{}", message.src_chain_id);
        let remote_gmp_endpoint_addr_arg =
            format!("hex:{}", remote_gmp_endpoint_addr_hex);
        let payload_arg = format!("hex:{}", payload_hex);

        // Normalize RPC URL (strip trailing /v1 if present for CLI)
        let rpc_url = self.mvm_client.base_url();
        let rpc_url_normalized = rpc_url
            .trim_end_matches('/')
            .trim_end_matches("/v1");

        debug!(
            "MVM chain_id={} deliver_message CLI call: function_id={}, src_chain_id={}, nonce={}",
            self.chain_id, function_id, message.src_chain_id, message.nonce
        );

        let output = Command::new("aptos")
            .args([
                "move",
                "run",
                "--private-key",
                &private_key_hex,
                "--url",
                rpc_url_normalized,
                "--assume-yes",
                "--function-id",
                &function_id,
                "--args",
                &src_chain_id_arg,
                &remote_gmp_endpoint_addr_arg,
                &payload_arg,
            ])
            .output()
            .context("Failed to execute aptos move run")?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        if !output.status.success() {
            debug!(
                "MVM chain_id={} deliver_message failed: stderr={}, stdout={}",
                self.chain_id, stderr, stdout
            );
            anyhow::bail!(
                "aptos move run failed for deliver_message_entry on chain_id={}: stderr={}, stdout={}",
                self.chain_id, stderr, stdout
            );
        }

        let tx_hash = extract_transaction_hash(&stdout);

        let vm_success = check_vm_status_success(&stdout)?;
        if !vm_success {
            error!(
                "MVM chain_id={} deliver_message VM execution failed: nonce={}, tx_hash={:?}, stdout={}",
                self.chain_id, message.nonce, tx_hash, stdout
            );
            anyhow::bail!(
                "deliver_message_entry VM execution failed on chain_id={}: tx_hash={:?}, stdout={}",
                self.chain_id, tx_hash, stdout
            );
        }

        info!(
            "MVM chain_id={} deliver_message submitted successfully: nonce={}, tx_hash={:?}",
            self.chain_id, message.nonce, tx_hash
        );

        Ok(())
    }
}

// ============================================================================
// HELPERS (moved from integrated_gmp_relay.rs)
// ============================================================================

pub fn normalize_address(addr: &str) -> String {
    if addr.starts_with("0x") {
        addr.to_string()
    } else {
        format!("0x{}", addr)
    }
}

pub fn parse_view_bytes(value: &serde_json::Value) -> Result<String> {
    if let Some(hex_str) = value.as_str() {
        Ok(hex_str.strip_prefix("0x").unwrap_or(hex_str).to_string())
    } else if let Some(arr) = value.as_array() {
        let mut bytes = Vec::with_capacity(arr.len());
        for elem in arr {
            let byte: u8 = elem
                .as_str()
                .and_then(|s| s.parse().ok())
                .or_else(|| elem.as_u64().map(|n| n as u8))
                .context("Invalid byte in view function result")?;
            bytes.push(byte);
        }
        Ok(hex::encode(bytes))
    } else {
        anyhow::bail!("Unexpected view function bytes format: {:?}", value)
    }
}

pub fn check_vm_status_success(output: &str) -> Result<bool, anyhow::Error> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(result) = json.get("Result") {
            if let Some(success) = result.get("success") {
                return success.as_bool().ok_or_else(|| {
                    anyhow::anyhow!(
                        "VM status 'success' field is not a boolean: {:?}",
                        success
                    )
                });
            }
        }
        if let Some(success) = json.get("success") {
            return success.as_bool().ok_or_else(|| {
                anyhow::anyhow!(
                    "VM status 'success' field is not a boolean: {:?}",
                    success
                )
            });
        }
    }
    // If we can't parse JSON at all, assume success if CLI returned 0
    warn!("Could not parse VM status from output, assuming success based on exit code");
    Ok(true)
}

pub fn extract_transaction_hash(output: &str) -> Option<String> {
    if let Some(start) = output.find("\"transaction_hash\"") {
        let after_key = &output[start..];
        if let Some(colon_pos) = after_key.find(':') {
            let value_part = &after_key[colon_pos + 1..];
            if let Some(quote_start) = value_part.find('"') {
                let after_quote = &value_part[quote_start + 1..];
                if let Some(quote_end) = after_quote.find('"') {
                    let hash = &after_quote[..quote_end];
                    if hash.starts_with("0x") {
                        return Some(hash.to_string());
                    }
                }
            }
        }
    }
    None
}
