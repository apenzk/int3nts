//! Configuration Management Module
//!
//! This module handles loading and managing configuration for the Integrated GMP service.
//! Configuration includes chain endpoints, cryptographic keys, API settings, and validation parameters.

use serde::{Deserialize, Serialize};

// ============================================================================
// CONFIGURATION STRUCTURES
// ============================================================================

/// Main configuration structure containing all service settings.
///
/// This structure holds configuration for:
/// - Hub chain connection details
/// - Connected Move VM chain connection details (supports multiple simultaneous MVM chains)
/// - Connected EVM chain configurations (supports multiple simultaneous EVM chains)
/// - Integrated GMP cryptographic keys and settings
/// - API server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Hub chain configuration (where intents are created)
    pub hub_chain: ChainConfig,
    /// Connected Move VM chain configurations (supports multiple simultaneous MVM chains)
    #[serde(default)]
    pub connected_chain_mvm: Vec<ChainConfig>,
    /// Connected EVM chain configurations (supports multiple simultaneous EVM chains)
    #[serde(default)]
    pub connected_chain_evm: Vec<EvmChainConfig>,
    /// Connected Solana chain configurations (supports multiple simultaneous SVM chains)
    #[serde(default)]
    pub connected_chain_svm: Vec<SvmChainConfig>,
    /// Integrated GMP configuration (keys, timeouts, etc.)
    pub integrated_gmp: IntegratedGmpConfig,
    /// API server configuration (host, port, CORS settings)
    pub api: ApiConfig,
}

/// Configuration for a blockchain connection.
///
/// Contains all necessary information to connect to and interact with a blockchain,
/// including RPC endpoints, chain identifiers, and module addresses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Human-readable name for the chain
    pub name: String,
    /// RPC endpoint URL for blockchain communication
    pub rpc_url: String,
    /// Unique chain identifier
    pub chain_id: u64,
    /// Address of the intent framework module
    pub intent_module_addr: String,
    /// Address of the escrow module (optional for hub chain)
    pub escrow_module_addr: Option<String>,
}

/// Configuration for an EVM-compatible chain (Ethereum, Hardhat, etc.)
///
/// Used when escrows are hosted on EVM chains instead of Move-based chains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmChainConfig {
    /// Human-readable name for the chain
    pub name: String,
    /// RPC endpoint URL for EVM chain communication
    pub rpc_url: String,
    /// Address of the IntentEscrow contract (single contract, one escrow per intentId)
    pub escrow_contract_addr: String,
    /// Chain ID (e.g., 31337 for Hardhat, 1 for Ethereum mainnet)
    pub chain_id: u64,
    /// Integrated-gmp EVM public key hash (keccak256 hash of ECDSA public key, last 20 bytes).
    /// This is the Ethereum address derived from the integrated-gmp's ECDSA public key (on-chain approver address).
    #[serde(rename = "approver_evm_pubkey_hash")]
    pub approver_evm_pubkey_hash: String,
    /// Address of the IntentGmp contract (GMP endpoint for message delivery/polling)
    #[serde(default)]
    pub gmp_endpoint_addr: Option<String>,
    /// Address of the IntentOutflowValidator contract
    #[serde(default)]
    pub outflow_validator_addr: Option<String>,
}

/// Configuration for a Solana chain (SVM).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvmChainConfig {
    /// Human-readable name for the chain
    pub name: String,
    /// RPC endpoint URL for Solana chain communication
    pub rpc_url: String,
    /// Chain ID (arbitrary unique ID used for routing)
    pub chain_id: u64,
    /// Program ID of the intent escrow program
    pub escrow_program_id: String,
    /// Program ID of the outflow validator program (for routing IntentRequirements)
    pub outflow_program_id: String,
    /// Program ID of the integrated GMP endpoint (for polling outbound messages)
    #[serde(default)]
    pub gmp_endpoint_program_id: Option<String>,
}

/// Integrated GMP configuration including cryptographic keys and timing parameters.
///
/// This configuration is critical for the service's operation and security.
/// The private key must be kept secure and never exposed.
///
/// Keys are loaded from environment variables at runtime for security.
/// The config file contains the environment variable names, not the actual keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegratedGmpConfig {
    /// Environment variable name containing Ed25519 private key (base64 encoded)
    /// Default: "INTEGRATED_GMP_PRIVATE_KEY"
    #[serde(default = "default_private_key_env")]
    pub private_key_env: String,
    /// Environment variable name containing Ed25519 public key (base64 encoded)
    /// Default: "INTEGRATED_GMP_PUBLIC_KEY"
    #[serde(default = "default_public_key_env")]
    pub public_key_env: String,
    /// Polling interval for event monitoring in milliseconds
    pub polling_interval_ms: u64,
    /// Timeout for validation operations in milliseconds
    pub validation_timeout_ms: u64,
}

