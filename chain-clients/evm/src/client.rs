//! EVM Client Implementation
//!
//! Provides a shared client for communicating with EVM-compatible blockchain nodes
//! via their JSON-RPC API. Used by coordinator, integrated-gmp, and solver.

use anyhow::{Context, Result};
use reqwest::Client;
use sha3::{Digest, Keccak256};
use std::time::Duration;

use crate::types::{EscrowCreatedEvent, EvmLog};

/// Client for communicating with EVM-compatible blockchain nodes via JSON-RPC
pub struct EvmClient {
    /// HTTP client for making requests
    client: Client,
    /// Base URL of the EVM node (e.g., "http://127.0.0.1:8545")
    base_url: String,
    /// Escrow contract address
    escrow_contract_addr: String,
}

impl EvmClient {
    /// Creates a new EVM client for the given node URL and escrow contract address.
    pub fn new(node_url: &str, escrow_contract_addr: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .no_proxy() // Avoid macOS system-configuration issues in tests
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            base_url: node_url.to_string(),
            escrow_contract_addr: escrow_contract_addr.to_string(),
        })
    }

    /// Creates a new EVM client without an escrow contract address.
    /// Used by consumers that only need generic RPC access (e.g., GmpEvmClient).
    pub fn new_rpc_only(node_url: &str) -> Result<Self> {
        Self::new(node_url, "")
    }

    /// Returns the base URL of this client
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Returns the escrow contract address
    pub fn escrow_contract_addr(&self) -> &str {
        &self.escrow_contract_addr
    }

    // ========================================================================
    // Generic JSON-RPC
    // ========================================================================

    /// Generic JSON-RPC call with 15-second timeout.
    pub async fn json_rpc<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<T> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1,
        });

        let rpc_future = async {
            let resp = self
                .client
                .post(&self.base_url)
                .json(&request)
                .send()
                .await
                .with_context(|| {
                    format!("Failed to send {} request to {}", method, self.base_url)
                })?;
            resp.json::<serde_json::Value>()
                .await
                .with_context(|| {
                    format!("Failed to parse {} response from {}", method, self.base_url)
                })
        };

        let response: serde_json::Value =
            tokio::time::timeout(Duration::from_secs(15), rpc_future)
                .await
                .map_err(|_| {
                    anyhow::anyhow!(
                        "Timed out after 15s waiting for {} from {}",
                        method,
                        self.base_url
                    )
                })??;

        if let Some(error) = response.get("error") {
            let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
            let message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            anyhow::bail!(
                "JSON-RPC error from {} ({}): {} (code: {})",
                self.base_url,
                method,
                message,
                code
            );
        }

        let result = response
            .get("result")
            .ok_or_else(|| anyhow::anyhow!("No result in {} response from {}", method, self.base_url))?
            .clone();

        serde_json::from_value(result).with_context(|| {
            format!("Failed to deserialize {} result from {}", method, self.base_url)
        })
    }

    // ========================================================================
    // Generic RPC methods
    // ========================================================================

    /// Gets the current block number via eth_blockNumber
    pub async fn get_block_number(&self) -> Result<u64> {
        let block_hex: String = self.json_rpc("eth_blockNumber", vec![]).await?;
        let clean = block_hex.strip_prefix("0x").unwrap_or(&block_hex);
        u64::from_str_radix(clean, 16).context("Failed to parse block number")
    }

    /// Generic eth_call to a contract.
    pub async fn eth_call(&self, to: &str, data: &str) -> Result<String> {
        self.json_rpc(
            "eth_call",
            vec![
                serde_json::json!({ "to": to, "data": data }),
                serde_json::json!("latest"),
            ],
        )
        .await
    }

    /// Query event logs via eth_getLogs.
    pub async fn get_logs(&self, filter: serde_json::Value) -> Result<Vec<EvmLog>> {
        self.json_rpc("eth_getLogs", vec![filter]).await
    }

    /// Get the pending transaction count (nonce) for an address.
    pub async fn get_transaction_count(&self, address: &str) -> Result<u64> {
        let hex: String = self
            .json_rpc(
                "eth_getTransactionCount",
                vec![serde_json::json!(address), serde_json::json!("pending")],
            )
            .await?;
        let clean = hex.strip_prefix("0x").unwrap_or(&hex);
        u64::from_str_radix(clean, 16).context("Failed to parse transaction count")
    }

    /// Get the current gas price.
    pub async fn gas_price(&self) -> Result<u64> {
        let hex: String = self.json_rpc("eth_gasPrice", vec![]).await?;
        let clean = hex.strip_prefix("0x").unwrap_or(&hex);
        u64::from_str_radix(clean, 16).context("Failed to parse gas price")
    }

    /// Broadcast a signed raw transaction, returns the transaction hash.
    pub async fn send_raw_transaction(&self, raw_tx: &str) -> Result<String> {
        self.json_rpc(
            "eth_sendRawTransaction",
            vec![serde_json::json!(raw_tx)],
        )
        .await
    }

    /// Get a transaction receipt, returns None if not yet mined.
    pub async fn get_transaction_receipt(
        &self,
        tx_hash: &str,
    ) -> Result<Option<serde_json::Value>> {
        self.json_rpc(
            "eth_getTransactionReceipt",
            vec![serde_json::json!(tx_hash)],
        )
        .await
    }

    // ========================================================================
    // Escrow-specific methods (require escrow_contract_addr)
    // ========================================================================

    /// Queries EVM chain for EscrowCreated events via eth_getLogs
    pub async fn get_escrow_created_events(
        &self,
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> Result<Vec<EscrowCreatedEvent>> {
        let signature_string =
            "EscrowCreated(bytes32,bytes32,address,uint64,address,bytes32,uint64)";
        let mut hasher = Keccak256::new();
        hasher.update(signature_string.as_bytes());
        let hash = hasher.finalize();
        let event_signature = format!("0x{}", hex::encode(hash));

        let from_block_str = from_block
            .map(|n| format!("0x{:x}", n))
            .unwrap_or_else(|| "latest".to_string());
        let to_block_str = to_block
            .map(|n| format!("0x{:x}", n))
            .unwrap_or_else(|| "latest".to_string());

        let filter = serde_json::json!({
            "address": self.escrow_contract_addr,
            "topics": [event_signature],
            "fromBlock": from_block_str,
            "toBlock": to_block_str,
        });

        let logs: Vec<EvmLog> = self.get_logs(filter).await?;
        let mut events = Vec::new();

        for log in logs {
            if log.topics.len() < 4 {
                continue;
            }

            let intent_id = log.topics[1].clone();
            let requester_addr = format!("0x{}", &log.topics[2][26..]);
            let token_addr = format!("0x{}", &log.topics[3][26..]);

            let data = log.data.strip_prefix("0x").unwrap_or(&log.data);
            if data.len() < 256 {
                continue;
            }

            let escrow_id = format!("0x{}", &data[0..64]);
            let amount = u64::from_str_radix(&data[112..128], 16)
                .context("Failed to parse escrow amount from EVM log data")?;
            let reserved_solver = format!("0x{}", &data[128..192]);
            let expiry = u64::from_str_radix(&data[240..256], 16)
                .context("Failed to parse escrow expiry from EVM log data")?;

            events.push(EscrowCreatedEvent {
                intent_id,
                escrow_id,
                requester_addr,
                amount,
                token_addr,
                reserved_solver,
                expiry,
                block_number: log.block_number,
                transaction_hash: log.transaction_hash,
            });
        }

        Ok(events)
    }

    /// Queries the ERC20 balance of an account via eth_call balanceOf(address)
    pub async fn get_token_balance(
        &self,
        token_addr: &str,
        account_addr: &str,
    ) -> Result<u128> {
        let token_normalized = normalize_evm_address(token_addr)?;
        let account_normalized = normalize_evm_address(account_addr)?;

        // balanceOf(address) selector: 0x70a08231
        let selector = "70a08231";
        let account_clean =
            account_normalized.strip_prefix("0x").unwrap_or(&account_normalized);
        let account_padded = format!("{:0>64}", account_clean);
        let calldata = format!("0x{}{}", selector, account_padded);

        let result: String = self.eth_call(&token_normalized, &calldata).await
            .context("Failed eth_call for balanceOf")?;

        let clean = result.strip_prefix("0x").unwrap_or(&result);
        if clean.is_empty() || clean == "0" {
            return Ok(0);
        }

        let hex_to_parse = if clean.len() > 32 {
            let high_bytes = &clean[..clean.len() - 32];
            if high_bytes.chars().any(|c| c != '0') {
                anyhow::bail!("Token balance exceeds u128 range: 0x{}", clean);
            }
            &clean[clean.len() - 32..]
        } else {
            clean
        };

        let balance =
            u128::from_str_radix(hex_to_parse, 16).context("Failed to parse balance from hex")?;

        Ok(balance)
    }

    /// Queries the native ETH balance of an account via eth_getBalance
    pub async fn get_native_balance(&self, account_addr: &str) -> Result<u128> {
        let account_normalized = normalize_evm_address(account_addr)?;

        let hex: String = self
            .json_rpc(
                "eth_getBalance",
                vec![
                    serde_json::json!(account_normalized),
                    serde_json::json!("latest"),
                ],
            )
            .await?;

        let clean = hex.strip_prefix("0x").unwrap_or(&hex);
        if clean.is_empty() || clean == "0" {
            return Ok(0);
        }

        let balance =
            u128::from_str_radix(clean, 16).context("Failed to parse ETH balance from hex")?;

        Ok(balance)
    }

    /// Checks if an inflow escrow has been released via eth_call isReleased(bytes32)
    pub async fn is_escrow_released(&self, intent_id: &str) -> Result<bool> {
        // Function selector: keccak256("isReleased(bytes32)")[0:4]
        let mut hasher = Keccak256::new();
        hasher.update(b"isReleased(bytes32)");
        let hash = hasher.finalize();
        let selector = hex::encode(&hash[..4]);

        let intent_id_clean = intent_id.strip_prefix("0x").unwrap_or(intent_id);
        let intent_id_padded = format!("{:0>64}", intent_id_clean);
        let calldata = format!("0x{}{}", selector, intent_id_padded);

        let result: String = self.eth_call(&self.escrow_contract_addr.clone(), &calldata).await
            .context("Failed eth_call for isReleased")?;

        // ABI bool: 32 bytes, last byte is 0x01 (true) or 0x00 (false)
        let clean = result.strip_prefix("0x").unwrap_or(&result);
        Ok(clean.ends_with('1'))
    }
}

/// Normalize an EVM address that may be 32-byte padded (for Move compatibility) to 20 bytes.
///
/// Addresses in solver configs may be stored as 32-byte hex (64 chars) for cross-chain
/// compatibility with Move VMs. EVM nodes expect 20-byte addresses (40 hex chars).
///
/// - 40 hex chars (20 bytes): returned as-is with 0x prefix
/// - 64 hex chars (32 bytes): extracts last 40 chars if first 24 are zeros
/// - Other lengths: returned as-is (let the RPC node validate)
pub fn normalize_evm_address(addr: &str) -> Result<String> {
    let clean = addr.strip_prefix("0x").unwrap_or(addr);
    if clean.len() == 64 {
        let high_bytes = &clean[..24];
        if high_bytes.chars().all(|c| c == '0') {
            return Ok(format!("0x{}", &clean[24..]));
        }
        anyhow::bail!(
            "32-byte address has non-zero high bytes, not a valid padded EVM address: {}",
            addr
        );
    }
    Ok(format!("0x{}", clean))
}
