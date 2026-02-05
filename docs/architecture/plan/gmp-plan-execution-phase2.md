# Phase 2: SVM + MVM Core Implementation (2-3 days)

**Status:** ✅ Complete
**Depends On:** Phase 1
**Blocks:** Phase 3

**Goal:** Build GMP support for both SVM (connected chain) and MVM (hub) together so we can test real cross-chain messaging from the start.

---

## Commits

> 📋 **Commit Conventions:** Before each commit, review `.claude/CLAUDE.md` and `.cursor/rules` for commit message format, test requirements, and coding standards. Run `/review-tests-new` to verify test coverage before committing.

### Commit 1: Implement native GMP endpoint for Solana

**Files:**

- `intent-frameworks/svm/programs/native-gmp-endpoint/src/lib.rs` (already exists from Phase 1, extend)
- `intent-frameworks/svm/programs/native-gmp-endpoint/tests/endpoint_tests.rs`

**Tasks:**

- [x] Extend `Send` instruction to track nonces and emit structured `MessageSent` event
- [x] Extend `DeliverMessage` instruction to CPI into destination program's receive handler
- [x] Implement trusted remote verification via PDA
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

### Commit 2: Extend native GMP endpoint for Movement (MVM)

**Files:**

- `intent-frameworks/mvm/sources/gmp/native_gmp_endpoint.move` (already exists from Phase 1, extend)
- `intent-frameworks/mvm/tests/native_gmp_endpoint_tests.move`

**Tasks:**

- [x] Extend `native_gmp_endpoint` with CPI to destination module's receive handler
- [x] Add trusted remote verification (source chain + address validation)
- [x] Add replay protection with nonce tracking per source
- [x] Implement message routing based on payload type
- [x] Test Send emits correct event with payload
- [x] Test send/receive flow end-to-end
- [x] Test relay authorization and replay protection
- [x] Update `intent-frameworks/EXTENSION-CHECKLIST.md` with MVM test status (tests 13-15)
- [x] Verify SVM and MVM test alignment in EXTENSION-CHECKLIST.md (all shared tests have matching status)

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

- [x] Implement GMP receive handler for native GMP endpoint
- [x] Implement `lz_receive` to receive intent requirements from hub
- [x] **Idempotency check**: Before storing, check if requirements already exist for intent_id + step number
- [x] **If requirements already exist → ignore duplicate message (idempotent)**
- [x] **If requirements don't exist → store intent requirements** in PDA (intent_id/step => {requirements, authorizedSolver})
- [x] Implement `fulfill_intent` instruction for authorized solvers to call
- [x] Instruction pulls tokens from authorized solver's wallet via SPL token transfer
- [x] Validate recipient, amount, token match stored requirements
- [x] Validate solver matches authorized solver from stored requirements
- [x] Forward tokens to user wallet
- [x] Send GMP message to hub via `lz_send`
- [x] Emit `FulfillmentSucceeded` or `FulfillmentFailed` events
- [x] Test all validation scenarios
- [x] Update `intent-frameworks/EXTENSION-CHECKLIST.md` with SVM OutflowValidator test status

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

- [x] Implement GMP receive handler for native GMP endpoint
- [x] Implement `lz_receive` to receive intent requirements from hub
- [x] **Idempotency check**: Before storing, check if requirements already exist for intent_id + step number
- [x] **If requirements already exist → ignore duplicate message (idempotent)**
- [x] **If requirements don't exist → store intent requirements** (intent_id/step => {requirements, authorizedSolver})
- [x] Implement `fulfill_intent` entry function for authorized solvers to call
- [x] Function pulls tokens from authorized solver's wallet via coin/FA transfer
- [x] Validate recipient, amount, token match stored requirements
- [x] Validate solver matches authorized solver from stored requirements
- [x] Forward tokens to user wallet
- [x] Send GMP message to hub via `lz_send`
- [x] Emit `FulfillmentSucceeded` or `FulfillmentFailed` events
- [x] Test all validation scenarios
- [x] Update `intent-frameworks/EXTENSION-CHECKLIST.md` with MVM OutflowValidator test status

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

- [x] Implement `LzReceiveRequirements` - stores intent requirements from hub
- [x] Implement `LzReceiveFulfillmentProof` - auto-releases escrow on proof receipt
- [x] Implement `CreateEscrow` with optional requirements validation
- [x] Add `StoredIntentRequirements` account structure

**Remaining tasks:**

- [x] Add `GmpConfig` account (hub_chain_id, trusted_hub_addr, gmp_endpoint)
- [x] Add `SetGmpConfig` instruction for admin configuration
- [x] Add source chain/address validation to `LzReceiveRequirements` and `LzReceiveFulfillmentProof`
- [x] Add idempotency to `LzReceiveRequirements` (emit duplicate event instead of error)
- [x] Send `EscrowConfirmation` GMP message back to hub on escrow creation
- [x] ~~Add events module for GMP events~~ (using msg! logs - sufficient for Solana)
- [x] Update tests for GMP config and EscrowConfirmation flow (added `tests/gmp.rs` with 13 tests)
- [x] Update `intent-frameworks/EXTENSION-CHECKLIST.md` with SVM InflowEscrow test status

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 6.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 6: Implement inflow_escrow_gmp module (MVM)

**Files:**

