# Phase 2: Complete GMP Implementation (MVM, SVM, EVM) & Architecture Alignment

**Status:** ✅ Complete (19 commits)
**Depends On:** Phase 1
**Blocks:** Phase 3

**Goal:** Build complete GMP support for all three chain types (MVM, SVM, EVM) including hub and connected chain implementations, integrated GMP relay, and cross-chain architecture alignment.

---

## Commits

> 📋 **Commit Conventions:** Before each commit, review `.claude/CLAUDE.md` and `.cursor/rules` for commit message format, test requirements, and coding standards. Run `/review-tests-new` to verify test coverage before committing.

### Part 1: MVM + SVM Core GMP Implementation (Commits 1-13)

### Commit 1: Implement integrated GMP endpoint for Solana

**Files:**

- `intent-frameworks/svm/programs/integrated-gmp-endpoint/src/lib.rs` (already exists from Phase 1, extend)
- `intent-frameworks/svm/programs/integrated-gmp-endpoint/tests/endpoint_tests.rs`

**Tasks:**

- [x] Extend `Send` instruction to track nonces and emit structured `MessageSent` event
- [x] Extend `DeliverMessage` instruction to CPI into destination program's receive handler
- [x] Implement remote GMP endpoint verification via PDA
- [x] Add relay authorization checks
- [x] Test `Send` emits correct event with payload
- [x] Test `DeliverMessage` calls receiver's handler
- [x] Test nonce tracking and replay protection

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 2.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 2: Extend integrated GMP endpoint for Movement (MVM)

**Files:**

- `intent-frameworks/mvm/sources/gmp/intent_gmp.move` (already exists from Phase 1, extend)
- `intent-frameworks/mvm/tests/intent_gmp_tests.move`

**Tasks:**

- [x] Extend `intent_gmp` with CPI to destination module's receive handler
- [x] Add remote GMP endpoint verification (source chain + address validation)
- [x] Add replay protection with nonce tracking per source
- [x] Implement message routing based on payload type
- [x] Test Send emits correct event with payload
- [x] Test send/receive flow end-to-end
- [x] Test relay authorization and replay protection
- [x] Update `intent-frameworks/extension-checklist.md` with MVM test status (tests 13-15)
- [x] Verify SVM and MVM test alignment in extension-checklist.md (all shared tests have matching status)

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 3.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 3: Implement OutflowValidator program (SVM)

**Files:**

- `intent-frameworks/svm/programs/outflow-validator/src/lib.rs` (already exists from Phase 1, implement)
- `intent-frameworks/svm/programs/outflow-validator/tests/validator_tests.rs`

**Tasks:**

- [x] Implement GMP receive handler for integrated GMP endpoint
- [x] Implement `gmp_receive` to receive intent requirements from hub
- [x] **Idempotency check**: Before storing, check if requirements already exist for intent_id + step number
- [x] **If requirements already exist → ignore duplicate message (idempotent)**
- [x] **If requirements don't exist → store intent requirements** in PDA (intent_id/step => {requirements, authorizedSolver})
- [x] Implement `fulfill_intent` instruction for authorized solvers to call
- [x] Instruction pulls tokens from authorized solver's wallet via SPL token transfer
- [x] Validate recipient, amount, token match stored requirements
- [x] Validate solver matches authorized solver from stored requirements
- [x] Forward tokens to user wallet
- [x] Send GMP message to hub via `gmp_send`
- [x] Emit `FulfillmentSucceeded` or `FulfillmentFailed` events
- [x] Test all validation scenarios
- [x] Update `intent-frameworks/extension-checklist.md` with SVM OutflowValidator test status

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 4.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 4: Implement OutflowValidator module (MVM)

**Files:**

- `intent-frameworks/mvm/sources/gmp/outflow_validator.move` (already exists from Phase 1, implement)
- `intent-frameworks/mvm/tests/outflow_validator_tests.move`

**Tasks:**

