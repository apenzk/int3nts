//! Coordinator Service
//!
//! A coordinator service that monitors blockchain events and provides negotiation
//! routing for cross-chain intents. This service is read-only - it does not hold
//! private keys or perform cryptographic signing.
//!
//! ## Overview
//!
//! The coordinator is an external service that:
//! 1. Monitors intent events on the hub chain for new intents
//! 2. Monitors escrow events from connected chains
//! 3. Caches events for API access
//! 4. Provides negotiation routing for draft intents (FCFS solver matching)
//!
//! ## Security Model
//!
//! The coordinator has NO private keys and CANNOT steal funds. All validation
//! and approval signing is handled by the separate Trusted GMP service.

use anyhow::Result;
use tracing::info;

mod api;
mod config;
mod monitor;
mod mvm_client;
mod storage;
mod svm_client;

use config::Config;

// ============================================================================
// MAIN APPLICATION ENTRY POINT
// ============================================================================

/// Main application entry point that initializes and runs the coordinator service.
///
/// This function:
/// 1. Initializes logging and tracing
/// 2. Loads configuration from TOML file
/// 3. Initializes the event monitor
/// 4. Starts the API server
/// 5. Runs the service until shutdown
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging for debugging and monitoring
    tracing_subscriber::fmt::init();

    info!("Starting Coordinator Service");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check for help flag
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("Coordinator Service");
        println!();
        println!("Usage: coordinator [OPTIONS]");
        println!();
        println!("Options:");
        println!("  --testnet, -t    Use testnet configuration (config/coordinator_testnet.toml)");
        println!("  --config <path>   Use custom config file path (overrides --testnet)");
        println!("  --help, -h        Show this help message");
        println!();
        println!("Environment variables:");
        println!("  COORDINATOR_CONFIG_PATH    Path to config file (overrides --config and --testnet)");
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
        std::env::set_var("COORDINATOR_CONFIG_PATH", &path);
        info!("Using custom config: {}", path);
    } else if args.iter().any(|arg| arg == "--testnet" || arg == "-t") {
        std::env::set_var("COORDINATOR_CONFIG_PATH", "config/coordinator_testnet.toml");
        info!("Using testnet configuration");
    }

    // Load configuration from config file (or COORDINATOR_CONFIG_PATH env var)
    let config = Config::load()?;
    info!("Configuration loaded successfully");

    // Initialize the event monitor
    let monitor = monitor::EventMonitor::new(&config).await?;

    info!("Event monitor initialized successfully");

    // Start the REST API server (coordinator API - read-only + negotiation)
    let api_server = api::ApiServer::new(config.clone(), monitor.clone());

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
