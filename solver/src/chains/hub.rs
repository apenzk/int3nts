//! Hub Chain Client
//!
//! Client for interacting with the hub chain (Movement) to query intent events
//! and call fulfillment functions.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Duration;

use crate::config::ChainConfig;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Extracts transaction hash from CLI output (handles both traditional and JSON formats)
fn extract_transaction_hash(output: &str) -> Option<String> {
    // Try JSON format first: "transaction_hash": "0x..."
    if let Some(start) = output.find("\"transaction_hash\"") {
        let after_key = &output[start..];
        // Find the value after the colon
        if let Some(colon_pos) = after_key.find(':') {
            let value_part = &after_key[colon_pos + 1..];
            // Find the quoted value
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
    
    // Fall back to traditional CLI format: "Transaction hash: 0x..."
    for line in output.lines() {
        if line.contains("hash") || line.contains("Hash") {
            if let Some(hash) = line.split_whitespace().find(|s| s.starts_with("0x")) {
                return Some(hash.to_string());
            }
        }
    }
    
    None
}

// ============================================================================
// TYPE DEFINITIONS
// ============================================================================

/// Move VM Optional wrapper: {"vec": [value]} or {"vec": []}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveOption<T> {
    pub vec: Vec<T>,
}

impl<T> MoveOption<T> {
    pub fn into_option(mut self) -> Option<T> {
        self.vec.pop()
    }
}

/// Move VM Inner wrapper: {"inner": value}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveInner {
    pub inner: String,
}

/// Event emitted when an intent is created on the hub chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentCreatedEvent {
    /// Intent object address
    pub intent_addr: String,
    /// Intent ID for cross-chain linking
    pub intent_id: String,
    /// Offered token metadata (wrapped in {"inner": "0x..."})
    pub offered_metadata: MoveInner,
    /// Offered amount
    pub offered_amount: String,
    /// Offered chain ID
    pub offered_chain_id: String,
    /// Desired token metadata (wrapped in {"inner": "0x..."})
    pub desired_metadata: MoveInner,
    /// Desired metadata address for cross-chain tokens (optional)
    #[serde(default)]
    pub desired_metadata_addr: Option<MoveOption<String>>,
    /// Desired amount
    pub desired_amount: String,
    /// Desired chain ID
    pub desired_chain_id: String,
    /// Requester address
    pub requester_addr: String,
    /// Expiry timestamp
    pub expiry_time: String,
    /// Minimum reported oracle value (optional)
    #[serde(default)]
    pub min_reported_value: Option<String>,
    /// Whether the intent is revocable (optional)
    #[serde(default)]
    pub revocable: Option<bool>,
    /// Reserved solver address (optional)
    #[serde(default)]
    pub reserved_solver: Option<MoveOption<String>>,
    /// Requester address on the connected chain (for outflow intents)
    /// Wrapped in Move Option: {"vec": ["0x..."]} or {"vec": []}
    #[serde(default)]
    pub requester_addr_connected_chain: Option<MoveOption<String>>,
}

/// Event emitted when an intent is fulfilled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentFulfilledEvent {
    /// Intent ID
    pub intent_id: String,
    /// Intent object address
    pub intent_addr: String,
    /// Solver address
    pub solver: String,
    /// Provided token metadata
    pub provided_metadata: serde_json::Value,
    /// Provided amount
    pub provided_amount: String,
    /// Timestamp
    pub timestamp: String,
}

/// Client for interacting with the hub chain
pub struct HubChainClient {
    /// HTTP client for RPC calls
    client: Client,
    /// Base RPC URL
    base_url: String,
    /// Module address
    module_addr: String,
    /// CLI profile name
    profile: String,
    /// E2E mode flag: if true, use aptos CLI with profiles; if false, use movement CLI with private keys
    e2e_mode: bool,
}

impl HubChainClient {
    /// Creates a new hub chain client
    ///
    /// # Arguments
    ///
    /// * `config` - Hub chain configuration
    ///
    /// # Returns
    ///
    /// * `Ok(HubChainClient)` - Successfully created client
    /// * `Err(anyhow::Error)` - Failed to create client
    pub fn new(config: &ChainConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .no_proxy() // Avoid macOS system-configuration issues in tests
            .build()
            .context("Failed to create HTTP client")?;

        // Normalize base_url: strip trailing /v1 if present (we add it in each request)
        let base_url = config.rpc_url
            .trim_end_matches('/')
            .trim_end_matches("/v1")
            .to_string();
        
        Ok(Self {
            client,
            base_url,
            module_addr: config.module_addr.clone(),
            profile: config.profile.clone(),
            e2e_mode: config.e2e_mode,
        })
    }

