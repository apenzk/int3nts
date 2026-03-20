//! Move VM REST client for communicating with Move VM-based blockchain nodes
//! (e.g., Aptos) via their HTTP REST API.
//!
//! Handles account queries, event polling, transaction verification, and
//! solver registry lookups.

use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;

use crate::types::*;

// ============================================================================
// ADDRESS NORMALIZATION
// ============================================================================

/// Normalizes a hex string to a 64-character (32-byte) 0x-prefixed address.
///
/// Move addresses are 32 bytes but leading zeros may be stripped in event data,
/// producing odd-length hex strings that the Aptos REST API rejects.
pub fn normalize_hex_to_address(hex: &str) -> String {
    let without_prefix = hex.strip_prefix("0x").unwrap_or(hex);
    format!("0x{:0>64}", without_prefix)
}

// ============================================================================
// MOVE VM CLIENT IMPLEMENTATION
// ============================================================================

/// Client for communicating with Move VM-based blockchain nodes (e.g., Aptos) via REST API
pub struct MvmClient {
    /// HTTP client for making requests
    client: Client,
    /// Base URL of the Move VM node (e.g., "http://127.0.0.1:8080")
    base_url: String,
}

impl MvmClient {
    /// Creates a new Move VM client for the given node URL
    ///
    /// # Arguments
    ///
    /// * `node_url` - Base URL of the Move VM node (e.g., "http://127.0.0.1:8080")
    pub fn new(node_url: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .no_proxy() // Avoid macOS system-configuration issues in tests
            .build()
            .context("Failed to create HTTP client")?;

        // Normalize base_url: strip trailing /v1 if present (we add it in each request)
        let base_url = node_url
            .trim_end_matches('/')
            .trim_end_matches("/v1")
            .to_string();

        Ok(Self { client, base_url })
    }

    /// Queries account information from the Move VM blockchain
    #[allow(dead_code)]
    pub async fn get_account(&self, address: &str) -> Result<AccountInfo> {
        let url = format!("{}/v1/accounts/{}", self.base_url, address);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send account request")?
            .error_for_status()
            .context("Account request failed")?;

        let account: AccountInfo = response
            .json()
            .await
            .context("Failed to parse account response")?;

        Ok(account)
    }