- [x] Implement GMP receive handler for integrated GMP endpoint
- [x] Implement `gmp_receive` to receive intent requirements from hub
- [x] **Idempotency check**: Before storing, check if requirements already exist for intent_id + step number
- [x] **If requirements already exist → ignore duplicate message (idempotent)**
- [x] **If requirements don't exist → store intent requirements** (intent_id/step => {requirements, authorizedSolver})
- [x] Implement `fulfill_intent` entry function for authorized solvers to call
- [x] Function pulls tokens from authorized solver's wallet via coin/FA transfer
- [x] Validate recipient, amount, token match stored requirements
- [x] Validate solver matches authorized solver from stored requirements
- [x] Forward tokens to user wallet
- [x] Send GMP message to hub via `gmp_send`
- [x] Emit `FulfillmentSucceeded` or `FulfillmentFailed` events
- [x] Test all validation scenarios
- [x] Update `intent-frameworks/extension-checklist.md` with MVM OutflowValidator test status

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 5.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 5: Extend intent_escrow with full GMP support (SVM)

**Files:**

- `intent-frameworks/svm/programs/intent_escrow/src/` (extend existing)
- `intent-frameworks/svm/programs/intent_escrow/tests/`

**Already done (from commit 4e6b251):**

- [x] Implement `GmpReceiveRequirements` - stores intent requirements from hub
- [x] Implement `GmpReceiveFulfillmentProof` - auto-releases escrow on proof receipt
- [x] Implement `CreateEscrow` with optional requirements validation
- [x] Add `StoredIntentRequirements` account structure

**Remaining tasks:**

- [x] Add `GmpConfig` account (hub_chain_id, hub_gmp_endpoint_addr, gmp_endpoint)
- [x] Add `SetGmpConfig` instruction for admin configuration
- [x] Add source chain/address validation to `GmpReceiveRequirements` and `GmpReceiveFulfillmentProof`
- [x] Add idempotency to `GmpReceiveRequirements` (emit duplicate event instead of error)
- [x] Send `EscrowConfirmation` GMP message back to hub on escrow creation
- [x] ~~Add events module for GMP events~~ (using msg! logs - sufficient for Solana)
- [x] Update tests for GMP config and EscrowConfirmation flow (added `tests/gmp.rs` with 13 tests)
- [x] Update `intent-frameworks/extension-checklist.md` with SVM InflowEscrow test status

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 6.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 6: Implement inflow_escrow module (MVM)

**Files:**

- `intent-frameworks/mvm/sources/gmp/inflow_escrow.move`
- `intent-frameworks/mvm/tests/inflow_escrow_tests.move`

**Tasks:**

- [x] Create `inflow_escrow` module with GMP config (hub_chain_id, hub_gmp_endpoint_addr)
- [x] Implement `receive_intent_requirements` - stores requirements from hub (with idempotency)
- [x] Implement `create_escrow_with_validation` - validates requirements exist and match escrow details
- [x] Implement `receive_fulfillment_proof` - marks escrow fulfilled (MVM uses manual release, not auto-release)
- [x] Send `EscrowConfirmation` GMP message back to hub on escrow creation via `gmp_sender::gmp_send`
- [x] Add routing in `intent_gmp` for inflow escrow messages
- [x] Implement MVM tests 8, 12, 13 in `inflow_escrow_tests.move`
- [x] Update `intent-frameworks/extension-checklist.md` with MVM InflowEscrow test status

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 7.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 7: Complete GMP hub interface with message sending and source validation

**Files:**

- `intent-frameworks/mvm/sources/interfaces/intent_gmp_hub.move` (extend existing)
- `intent-frameworks/mvm/tests/intent_gmp_tests.move`
- `intent-frameworks/mvm/tests/intent_gmp_tests.move`

**Completed tasks:**

- [x] Add GmpHubConfig with remote GMP endpoint mapping per chain ID
- [x] Add initialize() and set_remote_gmp_endpoint_addr() for configuration management
- [x] Integrate with `gmp_sender::gmp_send()` for actual message sending (send functions now call gmp_send and return nonce)
- [x] Add source validation in receive handlers (both receive functions validate source via registered remote GMP endpoint check)
- [x] Test message encoding (payload size and discriminator bytes verified)
- [x] Test source validation (added tests for rejecting unregistered sources)
- [x] Update intent_gmp tests to initialize intent_gmp_hub config

