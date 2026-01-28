//! Configuration Management Module
//!
//! This module handles loading and managing configuration for the coordinator service.
//! Configuration includes chain endpoints, timing settings, and API settings.

use serde::{Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use std::str::FromStr;

// ============================================================================
// CONFIGURATION STRUCTURES
// ============================================================================

/// Main configuration structure containing all service settings.
///
/// This structure holds configuration for:
/// - Hub chain connection details
/// - Connected Move VM chain connection details (optional, for Move VM escrow chains)
/// - Connected EVM chain configuration (optional, for EVM escrow chains)
/// - Coordinator timing settings
/// - API server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Hub chain configuration (where intents are created)
    pub hub_chain: ChainConfig,
    /// Connected Move VM chain configuration (optional, where escrow events occur on Move VM)
    #[serde(default)]
    pub connected_chain_mvm: Option<ChainConfig>,
    /// Connected EVM chain configuration (optional, for escrow on EVM)
    #[serde(default)]
    pub connected_chain_evm: Option<EvmChainConfig>,
    /// Connected Solana chain configuration (optional, for escrow on SVM)
    #[serde(default)]
    pub connected_chain_svm: Option<SvmChainConfig>,
    /// Coordinator-specific configuration (timing settings)
    pub coordinator: CoordinatorConfig,
    /// API server configuration (host, port, CORS settings)
    pub api: ApiConfig,
    /// Default solver acceptance criteria (exchange rates for token pairs)
    /// Used to provide exchange rate information to frontend
    #[serde(default)]
    pub acceptance: Option<AcceptanceConfig>,
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
}

/// Coordinator-specific configuration for timing parameters.
///
/// The coordinator is a read-only service that monitors events and handles
/// negotiation routing. It does NOT hold any cryptographic keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    /// Polling interval for event monitoring in milliseconds
    pub polling_interval_ms: u64,
    /// Timeout for validation operations in milliseconds
    pub validation_timeout_ms: u64,
}

/// API server configuration for external communication.
///
/// Controls how the coordinator service exposes its REST API endpoints
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

/// Acceptance criteria configuration for default solver.
///
/// Defines which token pairs are supported. Exchange rates are fetched live
/// from the solver per request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceConfig {
    /// Solver URL for live ratio lookup
    pub solver_url: String,
    /// Supported token pairs (no ratios)
    #[serde(default)]
    pub pairs: Vec<TokenPairConfig>,
}

/// Acceptance token pair configuration (single entry).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPairConfig {
    /// Source chain ID
    pub source_chain_id: u64,
    /// Source token address or mint
    pub source_token: String,
    /// Target chain ID
    pub target_chain_id: u64,
    /// Target token address or mint
    pub target_token: String,
}

// ============================================================================
// CONFIGURATION LOADING AND MANAGEMENT
// ============================================================================

impl Config {
    /// Validates the configuration for duplicate chain IDs.
    ///
    /// This function ensures that:
    /// - Hub chain ID is unique
    /// - Connected MVM chain ID (if present) is unique
    /// - Connected EVM chain ID (if present) is unique
    ///
    /// # Returns
    ///
    /// - `Ok(())` - Configuration is valid
    /// - `Err(anyhow::Error)` - Duplicate chain IDs detected
    pub fn validate(&self) -> anyhow::Result<()> {
        let hub_chain_id = self.hub_chain.chain_id;

        // Check hub vs connected_chain_mvm
        if let Some(ref mvm_config) = self.connected_chain_mvm {
            if hub_chain_id == mvm_config.chain_id {
                return Err(anyhow::anyhow!(
                    "Configuration error: Hub chain and connected MVM chain have the same chain ID {}. Each chain must have a unique chain ID.",
                    hub_chain_id
                ));
            }
        }

        // Check hub vs connected_chain_evm
        if let Some(ref evm_config) = self.connected_chain_evm {
            if hub_chain_id == evm_config.chain_id {
                return Err(anyhow::anyhow!(
                    "Configuration error: Hub chain and connected EVM chain have the same chain ID {}. Each chain must have a unique chain ID.",
                    hub_chain_id
                ));
            }
        }

        // Check hub vs connected_chain_svm
        if let Some(ref svm_config) = self.connected_chain_svm {
            if hub_chain_id == svm_config.chain_id {
                return Err(anyhow::anyhow!(
                    "Configuration error: Hub chain and connected SVM chain have the same chain ID {}. Each chain must have a unique chain ID.",
                    hub_chain_id
                ));
            }
        }

        // Check connected_chain_mvm vs connected_chain_evm
        if let (Some(ref mvm_config), Some(ref evm_config)) = (&self.connected_chain_mvm, &self.connected_chain_evm) {
            if mvm_config.chain_id == evm_config.chain_id {
                return Err(anyhow::anyhow!(
                    "Configuration error: Connected MVM chain and connected EVM chain have the same chain ID {}. Each chain must have a unique chain ID.",
                    mvm_config.chain_id
                ));
            }
        }

        // Check connected_chain_mvm vs connected_chain_svm
        if let (Some(ref mvm_config), Some(ref svm_config)) = (&self.connected_chain_mvm, &self.connected_chain_svm) {
            if mvm_config.chain_id == svm_config.chain_id {
                return Err(anyhow::anyhow!(
                    "Configuration error: Connected MVM chain and connected SVM chain have the same chain ID {}. Each chain must have a unique chain ID.",
                    mvm_config.chain_id
                ));
            }
        }

        // Check connected_chain_evm vs connected_chain_svm
        if let (Some(ref evm_config), Some(ref svm_config)) = (&self.connected_chain_evm, &self.connected_chain_svm) {
            if evm_config.chain_id == svm_config.chain_id {
                return Err(anyhow::anyhow!(
                    "Configuration error: Connected EVM chain and connected SVM chain have the same chain ID {}. Each chain must have a unique chain ID.",
                    evm_config.chain_id
                ));
            }
        }

        if let Some(acceptance) = &self.acceptance {
            for pair in &acceptance.pairs {
                let source_chain_type = self.chain_type_for_id(pair.source_chain_id)
                    .ok_or_else(|| anyhow::anyhow!(
                        "Unknown source_chain_id {} in acceptance pair",
                        pair.source_chain_id
                    ))?;
                let target_chain_type = self.chain_type_for_id(pair.target_chain_id)
                    .ok_or_else(|| anyhow::anyhow!(
                        "Unknown target_chain_id {} in acceptance pair",
                        pair.target_chain_id
                    ))?;

                validate_token_format(pair.source_token.as_str(), source_chain_type)
                    .map_err(|e| anyhow::anyhow!("Invalid source_token for chain {}: {}", source_chain_type, e))?;
                validate_token_format(pair.target_token.as_str(), target_chain_type)
                    .map_err(|e| anyhow::anyhow!("Invalid target_token for chain {}: {}", target_chain_type, e))?;
            }
        }

        Ok(())
    }