    /// Queries events for a specific account on the Move VM blockchain.
    ///
    /// For modern Move VM modules that use `event::emit()`, events are stored
    /// in transaction history, not as event handles. This method queries transactions
    /// to extract module events.
    ///
    /// For legacy EventHandle events, pass the `event_handle` parameter.
    pub async fn get_account_events(
        &self,
        address: &str,
        event_handle: Option<&str>,
        start: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<MvmEvent>> {
        // For legacy EventHandle events, use the old approach
        if let Some(handle) = event_handle {
            return self
                .get_events_by_creation_number(address, handle, start, limit)
                .await;
        }

        // For modern module events, query the account's transactions to find events
        let limit = limit.unwrap_or(100);
        let url = format!("{}/v1/accounts/{}/transactions", self.base_url, address);

        let response = self
            .client
            .get(&url)
            .query(&[("limit", limit.to_string())])
            .send()
            .await
            .context("Failed to query account transactions")?;

        if !response.status().is_success() {
            return Ok(vec![]); // Account might not exist or have no transactions
        }

        let transactions: Vec<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse transactions response")?;

        // Extract events from transactions
        let mut events = Vec::new();
        for tx in transactions {
            if let Some(tx_events) = tx.get("events").and_then(|e| e.as_array()) {
                for event_json in tx_events {
                    let event_type = event_json
                        .get("type")
                        .and_then(|t| t.as_str())
                        .context("Event missing 'type' field")?
                        .to_string();
                    let event_data = event_json
                        .get("data")
                        .cloned()
                        .context("Event missing 'data' field")?;

                    let sequence_number = event_json
                        .get("sequence_number")
                        .and_then(|s| s.as_str())
                        .context("Event missing 'sequence_number' field")?
                        .to_string();

                    let guid = event_json
                        .get("guid")
                        .and_then(|g| serde_json::from_value::<EventGuid>(g.clone()).ok());

                    let key = event_json
                        .get("key")
                        .and_then(|k| k.as_str())
                        .map(|s| s.to_string());

                    events.push(MvmEvent {
                        guid,
                        key,
                        sequence_number,
                        r#type: event_type,
                        data: event_data,
                    });
                }
            }
        }

        Ok(events)
    }

    /// Queries events for a specific creation number (legacy EventHandle events)
    async fn get_events_by_creation_number(
        &self,
        address: &str,
        creation_number: &str,
        start: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<MvmEvent>> {
        let mut url = format!(
            "{}/v1/accounts/{}/events/{}",
            self.base_url, address, creation_number
        );

        let mut query_params = vec![];
        if let Some(s) = start {
            query_params.push(format!("start={}", s));
        }
        if let Some(l) = limit {
            query_params.push(format!("limit={}", l));
        }
        if !query_params.is_empty() {
            url.push('?');
            url.push_str(&query_params.join("&"));
        }

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send events request")?;

        let status = response.status();
        if status == 404 {
            return Ok(vec![]);
        }

        let response = response
            .error_for_status()
            .context("Events request failed")?;

        let events: Vec<MvmEvent> = response
            .json()
            .await
            .context("Failed to parse events response")?;

        Ok(events)
    }

    /// Gets resources for an account
    pub async fn get_resources(&self, address: &str) -> Result<Vec<ResourceData>> {
        let url = format!("{}/v1/accounts/{}/resources", self.base_url, address);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send resources request")?
            .error_for_status()
            .context("Resources request failed")?;

        let resources: Vec<ResourceData> = response
            .json()
            .await
            .context("Failed to parse resources response")?;

        Ok(resources)
    }

    /// Finds event handles matching the given event type pattern
    #[allow(dead_code)]
    pub async fn find_event_handles(
        &self,
        address: &str,
        event_type_pattern: &str,
    ) -> Result<Vec<String>> {
        let resources = self.get_resources(address).await?;
        let mut creation_numbers = Vec::new();

        for resource in resources {
            if let Some(handle_obj) = resource.data.get("handle") {
                if let Ok(handle) = serde_json::from_value::<EventHandle>(handle_obj.clone()) {
                    creation_numbers.push(handle.guid.id.creation_num.clone());
                }
            }

            if resource.resource_type.contains(event_type_pattern) {
                if let Some(handle_obj) = resource.data.get("events") {
                    if handle_obj.is_object() {
                        for (_key, value) in handle_obj.as_object().unwrap() {
                            if let Ok(handle) =
                                serde_json::from_value::<EventHandle>(value.clone())
                            {
                                creation_numbers.push(handle.guid.id.creation_num.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(creation_numbers)
    }

    /// Queries transaction details by hash
    #[allow(dead_code)]
    pub async fn get_transaction(&self, hash: &str) -> Result<MvmTransaction> {
        let url = format!("{}/v1/transactions/by_hash/{}", self.base_url, hash);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send transaction request")?
            .error_for_status()
            .context("Transaction request failed")?;

        let tx: MvmTransaction = response
            .json()
            .await
            .context("Failed to parse transaction response")?;

        Ok(tx)
    }

    /// Checks if the node is healthy and responsive
    #[allow(dead_code)]
    pub async fn health_check(&self) -> Result<()> {
        let url = format!("{}/v1", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send health check request")?
            .error_for_status()
            .context("Health check failed")?;

        response
            .text()
            .await
            .context("Failed to read health check response")?;

        Ok(())
    }

    /// Returns the base URL of this client
    #[allow(dead_code)]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    // ========================================================================
    // SOLVER REGISTRY QUERIES
    // ========================================================================

    /// Queries an intent object's reservation to get the solver address.
    #[allow(dead_code)]
    pub async fn get_intent_solver(
        &self,
        intent_addr: &str,
        _module_addr: &str,
    ) -> Result<Option<String>> {
        let resources = self.get_resources(intent_addr).await?;

        for resource in resources {
            if resource.resource_type.contains("Intent")
                && !resource.resource_type.contains("IntentReserved")
            {
                if let Some(data) = resource.data.as_object() {
                    if let Some(reservation) = data.get("reservation") {
                        if reservation.is_object() {
                            if let Some(solver) = reservation.get("solver") {
                                if let Some(solver_str) = solver.as_str() {
                                    return Ok(Some(solver_str.to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Queries the solver registry to get a solver's public key.
    #[allow(dead_code)]
    pub async fn get_solver_public_key(
        &self,
        solver_addr: &str,
        solver_registry_addr: &str,
    ) -> Result<Option<Vec<u8>>> {
        if !solver_addr.starts_with("0x") {
            return Err(anyhow::anyhow!(
                "Invalid solver address '{}': must start with 0x prefix",
                solver_addr
            ));
        }

        tracing::debug!(
            "Querying solver public key for address '{}'",
            solver_addr
        );

        let result = self
            .call_view_function(
                solver_registry_addr,
                "solver_registry",
                "get_public_key",
                vec![],
                vec![serde_json::json!(solver_addr)],
            )
            .await;

        match result {
            Ok(value) => {
                tracing::debug!(
                    "View function returned for solver '{}': {:?}",
                    solver_addr,
                    value
                );
                let outer_array = value.as_array().ok_or_else(|| {
                    anyhow::anyhow!(
                        "Unexpected response format for solver '{}': expected array, got {:?}",
                        solver_addr,
                        value
                    )
                })?;

                let first_result = outer_array.first().ok_or_else(|| {
                    anyhow::anyhow!(
                        "Empty response array for solver '{}': expected at least one element",
                        solver_addr
                    )
                })?;

                let hex_str = first_result.as_str().ok_or_else(|| {
                    anyhow::anyhow!(
                        "Unexpected response format for solver '{}': expected hex string, got {:?}",
                        solver_addr,
                        first_result
                    )
                })?;

                let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
                if hex_str.is_empty() {
                    tracing::debug!("Solver '{}' not registered (empty public key)", solver_addr);
                    Ok(None)
                } else {
                    // Decode hex manually (no hex crate dependency)
                    let bytes = (0..hex_str.len())
                        .step_by(2)
                        .map(|i| {
                            u8::from_str_radix(&hex_str[i..i + 2], 16).map_err(|e| {
                                anyhow::anyhow!(
                                    "Failed to decode hex public key for solver '{}': {}",
                                    solver_addr,
                                    e
                                )
                            })
                        })
                        .collect::<Result<Vec<u8>>>()?;
                    tracing::debug!(
                        "Solver '{}' registered with public key ({} bytes)",
                        solver_addr,
                        bytes.len()
                    );
                    Ok(Some(bytes))
                }
            }
            Err(e) => Err(anyhow::anyhow!(
                "Failed to query solver public key for '{}': {}",
                solver_addr,
                e
            )),
        }
    }

    /// Queries the solver registry to get a solver's EVM address.
    pub async fn get_solver_evm_address(
        &self,
        solver_addr: &str,
        solver_registry_addr: &str,
    ) -> Result<Option<String>> {
        tracing::debug!(
            "get_solver_evm_address called with solver_addr='{}' (len: {}, type: str), solver_registry_addr='{}' (len: {}, type: str)",
            solver_addr,
            solver_addr.len(),
            solver_registry_addr,
            solver_registry_addr.len()
        );

        let solver_addr_stripped = solver_addr
            .strip_prefix("0x")
            .unwrap_or(solver_addr)
            .to_lowercase();
        let solver_addr_normalized = format!("{:0>64}", solver_addr_stripped);

        tracing::debug!(
            "Normalized solver_addr='{}' -> normalized='{}' (len: {})",
            solver_addr,
            solver_addr_normalized,
            solver_addr_normalized.len()
        );

        let resources = self.get_resources(solver_registry_addr).await?;

        let registry_resource =
            match Self::find_solver_registry_resource(&resources, solver_registry_addr) {
                Some(resource) => resource,
                None => return Ok(None),
            };

        let data_array = match Self::extract_solvers_data_array(registry_resource) {
            Some(array) => array,
            None => return Ok(None),
        };

        let entry_obj =
            match Self::find_solver_entry(data_array, solver_addr, &solver_addr_normalized) {
                Some(entry) => entry,
                None => return Ok(None),
            };

        let solver_info = match entry_obj.get("value").and_then(|v| v.as_object()) {
            Some(info) => info,
            None => {
                let entry_keys = entry_obj.keys().collect::<Vec<_>>();
                let entry_json = serde_json::to_string(entry_obj)
                    .unwrap_or_else(|_| "failed to serialize".to_string());
                tracing::warn!(
                    "SolverInfo 'value' field not found or not an object for solver '{}'. Entry object keys: {:?}, Full entry: {}",
                    solver_addr,
                    entry_keys,
                    entry_json
                );
                return Ok(None);
            }
        };

        let evm_addr_field: &serde_json::Value =
            match solver_info.get("connected_chain_evm_addr") {
                Some(field) => field,
                None => {
                    let solver_info_keys = solver_info.keys().collect::<Vec<_>>();
                    let solver_info_json = serde_json::to_string(solver_info)
                        .unwrap_or_else(|_| "failed to serialize".to_string());
                    tracing::error!(
                        "connected_chain_evm_addr field not found for solver '{}'. SolverInfo keys: {:?}, Full SolverInfo: {}",
                        solver_addr,
                        solver_info_keys,
                        solver_info_json
                    );
                    return Ok(None);
                }
            };

        tracing::debug!(
            "connected_chain_evm_addr field for solver '{}': {}",
            solver_addr,
            serde_json::to_string(evm_addr_field)
                .unwrap_or_else(|_| "failed to serialize".to_string())
        );

        let evm_addr = match evm_addr_field.as_object() {
            Some(obj) => obj,
            None => {
                let field_json = serde_json::to_string(evm_addr_field)
                    .unwrap_or_else(|_| "failed to serialize".to_string());
                tracing::error!(
                    "connected_chain_evm_addr is not an object for solver '{}'. Value: {}",
                    solver_addr,
                    field_json
                );
                return Ok(None);
            }
        };

        let vec_array: &serde_json::Value = match evm_addr.get("vec") {
            Some(vec) => vec,
            None => {
                let evm_addr_keys = evm_addr.keys().collect::<Vec<_>>();
                let evm_addr_json = serde_json::to_string(evm_addr)
                    .unwrap_or_else(|_| "failed to serialize".to_string());
                tracing::error!(
                    "connected_chain_evm_addr 'vec' field not found for solver '{}'. EVM address object keys: {:?}, Full object: {}",
                    solver_addr,
                    evm_addr_keys,
                    evm_addr_json
                );
                return Ok(None);
            }
        };

        let evm_bytes = match Self::parse_address_bytes(vec_array, solver_addr, 20, "EVM")? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };

        let hex_string = format!(
            "0x{}",
            evm_bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        );

        tracing::debug!(
            "Successfully extracted EVM address for solver '{}': {}",
            solver_addr,
            hex_string
        );

        Ok(Some(hex_string))
    }

    /// Queries the solver registry to get a solver's SVM address.
    pub async fn get_solver_svm_address(
        &self,
        solver_addr: &str,
        solver_registry_addr: &str,
    ) -> Result<Option<String>> {
        let solver_addr_stripped = solver_addr
            .strip_prefix("0x")
            .unwrap_or(solver_addr)
            .to_lowercase();
        let solver_addr_normalized = format!("{:0>64}", solver_addr_stripped);

        let resources = self.get_resources(solver_registry_addr).await?;

        let registry_resource =
            match Self::find_solver_registry_resource(&resources, solver_registry_addr) {
                Some(resource) => resource,
                None => return Ok(None),
            };

        let data_array = match Self::extract_solvers_data_array(registry_resource) {
            Some(array) => array,
            None => return Ok(None),
        };

        let entry_obj =
            match Self::find_solver_entry(data_array, solver_addr, &solver_addr_normalized) {
                Some(entry) => entry,
                None => return Ok(None),
            };

        let solver_info = match entry_obj.get("value").and_then(|v| v.as_object()) {
            Some(info) => info,
            None => return Ok(None),
        };

        let svm_addr_field = match solver_info.get("connected_chain_svm_addr") {
            Some(field) => field,
            None => {
                tracing::debug!(
                    "connected_chain_svm_addr field not found for solver '{}'",
                    solver_addr
                );
                return Ok(None);
            }
        };

        let svm_addr = match svm_addr_field.as_object() {
            Some(obj) => obj,
            None => return Ok(None),
        };

        let vec_array = match svm_addr.get("vec") {
            Some(vec) => vec,
            None => return Ok(None),
        };

        let svm_bytes = match Self::parse_address_bytes(vec_array, solver_addr, 32, "SVM")? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };

        let hex_string = format!(
            "0x{}",
            svm_bytes
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        );
        Ok(Some(hex_string))
    }

    /// Queries the solver registry to get a solver's connected chain Move VM address.
    ///
    /// This reads the `connected_chain_mvm_addr` field from SolverInfo, which is
    /// an `Option<address>` (stored as a string, not bytes).
    pub async fn get_solver_mvm_address(
        &self,
        solver_addr: &str,
        solver_registry_addr: &str,
    ) -> Result<Option<String>> {
        let solver_addr_stripped = solver_addr
            .strip_prefix("0x")
            .unwrap_or(solver_addr)
            .to_lowercase();
        let solver_addr_normalized = format!("{:0>64}", solver_addr_stripped);

        let resources = self.get_resources(solver_registry_addr).await?;

        let registry_resource =
            match Self::find_solver_registry_resource(&resources, solver_registry_addr) {
                Some(resource) => resource,
                None => return Ok(None),
            };

        let data_array = match Self::extract_solvers_data_array(registry_resource) {
            Some(array) => array,
            None => return Ok(None),
        };

        let entry_obj =
            match Self::find_solver_entry(data_array, solver_addr, &solver_addr_normalized) {
                Some(entry) => entry,
                None => return Ok(None),
            };

        let value = match entry_obj.get("value").and_then(|v| v.as_object()) {
            Some(v) => v,
            None => return Ok(None),
        };

        // connected_chain_mvm_addr is Option<address>, serialized as {"vec": [address_string]}
        let mvm_addr = match value
            .get("connected_chain_mvm_addr")
            .and_then(|m| m.as_object())
        {
            Some(m) => m,
            None => return Ok(None),
        };

        let vec_array = match mvm_addr.get("vec").and_then(|v| v.as_array()) {
            Some(v) => v,
            None => return Ok(None),
        };

        if vec_array.is_empty() {
            return Ok(None);
        }

        match vec_array.get(0).and_then(|a| a.as_str()) {
            Some(addr_str) => Ok(Some(addr_str.to_string())),
            None => Ok(None),
        }
    }

    // ========================================================================
    // SOLVER REGISTRY HELPERS (private)
    // ========================================================================

    /// Find the SolverRegistry resource from the resources list.
    ///
    /// Handles multiple resource type formats (with/without 0x prefix, with/without leading zeros).
    /// Move strips leading zeros from addresses in type names (e.g., 0x0a4c... becomes 0xa4c...).
    fn find_solver_registry_resource<'a>(
        resources: &'a [ResourceData],
        solver_registry_addr: &str,
    ) -> Option<&'a ResourceData> {
        let registry_addr_normalized = solver_registry_addr
            .strip_prefix("0x")
            .unwrap_or(solver_registry_addr)
            .trim_start_matches('0');

        let registry_resource_type_with_prefix =
            format!("0x{}::solver_registry::SolverRegistry", registry_addr_normalized);
        let registry_resource_type_without_prefix =
            format!("{}::solver_registry::SolverRegistry", registry_addr_normalized);

        let registry_addr_with_zeros = solver_registry_addr
            .strip_prefix("0x")
            .unwrap_or(solver_registry_addr);
        let registry_resource_type_with_zeros =
            format!("0x{}::solver_registry::SolverRegistry", registry_addr_with_zeros);

        let resource = resources.iter().find(|r| {
            r.resource_type == registry_resource_type_with_prefix
                || r.resource_type == registry_resource_type_without_prefix
                || r.resource_type == registry_resource_type_with_zeros
        });

        if resource.is_none() {
            tracing::warn!(
                "SolverRegistry resource not found. Registry address: {}, Tried types: '{}', '{}', and '{}', Available resources: {:?}",
                solver_registry_addr,
                registry_resource_type_with_prefix,
                registry_resource_type_without_prefix,
                registry_resource_type_with_zeros,
                resources.iter().map(|r| &r.resource_type).collect::<Vec<_>>()
            );
        }

        resource
    }

    /// Extract the solvers data array from the SolverRegistry resource.
    ///
    /// SimpleMap<address, SolverInfo> is serialized as {"data": [{"key": address, "value": SolverInfo}, ...]}
    fn extract_solvers_data_array(registry_resource: &ResourceData) -> Option<&serde_json::Value> {
        let data = registry_resource.data.as_object()?;
        let solvers = data.get("solvers")?.as_object()?;
        solvers.get("data")
    }

    /// Find the solver entry in the data array by matching normalized addresses.
    fn find_solver_entry<'a>(
        data_array: &'a serde_json::Value,
        solver_addr: &str,
        solver_addr_normalized: &str,
    ) -> Option<&'a serde_json::Map<String, serde_json::Value>> {
        let data_array = data_array.as_array()?;

        let available_solvers_debug: Vec<(String, String)> = data_array
            .iter()
            .filter_map(|entry| {
                let entry_obj = entry.as_object()?;
                let key = entry_obj.get("key")?.as_str()?;
                let key_without_prefix = key.strip_prefix("0x").unwrap_or(key).to_lowercase();
                let key_normalized = format!("{:0>64}", key_without_prefix);
                Some((key.to_string(), key_normalized))
            })
            .collect();

        tracing::debug!(
            "Looking for solver in registry. Input solver_addr='{}' (type: str), normalized='{}' (type: str, len: {}), Available solvers (original -> normalized): {:?}",
            solver_addr,
            solver_addr_normalized,
            solver_addr_normalized.len(),
            available_solvers_debug
        );

        let solver_entry = data_array.iter().find_map(|entry| {
            let entry_obj = entry.as_object()?;
            let key = entry_obj.get("key")?.as_str()?;
            let key_without_prefix = key.strip_prefix("0x").unwrap_or(key).to_lowercase();
            let key_normalized = format!("{:0>64}", key_without_prefix);

            tracing::debug!(
                "Comparing - Looking for: '{}' (normalized: '{}', len: {}) vs Registry key: '{}' (normalized: '{}', len: {}) -> Match: {}",
                solver_addr,
                solver_addr_normalized,
                solver_addr_normalized.len(),
                key,
                key_normalized,
                key_normalized.len(),
                key_normalized == solver_addr_normalized
            );

            (key_normalized == solver_addr_normalized).then_some(entry_obj)
        });

        if solver_entry.is_none() {
            tracing::error!(
                "Solver not found in registry. Looking for: '{}' (normalized: '{}', len: {}), Available solvers (original -> normalized): {:?}",
                solver_addr,
                solver_addr_normalized,
                solver_addr_normalized.len(),
                available_solvers_debug
            );
        }

        solver_entry
    }

    /// Parse address bytes from Option<vector<u8>> serialization.
    ///
    /// Aptos can serialize Option<vector<u8>> in two different formats:
    /// 1. Array format: {"vec": [bytes_array]} where bytes_array is [u64, u64, ...]
    /// 2. Hex string format: {"vec": ["0xhexstring"]}
    fn parse_address_bytes(
        vec_array: &serde_json::Value,
        solver_addr: &str,
        expected_len: usize,
        chain_label: &str,
    ) -> Result<Option<Vec<u8>>> {
        let vec_array = vec_array
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("vec field is not an array"))?;

        if vec_array.is_empty() {
            tracing::debug!(
                "Solver '{}' found but {} address vec is empty (None)",
                solver_addr,
                chain_label
            );
            return Ok(None);
        }

        tracing::debug!(
            "{} address vec for solver '{}': length={}, vec[0]={}",
            chain_label,
            solver_addr,
            vec_array.len(),
            serde_json::to_string(vec_array.get(0).unwrap_or(&serde_json::Value::Null))
                .unwrap_or_else(|_| "failed to serialize".to_string())
        );

        let bytes_opt = vec_array.get(0);

        let addr_bytes: Vec<u8> = if let Some(bytes_val) = bytes_opt {
            // Try to parse as array of u64 (most common case for Move vector<u8>)
            if let Some(bytes_array) = bytes_val.as_array() {
                let mut result = Vec::new();
                for byte_val in bytes_array {
                    if let Some(byte) = byte_val.as_u64() {
                        if byte > 255 {
                            tracing::error!(
                                "Invalid byte value {} (>255) in {} address for solver '{}'",
                                byte,
                                chain_label,
                                solver_addr
                            );
                            return Ok(None);
                        }
                        result.push(byte as u8);
                    } else {
                        let vec0_json = serde_json::to_string(byte_val)
                            .unwrap_or_else(|_| "failed to serialize".to_string());
                        tracing::error!(
                            "Non-u64 value in {} address bytes array for solver '{}': {}",
                            chain_label,
                            solver_addr,
                            vec0_json
                        );
                        return Ok(None);
                    }
                }
                result
            } else if let Some(hex_str) = bytes_val.as_str() {
                let hex_clean = hex_str.strip_prefix("0x").unwrap_or(hex_str);
                if hex_clean.len() % 2 != 0 {
                    tracing::error!(
                        "Invalid hex string length {} in {} address for solver '{}'",
                        hex_clean.len(),
                        chain_label,
                        solver_addr
                    );
                    return Ok(None);
                }
                (0..hex_clean.len())
                    .step_by(2)
                    .map(|i| {
                        u8::from_str_radix(&hex_clean[i..i + 2], 16).map_err(|e| {
                            anyhow::anyhow!(
                                "Invalid hex byte '{}' at offset {} in {} address for solver '{}': {}",
                                &hex_clean[i..i + 2], i, chain_label, solver_addr, e
                            )
                        })
                    })
                    .collect::<Result<Vec<u8>>>()?
            } else {
                let vec0_json = serde_json::to_string(bytes_val)
                    .unwrap_or_else(|_| "failed to serialize".to_string());
                tracing::error!(
                    "{} address vec[0] is neither an array nor a string for solver '{}'. vec[0] value: {}",
                    chain_label,
                    solver_addr,
                    vec0_json
                );
                return Ok(None);
            }
        } else {
            tracing::error!(
                "{} address vec is non-empty but vec[0] is missing for solver '{}'",
                chain_label,
                solver_addr
            );
            return Ok(None);
        };

        if addr_bytes.is_empty() {
            tracing::error!(
                "Solver '{}' found but {} address bytes array is empty",
                solver_addr,
                chain_label
            );
            return Ok(None);
        }

        if addr_bytes.len() != expected_len {
            tracing::error!(
                "Solver '{}' found but {} address has invalid length {} (expected {} bytes)",
                solver_addr,
                chain_label,
                addr_bytes.len(),
                expected_len
            );
            return Ok(None);
        }

        tracing::debug!(
            "Successfully parsed {} address bytes for solver '{}': length={}",
            chain_label,
            solver_addr,
            addr_bytes.len()
        );

        Ok(Some(addr_bytes))
    }

    // ========================================================================
    // VIEW FUNCTIONS
    // ========================================================================

    /// Calls a view function on the Move VM blockchain.
    pub async fn call_view_function(
        &self,
        module_addr: &str,
        module_name: &str,
        function_name: &str,
        type_args: Vec<String>,
        args: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let url = format!("{}/v1/view", self.base_url);

        let request_body = serde_json::json!({
            "function": format!("{}::{}::{}", module_addr, module_name, function_name),
            "type_arguments": type_args,
            "arguments": args,
        });

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send view function request")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await
                .unwrap_or_else(|_| "<failed to read error body>".to_string());
            return Err(anyhow::anyhow!(
                "View function request failed with status {}: {}",
                status,
                error_body
            ));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse view function response")?;

        Ok(result)
    }

    /// Queries the intent registry for active requester addresses.
    pub async fn get_active_requesters(
        &self,
        solver_registry_addr: &str,
    ) -> Result<Vec<String>> {
        let result = self
            .call_view_function(
                solver_registry_addr,
                "intent_registry",
                "get_active_requesters",
                vec![],
                vec![],
            )
            .await
            .context("Failed to call get_active_requesters view function")?;

        let addresses = result
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .context("Unexpected response format from get_active_requesters view function")?;

        Ok(addresses)
    }

    /// Queries the solver registry for all registered solver addresses.
    pub async fn get_all_solver_addresses(
        &self,
        solver_registry_addr: &str,
    ) -> Result<Vec<String>> {
        let result = self
            .call_view_function(
                solver_registry_addr,
                "solver_registry",
                "list_all_solver_addresses",
                vec![],
                vec![],
            )
            .await
            .context("Failed to call list_all_solver_addresses view function")?;

        let addresses = result
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .context("Unexpected response format from list_all_solver_addresses view function")?;

        Ok(addresses)
    }

    // ========================================================================
    // CONNECTED-CHAIN QUERY METHODS
    // ========================================================================

    /// Queries the fungible asset balance for an account.
    ///
    /// Calls `0x1::primary_fungible_store::balance` view function.
    pub async fn get_token_balance(
        &self,
        account_addr: &str,
        token_metadata: &str,
    ) -> Result<u128> {
        let account_normalized = normalize_hex_to_address(account_addr);
        let metadata_normalized = normalize_hex_to_address(token_metadata);

        let result = self
            .call_view_function(
                "0x1",
                "primary_fungible_store",
                "balance",
                vec!["0x1::fungible_asset::Metadata".to_string()],
                vec![
                    serde_json::json!(account_normalized),
                    serde_json::json!(metadata_normalized),
                ],
            )
            .await
            .context("Failed to query token balance")?;

        let balance_str = result
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.as_str())
            .context("Unexpected response format from balance view function")?;

        balance_str
            .parse::<u128>()
            .context("Failed to parse balance as u128")
    }

    /// Checks if an inflow escrow has been released (auto-released when FulfillmentProof received).
    ///
    /// Calls `{module_addr}::intent_inflow_escrow::is_released` view function.
    pub async fn is_escrow_released(
        &self,
        intent_id: &str,
        module_addr: &str,
    ) -> Result<bool> {
        let intent_id_hex = normalize_hex_to_address(intent_id);

        let result = self
            .call_view_function(
                module_addr,
                "intent_inflow_escrow",
                "is_released",
                vec![],
                vec![serde_json::json!(intent_id_hex)],
            )
            .await
            .context("Failed to query escrow release status")?;

        result
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.as_bool())
            .context("Unexpected response format from is_released view function")
    }

    /// Checks if a solver is registered in the solver registry.
    ///
    /// Calls `{solver_registry_addr}::solver_registry::is_registered` view function.
    pub async fn is_solver_registered(
        &self,
        solver_addr: &str,
        solver_registry_addr: &str,
    ) -> Result<bool> {
        // Normalize address (ensure 0x prefix)
        let solver_addr_normalized = if solver_addr.starts_with("0x") {
            solver_addr.to_string()
        } else {
            format!("0x{}", solver_addr)
        };

        let result = self
            .call_view_function(
                solver_registry_addr,
                "solver_registry",
                "is_registered",
                vec![],
                vec![serde_json::json!(solver_addr_normalized)],
            )
            .await
            .context("Failed to query solver registration")?;

        result
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.as_bool())
            .context("Unexpected response format from is_registered view function")
    }

    /// Checks if outflow requirements have been delivered via GMP.
    ///
    /// Calls `{module_addr}::intent_outflow_validator_impl::has_requirements` view function.
    pub async fn has_outflow_requirements(
        &self,
        intent_id: &str,
        module_addr: &str,
    ) -> Result<bool> {
        let intent_id_hex = normalize_hex_to_address(intent_id);

        let result = self
            .call_view_function(
                module_addr,
                "intent_outflow_validator_impl",
                "has_requirements",
                vec![],
                vec![serde_json::json!(intent_id_hex)],
            )
            .await
            .context("Failed to query outflow requirements")?;

        result
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.as_bool())
            .context("Unexpected response format from has_requirements view function")
    }
}
