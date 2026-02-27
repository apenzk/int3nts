//! Solver Service
//!
//! Main service binary that runs all solver services concurrently:
//! - Signing service: polls coordinator and signs accepted drafts
//! - Intent tracker: monitors hub chain for intent creation
//! - Inflow service: monitors escrows and fulfills inflow intents
//! - Outflow service: executes transfers and fulfills outflow intents
//!
//! ## Usage
//!
//! ```bash
//! cargo run --bin solver -- --config solver.toml
//! ```
//!
//! Or set the config path via environment variable:
//!
//! ```bash
//! SOLVER_CONFIG_PATH=solver.toml cargo run --bin solver
//! ```

use anyhow::{Context, Result};
use clap::Parser;
use solver::{
    chains::HubChainClient,
    config::SolverConfig,
    crypto::{get_private_key_from_profile, sign_intent_hash},
    api::run_acceptance_server,
    service::{InflowService, IntentTracker, LiquidityMonitor, OutflowService, SigningService},
};
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(name = "solver")]
#[command(about = "Solver service for intent framework - signs and fulfills intents")]
struct Args {
    /// Path to solver configuration file (default: solver.toml or SOLVER_CONFIG_PATH env var)
    #[arg(short, long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments first (before initializing logging)
    let args = Args::parse();

    // Initialize structured logging
    tracing_subscriber::fmt::init();

    info!("Starting Solver Service");

    // Load configuration
    // Priority: CLI arg > env var > default
    let config = if let Some(path) = args.config {
        info!("Loading configuration from: {}", path);
        SolverConfig::load_from_path(Some(&path))?
    } else {
        // Check if SOLVER_CONFIG_PATH is set
        if let Ok(path) = std::env::var("SOLVER_CONFIG_PATH") {
            info!("Loading configuration from SOLVER_CONFIG_PATH: {}", path);
        } else {
            info!("Loading configuration from default location");
        }
        SolverConfig::load()?
    };

    info!("Configuration loaded successfully");
    info!("Coordinator URL: {}", config.service.coordinator_url);
    info!("Polling interval: {}ms", config.service.polling_interval_ms);
    info!("Hub chain: {} (chain ID: {})", config.hub_chain.name, config.hub_chain.chain_id);
    info!("Solver address: {}", config.solver.address);

    // Check and update solver registration on-chain
    info!("Checking solver registration on hub chain...");
    let hub_client = HubChainClient::new(&config.hub_chain)?;
    
