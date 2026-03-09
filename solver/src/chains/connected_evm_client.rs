//! Connected EVM Chain Client
//!
//! Client for interacting with connected EVM chains. Query methods (balance,
//! escrow events, block number) delegate to chain-clients-evm::EvmClient.
//! Solver-specific operations (Hardhat script fulfillment, outflow requirements)
//! remain here.

use anyhow::{Context, Result};
use sha3::{Digest, Keccak256};
use std::process::Command;

use chain_clients_evm::EvmClient;

use crate::config::EvmChainConfig;

// Re-export shared types from chain-clients-evm
pub use chain_clients_evm::{normalize_evm_address, EscrowCreatedEvent};

/// Client for interacting with a connected EVM chain
pub struct ConnectedEvmClient {
    /// Shared EVM JSON-RPC client for query operations
    evm_client: EvmClient,
    /// Base RPC URL (kept for Hardhat script env vars)
    base_url: String,
    /// Escrow contract address (kept for Hardhat script env vars)
    escrow_contract_addr: String,
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
    pub fn new(config: &EvmChainConfig) -> Result<Self> {
        let evm_client = EvmClient::new(&config.rpc_url, &config.escrow_contract_addr)?;

        Ok(Self {
            evm_client,
            base_url: config.rpc_url.clone(),
            escrow_contract_addr: config.escrow_contract_addr.clone(),
            network_name: config.network_name.clone(),
            outflow_validator_addr: config.outflow_validator_addr.clone(),
            gmp_endpoint_addr: config.gmp_endpoint_addr.clone(),
            private_key_env: config.private_key_env.clone(),
        })
    }

    // ========================================================================
    // DELEGATED QUERY METHODS (from chain-clients-evm)
    // ========================================================================

    /// Gets the current block number from the EVM chain
    pub async fn get_block_number(&self) -> Result<u64> {
        self.evm_client.get_block_number().await
    }

    /// Queries the connected chain for EscrowCreated events
    pub async fn get_escrow_events(
        &self,
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> Result<Vec<EscrowCreatedEvent>> {
        self.evm_client
            .get_escrow_created_events(from_block, to_block)
            .await
    }

    /// Queries the ERC20 balance of an account via eth_call balanceOf(address)
    pub async fn get_token_balance(
        &self,
        token_addr: &str,
        account_addr: &str,
    ) -> Result<u128> {
        self.evm_client
            .get_token_balance(token_addr, account_addr)
            .await
    }

    /// Queries the native ETH balance of an account via eth_getBalance
    pub async fn get_native_balance(&self, account_addr: &str) -> Result<u128> {
        self.evm_client.get_native_balance(account_addr).await
    }

    // ========================================================================
    // SOLVER-SPECIFIC METHODS (Hardhat scripts, outflow validation)
    // ========================================================================

    /// Executes an ERC20 transfer with intent_id appended in calldata
    ///
    /// Calls the Hardhat script `transfer-with-intent-id.js` via `npx hardhat run`.
    pub async fn transfer_with_intent_id(
        &self,
        token_addr: &str,
        recipient: &str,
        amount: u64,
        intent_id: &str,
    ) -> Result<String> {
        let project_root = std::env::current_dir().context("Failed to get current directory")?;
        let evm_framework_dir = project_root.join("intent-frameworks/evm");
        if !evm_framework_dir.exists() {
            anyhow::bail!(
                "intent-frameworks/evm directory not found at: {}",
                evm_framework_dir.display()
            );
        }

        let intent_id_evm = if intent_id.starts_with("0x") {
            intent_id.to_string()
        } else {
            format!("0x{}", intent_id)
        };

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
            "Could not extract transaction hash from Hardhat output: {}",
            output_str
        )
    }

    /// Checks if an inflow escrow has been auto-released (via Hardhat script).
    pub async fn is_escrow_released(&self, intent_id: &str) -> Result<bool> {
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
    /// Calls `hasRequirements(bytes32)` on the outflow validator contract via eth_call.
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

        let intent_id_clean = intent_id.strip_prefix("0x").unwrap_or(intent_id);
        let intent_id_padded = format!("{:0>64}", intent_id_clean);
        let calldata = format!("0x{}{}", selector, intent_id_padded);

        let request = chain_clients_evm::JsonRpcRequest {
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

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .no_proxy()
            .build()
            .context("Failed to create HTTP client")?;

        let response: chain_clients_evm::JsonRpcResponse<String> = client
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

        let result = response.result.unwrap_or_else(|| "0x".to_string());

        // ABI bool: 32 bytes, last byte is 0x01 (true) or 0x00 (false)
        let clean = result.strip_prefix("0x").unwrap_or(&result);
        Ok(clean.ends_with('1'))
    }

    /// Fulfills an outflow intent on the EVM chain via IntentOutflowValidator.
    ///
    /// Calls the Hardhat script `fulfill-outflow-intent.js`.
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
