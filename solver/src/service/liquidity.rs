//! Liquidity Monitoring Service
//!
//! Periodically polls solver wallet balances across chains, tracks in-flight
//! commitments, and prevents accepting intents when budget is insufficient.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use solana_sdk::pubkey::Pubkey;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::chains::{ConnectedEvmClient, ConnectedMvmClient, ConnectedSvmClient, HubChainClient};
use crate::config::{gas_token_for_chain_type, ConnectedChainConfig, LiquidityMonitorConfig, SolverConfig};

/// Identifies a specific token on a specific chain.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ChainToken {
    pub chain_id: u64,
    pub token: String,
}

/// A reserved in-flight budget commitment for an accepted draft.
#[derive(Debug, Clone)]
pub struct InFlightCommitment {
    pub draft_id: String,
    pub amount: u64,
    pub committed_at: Instant,
}

/// Liquidity state for a single token on a single chain.
#[derive(Debug, Clone)]
pub struct TokenLiquidity {
    pub confirmed_balance: u128,
    pub last_updated: Instant,
    pub in_flight: Vec<InFlightCommitment>,
}

impl TokenLiquidity {
    /// Returns the available budget: confirmed balance minus in-flight commitments.
    pub fn available_budget(&self) -> u128 {
        let in_flight_total: u64 = self.in_flight.iter().map(|c| c.amount).sum();
        self.confirmed_balance.saturating_sub(in_flight_total as u128)
    }
}

/// Monitors solver liquidity across chains and manages budget reservations.
///
/// Runs as a concurrent service alongside signing, tracker, inflow, and outflow.
/// Shared via `Arc<LiquidityMonitor>` so that the signing service can check
/// budgets and the fulfillment services can release commitments.
pub struct LiquidityMonitor {
    state: Arc<RwLock<HashMap<ChainToken, TokenLiquidity>>>,
    config: LiquidityMonitorConfig,
    solver_config: SolverConfig,
    hub_client: HubChainClient,
    mvm_client: Option<ConnectedMvmClient>,
    evm_client: Option<ConnectedEvmClient>,
    svm_client: Option<ConnectedSvmClient>,
    /// Solver wallet address on each chain, keyed by chain_id.
    solver_addresses: HashMap<u64, String>,
}