    /// Resolves chain type for a chain ID based on configured chains.
    ///
    /// # Arguments
    ///
    /// * `chain_id` - Chain ID to resolve
    ///
    /// # Returns
    ///
    /// - `Some(&'static str)` - Chain type ("mvm", "evm", "svm") if found
    /// - `None` - Chain ID is not configured
    fn chain_type_for_id(&self, chain_id: u64) -> Option<&'static str> {
        if self.hub_chain.chain_id == chain_id {
            return Some("mvm");
        }
        if let Some(ref mvm_config) = self.connected_chain_mvm {
            if mvm_config.chain_id == chain_id {
                return Some("mvm");
            }
        }
        if let Some(ref evm_config) = self.connected_chain_evm {
            if evm_config.chain_id == chain_id {
                return Some("evm");
            }
        }
        if let Some(ref svm_config) = self.connected_chain_svm {
            if svm_config.chain_id == chain_id {
                return Some("svm");
            }
        }
        None
    }

    /// Loads configuration from the TOML file.
    ///
    /// This function:
    /// 1. Checks if config/coordinator.toml exists
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
        let config_path = std::env::var("COORDINATOR_CONFIG_PATH")
            .unwrap_or_else(|_| "config/coordinator.toml".to_string());

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
                cp config/coordinator.template.toml config/coordinator.toml\n\
                Then edit config/coordinator.toml with your actual values.",
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
            connected_chain_mvm: None, // Optional connected Move VM chain configuration
            coordinator: CoordinatorConfig {
                polling_interval_ms: 2000,
                validation_timeout_ms: 30000,
            },
            api: ApiConfig {
                host: "127.0.0.1".to_string(),
                port: 3333,
                cors_origins: vec!["http://localhost:3333".to_string()],
            },
            connected_chain_evm: None, // Optional connected EVM chain configuration
            connected_chain_svm: None, // Optional connected SVM chain configuration
            acceptance: None, // Optional acceptance criteria
        }
    }
}

/// Validates token address format for a chain type.
///
/// - MVM/EVM: `0x`-prefixed hex with expected byte length.
/// - SVM: base58-encoded mint (no `0x` prefix).
///
/// # Arguments
///
/// * `token` - Token address or mint string
/// * `chain_type` - Chain type label ("mvm", "evm", "svm")
///
/// # Returns
///
/// - `Ok(())` - Token format is valid for the chain type
/// - `Err(anyhow::Error)` - Token format is invalid
fn validate_token_format(token: &str, chain_type: &str) -> anyhow::Result<()> {
    match chain_type {
        "svm" => {
            if token.starts_with("0x") {
                anyhow::bail!("SVM tokens must be base58 (got 0x-prefixed value)");
            }
            Pubkey::from_str(token)
                .map_err(|_| anyhow::anyhow!("Invalid base58 SVM mint"))?;
        }
        "evm" => validate_hex_token(token, 20)?,
        "mvm" => validate_hex_token(token, 32)?,
        _ => anyhow::bail!("Unknown chain type {}", chain_type),
    }
    Ok(())
}

/// Validates a `0x`-prefixed hex token with expected byte length.
///
/// # Arguments
///
/// * `token` - `0x`-prefixed hex string
/// * `expected_len` - Expected byte length for the chain type
///
/// # Returns
///
/// - `Ok(())` - Token format matches expected length
/// - `Err(anyhow::Error)` - Token format is invalid
fn validate_hex_token(token: &str, expected_len: usize) -> anyhow::Result<()> {
    let stripped = token.strip_prefix("0x").ok_or_else(|| {
        anyhow::anyhow!("Token must be 0x-prefixed hex string")
    })?;
    let bytes = hex::decode(stripped).map_err(|_| anyhow::anyhow!("Invalid hex token"))?;
    if bytes.len() != expected_len {
        anyhow::bail!("Invalid token length: expected {} bytes", expected_len);
    }
    Ok(())
}
