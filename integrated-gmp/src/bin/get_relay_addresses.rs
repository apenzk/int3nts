//! Get Relay Addresses
//!
//! Reads the integrated-gmp configuration and derives the relay address on each
//! supported chain from the existing Ed25519 keypair.
//!
//! ## Usage
//!
//! ```bash
//! INTEGRATED_GMP_CONFIG_PATH=config/integrated-gmp_testnet.toml \
//!   cargo run --bin get_relay_addresses
//! ```

use anyhow::Result;
use integrated_gmp::config::Config;
use integrated_gmp::crypto::CryptoService;

fn main() -> Result<()> {
    let config = Config::load()?;
    let crypto = CryptoService::new(&config)?;

    let mvm_addr = crypto.get_move_address()?;
    let evm_addr = crypto.get_ethereum_address()?;
    let svm_addr = crypto.get_solana_address();

    println!("Relay addresses derived from existing keypair:");
    println!();
    println!("INTEGRATED_GMP_MVM_ADDR={}", mvm_addr);
    println!("INTEGRATED_GMP_EVM_PUBKEY_HASH={}", evm_addr);
    println!("INTEGRATED_GMP_SVM_ADDR={}", svm_addr);
    println!();
    println!("Update these files with the values above:");
    println!("  - testing-infra/testnet/.env.testnet  (all three addresses)");
    println!("  - integrated-gmp/config/integrated-gmp_testnet.toml  (approver_evm_pubkey_hash)");

    Ok(())
}
