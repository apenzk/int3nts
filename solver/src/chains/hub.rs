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
    /// * `verifier_signature_bytes` - Verifier's Ed25519 signature as bytes
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction hash
    /// * `Err(anyhow::Error)` - Failed to fulfill intent
    pub fn fulfill_outflow_intent(
        &self,
        intent_addr: &str,
        verifier_signature_bytes: &[u8],
    ) -> Result<String> {
        // Convert signature bytes to hex string
        let signature_hex = hex::encode(verifier_signature_bytes);

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
    /// * `evm_addr` - EVM address on connected chain (20 bytes), or empty vec if not applicable
    /// * `mvm_addr` - Move VM address on connected chain, or None if not applicable
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
        evm_addr: &[u8],
        mvm_addr: Option<&str>,
        private_key: Option<&[u8; 32]>,
    ) -> Result<String> {
        // Convert public key to hex
        let public_key_hex = hex::encode(public_key_bytes);
        
        // Convert EVM address to hex (pad to 20 bytes if needed)
        let evm_addr_hex = if evm_addr.is_empty() {
            "".to_string()
        } else {
            hex::encode(evm_addr)
        };
        
        // Prepare MVM address (use 0x0 if None)
        let mvm_addr_normalized = mvm_addr.unwrap_or("0x0");
        
        // Build command arguments - store formatted strings to avoid temporary value issues
        // Movement CLI expects 'hex:' for vector<u8> types, not 'vector<u8>:'
        let function_id = format!("{}::solver_registry::register_solver", self.module_addr);
        let public_key_arg = format!("hex:{}", public_key_hex);
        let evm_addr_arg = if evm_addr_hex.is_empty() {
            "hex:".to_string()
        } else {
            format!("hex:{}", evm_addr_hex)
        };
        let mvm_addr_arg = format!("address:{}", mvm_addr_normalized);
        
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
                    &evm_addr_arg,
                    &mvm_addr_arg,
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
                    &evm_addr_arg,
                    &mvm_addr_arg,
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
}