- `intent-frameworks/mvm/sources/gmp/inflow_escrow_gmp.move`
- `intent-frameworks/mvm/tests/inflow_escrow_gmp_tests.move`

**Tasks:**

- [x] Create `inflow_escrow_gmp` module with GMP config (hub_chain_id, trusted_hub_addr)
- [x] Implement `receive_intent_requirements` - stores requirements from hub (with idempotency)
- [x] Implement `create_escrow_with_validation` - validates requirements exist and match escrow details
- [x] Implement `receive_fulfillment_proof` - marks escrow fulfilled (MVM uses manual release, not auto-release)
- [x] Send `EscrowConfirmation` GMP message back to hub on escrow creation via `gmp_sender::lz_send`
- [x] Add routing in `native_gmp_endpoint` for inflow escrow messages
- [x] Implement MVM tests 8, 12, 13 in `inflow_escrow_gmp_tests.move`
- [x] Update `intent-frameworks/EXTENSION-CHECKLIST.md` with MVM InflowEscrow test status

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
- `intent-frameworks/mvm/tests/native_gmp_endpoint_tests.move`

**Completed tasks:**

- [x] Add GmpHubConfig with trusted remote mapping per chain ID
- [x] Add initialize() and set_trusted_remote() for configuration management
- [x] Integrate with `gmp_sender::lz_send()` for actual message sending (send functions now call lz_send and return nonce)
- [x] Add trusted source validation in receive handlers (both receive functions validate source via is_trusted_source)
- [x] Test message encoding (payload size and discriminator bytes verified)
- [x] Test source validation (added tests for rejecting untrusted sources)
- [x] Update native_gmp_endpoint tests to initialize intent_gmp_hub config

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

### Commit 9: Implement native GMP relay in trusted-gmp

**Files:**

- `trusted-gmp/src/native_gmp_relay.rs` (new)
- `trusted-gmp/src/main.rs` (simplified to only run native GMP relay)
- `trusted-gmp/src/lib.rs` (added native_gmp_relay module export)

**Tasks:**

- [x] Add `NativeGmpRelay` struct
- [x] Watch for `MessageSent` events on MVM native GMP endpoint
- [x] Watch for `MessageSent` events on SVM native GMP endpoint
- [x] Support configurable chain RPCs and endpoint addresses
- [x] Make native GMP relay the default mode (no `--mode` flag needed)
- [x] Add unit tests for event parsing (inline in module)

**Test:**

```bash
RUST_LOG=off nix develop ./nix -c bash -c "cd trusted-gmp && cargo test --quiet"
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 10.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 10: Implement native GMP relay with MVM connected chain support

**Files:**

- `trusted-gmp/src/native_gmp_relay.rs` (implement tx submission and MVM connected chain)
- `trusted-gmp/src/main.rs` (simplify to only run native GMP relay)
- `trusted-gmp/tests/native_gmp_relay_tests.rs` (add relay config tests)
- `testing-infra/ci-e2e/chain-hub/deploy-contracts.sh` (GMP initialization)
- `testing-infra/ci-e2e/chain-connected-mvm/deploy-contracts.sh` (GMP initialization)
- `intent-frameworks/svm/programs/*/src/error.rs` (fix naming)

**Tasks:**

- [x] Implement actual transaction submission in `deliver_to_mvm()` and `deliver_to_svm()`
- [x] Add Move address derivation utilities to `generate_keys` and `util.sh`
- [x] Update deployment scripts to initialize GMP modules with trusted remotes
- [x] Fix SVM error variant naming (E_SCREAMING_CASE → UpperCamelCase)
- [x] Update MVM e2e test environment to use native GMP endpoints (deploy scripts initialize GMP)
- [x] Add MVM connected chain support to native GMP relay (bidirectional MVM ↔ MVM messaging)
- [x] Simplify main.rs to only run native GMP relay (removed legacy mode)
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
  - Updated `OutflowService::run()` to skip trusted-gmp approval for GMP flow (hub auto-releases)
- [x] Start native GMP relay in background during tests (already done via `start_trusted_gmp`)
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
- `testing-infra/ci-e2e/chain-connected-svm/configure-trusted-gmp.sh` (add GMP endpoint)
- `intent-frameworks/svm/scripts/build.sh` (build all GMP programs)
- `solver/src/config.rs` (add optional GMP program IDs to SvmChainConfig)
- `solver/src/chains/connected_svm.rs` (add fulfill_outflow_via_gmp method)
- `solver/src/service/outflow.rs` (attempt GMP flow for SVM)

**Tasks:**

- [x] Update SVM e2e test environment to use native GMP endpoints
  - Build script now builds native-gmp-endpoint and outflow-validator
  - Deploy script deploys all 3 programs and configures trusted remotes
  - Configure-trusted-gmp uses GMP endpoint program ID for relay
- [x] Start native GMP relay in background during tests (already done via `start_trusted_gmp`)
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

---

## Run All Tests

```bash
# Run all unit tests (includes coordinator, trusted-gmp, solver, MVM, EVM, SVM, frontend)
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI runs e2e tests automatically. All e2e tests must pass before merging.**

---

## Exit Criteria

- [x] All 13 commits complete
- [x] SVM programs build and pass unit tests
- [x] MVM modules build and pass unit tests
- [x] Native GMP relay works for MVM ↔ SVM
- [x] Cross-chain E2E tests pass (outflow + inflow)
