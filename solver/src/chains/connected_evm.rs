//! Connected EVM Chain Client
//!
//! Client for interacting with connected EVM chains to query escrow events
//! and execute ERC20 transfers with intent_id metadata.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::process::Command;
use std::time::Duration;

use crate::config::EvmChainConfig;

/// EscrowCreated event data parsed from EVM logs
///
/// Event signature: EscrowCreated(bytes32 indexed intentId, bytes32 escrowId, address indexed requester, uint64 amount, address indexed token, bytes32 reservedSolver, uint64 expiry)
/// topics[0] = event signature hash
/// topics[1] = intentId (bytes32)
/// topics[2] = requester (address, padded to 32 bytes)
/// topics[3] = token (address, padded to 32 bytes)
/// data = abi.encode(escrowId, amount, reservedSolver, expiry) = 256 hex chars
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscrowCreatedEvent {
    /// Intent ID (indexed topic[1], bytes32)
    pub intent_id: String,
    /// Escrow ID (from data, bytes32)
    pub escrow_id: String,
    /// Requester address (indexed topic[2], address)
    pub requester_addr: String,
    /// Amount escrowed (from data, uint64)
    pub amount: u64,
    /// Token contract address (indexed topic[3], address)
    pub token_addr: String,
    /// Reserved solver address (from data, bytes32)
    pub reserved_solver: String,
    /// Expiry timestamp (from data, uint64)
    pub expiry: u64,
    /// Block number
    pub block_number: String,
    /// Transaction hash
    pub transaction_hash: String,
}

/// EVM JSON-RPC request wrapper
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Vec<serde_json::Value>,
    id: u64,
}

