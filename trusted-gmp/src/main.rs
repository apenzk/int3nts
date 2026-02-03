//! Native GMP Relay Service
//!
//! A trusted message relay service that watches GMP endpoint events and delivers
//! messages to destination contracts.
//!
//! ## Overview
//!
//! The native GMP relay:
//! 1. Watches for `MessageSent` events on MVM and SVM native GMP endpoints
//! 2. Delivers messages to destination chains by calling `deliver_message`
//! 3. Provides replay protection via nonce tracking
//!
//! ## Security Requirements
//!
//! **CRITICAL**: This service has operator wallet keys and can deliver arbitrary messages.
//! Ensure proper key management and access controls for production use.
//! In production, this can be used directly with your own relay infrastructure,
//! or replaced by LZ's endpoint.

use anyhow::Result;
use tracing::info;

mod config;
mod mvm_client;
mod native_gmp_relay;
mod svm_client;

use config::Config;
use native_gmp_relay::{NativeGmpRelay, NativeGmpRelayConfig};

// ============================================================================
// MAIN APPLICATION ENTRY POINT
// ============================================================================

/// Main application entry point that initializes and runs the native GMP relay.
///
/// This function:
/// 1. Initializes logging and tracing
/// 2. Loads configuration from TOML file
/// 3. Initializes the native GMP relay
/// 4. Runs the relay until shutdown
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging for debugging and monitoring
    tracing_subscriber::fmt::init();

    info!("Starting Native GMP Relay Service");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check for help flag
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("Native GMP Relay Service");
        println!();
        println!("Usage: trusted-gmp [OPTIONS]");
        println!();
        println!("Options:");
        println!("  --testnet, -t     Use testnet configuration (config/trusted-gmp_testnet.toml)");
        println!("  --config <path>   Use custom config file path (overrides --testnet)");
        println!("  --help, -h        Show this help message");
        println!();
        println!("Environment variables:");
        println!("  TRUSTED_GMP_CONFIG_PATH    Path to config file (overrides --config and --testnet)");
        return Ok(());
    }

    // Parse command line arguments
    let mut config_path = None;

    let mut i = 1; // Skip program name
    while i < args.len() {
        if args[i] == "--config" && i + 1 < args.len() {
            config_path = Some(args[i + 1].clone());
            i += 1;
        }
        i += 1;
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

    // Initialize and run the native GMP relay
    let relay_config = NativeGmpRelayConfig::from_config(&config)?;
    let relay = NativeGmpRelay::new(relay_config)?;

    info!("Native GMP relay initialized successfully");

    // Run the relay (this blocks until shutdown)
    relay.run().await
}
