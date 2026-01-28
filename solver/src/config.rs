//! Configuration Management Module
//!
//! This module handles loading and managing configuration for the solver service.
//! Configuration includes coordinator connection, chain settings, and acceptance criteria.

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;

use crate::acceptance::TokenPair;

// ============================================================================
// CONFIGURATION STRUCTURES
// ============================================================================

/// Main configuration structure containing all solver service settings.
///
/// This structure holds configuration for:
/// - Coordinator service connection
/// - Hub chain connection details
/// - Connected chain configurations (one or more, each with a type field)
/// - Acceptance criteria (token pairs and exchange rates)
/// - Solver profile and signing settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverConfig {
    /// Service configuration (coordinator URL, polling intervals)
    pub service: ServiceConfig,
    /// Hub chain configuration (where intents are created)
    pub hub_chain: ChainConfig,
    /// Connected chain configurations (use [[connected_chain]] in TOML for multiple)
    #[serde(default)]
    pub connected_chain: Vec<ConnectedChainConfig>,
    /// Acceptance criteria (token pairs and exchange rates)
    pub acceptance: AcceptanceConfig,
    /// Solver signing configuration
    pub solver: SolverSigningConfig,
}

impl SolverConfig {
    /// Get connected MVM chain config if configured
    pub fn get_mvm_config(&self) -> Option<&MvmChainConfig> {
        self.connected_chain.iter().find_map(|c| {
            if let ConnectedChainConfig::Mvm(cfg) = c {
                Some(cfg)
            } else {
                None
            }
        })
    }

    /// Get connected EVM chain config if configured
    pub fn get_evm_config(&self) -> Option<&EvmChainConfig> {
        self.connected_chain.iter().find_map(|c| {
            if let ConnectedChainConfig::Evm(cfg) = c {
                Some(cfg)
            } else {
                None
            }
        })
    }

    /// Get connected SVM chain config if configured
    pub fn get_svm_config(&self) -> Option<&SvmChainConfig> {
        self.connected_chain.iter().find_map(|c| {
            if let ConnectedChainConfig::Svm(cfg) = c {
                Some(cfg)
            } else {
                None
            }
        })
    }

    /// Get connected chain config by chain ID
    pub fn get_connected_chain_by_id(&self, chain_id: u64) -> Option<&ConnectedChainConfig> {
        self.connected_chain.iter().find(|c| c.chain_id() == chain_id)
    }
}

/// Configuration for a connected chain (can be MVM, EVM, or SVM).
/// Use the `type` field to specify which type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ConnectedChainConfig {
    /// Move VM chain configuration
    #[serde(rename = "mvm")]
    Mvm(MvmChainConfig),
    /// EVM chain configuration
    #[serde(rename = "evm")]
    Evm(EvmChainConfig),
    /// SVM chain configuration
    #[serde(rename = "svm")]
    Svm(SvmChainConfig),
}

impl ConnectedChainConfig {
    /// Get the chain ID for this connected chain
    pub fn chain_id(&self) -> u64 {
        match self {
            ConnectedChainConfig::Mvm(cfg) => cfg.chain_id,
            ConnectedChainConfig::Evm(cfg) => cfg.chain_id,
            ConnectedChainConfig::Svm(cfg) => cfg.chain_id,
        }
    }

    /// Get the chain type as a string
    pub fn chain_type(&self) -> &'static str {
        match self {
            ConnectedChainConfig::Mvm(_) => "mvm",
            ConnectedChainConfig::Evm(_) => "evm",
            ConnectedChainConfig::Svm(_) => "svm",
        }
    }
}

/// Service-level configuration for the solver.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Coordinator API base URL (e.g., "http://127.0.0.1:3333") - used for draft negotiation
    pub coordinator_url: String,
    /// Trusted GMP API base URL (e.g., "http://127.0.0.1:3334") - used for approvals and validation
    pub trusted_gmp_url: String,
    /// Polling interval for checking pending drafts in milliseconds
    pub polling_interval_ms: u64,
    /// E2E testing mode: if true, use aptos CLI with profiles; if false, use movement CLI with private keys
    #[serde(default)]
    pub e2e_mode: bool,
    /// Solver acceptance API host (used by coordinator to fetch ratios)
    #[serde(default = "default_acceptance_api_host")]
    pub acceptance_api_host: String,
    /// Solver acceptance API port
    #[serde(default = "default_acceptance_api_port")]
    pub acceptance_api_port: u16,
}

/// Configuration for a blockchain connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Human-readable name for the chain
    pub name: String,
    /// RPC endpoint URL for blockchain communication
    pub rpc_url: String,
    /// Unique chain identifier
    pub chain_id: u64,
    /// Address of the intent framework module
    pub module_addr: String,
    /// Aptos/Movement CLI profile name for this chain
    pub profile: String,
    /// E2E testing mode: if true, use aptos CLI with profiles; if false, use movement CLI with private keys
    #[serde(default)]
    pub e2e_mode: bool,
}