/// EVM JSON-RPC response wrapper
#[derive(Debug, Deserialize)]
struct JsonRpcResponse<T> {
    #[allow(dead_code)]
    jsonrpc: String,
    result: Option<T>,
    error: Option<JsonRpcError>,
    #[allow(dead_code)]
    id: u64,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

/// EVM event log entry
#[derive(Debug, Clone, Deserialize)]
struct EvmLog {
    /// Address of the contract that emitted the event
    #[allow(dead_code)]
    pub address: String,
    /// Array of topics (indexed event parameters)
    pub topics: Vec<String>,
    /// Event data (non-indexed parameters)
    pub data: String,
    /// Block number
    #[serde(rename = "blockNumber")]
    pub block_number: String,
    /// Transaction hash
    #[serde(rename = "transactionHash")]
    pub transaction_hash: String,
}

/// Client for interacting with a connected EVM chain
pub struct ConnectedEvmClient {
    /// HTTP client for JSON-RPC calls
    client: Client,
    /// Base RPC URL
    base_url: String,
    /// Escrow contract address
    escrow_contract_addr: String,
    /// Chain ID (for future transaction signing)
    #[allow(dead_code)]
    chain_id: u64,
    /// Hardhat network name (e.g., "localhost", "baseSepolia")
    network_name: String,
    /// IntentOutflowValidator contract address (for GMP outflow)
    outflow_validator_addr: Option<String>,
    /// IntentGmp contract address (for GMP endpoint)
    #[allow(dead_code)]
    gmp_endpoint_addr: Option<String>,
    /// Environment variable name containing the EVM private key for signing transactions
    private_key_env: String,
}

impl ConnectedEvmClient {
    /// Creates a new connected EVM chain client
    ///
    /// # Arguments
    ///
    /// * `config` - EVM chain configuration
    ///
    /// # Returns
    ///
    /// * `Ok(ConnectedEvmClient)` - Successfully created client
    /// * `Err(anyhow::Error)` - Failed to create client
    pub fn new(config: &EvmChainConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .no_proxy() // Avoid macOS system-configuration issues in tests
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            base_url: config.rpc_url.clone(),
            escrow_contract_addr: config.escrow_contract_addr.clone(),
            chain_id: config.chain_id,
            network_name: config.network_name.clone(),
            outflow_validator_addr: config.outflow_validator_addr.clone(),
            gmp_endpoint_addr: config.gmp_endpoint_addr.clone(),
            private_key_env: config.private_key_env.clone(),
        })
    }

    /// Gets the current block number from the EVM chain
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` - Current block number
    /// * `Err(anyhow::Error)` - Failed to get block number
    pub async fn get_block_number(&self) -> Result<u64> {
        use tracing::info;
        
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "eth_blockNumber".to_string(),
            params: vec![],
            id: 1,
        };

        let response: JsonRpcResponse<String> = self
            .client
            .post(&self.base_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send eth_blockNumber request")?
            .json()
            .await
            .context("Failed to parse eth_blockNumber response")?;

        if let Some(error) = response.error {
            anyhow::bail!("Failed to get block number: {} ({})", error.message, error.code);
        }

        let block_hex = response.result.unwrap_or_else(|| "0x0".to_string());
        let block_number = u64::from_str_radix(
            block_hex.strip_prefix("0x").unwrap_or(&block_hex),
            16,
        )
        .context("Failed to parse block number")?;

        info!("Current EVM block number: {}", block_number);
        Ok(block_number)
    }

    /// Queries the connected chain for EscrowCreated events
    ///
    /// Uses eth_getLogs to filter events by contract address and event signature.
    ///
    /// # Arguments
    ///
    /// * `from_block` - Starting block number (optional, "latest" if None)
    /// * `to_block` - Ending block number (optional, "latest" if None)
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<EscrowCreatedEvent>)` - List of escrow events
    /// * `Err(anyhow::Error)` - Failed to query events
    pub async fn get_escrow_events(
        &self,
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> Result<Vec<EscrowCreatedEvent>> {
        use tracing::{info, warn};

        // EscrowCreated(bytes32 indexed intentId, bytes32 escrowId, address indexed requester, uint64 amount, address indexed token, bytes32 reservedSolver, uint64 expiry)
        let event_signature = "EscrowCreated(bytes32,bytes32,address,uint64,address,bytes32,uint64)";
        let mut hasher = Keccak256::new();
        hasher.update(event_signature.as_bytes());
        let event_topic = format!("0x{}", hex::encode(hasher.finalize()));

        info!("Querying EVM escrow events: contract={}, from_block={:?}, to_block={:?}, event_topic={}", 
            self.escrow_contract_addr, from_block, to_block, event_topic);

        // Build filter
        let mut filter = serde_json::json!({
            "address": self.escrow_contract_addr,
            "topics": [event_topic]
        });

        if let Some(from) = from_block {
            filter["fromBlock"] = serde_json::json!(format!("0x{:x}", from));
        } else {
            filter["fromBlock"] = serde_json::json!("latest");
        }

        if let Some(to) = to_block {
            filter["toBlock"] = serde_json::json!(format!("0x{:x}", to));
        } else {
            filter["toBlock"] = serde_json::json!("latest");
        }

        // Call eth_getLogs
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "eth_getLogs".to_string(),
            params: vec![filter.clone()],
            id: 1,
        };

        info!("Sending eth_getLogs request to {}: filter={}", self.base_url, serde_json::to_string(&filter).unwrap_or_default());

        let response: JsonRpcResponse<Vec<EvmLog>> = self
            .client
            .post(&self.base_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send eth_getLogs request")?
            .json()
            .await
            .context("Failed to parse eth_getLogs response")?;

        if let Some(error) = response.error {
            warn!("JSON-RPC error: {} ({})", error.message, error.code);
            anyhow::bail!("JSON-RPC error: {} ({})", error.message, error.code);
        }

        let logs = response.result.unwrap_or_default();
        info!("Received {} log entries from eth_getLogs", logs.len());
        let mut events = Vec::new();

        for log in logs {
            // EscrowCreated(bytes32 indexed intentId, bytes32 escrowId, address indexed requester, uint64 amount, address indexed token, bytes32 reservedSolver, uint64 expiry)
            // topics[0] = event signature hash
            // topics[1] = intentId (bytes32)
            // topics[2] = requester (address, padded to 32 bytes)
            // topics[3] = token (address, padded to 32 bytes)
            // data = abi.encode(escrowId, amount, reservedSolver, expiry) = 256 hex chars
            if log.topics.len() < 4 {
                continue;
            }

            let intent_id = format!("0x{}", log.topics[1].strip_prefix("0x").unwrap_or(&log.topics[1]));
            let requester_addr = format!("0x{}", &log.topics[2][26..]); // Extract last 20 bytes (40 hex chars)
            let token_addr = format!("0x{}", &log.topics[3][26..]);

            // Parse data: escrowId (32 bytes), amount (32 bytes), reservedSolver (32 bytes), expiry (32 bytes)
            let data = log.data.strip_prefix("0x").unwrap_or(&log.data);
            if data.len() < 256 {
                continue; // 4 fields * 64 hex chars = 256
            }

            let escrow_id = format!("0x{}", &data[0..64]);
            let amount = u64::from_str_radix(&data[112..128], 16).unwrap_or(0); // uint64 in last 8 bytes of 32-byte word
            let reserved_solver = format!("0x{}", &data[128..192]);
            let expiry = u64::from_str_radix(&data[240..256], 16).unwrap_or(0); // uint64 in last 8 bytes of 32-byte word

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

    /// Executes an ERC20 transfer with intent_id appended in calldata
    ///
    /// The calldata format is: selector (4 bytes) + recipient (32 bytes) + amount (32 bytes) + intent_id (32 bytes).
    /// The ERC20 contract ignores the extra intent_id bytes, but they remain in the transaction
    /// data for on-chain validation tracking.
    ///
    /// Calls the Hardhat script `transfer-with-intent-id.js` via `npx hardhat run`,
    /// matching the approach used in E2E test scripts. The script uses Hardhat's signer[2]
    /// (Solver account) for signing the transaction.
    ///
    /// # Arguments
    ///
    /// * `token_addr` - ERC20 token contract address
    /// * `recipient` - Recipient address
    /// * `amount` - Transfer amount (in base units)
    /// * `intent_id` - Intent ID to include in calldata (hex format with 0x prefix)
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction hash
    /// * `Err(anyhow::Error)` - Failed to execute transfer
    ///
    /// # TODO
    ///
    /// Future improvement: Implement this directly using a Rust Ethereum library instead of
    /// calling Hardhat scripts. Good options include:
    /// - `ethers-rs` (https://github.com/gakonst/ethers-rs)
    /// - `alloy` (https://github.com/alloy-rs/alloy)
    pub async fn transfer_with_intent_id(
        &self,
        token_addr: &str,
        recipient: &str,
        amount: u64,
        intent_id: &str,
    ) -> Result<String> {
        // Solver runs from project root in CI and local E2E tests
        let project_root = std::env::current_dir().context("Failed to get current directory")?;
        let evm_framework_dir = project_root.join("intent-frameworks/evm");
        if !evm_framework_dir.exists() {
            anyhow::bail!(
                "intent-frameworks/evm directory not found at: {}",
                evm_framework_dir.display()
            );
        }

        // Convert intent_id to EVM format (uint256)
        let intent_id_evm = if intent_id.starts_with("0x") {
            intent_id.to_string()
        } else {
            format!("0x{}", intent_id)
        };

        // Call Hardhat script via nix develop
        // Pass BASE_SEPOLIA_RPC_URL so Hardhat can configure the baseSepolia network
        // Pass SOLVER_EVM_PRIVATE_KEY for signing (signers[2] in the script)
        let solver_private_key = std::env::var(&self.private_key_env).unwrap_or_default();
        let nix_dir = project_root.join("nix");
        let output = Command::new("nix")
            .args(&[
                "develop",
                nix_dir.to_str().unwrap(),
                "-c",
                "bash",
                "-c",
                &format!(
                    "cd '{}' && BASE_SEPOLIA_RPC_URL='{}' SOLVER_EVM_PRIVATE_KEY='{}' TOKEN_ADDR='{}' RECIPIENT='{}' AMOUNT='{}' INTENT_ID='{}' npx hardhat run scripts/transfer-with-intent-id.js --network {}",
                    evm_framework_dir.display(),
                    self.base_url,
                    solver_private_key,
                    token_addr,
                    recipient,
                    amount,
                    intent_id_evm,
                    self.network_name
                ),
            ])
            .output()
            .context("Failed to execute nix develop command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "Hardhat transfer-with-intent-id script failed:\nstderr: {}\nstdout: {}",
                stderr,
                stdout
            );
        }

        // Extract transaction hash from output
        // The script outputs: "Transaction hash: 0x..."
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(hash_line) = output_str.lines().find(|l| l.contains("hash") || l.contains("Hash")) {
            if let Some(hash) = hash_line.split_whitespace().find(|s| s.starts_with("0x")) {
                return Ok(hash.to_string());
            }
        }

        anyhow::bail!("Could not extract transaction hash from Hardhat output: {}", output_str)
    }

    /// Checks if an inflow escrow has been auto-released (via FulfillmentProof GMP message).
    ///
    /// Calls the Hardhat script `get-is-released.js` to check IntentInflowEscrow.isReleased().
    /// With GMP auto-release, when this returns true, tokens have already been transferred to solver.
    ///
    /// # Arguments
    ///
    /// * `intent_id` - Intent ID as hex string (e.g., "0x4b1e...")
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Escrow has been released to solver
    /// * `Ok(false)` - Escrow not yet released
    /// * `Err(anyhow::Error)` - Failed to query
    pub async fn is_escrow_released(&self, intent_id: &str) -> Result<bool> {
        // Convert intent_id to EVM format (uint256)
        let intent_id_evm = if intent_id.starts_with("0x") {
            intent_id.to_string()
        } else {
            format!("0x{}", intent_id)
        };

        // Solver runs from project root in CI and local E2E tests
        let project_root = std::env::current_dir().context("Failed to get current directory")?;
        let evm_framework_dir = project_root.join("intent-frameworks/evm");
        if !evm_framework_dir.exists() {
            anyhow::bail!(
                "intent-frameworks/evm directory not found at: {}",
                evm_framework_dir.display()
            );
        }

        // Call Hardhat script via nix develop to check isReleased()
        let nix_dir = project_root.join("nix");
        let output = Command::new("nix")
            .args(&[
                "develop",
                nix_dir.to_str().unwrap(),
                "-c",
                "bash",
                "-c",
                &format!(
                    "cd '{}' && ESCROW_GMP_ADDR='{}' INTENT_ID_EVM='{}' npx hardhat run scripts/get-is-released.js --network {}",
                    evm_framework_dir.display(),
                    self.escrow_contract_addr,
                    intent_id_evm,
                    self.network_name
                ),
            ])
            .output()
            .context("Failed to execute nix develop command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "Hardhat get-is-released script failed:\nstderr: {}\nstdout: {}",
                stderr,
                stdout
            );
        }

        // Parse output for "isReleased: true" or "isReleased: false"
        let output_str = String::from_utf8_lossy(&output.stdout);
        if output_str.contains("isReleased: true") {
            Ok(true)
        } else if output_str.contains("isReleased: false") {
            Ok(false)
        } else {
            anyhow::bail!("Unexpected output from get-is-released.js: {}", output_str)
        }
    }

    /// Checks if IntentOutflowValidator has requirements for an intent.
    ///
    /// Calls `hasRequirements(bytes32)` on the outflow validator contract via `eth_call`.
    /// Returns true once the GMP relay has delivered IntentRequirements from the hub.
    pub async fn has_outflow_requirements(&self, intent_id: &str) -> Result<bool> {
        let outflow_addr = self
            .outflow_validator_addr
            .as_ref()
            .context("outflow_validator_addr not configured for EVM chain")?;

        // Function selector: keccak256("hasRequirements(bytes32)")[0:4]
        let mut hasher = Keccak256::new();
        hasher.update(b"hasRequirements(bytes32)");
        let hash = hasher.finalize();
        let selector = hex::encode(&hash[..4]);

        // ABI-encode intent_id as bytes32
        let intent_id_clean = intent_id.strip_prefix("0x").unwrap_or(intent_id);
        let intent_id_padded = format!("{:0>64}", intent_id_clean);

        let calldata = format!("0x{}{}", selector, intent_id_padded);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "eth_call".to_string(),
            params: vec![
                serde_json::json!({
                    "to": outflow_addr,
                    "data": calldata,
                }),
                serde_json::json!("latest"),
            ],
            id: 1,
        };

        let response: JsonRpcResponse<String> = self
            .client
            .post(&self.base_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send eth_call for hasRequirements")?
            .json()
            .await
            .context("Failed to parse eth_call response")?;

        if let Some(error) = response.error {
            anyhow::bail!(
                "eth_call hasRequirements failed: {} (code: {})",
                error.message,
                error.code
            );
        }

        let result = response
            .result
            .unwrap_or_else(|| "0x".to_string());

        // ABI bool: 32 bytes, last byte is 0x01 (true) or 0x00 (false)
        let clean = result.strip_prefix("0x").unwrap_or(&result);
        Ok(clean.ends_with('1'))
    }

    /// Fulfills an outflow intent on the EVM chain via IntentOutflowValidator.
    ///
    /// Calls the Hardhat script `fulfill-outflow-intent.js` which:
    /// 1. Reads requirements from the outflow validator
    /// 2. Approves the outflow validator to spend solver's tokens
    /// 3. Calls `fulfillIntent(intentId, tokenAddr)` from solver (signers[2])
    ///
    /// The outflow validator then sends a FulfillmentProof via GMP to the hub.
    pub fn fulfill_outflow_via_gmp(
        &self,
        intent_id: &str,
        token_addr: &str,
    ) -> Result<String> {
        let outflow_addr = self
            .outflow_validator_addr
            .as_ref()
            .context("outflow_validator_addr not configured for EVM chain")?;

        let intent_id_evm = if intent_id.starts_with("0x") {
            intent_id.to_string()
        } else {
            format!("0x{}", intent_id)
        };

        let project_root = std::env::current_dir().context("Failed to get current directory")?;
        let evm_framework_dir = project_root.join("intent-frameworks/evm");
        if !evm_framework_dir.exists() {
            anyhow::bail!(
                "intent-frameworks/evm directory not found at: {}",
                evm_framework_dir.display()
            );
        }

        // Pass BASE_SEPOLIA_RPC_URL so Hardhat can configure the baseSepolia network
        // Pass SOLVER_EVM_PRIVATE_KEY for signing on testnet
        let solver_private_key = std::env::var(&self.private_key_env).unwrap_or_default();
        let nix_dir = project_root.join("nix");
        let output = Command::new("nix")
            .args(&[
                "develop",
                nix_dir.to_str().unwrap(),
                "-c",
                "bash",
                "-c",
                &format!(
                    "cd '{}' && BASE_SEPOLIA_RPC_URL='{}' SOLVER_EVM_PRIVATE_KEY='{}' OUTFLOW_VALIDATOR_ADDR='{}' TOKEN_ADDR='{}' INTENT_ID='{}' npx hardhat run scripts/fulfill-outflow-intent.js --network {}",
                    evm_framework_dir.display(),
                    self.base_url,
                    solver_private_key,
                    outflow_addr,
                    token_addr,
                    intent_id_evm,
                    self.network_name
                ),
            ])
            .output()
            .context("Failed to execute nix develop command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "Hardhat fulfill-outflow-intent script failed:\nstderr: {}\nstdout: {}",
                stderr,
                stdout
            );
        }

        // Extract transaction hash from output
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(hash_line) = output_str
            .lines()
            .find(|l| l.contains("hash") || l.contains("Hash"))
        {
            if let Some(hash) = hash_line.split_whitespace().find(|s| s.starts_with("0x")) {
                return Ok(hash.to_string());
            }
        }

        anyhow::bail!(
            "Could not extract transaction hash from fulfill-outflow-intent output: {}",
            output_str
        )
    }
}