fn default_private_key_env() -> String {
    "INTEGRATED_GMP_PRIVATE_KEY".to_string()
}

fn default_public_key_env() -> String {
    "INTEGRATED_GMP_PUBLIC_KEY".to_string()
}

impl IntegratedGmpConfig {
    /// Loads the private key from the environment variable.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The private key (base64 encoded)
    /// * `Err(anyhow::Error)` - Failed to load from environment
    pub fn get_private_key(&self) -> anyhow::Result<String> {
        std::env::var(&self.private_key_env)
            .map_err(|_| anyhow::anyhow!(
                "Environment variable '{}' not set. Please set it with your Ed25519 private key (base64 encoded).",
                self.private_key_env
            ))
    }

    /// Loads the public key from the environment variable.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The public key (base64 encoded)
    /// * `Err(anyhow::Error)` - Failed to load from environment
    pub fn get_public_key(&self) -> anyhow::Result<String> {
        std::env::var(&self.public_key_env)
            .map_err(|_| anyhow::anyhow!(
                "Environment variable '{}' not set. Please set it with your Ed25519 public key (base64 encoded).",
                self.public_key_env
            ))
    }
}

/// API server configuration for external communication.
///
/// Controls how the integrated-gmp service exposes its REST API endpoints
/// and handles cross-origin requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Host address to bind the API server to
    pub host: String,
    /// Port number to bind the API server to
    pub port: u16,
    /// Allowed CORS origins for cross-origin requests
    pub cors_origins: Vec<String>,
}

// ============================================================================
// CONFIGURATION LOADING AND MANAGEMENT
// ============================================================================

impl Config {
    /// Validates the configuration for duplicate chain IDs.
    ///
    /// Ensures all configured chains (hub + all connected MVM/EVM/SVM chains) have unique chain IDs.
    ///
    /// # Returns
    ///
    /// - `Ok(())` - Configuration is valid
    /// - `Err(anyhow::Error)` - Duplicate chain IDs detected
    pub fn validate(&self) -> anyhow::Result<()> {
        let hub_chain_id = self.hub_chain.chain_id;

        // Check all MVM chains against hub and each other
        for mvm_config in &self.connected_chain_mvm {
            if hub_chain_id == mvm_config.chain_id {
                return Err(anyhow::anyhow!(
                    "Configuration error: Hub chain and connected MVM chain have the same chain ID {} (chain: '{}'). Each chain must have a unique chain ID.",
                    hub_chain_id, mvm_config.name
                ));
            }
        }
        for i in 0..self.connected_chain_mvm.len() {
            for j in (i + 1)..self.connected_chain_mvm.len() {
                if self.connected_chain_mvm[i].chain_id == self.connected_chain_mvm[j].chain_id {
                    return Err(anyhow::anyhow!(
                        "Configuration error: Connected MVM chains '{}' and '{}' have the same chain ID {}. Each chain must have a unique chain ID.",
                        self.connected_chain_mvm[i].name, self.connected_chain_mvm[j].name, self.connected_chain_mvm[i].chain_id
                    ));
                }
            }
        }

        // Check all SVM chains against hub, MVM, and each other
        for svm_config in &self.connected_chain_svm {
            if hub_chain_id == svm_config.chain_id {
                return Err(anyhow::anyhow!(
                    "Configuration error: Hub chain and connected SVM chain have the same chain ID {} (chain: '{}'). Each chain must have a unique chain ID.",
                    hub_chain_id, svm_config.name
                ));
            }
            for mvm_config in &self.connected_chain_mvm {
                if mvm_config.chain_id == svm_config.chain_id {
                    return Err(anyhow::anyhow!(
                        "Configuration error: Connected MVM chain and connected SVM chain have the same chain ID {}. Each chain must have a unique chain ID.",
                        svm_config.chain_id
                    ));
                }
            }
        }
        for i in 0..self.connected_chain_svm.len() {
            for j in (i + 1)..self.connected_chain_svm.len() {
                if self.connected_chain_svm[i].chain_id == self.connected_chain_svm[j].chain_id {
                    return Err(anyhow::anyhow!(
                        "Configuration error: Connected SVM chains '{}' and '{}' have the same chain ID {}. Each chain must have a unique chain ID.",
                        self.connected_chain_svm[i].name, self.connected_chain_svm[j].name, self.connected_chain_svm[i].chain_id
                    ));
                }
            }
        }

        // Check all EVM chains against hub, MVM, SVM, and each other
        for evm_config in &self.connected_chain_evm {
            if hub_chain_id == evm_config.chain_id {
                return Err(anyhow::anyhow!(
                    "Configuration error: Hub chain and connected EVM chain have the same chain ID {} (chain: '{}'). Each chain must have a unique chain ID.",
                    hub_chain_id, evm_config.name
                ));
            }
            for mvm_config in &self.connected_chain_mvm {
                if mvm_config.chain_id == evm_config.chain_id {
                    return Err(anyhow::anyhow!(
                        "Configuration error: Connected MVM chain and connected EVM chain have the same chain ID {} (chain: '{}'). Each chain must have a unique chain ID.",
                        evm_config.chain_id, evm_config.name
                    ));
                }
            }
            for svm_config in &self.connected_chain_svm {
                if evm_config.chain_id == svm_config.chain_id {
                    return Err(anyhow::anyhow!(
                        "Configuration error: Connected EVM chain '{}' and connected SVM chain have the same chain ID {}. Each chain must have a unique chain ID.",
                        evm_config.name, evm_config.chain_id
                    ));
                }
            }
        }
        for i in 0..self.connected_chain_evm.len() {
            for j in (i + 1)..self.connected_chain_evm.len() {
                if self.connected_chain_evm[i].chain_id == self.connected_chain_evm[j].chain_id {
                    return Err(anyhow::anyhow!(
                        "Configuration error: Connected EVM chains '{}' and '{}' have the same chain ID {}. Each chain must have a unique chain ID.",
                        self.connected_chain_evm[i].name, self.connected_chain_evm[j].name, self.connected_chain_evm[i].chain_id
                    ));
                }
            }
        }

        Ok(())
    }