impl LiquidityMonitor {
    /// Creates a new liquidity monitor.
    ///
    /// Initializes chain clients, solver addresses, and tracking state from
    /// the configured thresholds and acceptance token pairs.
    pub fn new(
        solver_config: SolverConfig,
        liquidity_config: LiquidityMonitorConfig,
    ) -> Result<Self> {
        // Build solver address map from config and environment
        let mut solver_addresses = HashMap::new();

        // Hub chain solver address (always available from config)
        solver_addresses.insert(
            solver_config.hub_chain.chain_id,
            solver_config.solver.address.clone(),
        );

        // Connected chain solver addresses from environment variables
        for chain in &solver_config.connected_chain {
            match chain {
                ConnectedChainConfig::Evm(cfg) => {
                    if let Ok(addr) = std::env::var("SOLVER_EVM_ADDR") {
                        solver_addresses.insert(cfg.chain_id, addr);
                    }
                }
                ConnectedChainConfig::Svm(cfg) => {
                    if let Ok(addr) = std::env::var("SOLVER_SVM_ADDR") {
                        solver_addresses.insert(cfg.chain_id, addr);
                    }
                }
                ConnectedChainConfig::Mvm(cfg) => {
                    if let Ok(addr) = std::env::var("SOLVER_MVMCON_ADDR") {
                        solver_addresses.insert(cfg.chain_id, addr);
                    }
                }
            }
        }

        // Validate: every threshold must reference a chain with a solver address
        for threshold in &liquidity_config.thresholds {
            if !solver_addresses.contains_key(&threshold.chain_id) {
                anyhow::bail!(
                    "Liquidity threshold references chain {} but no solver address is configured for it \
                     (set the corresponding SOLVER_*_ADDR env var)",
                    threshold.chain_id
                );
            }
        }

        // Create chain clients
        let hub_client = HubChainClient::new(&solver_config.hub_chain)?;
        let mvm_client = solver_config
            .get_mvm_config()
            .map(ConnectedMvmClient::new)
            .transpose()?;
        let evm_client = solver_config
            .get_evm_config()
            .map(ConnectedEvmClient::new)
            .transpose()?;
        let svm_client = solver_config
            .get_svm_config()
            .map(ConnectedSvmClient::new)
            .transpose()?;

        // Initialize tracking state from thresholds and acceptance pairs
        let mut initial_state = HashMap::new();

        for threshold in &liquidity_config.thresholds {
            let chain_token = ChainToken {
                chain_id: threshold.chain_id,
                token: threshold.token.clone(),
            };
            initial_state.entry(chain_token).or_insert(TokenLiquidity {
                confirmed_balance: 0,
                last_updated: Instant::now(),
                in_flight: Vec::new(),
            });
        }

        // Track all tokens the solver might spend (desired side of each pair)
        for pair in &solver_config.acceptance.token_pairs {
            let chain_token = ChainToken {
                chain_id: pair.target_chain_id,
                token: pair.target_token.clone(),
            };
            initial_state.entry(chain_token).or_insert(TokenLiquidity {
                confirmed_balance: 0,
                last_updated: Instant::now(),
                in_flight: Vec::new(),
            });
        }

        info!(
            "Liquidity monitor initialized: tracking {} token(s) across chains",
            initial_state.len()
        );

        Ok(Self {
            state: Arc::new(RwLock::new(initial_state)),
            config: liquidity_config,
            solver_config,
            hub_client,
            mvm_client,
            evm_client,
            svm_client,
            solver_addresses,
        })
    }

    /// Returns a reference to the shared state for test access.
    pub fn state(&self) -> &Arc<RwLock<HashMap<ChainToken, TokenLiquidity>>> {
        &self.state
    }

    /// Service loop: poll balances, cleanup expired commitments, check thresholds.
    pub async fn run(&self) -> Result<()> {
        let poll_interval = Duration::from_millis(self.config.balance_poll_interval_ms);
        info!(
            "Starting liquidity monitor (poll interval: {:?})",
            poll_interval
        );

        loop {
            self.poll_balances().await;
            self.cleanup_expired_commitments().await;
            if let Err(e) = self.check_and_warn_thresholds().await {
                error!("Error checking thresholds: {}", e);
            }
            tokio::time::sleep(poll_interval).await;
        }
    }

    /// Reserve budget for an accepted draft.
    ///
    /// Fails if available budget is insufficient for the requested amount.
    pub async fn reserve(
        &self,
        chain_token: &ChainToken,
        draft_id: &str,
        amount: u64,
    ) -> Result<()> {
        let mut state = self.state.write().await;
        let liquidity = state.entry(chain_token.clone()).or_insert(TokenLiquidity {
            confirmed_balance: 0,
            last_updated: Instant::now(),
            in_flight: Vec::new(),
        });

        if liquidity.available_budget() < amount as u128 {
            anyhow::bail!(
                "Insufficient budget for chain {} token {}: available={}, requested={}",
                chain_token.chain_id,
                chain_token.token,
                liquidity.available_budget(),
                amount
            );
        }

        liquidity.in_flight.push(InFlightCommitment {
            draft_id: draft_id.to_string(),
            amount,
            committed_at: Instant::now(),
        });

        info!(
            "Reserved {} for draft {} on chain {} token {} (remaining: {})",
            amount,
            draft_id,
            chain_token.chain_id,
            chain_token.token,
            liquidity.available_budget()
        );

        Ok(())
    }

