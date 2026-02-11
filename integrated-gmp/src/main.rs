//! Integrated GMP Service - Integrated GMP Relay
//!
//! A trusted message relay service that watches for `MessageSent` events on source chains
//! and delivers cross-chain messages by calling `deliver_message` on destination chains.
//!
//! Integrated-gmp is a pure relay — invisible to clients. The coordinator is the single
//! API surface for frontends and solvers.
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
mod crypto;
mod evm_client;
mod monitor;
mod mvm_client;
mod integrated_gmp_relay;
mod svm_client;
mod validator;

use config::Config;
use crypto::CryptoService;
use integrated_gmp_relay::{NativeGmpRelay, NativeGmpRelayConfig};

// ============================================================================
// MAIN APPLICATION ENTRY POINT
// ============================================================================

/// Main application entry point that initializes and runs the integrated GMP relay.
///
/// This function:
/// 1. Initializes logging and tracing
/// 2. Loads configuration from TOML file
/// 3. Initializes the integrated GMP relay
/// 4. Runs the relay until shutdown
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging for debugging and monitoring
    tracing_subscriber::fmt::init();

    info!("Starting Integrated GMP Relay Service");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check for help flag
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("Integrated GMP Relay Service");
        println!();
        println!("Usage: integrated-gmp [OPTIONS]");
        println!();
        println!("Options:");
        println!("  --testnet, -t     Use testnet configuration (config/integrated-gmp_testnet.toml)");
        println!("  --config <path>   Use custom config file path (overrides --testnet)");
        println!("  --help, -h        Show this help message");
        println!();
        println!("Environment variables:");
        println!(
            "  INTEGRATED_GMP_CONFIG_PATH    Path to config file (overrides --config and --testnet)"
        );
        return Ok(());
    }

    // Parse config arguments
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
        std::env::set_var("INTEGRATED_GMP_CONFIG_PATH", &path);
        info!("Using custom config: {}", path);
    } else if args.iter().any(|arg| arg == "--testnet" || arg == "-t") {
        std::env::set_var("INTEGRATED_GMP_CONFIG_PATH", "config/integrated-gmp_testnet.toml");
        info!("Using testnet configuration");
    }

    // Load configuration from config/integrated-gmp.toml (or INTEGRATED_GMP_CONFIG_PATH)
    let config = Config::load()?;
    info!("Configuration loaded successfully");

    // Initialize and run the integrated GMP relay
    let relay_config = NativeGmpRelayConfig::from_config(&config)?;
    let crypto_service = CryptoService::new(&config)?;
    let relay = NativeGmpRelay::new(relay_config, crypto_service)?;

    info!("Integrated GMP relay initialized successfully");

    // Run the relay (this blocks until shutdown)
    relay.run().await
}