    /// Loads configuration from the TOML file.
    ///
    /// This function:
    /// 1. Checks if config/integrated-gmp.toml exists
    /// 2. If it exists, loads and parses the configuration
    /// 3. Validates the configuration for duplicate chain IDs
    /// 4. If it doesn't exist, returns an error asking user to copy template
    ///
    /// # Returns
    ///
    /// - `Ok(Config)` - Successfully loaded and validated configuration
    /// - `Err(anyhow::Error)` - Failed to load configuration, file doesn't exist, or validation failed
    pub fn load() -> anyhow::Result<Self> {
        // Check for custom config path via environment variable (for tests)
        let config_path = std::env::var("INTEGRATED_GMP_CONFIG_PATH")
            .unwrap_or_else(|_| "config/integrated-gmp.toml".to_string());

        if std::path::Path::new(&config_path).exists() {
            // Load existing configuration
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            // Validate configuration
            config.validate()?;
            Ok(config)
        } else {
            // Configuration file doesn't exist - user needs to copy template
            Err(anyhow::anyhow!(
                "Configuration file '{}' not found. Please copy the template:\n\
                cp config/integrated-gmp.template.toml config/integrated-gmp.toml\n\
                Then edit config/integrated-gmp.toml with your actual values.",
                config_path
            ))
        }
    }

    /// Creates a default configuration with placeholder values.
    ///
    /// This configuration is suitable for local development and testing.
    /// For production use, all placeholder values must be replaced with
    /// actual chain URLs, module addresses, and cryptographic keys.
    #[allow(dead_code)]
    pub fn default() -> Self {
        Self {
            hub_chain: ChainConfig {
                name: "Hub Chain".to_string(),
                rpc_url: "http://127.0.0.1:8080".to_string(),
                chain_id: 1,
                intent_module_addr: "0x123".to_string(),
                escrow_module_addr: None,
            },
            connected_chain_mvm: vec![], // No connected MVM chains by default
            integrated_gmp: IntegratedGmpConfig {
                private_key_env: "INTEGRATED_GMP_PRIVATE_KEY".to_string(),
                public_key_env: "INTEGRATED_GMP_PUBLIC_KEY".to_string(),
                polling_interval_ms: 2000,
                validation_timeout_ms: 30000,
            },
            api: ApiConfig {
                host: "127.0.0.1".to_string(),
                port: 3333,
                cors_origins: vec!["http://localhost:3333".to_string()],
            },
            connected_chain_evm: vec![], // No connected EVM chains by default
            connected_chain_svm: vec![], // No connected SVM chains by default
        }
    }
}