    /// Release budget for a fulfilled draft.
    ///
    /// Removes the in-flight commitment AND deducts the spent amount from
    /// `confirmed_balance`.  This prevents a stale-balance window between the
    /// release and the next `poll_balances()` cycle — without this deduction
    /// the monitor would briefly think the pre-fulfillment balance is still
    /// available, causing it to accept drafts it cannot actually cover.
    ///
    /// The next balance poll will overwrite `confirmed_balance` with the true
    /// on-chain value, correcting any minor discrepancy (e.g., gas costs).
    ///
    /// No-op if the draft_id is not found (may have already been released via timeout).
    pub async fn release(&self, draft_id: &str) {
        let mut state = self.state.write().await;
        for liquidity in state.values_mut() {
            let before = liquidity.in_flight.len();
            let spent: u128 = liquidity
                .in_flight
                .iter()
                .filter(|c| c.draft_id == draft_id)
                .map(|c| c.amount as u128)
                .sum();
            liquidity.in_flight.retain(|c| c.draft_id != draft_id);
            if liquidity.in_flight.len() < before {
                liquidity.confirmed_balance = liquidity.confirmed_balance.saturating_sub(spent);
                info!("Released budget for draft {}", draft_id);
                return;
            }
        }
    }

    /// Check if there is sufficient budget for a given amount on a chain+token.
    pub async fn has_sufficient_budget(&self, chain_token: &ChainToken, amount: u64) -> bool {
        let state = self.state.read().await;
        match state.get(chain_token) {
            Some(liquidity) => liquidity.available_budget() >= amount as u128,
            None => false,
        }
    }

    /// Returns the gas token ChainToken for a given chain ID, based on chain type.
    ///
    /// Hub and MVM chains use MOVE (full 32-byte FA metadata), EVM chains use
    /// native ETH (zero address), SVM chains use native SOL (system program).
    pub fn gas_token_for_chain(&self, chain_id: u64) -> Result<ChainToken> {
        let chain_type = if chain_id == self.solver_config.hub_chain.chain_id {
            "mvm"
        } else {
            self.solver_config
                .get_connected_chain_by_id(chain_id)
                .ok_or_else(|| anyhow::anyhow!(
                    "No chain config for chain_id {} — \
                     startup validation should have caught this",
                    chain_id
                ))?
                .chain_type()
        };
        Ok(ChainToken {
            chain_id,
            token: gas_token_for_chain_type(chain_type)?.to_string(),
        })
    }

    /// Check if there is sufficient budget for a spend AND the remaining balance
    /// stays above the configured minimum threshold.
    ///
    /// Returns `true` if `available_budget >= amount + threshold`.
    pub async fn has_budget_after_spend(&self, chain_token: &ChainToken, amount: u64) -> Result<bool> {
        let state = self.state.read().await;
        let available = state
            .get(chain_token)
            .ok_or_else(|| anyhow::anyhow!(
                "No liquidity state for chain {} token {} — \
                 startup validation should have caught this",
                chain_token.chain_id, chain_token.token
            ))?
            .available_budget();

        let threshold = self
            .config
            .thresholds
            .iter()
            .find(|t| t.chain_id == chain_token.chain_id && t.token == chain_token.token)
            .ok_or_else(|| anyhow::anyhow!(
                "No liquidity threshold for chain {} token {} — \
                 startup validation should have caught this",
                chain_token.chain_id, chain_token.token
            ))?
            .min_balance;

        Ok(available >= (amount as u128).saturating_add(threshold as u128))
    }

    /// Check if available budget is above the configured minimum threshold.
    pub async fn is_above_threshold(&self, chain_token: &ChainToken) -> Result<bool> {
        let threshold = self
            .config
            .thresholds
            .iter()
            .find(|t| t.chain_id == chain_token.chain_id && t.token == chain_token.token)
            .ok_or_else(|| anyhow::anyhow!(
                "No liquidity threshold for chain {} token {} — \
                 startup validation should have caught this",
                chain_token.chain_id, chain_token.token
            ))?;

        let state = self.state.read().await;
        let liquidity = state
            .get(chain_token)
            .ok_or_else(|| anyhow::anyhow!(
                "No liquidity state for chain {} token {} — \
                 startup validation should have caught this",
                chain_token.chain_id, chain_token.token
            ))?;

        Ok(liquidity.available_budget() >= threshold.min_balance as u128)
    }

