# Solver Supply Monitoring

## Context

The solver currently accepts intents based purely on static config (token pairs + exchange rates). It has **no awareness of its own liquidity**. This means it can sign drafts it cannot fulfill, wasting time and blocking requesters. This feature adds an in-memory liquidity tracker that:

- Periodically polls the solver's wallet balances on each chain
- Deducts in-flight (committed but unconfirmed) amounts from available budget
- Releases budget for in-flight amounts that time out (assumed failed)
- Rejects new drafts when available budget is below a configurable threshold
- Logs warnings when liquidity is low

## Architecture

A new `LiquidityMonitor` runs as a 5th concurrent service alongside signing, tracker, inflow, and outflow. It owns shared state (`Arc<RwLock<HashMap<ChainToken, TokenLiquidity>>>`) that the signing service reads before accepting drafts.

```text
                        ┌─────────────────────┐
                        │  LiquidityMonitor    │
                        │  (polls balances,    │
                        │   cleans up expired) │
                        └─────────┬───────────┘
                                  │ Arc<RwLock<state>>
              ┌───────────────────┼───────────────────┐
              │                   │                   │
    ┌─────────▼──────┐  ┌────────▼───────┐  ┌───────▼────────┐
    │ SigningService  │  │ InflowService  │  │ OutflowService │
    │ (check + reserve)│ │ (release)      │  │ (release)      │
    └────────────────┘  └────────────────┘  └────────────────┘
```

**Key principle**: The solver spends `desired_amount` of `desired_token` on `desired_chain_id` for both inflow and outflow. So the "target" for budget tracking is always `(desired_chain_id, desired_token)`.

## Implementation

### Phase 1: Config & Balance Queries

#### 1a. Config additions — `solver/src/config.rs`

Add optional `[liquidity]` section to `SolverConfig`:

```toml
[liquidity]
balance_poll_interval_ms = 10000    # poll every 10s
in_flight_timeout_secs = 300        # release budget after 5min if unconfirmed

[[liquidity.threshold]]
chain_id = 1
token = "0x..."
min_balance = 100000
```

New structs: `LiquidityMonitorConfig` (with `balance_poll_interval_ms`, `in_flight_timeout_secs`, `thresholds: Vec<LiquidityThresholdConfig>`) and `LiquidityThresholdConfig` (with `chain_id`, `token`, `min_balance`).

Field on `SolverConfig`: `pub liquidity: Option<LiquidityMonitorConfig>` with `#[serde(default)]`.

Add validation in `validate()`: intervals > 0, thresholds reference known chain_ids, valid token formats, min_balance > 0.

#### 1b. Hub balance query — `solver/src/chains/hub.rs`

Add `get_token_balance(&self, account_addr: &str, token_metadata: &str) -> Result<u64>`.

Uses the existing view function pattern (`POST /v1/view`) calling `0x1::primary_fungible_store::balance(account_addr, token_metadata)`. Same pattern as `is_escrow_confirmed`.

#### 1c. EVM balance query — `solver/src/chains/connected_evm.rs`

Add `get_token_balance(&self, token_addr: &str, account_addr: &str) -> Result<u64>`.

Uses `eth_call` with ERC20 `balanceOf(address)` selector `0x70a08231`. Same JSON-RPC pattern as existing `has_outflow_requirements`.

#### 1d. SVM balance query — `solver/src/chains/connected_svm.rs`

Add `get_token_balance(&self, token_mint: &str, owner: &str) -> Result<u64>`.

Derives the Associated Token Account, calls `rpc_client.get_token_account_balance()`. Uses existing `solana_client::RpcClient`.

#### 1e. Connected MVM balance query — `solver/src/chains/connected_mvm.rs`

Add `get_token_balance(&self, account_addr: &str, token_metadata: &str) -> Result<u64>`.

Same view function pattern as hub client.

### Phase 2: LiquidityMonitor Module

#### 2a. New file — `solver/src/service/liquidity.rs`

Core data structures:

