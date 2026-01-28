//! Trusted GMP Service
//!
//! A trusted message relay service that watches GMP endpoint events and delivers
//! messages to destination contracts. This service provides cross-chain validation
//! and cryptographic approval signatures for escrow operations.
//!
//! ## Overview
//!
//! The trusted GMP service:
//! 1. Monitors intent events on the hub chain for new intents
//! 2. Monitors escrow events from escrow systems
//! 3. Validates fulfillment of intent (deposit conditions) on the connected chain
//! 4. Provides approval/rejection confirmation for intent fulfillment
//! 5. Provides approval/rejection for escrow completion
//!
//! ## Security Requirements
//!
//! **CRITICAL**: This service has operator wallet keys and can forge messages.
//! Only use for testing. Production should use real LayerZero.

use anyhow::Result;
use tracing::info;

mod api;
mod config;
mod crypto;
mod mvm_client;
mod evm_client;
mod svm_client;
mod monitor;
mod validator;

use config::Config;

// ============================================================================
// MAIN APPLICATION ENTRY POINT
// ============================================================================

/// Main application entry point that initializes and runs the trusted-gmp service.
///
/// This function:
/// 1. Initializes logging and tracing
/// 2. Loads configuration from TOML file
/// 3. Initializes all service components (monitor, validator, crypto)
/// 4. Starts the API server
/// 5. Runs the service until shutdown
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging for debugging and monitoring
    tracing_subscriber::fmt::init();

    info!("Starting Trusted GMP Service");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check for help flag
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("Trusted GMP Service");
        println!();
        println!("Usage: trusted-gmp [OPTIONS]");
        println!();
        println!("Options:");
        println!("  --testnet, -t    Use testnet configuration (config/trusted-gmp_testnet.toml)");
        println!("  --config <path>   Use custom config file path (overrides --testnet)");
        println!("  --help, -h        Show this help message");
        println!();
        println!("Environment variables:");
        println!("  TRUSTED_GMP_CONFIG_PATH    Path to config file (overrides --config and --testnet)");
        return Ok(());
    }
    
    // Check for custom config path
    let mut config_path = None;
    for (i, arg) in args.iter().enumerate() {
        if arg == "--config" && i + 1 < args.len() {
            config_path = Some(args[i + 1].clone());
            break;
        }
    }
    
    // Set config path based on flags
    if let Some(path) = config_path {
        std::env::set_var("TRUSTED_GMP_CONFIG_PATH", &path);
        info!("Using custom config: {}", path);
    } else if args.iter().any(|arg| arg == "--testnet" || arg == "-t") {
        std::env::set_var("TRUSTED_GMP_CONFIG_PATH", "config/trusted-gmp_testnet.toml");
        info!("Using testnet configuration");
    }

    // Load configuration from config/trusted-gmp.toml (or TRUSTED_GMP_CONFIG_PATH)
    let config = Config::load()?;
    info!("Configuration loaded successfully");

    // Initialize all service components
    let monitor = monitor::EventMonitor::new(&config).await?;
    let validator = validator::CrossChainValidator::new(&config).await?;
    let crypto_service = crypto::CryptoService::new(&config)?;

    info!("All components initialized successfully");

    // Start the REST API server
    let api_server =
        api::ApiServer::new(config.clone(), monitor.clone(), validator, crypto_service);

    // Start background monitoring
    info!("Starting background event monitoring");
    let monitor_for_background = monitor.clone();
    tokio::spawn(async move {
        if let Err(e) = monitor_for_background.start_monitoring().await {
            eprintln!("Monitoring error: {}", e);
        }
    });

    // Run the service (this blocks until shutdown)
    api_server.run().await?;

    Ok(())
}