    // =========================================================================
    // Internal methods
    // =========================================================================

    /// Poll on-chain balances for all tracked tokens.
    async fn poll_balances(&self) {
        let chain_tokens: Vec<ChainToken> = {
            let state = self.state.read().await;
            state.keys().cloned().collect()
        };

        for chain_token in chain_tokens {
            let solver_addr = match self.solver_addresses.get(&chain_token.chain_id) {
                Some(addr) => addr.clone(),
                None => continue,
            };

            match self.query_balance(&chain_token, &solver_addr).await {
                Ok(balance) => {
                    let mut state = self.state.write().await;
                    if let Some(liquidity) = state.get_mut(&chain_token) {
                        liquidity.confirmed_balance = balance;
                        liquidity.last_updated = Instant::now();
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to poll balance for chain {} token {}: {}",
                        chain_token.chain_id, chain_token.token, e
                    );
                }
            }
        }
    }

    /// Query the on-chain balance for a specific token, dispatching to the
    /// correct chain client based on chain_id.
    async fn query_balance(&self, chain_token: &ChainToken, solver_addr: &str) -> Result<u128> {
        let hub_chain_id = self.solver_config.hub_chain.chain_id;

        if chain_token.chain_id == hub_chain_id {
            return self
                .hub_client
                .get_token_balance(solver_addr, &chain_token.token)
                .await;
        }

        let chain_config = self
            .solver_config
            .get_connected_chain_by_id(chain_token.chain_id)
            .context(format!(
                "No connected chain config for chain_id {}",
                chain_token.chain_id
            ))?;

        match chain_config {
            ConnectedChainConfig::Mvm(_) => {
                let client = self
                    .mvm_client
                    .as_ref()
                    .context("MVM client not available")?;
                client
                    .get_token_balance(solver_addr, &chain_token.token)
                    .await
            }
            ConnectedChainConfig::Evm(_) => {
                let client = self
                    .evm_client
                    .as_ref()
                    .context("EVM client not available")?;
                if chain_token.token == gas_token_for_chain_type("evm")? {
                    client.get_native_balance(solver_addr).await
                } else {
                    client
                        .get_token_balance(&chain_token.token, solver_addr)
                        .await
                }
            }
            ConnectedChainConfig::Svm(_) => {
                let client = self
                    .svm_client
                    .as_ref()
                    .context("SVM client not available")?;
                if chain_token.token == gas_token_for_chain_type("svm")? {
                    let owner_b58 = to_base58_pubkey(solver_addr)?;
                    client.get_native_balance(&owner_b58)
                } else {
                    let mint_b58 = to_base58_pubkey(&chain_token.token)?;
                    let owner_b58 = to_base58_pubkey(solver_addr)?;
                    client.get_token_balance(&mint_b58, &owner_b58)
                }
            }
        }
    }

    /// Remove in-flight commitments that have exceeded the configured timeout.
    async fn cleanup_expired_commitments(&self) {
        let timeout = Duration::from_secs(self.config.in_flight_timeout_secs);
        let mut state = self.state.write().await;

        for (chain_token, liquidity) in state.iter_mut() {
            let before = liquidity.in_flight.len();
            liquidity.in_flight.retain(|c| {
                if c.committed_at.elapsed() > timeout {
                    warn!(
                        "Releasing expired in-flight commitment: draft={}, amount={}, \
                         chain={}, token={}, age={:?}",
                        c.draft_id,
                        c.amount,
                        chain_token.chain_id,
                        chain_token.token,
                        c.committed_at.elapsed()
                    );
                    false
                } else {
                    true
                }
            });
            let removed = before - liquidity.in_flight.len();
            if removed > 0 {
                info!(
                    "Cleaned up {} expired commitment(s) for chain {} token {}",
                    removed, chain_token.chain_id, chain_token.token
                );
            }
        }
    }