**Test:**

```bash
nix develop ./nix -c bash -c "cd intent-frameworks/mvm && movement move test --dev --named-addresses mvmt_intent=0x123"
```

> ⚠️ **All MVM unit tests must pass before proceeding to Commit 8.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 8: Integrate GMP hub with intent creation and fulfillment flows

**Files:**

- `intent-frameworks/mvm/sources/fa_intent_inflow.move`
- `intent-frameworks/mvm/sources/fa_intent_outflow.move`
- `intent-frameworks/mvm/tests/fa_intent_inflow_tests.move` (if exists)
- `intent-frameworks/mvm/tests/fa_intent_outflow_tests.move` (if exists)

**Tasks:**

- [x] Update `fa_intent_inflow::create_inflow_intent()` to call `intent_gmp_hub::send_intent_requirements()`
- [x] Update `fa_intent_inflow::fulfill_inflow_intent()` to:
  - Check for escrow confirmation receipt before allowing fulfillment
  - Call `intent_gmp_hub::send_fulfillment_proof()` after fulfillment
- [x] Update `fa_intent_outflow::create_outflow_intent()` to call `intent_gmp_hub::send_intent_requirements()`
- [x] Add GMP receive handler in `fa_intent_outflow` to process fulfillment proofs and trigger token release
- [x] Add intent state tracking (escrow_confirmed, fulfillment_proof_received) in intent storage
- [x] Test intent creation sends GMP requirements
- [x] Test fulfillment blocked without escrow confirmation
- [x] Test fulfillment proof triggers token release

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 9.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 9: Implement integrated GMP relay in integrated-gmp

**Files:**

- `integrated-gmp/src/integrated_gmp_relay.rs` (new)
- `integrated-gmp/src/main.rs` (simplified to only run integrated GMP relay)
- `integrated-gmp/src/lib.rs` (added integrated_gmp_relay module export)

**Tasks:**

- [x] Add `NativeGmpRelay` struct
- [x] Watch for `MessageSent` events on MVM integrated GMP endpoint
- [x] Watch for `MessageSent` events on SVM integrated GMP endpoint
- [x] Support configurable chain RPCs and endpoint addresses
- [x] Make integrated GMP relay the default mode (no `--mode` flag needed)
- [x] Add unit tests for event parsing (inline in module)

**Test:**