    /// Queries the hub chain for intent creation events
    ///
    /// This queries known accounts for LimitOrderEvent and OracleLimitOrderEvent
    /// to detect when new intents are created.
    ///
    /// # Arguments
    ///
    /// * `known_accounts` - List of account addresses to query
    /// * `since_version` - Optional transaction version to start from (for pagination)
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<IntentCreatedEvent>)` - List of intent creation events
    /// * `Err(anyhow::Error)` - Failed to query events
    pub async fn get_intent_events(
        &self,
        known_accounts: &[String],
        since_version: Option<u64>,
        processed_transactions: Option<&std::collections::HashSet<String>>,
    ) -> Result<(Vec<IntentCreatedEvent>, Vec<String>)> {
        let mut events = Vec::new();
        let mut transaction_hashes = Vec::new();

        for account in known_accounts {
            let account_addr = account.strip_prefix("0x").unwrap_or(account);
            let url = format!("{}/v1/accounts/{}/transactions", self.base_url, account_addr);

            tracing::trace!("Querying transactions from: {}", url);

            let mut query_params = vec![("limit", "100".to_string())];
            if let Some(version) = since_version {
                query_params.push(("start", version.to_string()));
            }

            let response = self
                .client
                .get(&url)
                .query(&query_params)
                .send()
                .await
                .context(format!("Failed to query transactions for account {}", account))?;

            let status = response.status();
            if !status.is_success() {
                let error_body = response.text().await.unwrap_or_default();
                tracing::debug!("Query failed for account {}: HTTP {} - {}", account, status, error_body);
                continue;
            }

            let transactions: Vec<serde_json::Value> = response
                .json()
                .await
                .context("Failed to parse transactions response")?;

            // Count skipped vs new transactions
            let mut skipped_count = 0;
            let mut new_count = 0;
            
            // Extract intent creation events from transactions
            for tx in &transactions {
                let tx_hash = tx.get("hash").and_then(|h| h.as_str()).unwrap_or("unknown");
                
                // Skip already-processed transactions
                if let Some(processed) = processed_transactions {
                    if processed.contains(tx_hash) {
                        skipped_count += 1;
                        continue; // Skip this transaction - already processed
                    }
                }
                new_count += 1;
                
                // Track this transaction hash (will be added to processed set after processing)
                let tx_hash_owned = tx_hash.to_string();
                
                if let Some(tx_events) = tx.get("events").and_then(|e| e.as_array()) {
                    tracing::debug!("Transaction {} has {} events", tx_hash, tx_events.len());
                    for (idx, event_json) in tx_events.iter().enumerate() {
                        let event_type = event_json
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("");

                        // Log ALL event types for debugging (only for new transactions)
                        tracing::debug!("Transaction {} event {}: type = '{}'", tx_hash, idx, event_type);
                        
                        // Log full event structure for first event of each transaction to see structure
                        if idx == 0 {
                            tracing::debug!("Transaction {} first event keys: {:?}", tx_hash, 
                                event_json.as_object().map(|o| o.keys().collect::<Vec<_>>()));
                        }
                        
                        // Log all event types for debugging
                        if event_type.contains("intent") || event_type.contains("Intent") || 
                           event_type.contains("Order") || event_type.contains("order") {
                            tracing::info!("Found potentially relevant event type: {}", event_type);
                        }

                        // Check for LimitOrderEvent (inflow) or OracleLimitOrderEvent (outflow)
                        // IMPORTANT: Check OracleLimitOrderEvent BEFORE LimitOrderEvent because
                        // "OracleLimitOrderEvent".contains("LimitOrderEvent") is true!
                        if event_type.contains("OracleLimitOrderEvent") || event_type.contains("LimitOrderEvent") {
                            tracing::info!("Found intent creation event: {}", event_type);
                            match serde_json::from_value::<IntentCreatedEvent>(
                                event_json.get("data").cloned().unwrap_or(serde_json::Value::Null),
                            ) {
                                Ok(event_data) => {
                                    // Check if event is expired before logging/adding
                                    let current_time = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs();
                                    if let Ok(expiry) = event_data.expiry_time.parse::<u64>() {
                                        if expiry >= current_time {
                                            tracing::info!("Intent {} is valid (expiry {} >= now {})", 
                                                event_data.intent_id, expiry, current_time);
                                            events.push(event_data);
                                        } else {
                                            tracing::debug!("Intent {} is expired (expiry {} < now {})", 
                                                event_data.intent_id, expiry, current_time);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to parse intent event data: {} - data: {:?}", e, event_json.get("data"));
                                }
                            }
                        }
                    }
                }
                
                // Add transaction hash to processed list (even if no intent event found, to avoid re-parsing)
                transaction_hashes.push(tx_hash_owned);
            }
            
            // Only log when there are new transactions to process
            if new_count > 0 {
                tracing::debug!("Account {}: {} new transaction(s), {} already processed", account, new_count, skipped_count);
            }
        }

        Ok((events, transaction_hashes))
    }

    /// Fulfills an inflow request intent
    ///
    /// Calls the `fulfill_inflow_intent` entry function on the hub chain.
    ///
    /// # Arguments
    ///
    /// * `intent_addr` - Object address of the intent to fulfill
    /// * `payment_amount` - Amount of tokens to provide
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction hash
    /// * `Err(anyhow::Error)` - Failed to fulfill intent
    pub fn fulfill_inflow_intent(
        &self,
        intent_addr: &str,
        payment_amount: u64,
    ) -> Result<String> {
        // Determine CLI based on e2e_mode flag
        let cli = if self.e2e_mode {
            "aptos"
        } else {
            "movement"
        };

        // Store formatted strings to avoid lifetime issues
        let function_id = format!("{}::fa_intent_inflow::fulfill_inflow_intent", self.module_addr);
        let intent_addr_arg = format!("address:{}", intent_addr);
        let payment_amount_arg = format!("u64:{}", payment_amount);

        // Prepare private key if needed (for testnet mode)
        let pk_hex_stripped = if !self.e2e_mode {
            let pk_hex = std::env::var("MOVEMENT_SOLVER_PRIVATE_KEY")
                .context("MOVEMENT_SOLVER_PRIVATE_KEY not set")?;
            Some(pk_hex.strip_prefix("0x").unwrap_or(&pk_hex).to_string())
        } else {
            None
        };

        let mut args = vec![
            "move",
            "run",
            "--assume-yes",
            "--function-id",
            &function_id,
            "--args",
            &intent_addr_arg,
            &payment_amount_arg,
        ];

        // Add authentication based on e2e_mode flag
        if self.e2e_mode {
            // Use profile for E2E tests
            args.extend(vec![
                "--profile",
                &self.profile,
            ]);
        } else {
            // Use private key directly for testnet
            args.extend(vec![
                "--private-key",
                pk_hex_stripped.as_ref().unwrap(),
                "--url",
                &self.base_url,
            ]);
        }

        let output = Command::new(cli)
            .args(&args)
            .output()
            .context(format!("Failed to execute {} move run", cli))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "{} move run failed:\nstderr: {}\nstdout: {}",
                cli,
                stderr,
                stdout
            );
        }

        // Extract transaction hash from output
        // Handles both formats:
        // - Traditional CLI: "Transaction hash: 0x..."
        // - JSON format: "transaction_hash": "0x..."
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(hash) = extract_transaction_hash(&output_str) {
            return Ok(hash);
        }

        anyhow::bail!("Could not extract transaction hash from output: {}", output_str)
    }

    /// Fulfills an outflow request intent
    ///
    /// Calls the `fulfill_outflow_intent` entry function on the hub chain.
    ///
    /// # Arguments
    ///
    /// * `intent_addr` - Object address of the intent to fulfill
    /// * `approval_signature_bytes` - Trusted-gmp's Ed25519 signature as bytes (on-chain approval address)
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction hash
    /// * `Err(anyhow::Error)` - Failed to fulfill intent
    pub fn fulfill_outflow_intent(
        &self,
        intent_addr: &str,
        approval_signature_bytes: &[u8],
    ) -> Result<String> {
        // Convert signature bytes to hex string
        let signature_hex = hex::encode(approval_signature_bytes);

        // Determine CLI based on e2e_mode flag
        let cli = if self.e2e_mode {
            "aptos"
        } else {
            "movement"
        };

        // Store formatted strings to avoid lifetime issues
        let function_id = format!("{}::fa_intent_outflow::fulfill_outflow_intent", self.module_addr);
        let intent_addr_arg = format!("address:{}", intent_addr);
        let signature_hex_arg = format!("hex:{}", signature_hex);

        // Prepare private key if needed (for testnet mode)
        let pk_hex_stripped = if !self.e2e_mode {
            let pk_hex = std::env::var("MOVEMENT_SOLVER_PRIVATE_KEY")
                .context("MOVEMENT_SOLVER_PRIVATE_KEY not set")?;
            Some(pk_hex.strip_prefix("0x").unwrap_or(&pk_hex).to_string())
        } else {
            None
        };

        let mut args = vec![
            "move",
            "run",
            "--assume-yes",
            "--function-id",
            &function_id,
            "--args",
            &intent_addr_arg,
            &signature_hex_arg,
        ];

        // Add authentication based on e2e_mode flag
        if self.e2e_mode {
            // Use profile for E2E tests
            args.extend(vec![
                "--profile",
                &self.profile,
            ]);
        } else {
            // Use private key directly for testnet
            args.extend(vec![
                "--private-key",
                pk_hex_stripped.as_ref().unwrap(),
                "--url",
                &self.base_url,
            ]);
        }

        let output = Command::new(cli)
            .args(&args)
            .output()
            .context(format!("Failed to execute {} move run", cli))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "{} move run failed:\nstderr: {}\nstdout: {}",
                cli,
                stderr,
                stdout
            );
        }

        // Extract transaction hash from output
        // Handles both formats:
        // - Traditional CLI: "Transaction hash: 0x..."
        // - JSON format: "transaction_hash": "0x..."
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(hash) = extract_transaction_hash(&output_str) {
            return Ok(hash);
        }

        anyhow::bail!("Could not extract transaction hash from output: {}", output_str)
    }

    /// Checks if escrow is confirmed for an intent on the hub chain.
    ///
    /// Calls the `gmp_intent_state::is_escrow_confirmed` view function.
    /// Returns true if the EscrowConfirmation GMP message was received and processed.
    ///
    /// # Arguments
    ///
    /// * `intent_id` - Intent ID as hex string (e.g., "0x4b1e...")
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - True if escrow is confirmed
    /// * `Err(anyhow::Error)` - Failed to query
    pub async fn is_escrow_confirmed(&self, intent_id: &str) -> Result<bool> {
        // Normalize to 64-char hex: Move strips leading zeros from addresses in events,
        // producing odd-length hex that the Aptos REST API rejects.
        let without_prefix = intent_id.strip_prefix("0x").unwrap_or(intent_id);
        let intent_id_hex = format!("0x{:0>64}", without_prefix);

        let view_url = format!("{}/v1/view", self.base_url);
        let request_body = serde_json::json!({
            "function": format!("{}::gmp_intent_state::is_escrow_confirmed", self.module_addr),
            "type_arguments": [],
            "arguments": [intent_id_hex]
        });

        let response = self
            .client
            .post(&view_url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to query escrow confirmation")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to query escrow confirmation: HTTP {} - {}",
                status,
                error_body
            );
        }

        let result: Vec<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse escrow confirmation response")?;

        if let Some(first_result) = result.first() {
            if let Some(is_confirmed) = first_result.as_bool() {
                return Ok(is_confirmed);
            }
        }

        anyhow::bail!("Unexpected response format from is_escrow_confirmed view function")
    }

    /// Checks if a FulfillmentProof has been received via GMP for an outflow intent.
    ///
    /// Calls the `gmp_intent_state::is_fulfillment_proof_received` view function.
    /// Returns true if the FulfillmentProof GMP message was received from the connected chain.
    ///
    /// # Arguments
    ///
    /// * `intent_id` - Intent ID as hex string (e.g., "0x4b1e...")
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - True if FulfillmentProof was received
    /// * `Err(anyhow::Error)` - Failed to query
    pub async fn is_fulfillment_proof_received(&self, intent_id: &str) -> Result<bool> {
        let without_prefix = intent_id.strip_prefix("0x").unwrap_or(intent_id);
        let intent_id_hex = format!("0x{:0>64}", without_prefix);

        let view_url = format!("{}/v1/view", self.base_url);
        let request_body = serde_json::json!({
            "function": format!("{}::gmp_intent_state::is_fulfillment_proof_received", self.module_addr),
            "type_arguments": [],
            "arguments": [intent_id_hex]
        });

        let response = self
            .client
            .post(&view_url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to query fulfillment proof status")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to query fulfillment proof status: HTTP {} - {}",
                status,
                error_body
            );
        }

        let result: Vec<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse fulfillment proof response")?;

        if let Some(first_result) = result.first() {
            if let Some(received) = first_result.as_bool() {
                return Ok(received);
            }
        }

        anyhow::bail!("Unexpected response format from is_fulfillment_proof_received view function")
    }

    /// Fulfills an outflow intent on the hub using GMP proof (no approval signature needed).
    ///
    /// After FulfillmentProof is delivered via GMP, the solver calls this to claim locked tokens.
    /// The Move function checks `gmp_intent_state::is_fulfillment_proof_received` internally.
    ///
    /// # Arguments
    ///
    /// * `intent_addr` - Object address of the intent to fulfill
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction hash
    /// * `Err(anyhow::Error)` - Failed to fulfill intent
    pub fn fulfill_outflow_intent_gmp(&self, intent_addr: &str) -> Result<String> {
        let cli = if self.e2e_mode { "aptos" } else { "movement" };
        let function_id = format!("{}::fa_intent_outflow::fulfill_outflow_intent", self.module_addr);
        let intent_addr_arg = format!("address:{}", intent_addr);

        let pk_hex_stripped = if !self.e2e_mode {
            let pk_hex = std::env::var("MOVEMENT_SOLVER_PRIVATE_KEY")
                .context("MOVEMENT_SOLVER_PRIVATE_KEY not set")?;
            Some(pk_hex.strip_prefix("0x").unwrap_or(&pk_hex).to_string())
        } else {
            None
        };

        let mut args = vec![
            "move",
            "run",
            "--assume-yes",
            "--function-id",
            &function_id,
            "--args",
            &intent_addr_arg,
        ];

        if self.e2e_mode {
            args.extend(vec!["--profile", &self.profile]);
        } else {
            args.extend(vec![
                "--private-key",
                pk_hex_stripped.as_ref().unwrap(),
                "--url",
                &self.base_url,
            ]);
        }

        let output = Command::new(cli)
            .args(&args)
            .output()
            .context(format!("Failed to execute {} move run", cli))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "{} fulfill_outflow_intent_gmp failed:\nstderr: {}\nstdout: {}",
                cli,
                stderr,
                stdout
            );
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(hash) = extract_transaction_hash(&output_str) {
            return Ok(hash);
        }

        anyhow::bail!(
            "Could not extract transaction hash from output: {}",
            output_str
        )
    }

    /// Checks if a solver is registered in the solver registry
    ///
    /// # Arguments
    ///
    /// * `solver_addr` - Solver address to check
    ///
    /// # Returns
    ///
    /// * `Ok(bool)` - True if solver is registered, false otherwise
    /// * `Err(anyhow::Error)` - Failed to query registration status
    pub async fn is_solver_registered(&self, solver_addr: &str) -> Result<bool> {
        // Normalize address (ensure 0x prefix)
        let solver_addr_normalized = if solver_addr.starts_with("0x") {
            solver_addr.to_string()
        } else {
            format!("0x{}", solver_addr)
        };

        // Call the view function via RPC
        let view_url = format!("{}/v1/view", self.base_url);
        let request_body = serde_json::json!({
            "function": format!("{}::solver_registry::is_registered", self.module_addr),
            "type_arguments": [],
            "arguments": [solver_addr_normalized]
        });

        let response = self
            .client
            .post(&view_url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to query solver registration")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to query solver registration: HTTP {} - {}",
                status,
                error_body
            );
        }

        let result: Vec<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse registration check response")?;

        // The view function returns a bool, which is serialized as a JSON boolean
        if let Some(first_result) = result.first() {
            if let Some(is_registered) = first_result.as_bool() {
                return Ok(is_registered);
            }
        }

        anyhow::bail!("Unexpected response format from is_registered view function")
    }

    /// Registers the solver on-chain
    ///
    /// # Arguments
    ///
    /// * `public_key_bytes` - Ed25519 public key as bytes (32 bytes)
    /// * `mvm_addr` - Move VM address on connected chain, or None if not applicable
    /// * `evm_addr` - EVM address on connected chain (20 bytes), or empty vec if not applicable
    /// * `svm_addr` - SVM address on connected chain (32 bytes), or empty vec if not applicable
    /// * `private_key` - Optional private key bytes. If provided, uses --private-key flag with movement CLI.
    ///                   If None, uses --profile flag with aptos CLI (for E2E tests).
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction hash
    /// * `Err(anyhow::Error)` - Failed to register solver
    pub fn register_solver(
        &self,
        public_key_bytes: &[u8],
        mvm_addr: Option<&str>,
        evm_addr: &[u8],
        svm_addr: &[u8],
        private_key: Option<&[u8; 32]>,
    ) -> Result<String> {
        // Convert public key to hex
        let public_key_hex = hex::encode(public_key_bytes);
        
        // Prepare MVM address (use 0x0 if None)
        let mvm_addr_normalized = mvm_addr.unwrap_or("0x0");

        // Convert EVM address to hex (pad to 20 bytes if needed)
        let evm_addr_hex = if evm_addr.is_empty() {
            "".to_string()
        } else {
            hex::encode(evm_addr)
        };
        
        // Build command arguments - store formatted strings to avoid temporary value issues
        // Movement CLI expects 'hex:' for vector<u8> types, not 'vector<u8>:'
        let function_id = format!("{}::solver_registry::register_solver", self.module_addr);
        let public_key_arg = format!("hex:{}", public_key_hex);
        let mvm_addr_arg = format!("address:{}", mvm_addr_normalized);
        let evm_addr_arg = if evm_addr_hex.is_empty() {
            "hex:".to_string()
        } else {
            format!("hex:{}", evm_addr_hex)
        };
        let svm_addr_hex = if svm_addr.is_empty() {
            "".to_string()
        } else {
            hex::encode(svm_addr)
        };
        let svm_addr_arg = if svm_addr_hex.is_empty() {
            "hex:".to_string()
        } else {
            format!("hex:{}", svm_addr_hex)
        };
        
        // Format private key if provided
        let private_key_hex = private_key.map(|pk| format!("0x{}", hex::encode(pk)));
        
        // Build command based on whether we have a private key or profile
        let (cli, args): (&str, Vec<&str>) = if let Some(ref pk_hex) = private_key_hex {
            // Use movement CLI with --private-key for testnet
            (
                "movement",
                vec![
                    "move",
                    "run",
                    "--private-key",
                    pk_hex,
                    "--url",
                    &self.base_url,
                    "--assume-yes",
                    "--function-id",
                    &function_id,
                    "--args",
                    &public_key_arg,
                    &mvm_addr_arg,
                    &evm_addr_arg,
                    &svm_addr_arg,
                ],
            )
        } else {
            // Use aptos CLI with --profile for E2E tests
            (
                "aptos",
                vec![
                    "move",
                    "run",
                    "--profile",
                    &self.profile,
                    "--assume-yes",
                    "--function-id",
                    &function_id,
                    "--args",
                    &public_key_arg,
                    &mvm_addr_arg,
                    &evm_addr_arg,
                    &svm_addr_arg,
                ],
            )
        };
        
        let output = Command::new(cli)
            .args(&args)
            .output()
            .context(format!("Failed to execute {} move run for solver registration", cli))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "{} move run failed for solver registration:\nstderr: {}\nstdout: {}",
                cli,
                stderr,
                stdout
            );
        }

        // Extract transaction hash from output
        // Handles both formats:
        // - Traditional CLI: "Transaction hash: 0x..."
        // - JSON format: "transaction_hash": "0x..."
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(hash) = extract_transaction_hash(&output_str) {
            return Ok(hash);
        }

        anyhow::bail!("Could not extract transaction hash from registration output: {}", output_str)
    }

    /// Gets the solver's current registration info from the registry
    ///
    /// # Arguments
    ///
    /// * `solver_addr` - Solver address to query
    ///
    /// # Returns
    ///
    /// * `Ok(SolverRegistrationInfo)` - Current registration info
    /// * `Err(anyhow::Error)` - Failed to query or solver not registered
    pub async fn get_solver_info(&self, solver_addr: &str) -> Result<SolverRegistrationInfo> {
        // Normalize address (ensure 0x prefix)
        let solver_addr_normalized = if solver_addr.starts_with("0x") {
            solver_addr.to_string()
        } else {
            format!("0x{}", solver_addr)
        };

        // Call the view function via RPC
        let view_url = format!("{}/v1/view", self.base_url);
        let request_body = serde_json::json!({
            "function": format!("{}::solver_registry::get_solver_info", self.module_addr),
            "type_arguments": [],
            "arguments": [solver_addr_normalized]
        });

        let response = self
            .client
            .post(&view_url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to query solver info")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to query solver info: HTTP {} - {}",
                status,
                error_body
            );
        }

        let result: Vec<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse solver info response")?;

        // Handle different response formats depending on deployed module version:
        // - 6 elements: (is_registered, public_key, mvm_addr, evm_addr, svm_addr, registered_at)
        // - 5 elements: (is_registered, public_key, evm_addr, svm_addr, registered_at) - older version without mvm_addr
        
        tracing::debug!("get_solver_info response: {} elements", result.len());
        
        if result.len() >= 5 {
            let is_registered = result[0].as_bool().unwrap_or(false);
            if !is_registered {
                anyhow::bail!("Solver is not registered");
            }

            let public_key = parse_hex_from_json(&result[1]);
            
            // Parse based on number of elements
            let (mvm_addr, evm_addr, svm_addr) = if result.len() >= 6 {
                // New format with mvm_addr
                (
                    parse_optional_address(&result[2]),
                    parse_optional_hex(&result[3]),
                    parse_optional_hex(&result[4]),
                )
            } else {
                // Old format without mvm_addr (5 elements)
                (
                    None,
                    parse_optional_hex(&result[2]),
                    parse_optional_hex(&result[3]),
                )
            };

            tracing::debug!(
                "Parsed solver info: public_key={} bytes, mvm={:?}, evm={} bytes, svm={} bytes",
                public_key.len(),
                mvm_addr,
                evm_addr.len(),
                svm_addr.len()
            );

            return Ok(SolverRegistrationInfo {
                public_key,
                mvm_addr,
                evm_addr,
                svm_addr,
            });
        }

        anyhow::bail!(
            "Unexpected response format from get_solver_info view function: got {} elements, expected 5 or 6. Response: {:?}",
            result.len(),
            result
        )
    }

    /// Updates the solver's registration on-chain
    ///
    /// # Arguments
    ///
    /// * `public_key_bytes` - Ed25519 public key as bytes (32 bytes)
    /// * `mvm_addr` - Move VM address on connected chain, or None if not applicable
    /// * `evm_addr` - EVM address on connected chain (20 bytes), or empty vec if not applicable
    /// * `svm_addr` - SVM address on connected chain (32 bytes), or empty vec if not applicable
    /// * `private_key` - Optional private key bytes. If provided, uses --private-key flag with movement CLI.
    ///                   If None, uses --profile flag with aptos CLI (for E2E tests).
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction hash
    /// * `Err(anyhow::Error)` - Failed to update solver
    pub fn update_solver(
        &self,
        public_key_bytes: &[u8],
        mvm_addr: Option<&str>,
        evm_addr: &[u8],
        svm_addr: &[u8],
        private_key: Option<&[u8; 32]>,
    ) -> Result<String> {
        // Convert public key to hex
        let public_key_hex = hex::encode(public_key_bytes);
        
        // Prepare MVM address (use 0x0 if None)
        let mvm_addr_normalized = mvm_addr.unwrap_or("0x0");

        // Convert EVM address to hex
        let evm_addr_hex = if evm_addr.is_empty() {
            "".to_string()
        } else {
            hex::encode(evm_addr)
        };
        
        // Convert SVM address to hex
        let svm_addr_hex = if svm_addr.is_empty() {
            "".to_string()
        } else {
            hex::encode(svm_addr)
        };
        
        // Build command arguments
        let function_id = format!("{}::solver_registry::update_solver", self.module_addr);
        let public_key_arg = format!("hex:{}", public_key_hex);
        let mvm_addr_arg = format!("address:{}", mvm_addr_normalized);
        let evm_addr_arg = if evm_addr_hex.is_empty() {
            "hex:".to_string()
        } else {
            format!("hex:{}", evm_addr_hex)
        };
        let svm_addr_arg = if svm_addr_hex.is_empty() {
            "hex:".to_string()
        } else {
            format!("hex:{}", svm_addr_hex)
        };
        
        // Format private key if provided
        let private_key_hex = private_key.map(|pk| format!("0x{}", hex::encode(pk)));
        
        // Build command based on whether we have a private key or profile
        let (cli, args): (&str, Vec<&str>) = if let Some(ref pk_hex) = private_key_hex {
            // Use movement CLI with --private-key for testnet
            (
                "movement",
                vec![
                    "move",
                    "run",
                    "--private-key",
                    pk_hex,
                    "--url",
                    &self.base_url,
                    "--assume-yes",
                    "--function-id",
                    &function_id,
                    "--args",
                    &public_key_arg,
                    &mvm_addr_arg,
                    &evm_addr_arg,
                    &svm_addr_arg,
                ],
            )
        } else {
            // Use aptos CLI with --profile for E2E tests
            (
                "aptos",
                vec![
                    "move",
                    "run",
                    "--profile",
                    &self.profile,
                    "--assume-yes",
                    "--function-id",
                    &function_id,
                    "--args",
                    &public_key_arg,
                    &mvm_addr_arg,
                    &evm_addr_arg,
                    &svm_addr_arg,
                ],
            )
        };
        
        tracing::info!("Updating solver registration with {} CLI", cli);
        
        let output = Command::new(cli)
            .args(&args)
            .output()
            .context(format!("Failed to execute {} move run for solver update", cli))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "{} move run failed for solver update:\nstderr: {}\nstdout: {}",
                cli,
                stderr,
                stdout
            );
        }

        // Extract transaction hash from output
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(hash) = extract_transaction_hash(&output_str) {
            return Ok(hash);
        }

        anyhow::bail!("Could not extract transaction hash from update output: {}", output_str)
    }
}

/// Solver registration info from the registry
#[derive(Debug, Clone)]
pub struct SolverRegistrationInfo {
    pub public_key: Vec<u8>,
    pub mvm_addr: Option<String>,
    pub evm_addr: Vec<u8>,
    pub svm_addr: Vec<u8>,
}

/// Parse hex bytes from JSON value (handles Move's vector<u8> format)
fn parse_hex_from_json(value: &serde_json::Value) -> Vec<u8> {
    if let Some(s) = value.as_str() {
        // Format: "0x..." hex string
        let hex_str = s.strip_prefix("0x").unwrap_or(s);
        hex::decode(hex_str).unwrap_or_default()
    } else {
        Vec::new()
    }
}

/// Parse optional address from JSON (Move's Option<address>)
fn parse_optional_address(value: &serde_json::Value) -> Option<String> {
    // Move Option is serialized as: {"vec": []} for None, {"vec": ["0x..."]} for Some
    if let Some(obj) = value.as_object() {
        if let Some(vec_val) = obj.get("vec") {
            if let Some(arr) = vec_val.as_array() {
                if let Some(first) = arr.first() {
                    if let Some(s) = first.as_str() {
                        return Some(s.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Parse optional hex bytes from JSON (Move's Option<vector<u8>>)
fn parse_optional_hex(value: &serde_json::Value) -> Vec<u8> {
    // Move Option<vector<u8>> is serialized as: {"vec": []} for None, {"vec": ["0x..."]} for Some
    if let Some(obj) = value.as_object() {
        if let Some(vec_val) = obj.get("vec") {
            if let Some(arr) = vec_val.as_array() {
                if let Some(first) = arr.first() {
                    return parse_hex_from_json(first);
                }
            }
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_hex_from_json_with_prefix() {
        let value = json!("0xdeadbeef");
        let result = parse_hex_from_json(&value);
        assert_eq!(result, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn test_parse_hex_from_json_without_prefix() {
        let value = json!("deadbeef");
        let result = parse_hex_from_json(&value);
        assert_eq!(result, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn test_parse_hex_from_json_empty() {
        let value = json!("");
        let result = parse_hex_from_json(&value);
        assert_eq!(result, Vec::<u8>::new());
    }

    #[test]
    fn test_parse_hex_from_json_not_string() {
        let value = json!(123);
        let result = parse_hex_from_json(&value);
        assert_eq!(result, Vec::<u8>::new());
    }

    #[test]
    fn test_parse_optional_address_some() {
        // Move Option<address> with value: {"vec": ["0x1234..."]}
        let value = json!({"vec": ["0x92759d64e3225b2c8455562cdbf5be4f7461cd3555d29a1b124db503874603f5"]});
        let result = parse_optional_address(&value);
        assert_eq!(result, Some("0x92759d64e3225b2c8455562cdbf5be4f7461cd3555d29a1b124db503874603f5".to_string()));
    }

    #[test]
    fn test_parse_optional_address_none() {
        // Move Option<address> with no value: {"vec": []}
        let value = json!({"vec": []});
        let result = parse_optional_address(&value);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_optional_address_invalid() {
        let value = json!("not an option");
        let result = parse_optional_address(&value);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_optional_hex_some() {
        // Move Option<vector<u8>> with value: {"vec": ["0xdeadbeef"]}
        let value = json!({"vec": ["0xdeadbeef"]});
        let result = parse_optional_hex(&value);
        assert_eq!(result, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn test_parse_optional_hex_none() {
        // Move Option<vector<u8>> with no value: {"vec": []}
        let value = json!({"vec": []});
        let result = parse_optional_hex(&value);
        assert_eq!(result, Vec::<u8>::new());
    }

    #[test]
    fn test_parse_optional_hex_invalid() {
        let value = json!("not an option");
        let result = parse_optional_hex(&value);
        assert_eq!(result, Vec::<u8>::new());
    }

    #[test]
    fn test_parse_optional_hex_32_byte_address() {
        // Typical SVM address (32 bytes)
        let svm_hex = "6e5f2e9b6d3f4a1c8e7d0b2a5f4c3e8d1a0b9f7e6c5d4a3b2c1e0f9a8b7c6d5e";
        let value = json!({"vec": [format!("0x{}", svm_hex)]});
        let result = parse_optional_hex(&value);
        assert_eq!(result.len(), 32);
        assert_eq!(hex::encode(&result), svm_hex);
    }
}


