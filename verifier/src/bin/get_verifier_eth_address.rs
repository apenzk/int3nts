//! Get Verifier EVM Public Key Hash
//!
//! This binary reads the verifier configuration and outputs the EVM public key hash
//! (keccak256 hash of ECDSA public key, last 20 bytes). This is the Ethereum address
//! derived from the verifier's ECDSA public key and should be used as the verifier
//! address in the IntentEscrow contract deployment.

use anyhow::Result;
use verifier::config::Config;
use verifier::crypto::CryptoService;

fn main() -> Result<()> {
    // Load config
    let config = Config::load()?;

    // Create crypto service
    let crypto = CryptoService::new(&config)?;

    // Get EVM public key hash (Ethereum address derived from ECDSA public key)
    let eth_address = crypto.get_ethereum_address()?;

    println!("{}", eth_address);

    Ok(())
}