    /// Log warnings for any chain+token where available budget is below threshold.
    async fn check_and_warn_thresholds(&self) -> Result<()> {
        let state = self.state.read().await;

        for threshold in &self.config.thresholds {
            let chain_token = ChainToken {
                chain_id: threshold.chain_id,
                token: threshold.token.clone(),
            };

            let liquidity = state
                .get(&chain_token)
                .ok_or_else(|| anyhow::anyhow!(
                    "No liquidity state for chain {} token {} — \
                     startup validation should have caught this",
                    chain_token.chain_id, chain_token.token
                ))?;

            let available = liquidity.available_budget();
            if available < threshold.min_balance as u128 {
                warn!(
                    "LOW LIQUIDITY: chain {} token {} \u{2014} available: {}, threshold: {}",
                    threshold.chain_id, threshold.token, available, threshold.min_balance
                );
            }
        }
        Ok(())
    }
}

/// Convert a 0x-prefixed hex address or base58 string to base58 pubkey format.
///
/// SVM balance queries require base58 addresses, but the config and env vars
/// may store addresses in 0x-hex format for cross-chain compatibility.
fn to_base58_pubkey(value: &str) -> Result<String> {
    if value.starts_with("0x") {
        let hex_str = &value[2..];
        let padded = format!("{:0>64}", hex_str);
        let bytes = hex::decode(&padded).context("Invalid hex in SVM address")?;
        if bytes.len() != 32 {
            anyhow::bail!("Expected 32 bytes for SVM pubkey, got {}", bytes.len());
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(Pubkey::new_from_array(array).to_string())
    } else {
        // Validate as base58 pubkey
        Pubkey::from_str(value).context("Invalid base58 SVM pubkey")?;
        Ok(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available_budget_no_in_flight() {
        let liquidity = TokenLiquidity {
            confirmed_balance: 1000,
            last_updated: Instant::now(),
            in_flight: Vec::new(),
        };
        assert_eq!(liquidity.available_budget(), 1000);
    }

    #[test]
    fn test_available_budget_with_in_flight() {
        let liquidity = TokenLiquidity {
            confirmed_balance: 1000,
            last_updated: Instant::now(),
            in_flight: vec![
                InFlightCommitment {
                    draft_id: "d1".to_string(),
                    amount: 300,
                    committed_at: Instant::now(),
                },
                InFlightCommitment {
                    draft_id: "d2".to_string(),
                    amount: 200,
                    committed_at: Instant::now(),
                },
            ],
        };
        assert_eq!(liquidity.available_budget(), 500);
    }

    #[test]
    fn test_available_budget_saturating_sub() {
        let liquidity = TokenLiquidity {
            confirmed_balance: 100,
            last_updated: Instant::now(),
            in_flight: vec![InFlightCommitment {
                draft_id: "d1".to_string(),
                amount: 500,
                committed_at: Instant::now(),
            }],
        };
        assert_eq!(liquidity.available_budget(), 0);
    }

    #[test]
    fn test_to_base58_pubkey_hex() {
        // 32 zero bytes in hex
        let hex = "0x0000000000000000000000000000000000000000000000000000000000000001";
        let result = to_base58_pubkey(hex).unwrap();
        let pubkey = Pubkey::from_str(&result).unwrap();
        assert_eq!(pubkey.to_bytes()[31], 1);
    }

    #[test]
    fn test_to_base58_pubkey_base58() {
        let b58 = "11111111111111111111111111111112";
        let result = to_base58_pubkey(b58).unwrap();
        assert_eq!(result, b58);
    }
}