```bash
RUST_LOG=off nix develop ./nix -c bash -c "cd integrated-gmp && cargo test --quiet"
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 10.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 10: Implement integrated GMP relay with MVM connected chain support

**Files:**

- `integrated-gmp/src/integrated_gmp_relay.rs` (implement tx submission and MVM connected chain)
- `integrated-gmp/src/main.rs` (simplify to only run integrated GMP relay)
- `integrated-gmp/tests/integrated_gmp_relay_tests.rs` (add relay config tests)
- `testing-infra/ci-e2e/chain-hub/deploy-contracts.sh` (GMP initialization)
- `testing-infra/ci-e2e/chain-connected-mvm/deploy-contracts.sh` (GMP initialization)
- `intent-frameworks/svm/programs/*/src/error.rs` (fix naming)

**Tasks:**

- [x] Implement actual transaction submission in `deliver_to_mvm()` and `deliver_to_svm()`
- [x] Add Move address derivation utilities to `generate_keys` and `util.sh`
- [x] Update deployment scripts to initialize GMP modules with remote GMP endpoints
- [x] Fix SVM error variant naming (E_SCREAMING_CASE → UpperCamelCase)
- [x] Update MVM e2e test environment to use integrated GMP endpoints (deploy scripts initialize GMP)
- [x] Add MVM connected chain support to integrated GMP relay (bidirectional MVM ↔ MVM messaging)
- [x] Simplify main.rs to only run integrated GMP relay (removed legacy mode)
- [x] Add relay config tests for MVM connected chain extraction

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **Unit tests must pass before proceeding to Commit 11.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 11: Update solver and MVM E2E tests to use GMP flow

**Files:**

- `solver/src/service/outflow.rs` (update to use outflow_validator)
- `solver/src/chains/connected_mvm.rs` (add fulfill_outflow_via_gmp method)
- `testing-infra/ci-e2e/e2e-tests-mvm/` (no changes needed - solver handles flow)

**Tasks:**

- [x] Update solver outflow service to call `outflow_validator::fulfill_intent` on connected chain
  - Added `fulfill_outflow_via_gmp()` method to `ConnectedMvmClient`
  - Updated `OutflowService::execute_connected_transfer()` to use GMP flow for MVM
  - Updated `OutflowService::run()` to skip integrated-gmp approval for GMP flow (hub auto-releases)
- [x] Start integrated GMP relay in background during tests (already done via `start_integrated_gmp`)
- [x] Update `run-tests-outflow.sh` to use GMP flow (no changes needed - solver handles flow internally)
- [x] Update `run-tests-inflow.sh` to use GMP flow (no changes needed - MVM inflow uses existing mechanism)
- [x] Verify GMP messages are sent and received correctly (MVM hub ↔ MVM connected) - requires E2E testing in CI
- [x] Ensure existing test assertions still pass - requires E2E testing in CI

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh

# Run MVM e2e tests
./testing-infra/ci-e2e/e2e-tests-mvm/run-tests-outflow.sh
./testing-infra/ci-e2e/e2e-tests-mvm/run-tests-inflow.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 12.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 12: Fix existing E2E tests for GMP: MVM ↔ SVM outflow

**Files:**

- `testing-infra/ci-e2e/chain-connected-svm/deploy-contract.sh` (deploy GMP programs)
- `testing-infra/ci-e2e/chain-connected-svm/configure-integrated-gmp.sh` (add GMP endpoint)
- `intent-frameworks/svm/scripts/build.sh` (build all GMP programs)
- `solver/src/config.rs` (add optional GMP program IDs to SvmChainConfig)
- `solver/src/chains/connected_svm.rs` (add fulfill_outflow_via_gmp method)
- `solver/src/service/outflow.rs` (attempt GMP flow for SVM)

**Tasks:**

- [x] Update SVM e2e test environment to use integrated GMP endpoints
  - Build script now builds integrated-gmp-endpoint and outflow-validator
  - Deploy script deploys all 3 programs and configures remote GMP endpoints
  - Configure-integrated-gmp uses GMP endpoint program ID for relay
- [x] Start integrated GMP relay in background during tests (already done via `start_integrated_gmp`)
- [x] Update outflow test to use GMP flow (solver calls validation contract)
  - Implemented `fulfill_outflow_via_gmp()` in ConnectedSvmClient (builds FulfillIntent tx directly)
  - OutflowService uses GMP-only flow for SVM (no fallback to direct transfer)
  - Removed dead code: `transfer_with_intent_id()`, `build_memo_instruction()`, `MEMO_PROGRAM_ID`
- [x] Verify GMP messages are sent and received correctly - requires E2E testing in CI
- [x] Ensure existing test assertions still pass - requires E2E testing in CI

**Note:** SVM GMP flow implementation complete. The solver builds and submits `outflow_validator::FulfillIntent` transactions directly using Solana RPC. Commit 13 will complete the SVM inflow flow.

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh

# Run SVM e2e tests
./testing-infra/ci-e2e/e2e-tests-svm/run-tests-outflow.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 13.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 13: Fix existing E2E tests for GMP: MVM ↔ SVM inflow

**Files:**

- `testing-infra/ci-e2e/e2e-tests-svm/` (update existing)

**Tasks:**

- [x] Update inflow test to use GMP flow (escrow receives requirements via GMP)
- [x] Verify escrow confirmation GMP message sent back to MVM
- [x] Verify fulfillment proof GMP message sent to SVM
- [x] Verify escrow auto-releases on fulfillment proof receipt
- [x] Ensure existing test assertions still pass

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh

# Run SVM e2e tests
./testing-infra/ci-e2e/e2e-tests-svm/run-tests-inflow.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 14.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Part 2: EVM Integration & Cross-Chain Architecture Alignment (Commits 14-19)

### Commit 14: Audit MVM connected chain modules

**Files:**

- `intent-frameworks/mvm/sources/gmp/intent_inflow_escrow.move`
- `intent-frameworks/mvm/sources/gmp/intent_outflow_validator.move`
- `docs/architecture/plan/gmp-phase6-audit-mvm-connected-chain.md`

**Tasks:**

- [x] Review `intent_inflow_escrow.move` dependencies
- [x] Review `intent_outflow_validator.move` dependencies
- [x] Identify shared code with hub modules
- [x] Document minimal required dependencies
- [x] Document audit results in `gmp-phase6-audit-mvm-connected-chain.md`

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **All unit tests must pass before proceeding to Commit 15.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 15: Split MVM package into three separate packages

**Files:**

- `intent-frameworks/mvm/intent-gmp/` (new package)
- `intent-frameworks/mvm/intent-hub/` (new package)
- `intent-frameworks/mvm/intent-connected/` (new package)
- Deployment scripts updated

**Tasks:**

- [x] Create three packages:
  - **`intent-gmp`** (8KB bytecode, 16KB deploy) - gmp_common, gmp_sender, gmp_intent_state, gmp_endpoints
  - **`intent-hub`** (35KB bytecode, 75KB deploy) - All core intent modules + hub-specific intent_gmp
  - **`intent-connected`** (14KB bytecode, 14KB deploy) - intent_outflow_validator, intent_inflow_escrow + connected-specific intent_gmp
- [x] Remove `is_initialized()` conditional routing - missing init is now a hard failure
- [x] Update deployment scripts (hub deploys intent-gmp then intent-hub with `--chunked-publish`)
- [x] Verify all 164 MVM tests passing across 3 packages

**Note:** intent-hub still exceeds 60KB (75KB) and requires `--chunked-publish`

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **All unit tests must pass before proceeding to Commit 16.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 16: Rename SVM programs for consistency

**Files:**

- `intent-frameworks/svm/programs/intent-gmp/` (renamed from integrated-gmp-endpoint)
- `intent-frameworks/svm/programs/intent-outflow-validator/` (renamed from outflow-validator)
- `intent-frameworks/svm/Cargo.toml`
- `intent-frameworks/svm/scripts/build.sh`
- `intent-frameworks/svm/scripts/test.sh`

**Tasks:**

- [x] Rename `integrated-gmp-endpoint` → `intent-gmp`
- [x] Rename `outflow-validator` → `intent-outflow-validator`
- [x] Update Cargo.toml workspace members
- [x] Update Rust imports across all SVM programs
- [x] Update build.sh and test.sh scripts
- [x] Final SVM structure (2 logical groups, 3 programs):
  - **`intent-gmp`** - GMP infrastructure
  - **`intent-connected`** = `intent-escrow` + `intent-outflow-validator` (2 programs, logically grouped)

**Note:** Unlike Move, Solana cannot bundle programs into packages - each is deployed separately

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **All unit tests must pass before proceeding to Commit 17.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 17: Align EVM architecture with MVM/SVM patterns

**Files:**

- `intent-frameworks/evm/contracts/IntentGmp.sol` (new)
- `intent-frameworks/evm/contracts/IntentOutflowValidator.sol` (new)
- `intent-frameworks/evm/contracts/IntentInflowEscrow.sol` (new)
- `intent-frameworks/evm/contracts/gmp-common/Messages.sol` (new)
- `intent-frameworks/evm/contracts/gmp-common/Endpoints.sol` (new)
- `integrated-gmp/src/integrated_gmp_relay.rs` (extended for EVM)
- `solver/src/chains/connected_evm.rs` (added fulfill_outflow_via_gmp)
- `solver/src/service/outflow.rs` (EVM uses GMP flow)
- E2E deployment and test scripts

**Tasks:**

- [x] Create `IntentGmp.sol` - GMP infrastructure (like MVM intent-gmp, SVM intent-gmp)
  - `send()` function emits `MessageSent` event
  - `deliverMessage()` for relay to inject messages
  - Remote GMP endpoint verification and relay authorization
  - Message nonce tracking for replay protection
- [x] Create `IntentOutflowValidator.sol` - Outflow validation (like MVM/SVM intent-outflow-validator)
  - `receiveMessage()` receives intent requirements from hub (idempotent)
  - `fulfillIntent()` for authorized solvers (pulls tokens via `transferFrom`, validates, forwards, sends GMP)
- [x] Create `IntentInflowEscrow.sol` - Escrow for inflow (like SVM intent-inflow-escrow)
  - `receiveMessage()` for intent requirements from hub (idempotent)
  - `createEscrowWithValidation()` validates requirements match escrow details
  - Auto-release on fulfillment proof receipt
  - Sends `EscrowConfirmation` back to hub on creation
- [x] Create shared message encoding/decoding libraries in `gmp-common/`
- [x] Extend integrated GMP relay for EVM (event parsing, message delivery)
- [x] Update solver to use GMP flow for EVM (added `fulfill_outflow_via_gmp()`)
- [x] Update E2E deployment scripts for EVM GMP contracts
- [x] Update E2E tests for GMP flow (inflow + outflow)

**Tests:** EVM 161 unit tests, Solver 149, Integrated-GMP 191

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh

# Run EVM e2e tests
./testing-infra/ci-e2e/e2e-tests-evm/run-tests-outflow.sh
./testing-infra/ci-e2e/e2e-tests-evm/run-tests-inflow.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 18.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 18: Auto-release escrow on FulfillmentProof receipt (GMP flow)

**Files:**

- `intent-frameworks/mvm/intent-connected/sources/intent_inflow_escrow.move`
- `solver/src/service/inflow.rs`
- `solver/src/chains/connected_mvm.rs`
- MVM escrow tests

**Tasks:**

- [x] Update `receive_fulfillment_proof` to auto-release (transfer tokens to solver and mark fulfilled+released)
- [x] Keep `release_escrow` as manual fallback
- [x] Update solver to poll `is_escrow_released` instead of calling manual release
- [x] Mark `release_gmp_escrow` as dead code in solver
- [x] Update 5 Move tests to reflect auto-release behavior
- [x] Verify E2E tests still pass (release happens faster now)

**Note:** Collapsed two-step release into single step matching SVM behavior

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh

# Run MVM e2e tests
./testing-infra/ci-e2e/e2e-tests-mvm/run-tests-inflow.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 19.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 19: Document current intent type differences

**Files:**

- `docs/architecture/plan/gmp-phase3-intent-type-analysis.md` (new)

**Tasks:**

- [x] Investigate whether hub intents can be unified into a single base type
- [x] Document 5 shared fields between `FALimitOrder` and `OracleGuardedLimitOrder`
- [x] Document key differences:
  - `OracleGuardedLimitOrder` has defense-in-depth (type-level authorization)
  - `FALimitOrder` relies on wrapper-level security only
  - `intent_id` differs: `Option<address>` (inflow) vs `address` (outflow, always present)
  - `OracleGuardedLimitOrder` has 2 unique fields: `requirement` (oracle) and `requester_addr_connected_chain`
  - Cross-chain payment: outflow uses 0 payment on hub vs inflow always requires payment
  - Outflow is non-revocable (security); inflow is revocable
- [x] Document conclusion: separate types justified by security model

**Test:**

No code changes - documentation only

> ⚠️ **Documentation review complete.** Run `/review-commit-tasks` then `/commit` to finalize.

---

---

## Run All Tests

```bash
# Run all unit tests (includes coordinator, integrated-gmp, solver, MVM, EVM, SVM, frontend)
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI runs e2e tests automatically. All e2e tests must pass before merging.**

---

## Exit Criteria

- [x] All 19 commits complete
- [x] SVM programs build and pass unit tests
- [x] MVM modules build and pass unit tests (split into 3 packages)
- [x] EVM contracts build and pass unit tests
- [x] Integrated GMP relay works for MVM ↔ MVM, MVM ↔ SVM, MVM ↔ EVM
- [x] Cross-chain E2E tests pass for all chain combinations (outflow + inflow)
- [x] MVM escrow auto-releases on FulfillmentProof (matches SVM/EVM behavior)
- [x] Cross-chain architecture aligned across all three VMs (consistent naming, package structure)
- [x] Intent type differences documented