- `ChainToken { chain_id: u64, token: String }` — identifies a token on a chain
- `InFlightCommitment { draft_id: String, amount: u64, committed_at: Instant }` — budget reservation
- `TokenLiquidity { confirmed_balance: u64, last_updated: Instant, in_flight: Vec<InFlightCommitment> }` — per-token state
  - `available_budget()` = `confirmed_balance.saturating_sub(sum(in_flight amounts))`

`LiquidityMonitor` struct:

- `state: Arc<RwLock<HashMap<ChainToken, TokenLiquidity>>>`
- `config: LiquidityMonitorConfig`
- `solver_config: SolverConfig`

Public methods:

- `new(solver_config, liquidity_config) -> Result<Self>` — initializes state from configured thresholds
- `run() -> Result<()>` — service loop: poll_balances → cleanup_expired → check_thresholds → sleep
- `reserve(chain_token, draft_id, amount) -> Result<()>` — add in-flight commitment, fail if insufficient
- `release(draft_id)` — remove in-flight commitment by draft_id
- `has_sufficient_budget(chain_token, amount) -> bool` — check available_budget >= amount
- `is_above_threshold(chain_token) -> bool` — check available_budget >= configured min_balance

Internal methods:

- `poll_balances()` — queries each chain client for solver's balance, updates `confirmed_balance`
- `cleanup_expired_commitments()` — removes commitments where `committed_at.elapsed() > in_flight_timeout_secs`, logs warning for each
- `check_and_warn_thresholds()` — logs `warn!` for any chain_token below threshold

#### 2b. Export — `solver/src/service/mod.rs`

Add `pub mod liquidity;` and re-export `LiquidityMonitor`.

### Phase 3: Integration

#### 3a. SigningService — `solver/src/service/signing.rs`

- Add field: `liquidity_monitor: Option<Arc<LiquidityMonitor>>`
- Update `new()` to accept `Option<Arc<LiquidityMonitor>>`
- In `process_draft()`, after `AcceptanceResult::Accept` and before `sign_and_submit()`:
  1. Check `is_above_threshold(target_chain_token)` — reject with warning if below
  2. Check `has_sufficient_budget(target_chain_token, desired_amount)` — reject with warning if insufficient
- After successful `sign_and_submit()` + tracker add, call `reserve(target_chain_token, draft_id, desired_amount)`

#### 3b. InflowService — `solver/src/service/inflow.rs`

- Add field: `liquidity_monitor: Option<Arc<LiquidityMonitor>>`
- Update `new()` to accept it
- After `tracker.mark_fulfilled()` succeeds in `run()`, call `liquidity_monitor.release(draft_id)`

#### 3c. OutflowService — `solver/src/service/outflow.rs`

- Add field: `liquidity_monitor: Option<Arc<LiquidityMonitor>>`
- Update `new()` to accept it
- After `tracker.mark_fulfilled()` succeeds in `run()`, call `liquidity_monitor.release(draft_id)`

#### 3d. Main entry point — `solver/src/bin/solver.rs`

- Create `LiquidityMonitor` if `config.liquidity` is `Some`
- Pass `Option<Arc<LiquidityMonitor>>` to all three services
- Add liquidity monitor to `tokio::select!` block (use `std::future::pending()` if None)

### Phase 4: Tests

#### New file: `solver/tests/liquidity_tests.rs`

Tests for `LiquidityMonitor` (in-memory, no RPC):

- **Budget calculation**: no in-flight → full balance; with in-flight → reduced; saturating_sub on overflow
- **Reserve/release**: reserve reduces budget; release restores; release unknown draft_id is no-op; reserve fails when insufficient
- **Threshold checks**: above threshold → true; below → false; no threshold configured → true
- **Timeout cleanup**: expired commitments released; fresh commitments preserved
- **Independence**: reservations on chain A don't affect chain B

#### Existing files:

- `solver/tests/config_tests.rs`: config with/without `[liquidity]` section; validation rejects zero interval, unknown chain_id
- Chain client tests: verify `get_token_balance()` constructs correct RPC request (wiremock for hub/EVM/MVM, ATA derivation for SVM)

## Verification

```bash
RUST_LOG=off nix develop ./nix -c bash -c "cd solver && cargo test --quiet"
```

All existing tests must continue to pass (LiquidityMonitor is optional/None in existing test configs). New tests validate the liquidity logic in isolation.
