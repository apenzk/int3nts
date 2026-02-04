//! Connected Move VM Chain Client
//!
//! Client for interacting with connected Move VM chains to query escrow state
//! and execute transfers via GMP flow.

use anyhow::{Context, Result};
use reqwest::Client;
use std::process::Command;
use std::time::Duration;

use crate::config::MvmChainConfig;

/// Client for interacting with a connected Move VM chain
pub struct ConnectedMvmClient {
    /// HTTP client for RPC calls
    client: Client,
    /// Base RPC URL (includes /v1, e.g., http://127.0.0.1:8082/v1)
    base_url: String,
    /// Module address
    module_addr: String,
    /// CLI profile name
    profile: String,
}

impl ConnectedMvmClient {
    /// Normalizes a hex string to a 64-character (32-byte) 0x-prefixed address.
    ///
    /// Move addresses are 32 bytes but leading zeros may be stripped in event data,
    /// producing odd-length hex strings that the Aptos REST API rejects.
    pub fn normalize_hex_to_address(hex: &str) -> String {
        let without_prefix = hex.strip_prefix("0x").unwrap_or(hex);
        format!("0x{:0>64}", without_prefix)
    }

    /// Creates a new connected MVM chain client
    ///
    /// # Arguments
    ///
    /// * `config` - Connected chain configuration
    ///
    /// # Returns
    ///
    /// * `Ok(ConnectedMvmClient)` - Successfully created client
    /// * `Err(anyhow::Error)` - Failed to create client
    pub fn new(config: &MvmChainConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .no_proxy() // Avoid macOS system-configuration issues in tests
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            base_url: config.rpc_url.clone(),
            module_addr: config.module_addr.clone(),
            profile: config.profile.clone(),
        })
    }

    /// Checks if outflow requirements have been delivered via GMP to the connected chain.
    ///
    /// Calls the `outflow_validator_impl::has_requirements` view function.
    /// Returns true if IntentRequirements were delivered for this intent_id.
    ///
    /// # Arguments
    ///
    /// * `intent_id` - Intent ID as hex string (e.g., "0x4b1e...")
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - True if requirements are available
    /// * `Err(anyhow::Error)` - Failed to query
    pub async fn has_outflow_requirements(&self, intent_id: &str) -> Result<bool> {
        let intent_id_hex = Self::normalize_hex_to_address(intent_id);

        // base_url already includes /v1
        let view_url = format!("{}/view", self.base_url);
        let request_body = serde_json::json!({
            "function": format!("{}::outflow_validator_impl::has_requirements", self.module_addr),
            "type_arguments": [],
            "arguments": [intent_id_hex]
        });

        let response = self
            .client
            .post(&view_url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to query outflow requirements")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to query outflow requirements: HTTP {} - {}",
                status,
                error_body
            );
        }

        let result: Vec<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse outflow requirements response")?;

        if let Some(first_result) = result.first() {
            if let Some(has_req) = first_result.as_bool() {
                return Ok(has_req);
            }
        }

        anyhow::bail!("Unexpected response format from has_requirements view function")
    }

    /// Checks if an inflow escrow has been fulfilled (FulfillmentProof received via GMP).
    ///
    /// Calls the `inflow_escrow_gmp::is_fulfilled` view function on the connected chain.
    ///
    /// # Arguments
    ///
    /// * `intent_id` - Intent ID as hex string (e.g., "0x4b1e...")
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - True if FulfillmentProof was received
    /// * `Err(anyhow::Error)` - Failed to query
    pub async fn is_escrow_fulfilled(&self, intent_id: &str) -> Result<bool> {
        let intent_id_hex = Self::normalize_hex_to_address(intent_id);

        // base_url already includes /v1
        let view_url = format!("{}/view", self.base_url);
        let request_body = serde_json::json!({
            "function": format!("{}::inflow_escrow_gmp::is_fulfilled", self.module_addr),
            "type_arguments": [],
            "arguments": [intent_id_hex]
        });

        let response = self
            .client
            .post(&view_url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to query escrow fulfillment")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to query escrow fulfillment: HTTP {} - {}",
                status,
                error_body
            );
        }

        let result: Vec<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse escrow fulfillment response")?;

        if let Some(first_result) = result.first() {
            if let Some(is_fulfilled) = first_result.as_bool() {
                return Ok(is_fulfilled);
            }
        }

        anyhow::bail!("Unexpected response format from is_fulfilled view function")
    }

    /// Releases an inflow escrow on the connected chain via GMP flow.
    ///
    /// Calls `inflow_escrow_gmp::release_escrow` which transfers locked tokens to the solver.
    /// No signature needed — the function checks that FulfillmentProof was received via GMP.
    ///
    /// # Arguments
    ///
    /// * `intent_id` - Intent ID as hex string (e.g., "0x4b1e...")
    /// * `token_metadata` - Token metadata object address
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction hash
    /// * `Err(anyhow::Error)` - Failed to release escrow
    pub fn release_gmp_escrow(
        &self,
        intent_id: &str,
        token_metadata: &str,
    ) -> Result<String> {
        use tracing::info;

        info!(
            "Calling inflow_escrow_gmp::release_escrow - intent_id: {}, token_metadata: {}",
            intent_id, token_metadata
        );

        // Function signature: release_escrow(solver: &signer, intent_id: vector<u8>, token_metadata: Object<Metadata>)
        let normalized = Self::normalize_hex_to_address(intent_id);
        let intent_id_bare = &normalized[2..]; // strip "0x" for hex: arg
        let output = Command::new("aptos")
            .args(&[
                "move",
                "run",
                "--profile",
                &self.profile,
                "--assume-yes",
                "--function-id",
                &format!("{}::inflow_escrow_gmp::release_escrow", self.module_addr),
                "--args",
                &format!("hex:{}", intent_id_bare),
                &format!("address:{}", token_metadata),
            ])
            .output()
            .context("Failed to execute aptos move run")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "inflow_escrow_gmp::release_escrow failed:\nstderr: {}\nstdout: {}",
                stderr,
                stdout
            );
        }

        let output_str = String::from_utf8_lossy(&output.stdout);

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output_str) {
            if let Some(hash) = json
                .get("Result")
                .and_then(|r| r.get("transaction_hash"))
                .and_then(|h| h.as_str())
            {
                return Ok(hash.to_string());
            }
        }

        if let Some(hash_line) = output_str
            .lines()
            .find(|l| l.contains("hash") || l.contains("Hash"))
        {
            if let Some(hash) = hash_line
                .split_whitespace()
                .find(|s| s.starts_with("0x"))
            {
                return Ok(hash.to_string());
            }
            if let Some(start) = hash_line.find("\"0x") {
                if let Some(end) = hash_line[start + 1..].find('"') {
                    return Ok(hash_line[start + 1..start + 1 + end].to_string());
                }
            }
        }

        anyhow::bail!(
            "Could not extract transaction hash from output: {}",
            output_str
        )
    }

    /// Executes a transfer with intent ID on the connected chain
    ///
    /// Calls the `transfer_with_intent_id` entry function to transfer tokens
    /// and include the intent_id in the transaction (for outflow fulfillment).
    ///
    /// # Arguments
    ///
    /// * `recipient` - Recipient address
    /// * `metadata` - Token metadata object address
    /// * `amount` - Amount to transfer
    /// * `intent_id` - Intent ID to include in the transaction
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction hash
    /// * `Err(anyhow::Error)` - Failed to execute transfer
    pub fn transfer_with_intent_id(
        &self,
        recipient: &str,
        metadata: &str,
        amount: u64,
        intent_id: &str,
    ) -> Result<String> {
        use tracing::{info, warn};

        // Debug: Get solver's address from profile
        let address_check = Command::new("aptos")
            .args(&["config", "show-profiles"])
            .output();

        if let Ok(address_output) = address_check {
            let address_str = String::from_utf8_lossy(&address_output.stdout);
            info!(
                "Transfer attempt - profile: {}, recipient: {}, amount: {}, metadata: {}",
                self.profile, recipient, amount, metadata
            );
            info!("Aptos profiles: {}", address_str);
        }

        // Debug: Check solver's balance before transfer
        let balance_check = Command::new("aptos")
            .args(&["account", "balance", "--profile", &self.profile])
            .output();

        if let Ok(balance_output) = balance_check {
            let balance_str = String::from_utf8_lossy(&balance_output.stdout);
            info!(
                "Solver balance check (profile: {}): {}",
                self.profile, balance_str
            );
        } else {
            warn!(
                "Failed to check solver balance for profile: {}",
                self.profile
            );
        }

        // Use aptos CLI for compatibility with E2E tests which create aptos profiles
        let output = Command::new("aptos")
            .args(&[
                "move",
                "run",
                "--profile",
                &self.profile,
                "--assume-yes",
                "--function-id",
                &format!("{}::utils::transfer_with_intent_id", self.module_addr),
                "--args",
                &format!("address:{}", recipient),
                &format!("address:{}", metadata),
                &format!("u64:{}", amount),
                &format!("address:{}", intent_id),
            ])
            .output()
            .context("Failed to execute aptos move run")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "movement move run failed:\nstderr: {}\nstdout: {}",
                stderr,
                stdout
            );
        }

        // Extract transaction hash from output
        let output_str = String::from_utf8_lossy(&output.stdout);

        // Try to parse as JSON first (aptos CLI outputs JSON with Result wrapper)
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output_str) {
            // Handle {"Result": {"transaction_hash": "0x...", ...}}
            if let Some(hash) = json
                .get("Result")
                .and_then(|r| r.get("transaction_hash"))
                .and_then(|h| h.as_str())
            {
                return Ok(hash.to_string());
            }
        }

        // Fallback: line-based parsing for "Transaction hash: 0x..." format
        if let Some(hash_line) = output_str
            .lines()
            .find(|l| l.contains("hash") || l.contains("Hash"))
        {
            // Try finding 0x directly or quoted "0x
            if let Some(hash) = hash_line
                .split_whitespace()
                .find(|s| s.starts_with("0x"))
            {
                return Ok(hash.to_string());
            }
            // Handle quoted hash like "0x..."
            if let Some(start) = hash_line.find("\"0x") {
                if let Some(end) = hash_line[start + 1..].find('"') {
                    return Ok(hash_line[start + 1..start + 1 + end].to_string());
                }
            }
        }

        anyhow::bail!(
            "Could not extract transaction hash from output: {}",
            output_str
        )
    }

    /// Fulfills an outflow intent via the GMP flow on the connected chain.
    ///
    /// Calls `outflow_validator::fulfill_intent` which:
    /// 1. Validates the solver is authorized and requirements exist
    /// 2. Transfers tokens from solver to recipient
    /// 3. Sends FulfillmentProof back to hub via GMP
    ///
    /// The hub will automatically release tokens when it receives the FulfillmentProof.
    ///
    /// # Arguments
    ///
    /// * `intent_id` - 32-byte intent identifier (0x-prefixed hex)
    /// * `token_metadata` - Token metadata object address
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction hash
    /// * `Err(anyhow::Error)` - Failed to fulfill intent
    pub fn fulfill_outflow_via_gmp(
        &self,
        intent_id: &str,
        token_metadata: &str,
    ) -> Result<String> {
        use tracing::info;

        info!(
            "Calling outflow_validator::fulfill_intent - intent_id: {}, token_metadata: {}",
            intent_id, token_metadata
        );

        // Use aptos CLI for compatibility with E2E tests which create aptos profiles
        // Function signature: fulfill_intent(solver: &signer, intent_id: vector<u8>, token_metadata: Object<Metadata>)
        let normalized = Self::normalize_hex_to_address(intent_id);
        let intent_id_bare = &normalized[2..]; // strip "0x" for hex: arg
        let output = Command::new("aptos")
            .args(&[
                "move",
                "run",
                "--profile",
                &self.profile,
                "--assume-yes",
                "--function-id",
                &format!(
                    "{}::outflow_validator_impl::fulfill_intent",
                    self.module_addr
                ),
                "--args",
                &format!("hex:{}", intent_id_bare),
                &format!("address:{}", token_metadata),
            ])
            .output()
            .context("Failed to execute aptos move run")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "outflow_validator::fulfill_intent failed:\nstderr: {}\nstdout: {}",
                stderr,
                stdout
            );
        }

        // Extract transaction hash from output
        let output_str = String::from_utf8_lossy(&output.stdout);

        // Try to parse as JSON first (aptos CLI outputs JSON with Result wrapper)
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&output_str) {
            if let Some(hash) = json
                .get("Result")
                .and_then(|r| r.get("transaction_hash"))
                .and_then(|h| h.as_str())
            {
                return Ok(hash.to_string());
            }
        }

        // Fallback: line-based parsing
        if let Some(hash_line) = output_str
            .lines()
            .find(|l| l.contains("hash") || l.contains("Hash"))
        {
            if let Some(hash) = hash_line
                .split_whitespace()
                .find(|s| s.starts_with("0x"))
            {
                return Ok(hash.to_string());
            }
            if let Some(start) = hash_line.find("\"0x") {
                if let Some(end) = hash_line[start + 1..].find('"') {
                    return Ok(hash_line[start + 1..start + 1 + end].to_string());
                }
            }
        }

        anyhow::bail!(
            "Could not extract transaction hash from output: {}",
            output_str
        )
    }
}