/// Configuration for a connected Move VM chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MvmChainConfig {
    /// Human-readable name for the chain
    pub name: String,
    /// RPC endpoint URL for blockchain communication
    pub rpc_url: String,
    /// Unique chain identifier
    pub chain_id: u64,
    /// Address of the intent framework module
    pub module_addr: String,
    /// Aptos/Movement CLI profile name for this chain
    pub profile: String,
    /// E2E testing mode: if true, use aptos CLI with profiles; if false, use movement CLI with private keys
    #[serde(default)]
    pub e2e_mode: bool,
}

/// Configuration for an EVM-compatible chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmChainConfig {
    /// Human-readable name for the chain
    pub name: String,
    /// RPC endpoint URL for EVM chain communication
    pub rpc_url: String,
    /// Chain ID (e.g., 84532 for Base Sepolia)
    pub chain_id: u64,
    /// Address of the IntentEscrow contract
    pub escrow_contract_addr: String,
    /// Environment variable name containing the EVM private key
    pub private_key_env: String,
    /// Hardhat network name (e.g., "localhost", "baseSepolia")
    #[serde(default = "default_network_name")]
    pub network_name: String,
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
    /// Environment variable name containing the solver private key (base58)
    pub private_key_env: String,
}

fn default_network_name() -> String {
    "localhost".to_string()
}

fn default_acceptance_api_host() -> String {
    "127.0.0.1".to_string()
}

fn default_acceptance_api_port() -> u16 {
    4444
}

/// Acceptance criteria configuration.
///
/// Defines which token pairs are supported and their exchange rates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceConfig {
    /// Supported token pairs with exchange rates.
    #[serde(rename = "tokenpair", default)]
    pub token_pairs: Vec<TokenPairConfig>,
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
    /// Exchange rate (how many source tokens per 1 target token)
    pub ratio: f64,
}

/// Solver signing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverSigningConfig {
    /// Aptos/Movement CLI profile name for the solver account
    pub profile: String,
    /// Solver address (0x-prefixed hex)
    pub address: String,
}

impl SolverConfig {
    /// Loads configuration from a TOML file.
    ///
    /// This function:
    /// 1. Checks if config/solver.toml exists (or uses SOLVER_CONFIG_PATH env var or provided path)
    /// 2. If it exists, loads and parses the configuration
    /// 3. Validates the configuration
    /// 4. Converts token pair configs to TokenPair structs
    /// 5. If it doesn't exist, returns an error asking user to copy template
    ///
    /// # Arguments
    ///
    /// * `path` - Optional path to config file. If None, uses SOLVER_CONFIG_PATH env var or default.
    ///
    /// # Returns
    ///
    /// * `Ok(SolverConfig)` - Successfully loaded and validated configuration
    /// * `Err(anyhow::Error)` - Failed to load configuration, file doesn't exist, or validation failed
    pub fn load_from_path(path: Option<&str>) -> anyhow::Result<Self> {
        // Use provided path, or check for custom config path via environment variable, or use default
        let config_path = path
            .map(|p| p.to_string())
            .or_else(|| std::env::var("SOLVER_CONFIG_PATH").ok())
            .unwrap_or_else(|| "config/solver.toml".to_string());

        if std::path::Path::new(&config_path).exists() {
            // Load existing configuration
            let content = std::fs::read_to_string(&config_path)?;
            let config: SolverConfig = toml::from_str(&content)?;
            // Validate configuration
            config.validate()?;
            Ok(config)
        } else {
            // Configuration file doesn't exist - user needs to copy template
            Err(anyhow::anyhow!(
                "Configuration file '{}' not found. Please copy the template:\n\
                cp config/solver.template.toml config/solver.toml\n\
                Then edit config/solver.toml with your actual values.",
                config_path
            ))
        }
    }

    /// Loads configuration from a TOML file (convenience method that uses default path).
    ///
    /// This is equivalent to calling `load_from_path(None)`.
    pub fn load() -> anyhow::Result<Self> {
        Self::load_from_path(None)
    }