    // Get solver's private key - try environment variable first, then profile
    let private_key = if let Ok(key_str) = std::env::var("MOVEMENT_SOLVER_PRIVATE_KEY") {
        // Read from environment variable (hex format)
        let key_hex = key_str.strip_prefix("0x").unwrap_or(&key_str);
        let key_bytes = hex::decode(key_hex)
            .context("Failed to decode MOVEMENT_SOLVER_PRIVATE_KEY from hex")?;
        if key_bytes.len() != 32 {
            anyhow::bail!("MOVEMENT_SOLVER_PRIVATE_KEY must be 32 bytes (64 hex chars)");
        }
        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&key_bytes);
        key_array
    } else {
        // Fall back to reading from profile
        get_private_key_from_profile(&config.solver.profile)
            .context("Failed to get private key from profile or MOVEMENT_SOLVER_PRIVATE_KEY env var")?
    };
    
    // Derive public key from private key
    let dummy_hash = [0u8; 32];
    let (_signature, public_key_bytes) = sign_intent_hash(&dummy_hash, &private_key)
        .context("Failed to derive public key from private key")?;
    
    // Get expected addresses for all configured connected chains from environment variables
    let expected_mvm_addr: Option<String> = if config.get_mvm_config().is_some() {
        std::env::var("SOLVER_MVMCON_ADDR").ok()
    } else {
        None
    };
    
    let expected_evm_addr: Vec<u8> = if config.get_evm_config().is_some() {
        let addr_str = std::env::var("SOLVER_EVM_ADDR").context(
            "SOLVER_EVM_ADDR env var is required when an EVM connected chain is configured"
        )?;
        let addr_hex = addr_str.strip_prefix("0x").unwrap_or(&addr_str);
        hex::decode(addr_hex).context(
            "SOLVER_EVM_ADDR contains invalid hex"
        )?
    } else {
        vec![]
    };
    
    let expected_svm_addr: Vec<u8> = if config.get_svm_config().is_some() {
        let addr_str = std::env::var("SOLVER_SVM_ADDR").context(
            "SOLVER_SVM_ADDR env var is required when an SVM connected chain is configured"
        )?;
        let addr_hex = addr_str.strip_prefix("0x").unwrap_or(&addr_str);
        hex::decode(addr_hex).context(
            "SOLVER_SVM_ADDR contains invalid hex"
        )?
    } else {
        vec![]
    };
    
    // Log expected addresses for registration
    info!("Expected registration addresses:");
    if !expected_evm_addr.is_empty() {
        info!("  EVM: 0x{}", hex::encode(&expected_evm_addr));
    } else if config.get_evm_config().is_some() {
        info!("  EVM: (not set - SOLVER_EVM_ADDR env var missing)");
    }
    if !expected_svm_addr.is_empty() {
        info!("  SVM: 0x{}", hex::encode(&expected_svm_addr));
    } else if config.get_svm_config().is_some() {
        info!("  SVM: (not set - SOLVER_SVM_ADDR env var missing)");
    }
    if let Some(ref addr) = expected_mvm_addr {
        info!("  MVM: {}", addr);
    } else if config.get_mvm_config().is_some() {
        info!("  MVM: (not set - SOLVER_MVMCON_ADDR env var missing)");
    }
    
    // Private key for registration/update (testnet mode uses env var, E2E uses profile)
    let pk_for_registration = if std::env::var("MOVEMENT_SOLVER_PRIVATE_KEY").is_ok() {
        Some(&private_key)
    } else {
        None
    };

    match hub_client.is_solver_registered(&config.solver.address).await {
        Ok(true) => {
            // Solver is registered - check if addresses match
            info!("Solver is registered. Checking if addresses need update...");
            
            match hub_client.get_solver_info(&config.solver.address).await {
                Ok(current_info) => {
                    // Log currently registered addresses
                    info!("Currently registered addresses:");
                    if !current_info.evm_addr.is_empty() {
                        info!("  EVM: 0x{}", hex::encode(&current_info.evm_addr));
                    } else {
                        info!("  EVM: (none)");
                    }
                    if !current_info.svm_addr.is_empty() {
                        info!("  SVM: 0x{}", hex::encode(&current_info.svm_addr));
                    } else {
                        info!("  SVM: (none)");
                    }
                    if let Some(ref addr) = current_info.mvm_addr {
                        info!("  MVM: {}", addr);
                    } else {
                        info!("  MVM: (none)");
                    }
                    
                    // Compare registered addresses with expected
                    let evm_matches = current_info.evm_addr == expected_evm_addr;
                    let svm_matches = current_info.svm_addr == expected_svm_addr;
                    let mvm_matches = match (&current_info.mvm_addr, &expected_mvm_addr) {
                        (Some(a), Some(b)) => a == b,
                        (None, None) => true,
                        _ => false,
                    };
                    
                    if evm_matches && svm_matches && mvm_matches {
                        info!("✅ Solver registration is up to date");
                    } else {
                        info!("Solver addresses need update:");
                        if !evm_matches {
                            info!("  EVM: registered={} expected={}", 
                                hex::encode(&current_info.evm_addr), 
                                hex::encode(&expected_evm_addr));
                        }
                        if !svm_matches {
                            info!("  SVM: registered={} expected={}", 
                                hex::encode(&current_info.svm_addr), 
                                hex::encode(&expected_svm_addr));
                        }
                        if !mvm_matches {
                            info!("  MVM: registered={:?} expected={:?}", 
                                current_info.mvm_addr, expected_mvm_addr);
                        }
                        
                        // Update the registration
                        match hub_client.update_solver(
                            &public_key_bytes,
                            expected_mvm_addr.as_deref(),
                            &expected_evm_addr,
                            &expected_svm_addr,
                            pk_for_registration,
                        ) {
                            Ok(tx_hash) => {
                                info!("✅ Solver registration updated. Transaction: {}", tx_hash);
                            }
                            Err(e) => {
                                error!("Failed to update solver registration: {}", e);
                                anyhow::bail!("Failed to update solver registration: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    anyhow::bail!("Failed to verify solver addresses after registration: {}", e);
                }
            }
        }
        Ok(false) => {
            info!("Solver is not registered. Registering on-chain...");
            
            match hub_client.register_solver(
                &public_key_bytes,
                expected_mvm_addr.as_deref(),
                &expected_evm_addr,
                &expected_svm_addr,
                pk_for_registration,
            ) {
                Ok(tx_hash) => {
                    info!("✅ Solver registered successfully. Transaction: {}", tx_hash);
                }
                Err(e) => {
                    // If registration fails (e.g., already registered by another process),
                    // check again to see if we're now registered
                    match hub_client.is_solver_registered(&config.solver.address).await {
                        Ok(true) => {
                            info!("✅ Solver is now registered (may have been registered by another process)");
                        }
                        _ => {
                            anyhow::bail!("Failed to register solver: {}", e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            anyhow::bail!(
                "Failed to check solver registration: {}\n\
                This may indicate:\n\
                - RPC endpoint is unreachable\n\
                - Module address is incorrect\n\
                - View function is not available (module may need to be redeployed with #[view] attribute)\n\
                - Network connectivity issues",
                e
            );
        }
    }

    let config_arc = Arc::new(config.clone());

    // Create shared intent tracker
    let tracker = Arc::new(IntentTracker::new(&config)?);
    info!("Intent tracker initialized");

    // Create liquidity monitor
    let liquidity_monitor = Arc::new(
        LiquidityMonitor::new(config.clone(), config.liquidity.clone())?
    );
    info!("Liquidity monitor initialized");

    // Create services
    let signing_service = SigningService::new(config.clone(), tracker.clone(), Arc::clone(&liquidity_monitor))?;
    info!("Signing service initialized");

    let inflow_service = InflowService::new(config.clone(), tracker.clone(), Arc::clone(&liquidity_monitor))?;
    info!("Inflow service initialized");

    let outflow_service = OutflowService::new(config.clone(), tracker.clone(), Arc::clone(&liquidity_monitor))?;
    info!("Outflow service initialized");

    let polling_interval = Duration::from_millis(config.service.polling_interval_ms);

    // Run all services concurrently with graceful shutdown
    info!("Starting all services...");

    let acceptance_host = config.service.acceptance_api_host.clone();
    let acceptance_port = config.service.acceptance_api_port;
    let acceptance_server = run_acceptance_server(config_arc.clone(), acceptance_host, acceptance_port);

    tokio::select! {
        // Signing service loop
        result = signing_service.run() => {
            if let Err(e) = result {
                error!("Signing service error: {}", e);
            }
        }

        // Intent tracker loop (polls hub chain for created intents)
        _ = async {
            loop {
                if let Err(e) = tracker.poll_for_created_intents().await {
                    error!("Intent tracker error: {}", e);
                }
                tokio::time::sleep(polling_interval).await;
            }
        } => {}

        // Inflow fulfillment service loop
        result = inflow_service.run() => {
            if let Err(e) = result {
                error!("Inflow service error: {}", e);
            }
        }

        // Outflow fulfillment service loop
        _ = outflow_service.run(polling_interval) => {}

        // Acceptance API server for live ratio lookup
        _ = acceptance_server => {}

        // Liquidity monitor loop (polls balances and cleans up expired commitments)
        result = liquidity_monitor.run() => {
            if let Err(e) = result {
                error!("Liquidity monitor error: {}", e);
            }
        }

        // Graceful shutdown on Ctrl+C
        _ = signal::ctrl_c() => {
            info!("Received shutdown signal, stopping services...");
        }
    }

    info!("Solver service stopped");
    Ok(())
}

