//! Connected Move VM Chain Client
//!
//! Client for interacting with connected Move VM chains to query escrow state
//! and execute transfers via GMP flow.
//!
//! Query methods (get_token_balance, is_escrow_released, has_outflow_requirements)
//! delegate to the shared chain-clients-mvm MvmClient. CLI methods
//! (transfer_with_intent_id, fulfill_outflow_via_gmp) remain solver-specific.

use anyhow::{Context, Result};
use chain_clients_mvm::{normalize_hex_to_address, MvmClient};
use std::process::Command;

use super::tx_hash::extract_tx_hash;

use crate::config::MvmChainConfig;

/// Client for interacting with a connected Move VM chain
pub struct ConnectedMvmClient {
    /// Shared MVM client for query methods (balance, escrow, view functions)
    mvm_client: MvmClient,
    /// Module address (for CLI methods and view function calls)
    module_addr: String,
    /// CLI profile name
    profile: String,
}

impl ConnectedMvmClient {
    /// Creates a new connected MVM chain client
    ///
    /// # Arguments
    ///
    /// * `config` - Connected chain configuration
    ///
    /// # Returns
    ///
    /// * `Ok(ConnectedMvmClient)` - Successfully created client
    /// * `Err(anyhow::Error)` - Failed to create client
    pub fn new(config: &MvmChainConfig) -> Result<Self> {
        // MvmClient::new normalizes the URL (strips trailing /v1 if present)
        let mvm_client = MvmClient::new(&config.rpc_url)
            .context("Failed to create MVM client")?;

        Ok(Self {
            mvm_client,
            module_addr: config.module_addr.clone(),
            profile: config.profile.clone(),
        })
    }

    /// Queries the fungible asset balance for an account on the connected MVM chain.
    ///
    /// Delegates to `MvmClient::get_token_balance`.
    pub async fn get_token_balance(
        &self,
        account_addr: &str,
        token_metadata: &str,
    ) -> Result<u128> {
        self.mvm_client
            .get_token_balance(account_addr, token_metadata)
            .await
    }

    /// Checks if outflow requirements have been delivered via GMP to the connected chain.
    ///
    /// Delegates to `MvmClient::has_outflow_requirements`.
    pub async fn has_outflow_requirements(&self, intent_id: &str) -> Result<bool> {
        self.mvm_client
            .has_outflow_requirements(intent_id, &self.module_addr)
            .await
    }

    /// Checks if an inflow escrow has been released (auto-released when FulfillmentProof received).
    ///
    /// Delegates to `MvmClient::is_escrow_released`.
    pub async fn is_escrow_released(&self, intent_id: &str) -> Result<bool> {
        self.mvm_client
            .is_escrow_released(intent_id, &self.module_addr)
            .await
    }

    /// Executes a transfer with intent ID on the connected chain
    ///
    /// Calls the `transfer_with_intent_id` entry function to transfer tokens
    /// and include the intent_id in the transaction (for outflow fulfillment).
    ///
    /// # Arguments
    ///
    /// * `recipient` - Recipient address
    /// * `metadata` - Token metadata object address
    /// * `amount` - Amount to transfer
    /// * `intent_id` - Intent ID to include in the transaction
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction hash
    /// * `Err(anyhow::Error)` - Failed to execute transfer
    pub fn transfer_with_intent_id(
        &self,
        recipient: &str,
        metadata: &str,
        amount: u64,
        intent_id: &str,
    ) -> Result<String> {
        use tracing::{info, warn};

        // Debug: Get solver's address from profile
        let address_check = Command::new("aptos")
            .args(&["config", "show-profiles"])
            .output();

        if let Ok(address_output) = address_check {
            let address_str = String::from_utf8_lossy(&address_output.stdout);
            info!(
                "Transfer attempt - profile: {}, recipient: {}, amount: {}, metadata: {}",
                self.profile, recipient, amount, metadata
            );
            info!("Aptos profiles: {}", address_str);
        }

        // Debug: Check solver's balance before transfer
        let balance_check = Command::new("aptos")
            .args(&["account", "balance", "--profile", &self.profile])
            .output();

        if let Ok(balance_output) = balance_check {
            let balance_str = String::from_utf8_lossy(&balance_output.stdout);
            info!(
                "Solver balance check (profile: {}): {}",
                self.profile, balance_str
            );
        } else {
            warn!(
                "Failed to check solver balance for profile: {}",
                self.profile
            );
        }

        // Use aptos CLI for compatibility with E2E tests which create aptos profiles
        let output = Command::new("aptos")
            .args(&[
                "move",
                "run",
                "--profile",
                &self.profile,
                "--assume-yes",
                "--function-id",
                &format!("{}::utils::transfer_with_intent_id", self.module_addr),
                "--args",
                &format!("address:{}", recipient),
                &format!("address:{}", metadata),
                &format!("u64:{}", amount),
                &format!("address:{}", intent_id),
            ])
            .output()
            .context("Failed to execute aptos move run")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "movement move run failed:\nstderr: {}\nstdout: {}",
                stderr,
                stdout
            );
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        extract_tx_hash(&output_str, "transfer_with_intent_id")
    }

    /// Fulfills an outflow intent via the GMP flow on the connected chain.
    ///
    /// Calls `outflow_validator::fulfill_intent` which:
    /// 1. Validates the solver is authorized and requirements exist
    /// 2. Transfers tokens from solver to recipient
    /// 3. Sends FulfillmentProof back to hub via GMP
    ///
    /// The hub will automatically release tokens when it receives the FulfillmentProof.
    ///
    /// # Arguments
    ///
    /// * `intent_id` - 32-byte intent identifier (0x-prefixed hex)
    /// * `token_metadata` - Token metadata object address
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Transaction hash
    /// * `Err(anyhow::Error)` - Failed to fulfill intent
    pub fn fulfill_outflow_via_gmp(
        &self,
        intent_id: &str,
        token_metadata: &str,
    ) -> Result<String> {
        use tracing::info;

        info!(
            "Calling outflow_validator::fulfill_intent - intent_id: {}, token_metadata: {}",
            intent_id, token_metadata
        );

        // Use aptos CLI for compatibility with E2E tests which create aptos profiles
        // Function signature: fulfill_intent(solver: &signer, intent_id: vector<u8>, token_metadata: Object<Metadata>)
        let normalized = normalize_hex_to_address(intent_id);
        let intent_id_bare = &normalized[2..]; // strip "0x" for hex: arg
        let output = Command::new("aptos")
            .args(&[
                "move",
                "run",
                "--profile",
                &self.profile,
                "--assume-yes",
                "--function-id",
                &format!(
                    "{}::intent_outflow_validator_impl::fulfill_intent",
                    self.module_addr
                ),
                "--args",
                &format!("hex:{}", intent_id_bare),
                &format!("address:{}", token_metadata),
            ])
            .output()
            .context("Failed to execute aptos move run")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "outflow_validator::fulfill_intent failed:\nstderr: {}\nstdout: {}",
                stderr,
                stdout
            );
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        extract_tx_hash(&output_str, "fulfill_outflow_via_gmp")
    }
}