    /// Resolve chain type for a chain ID based on hub/connected configs.
    fn chain_type_for_id(&self, chain_id: u64) -> Option<&'static str> {
        if self.hub_chain.chain_id == chain_id {
            return Some("mvm");
        }
        for chain in &self.connected_chain {
            if chain.chain_id() == chain_id {
                return Some(chain.chain_type());
            }
        }
        None
    }

    /// Validates the configuration for consistency and correctness.
    ///
    /// Checks:
    /// - At least one connected chain is configured
    /// - Hub and connected chains have different chain IDs
    /// - All connected chains have unique chain IDs
    /// - Token pairs reference known chains and valid token formats
    /// - Exchange rates are positive
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Configuration is valid
    /// * `Err(anyhow::Error)` - Validation failed with error message
    pub fn validate(&self) -> anyhow::Result<()> {
        // Check at least one connected chain is configured
        if self.connected_chain.is_empty() {
            return Err(anyhow::anyhow!(
                "Configuration error: At least one [[connected_chain]] must be configured"
            ));
        }

        // Collect all chain IDs and check for duplicates with hub
        let hub_chain_id = self.hub_chain.chain_id;

        for chain in &self.connected_chain {
            if chain.chain_id() == hub_chain_id {
                return Err(anyhow::anyhow!(
                    "Configuration error: Connected {} chain has same chain ID ({}) as hub chain",
                    chain.chain_type(),
                    hub_chain_id
                ));
            }
        }

        // Check for duplicate chain IDs among connected chains
        for i in 0..self.connected_chain.len() {
            for j in (i + 1)..self.connected_chain.len() {
                if self.connected_chain[i].chain_id() == self.connected_chain[j].chain_id() {
                    return Err(anyhow::anyhow!(
                        "Configuration error: Connected chains {} and {} have the same chain ID {}",
                        self.connected_chain[i].chain_type(),
                        self.connected_chain[j].chain_type(),
                        self.connected_chain[i].chain_id()
                    ));
                }
            }
        }

        // Validate token pairs and exchange rates
        for pair in &self.acceptance.token_pairs {
            // Validate chain IDs exist
            let source_chain_type = self.chain_type_for_id(pair.source_chain_id)
                .ok_or_else(|| anyhow::anyhow!(
                    "Unknown source_chain_id {} in token pair",
                    pair.source_chain_id
                ))?;
            let target_chain_type = self.chain_type_for_id(pair.target_chain_id)
                .ok_or_else(|| anyhow::anyhow!(
                    "Unknown target_chain_id {} in token pair",
                    pair.target_chain_id
                ))?;

            // Validate token formats by chain type
            validate_token_format(pair.source_token.as_str(), source_chain_type)
                .map_err(|e| anyhow::anyhow!("Invalid source_token for chain {}: {}", source_chain_type, e))?;
            validate_token_format(pair.target_token.as_str(), target_chain_type)
                .map_err(|e| anyhow::anyhow!("Invalid target_token for chain {}: {}", target_chain_type, e))?;

            // Validate exchange rate is positive
            if pair.ratio <= 0.0 {
                return Err(anyhow::anyhow!(
                    "Invalid exchange rate {} for token pair {}:{} -> {}:{}: must be positive",
                    pair.ratio,
                    pair.source_chain_id,
                    pair.source_token,
                    pair.target_chain_id,
                    pair.target_token
                ));
            }
        }

        Ok(())
    }

    /// Converts token pair configs to TokenPair structs.
    ///
    /// This is a helper method for the acceptance module to use.
    ///
    /// # Returns
    ///
    /// * `HashMap<TokenPair, f64>` - Token pairs with exchange rates
    pub fn get_token_pairs(&self) -> anyhow::Result<HashMap<TokenPair, f64>> {
        let mut pairs = HashMap::new();

        for pair in &self.acceptance.token_pairs {
            let token_pair = TokenPair {
                offered_chain_id: pair.source_chain_id,
                offered_token: pair.source_token.clone(),
                desired_chain_id: pair.target_chain_id,
                desired_token: pair.target_token.clone(),
            };

            pairs.insert(token_pair, pair.ratio);
        }

        Ok(pairs)
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
            // SVM tokens can be base58 (native) or 32-byte hex (when stored on Move hub chain)
            if token.starts_with("0x") {
                // Hex format - must be 32 bytes (same as Move)
                validate_hex_token(token, 32)?;
            } else {
                // Base58 format - validate as Solana pubkey
                Pubkey::from_str(token)
                    .map_err(|_| anyhow::anyhow!("Invalid base58 SVM mint"))?;
            }
        }
        "evm" => {
            // EVM tokens can be 20 bytes (native) or 32 bytes (padded for Move compatibility)
            let stripped = token.strip_prefix("0x").ok_or_else(|| {
                anyhow::anyhow!("EVM token must be 0x-prefixed hex string")
            })?;
            let bytes = hex::decode(stripped).map_err(|_| anyhow::anyhow!("Invalid hex EVM token"))?;
            if bytes.len() != 20 && bytes.len() != 32 {
                anyhow::bail!("Invalid EVM token length: expected 20 or 32 bytes, got {}", bytes.len());
            }
        }
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

