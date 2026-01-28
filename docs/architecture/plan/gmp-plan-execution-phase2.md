# Phase 2: SVM + MVM Core Implementation (5-7 days)

**Status:** Not Started
**Depends On:** Phase 1
**Blocks:** Phase 3

**Goal:** Build GMP support for both SVM (connected chain) and MVM (hub) together so we can test real cross-chain messaging from the start.

---

## Commits

> 📋 **Commit Conventions:** Before each commit, review `.claude/CLAUDE.md` and `.cursor/rules` for commit message format, test requirements, and coding standards.

### Commit 1: Implement MockLayerZeroEndpoint for Solana

**Files:**

- `intent-frameworks/svm/programs/mock-lz-endpoint/src/lib.rs`
- `intent-frameworks/svm/programs/mock-lz-endpoint/tests/mock_tests.rs`

**Tasks:**

- [ ] Implement `send` instruction that emits `MessageSent` event (no actual cross-chain)
- [ ] Implement `deliver_message` instruction for simulator to inject messages
- [ ] Implement trusted remote verification via PDA
- [ ] Track message nonces for realistic behavior
- [ ] Test `send` emits correct event with payload
- [ ] Test `deliver_message` calls receiver's `lz_receive`
- [ ] Test nonce tracking works correctly

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 2.**

---

### Commit 2: Add LayerZero OApp base for Movement (MVM)

**Files:**

- `intent-frameworks/mvm/sources/layerzero/oapp.move`
- `intent-frameworks/mvm/sources/layerzero/endpoint.move`
- `intent-frameworks/mvm/sources/mocks/mock_lz_endpoint.move`
- `intent-frameworks/mvm/tests/layerzero_tests.move`

**Tasks:**

- [ ] Port LayerZero OApp pattern to Move
- [ ] Implement `lz_receive()` entry function
- [ ] Implement `lz_send()` internal function
- [ ] Implement trusted remote verification
- [ ] Implement mock endpoint for testing
- [ ] Test send/receive with mock endpoint

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 3.**

---

### Commit 3: Implement OutflowValidator program (SVM)

**Files:**

- `intent-frameworks/svm/programs/outflow-validator/src/lib.rs`
- `intent-frameworks/svm/programs/outflow-validator/tests/validator_tests.rs`

**Tasks:**

- [ ] Implement LayerZero OApp pattern in native Solana Rust
- [ ] Implement `lz_receive` to receive intent requirements from hub
- [ ] **Idempotency check**: Before storing, check if requirements already exist for intent_id + step number
- [ ] **If requirements already exist → ignore duplicate message (idempotent)**
- [ ] **If requirements don't exist → store intent requirements** in PDA (intent_id/step => {requirements, authorizedSolver})
- [ ] Implement `fulfill_intent` instruction for authorized solvers to call
- [ ] Instruction pulls tokens from authorized solver's wallet via SPL token transfer
- [ ] Validate recipient, amount, token match stored requirements
- [ ] Validate solver matches authorized solver from stored requirements
- [ ] Forward tokens to user wallet
- [ ] Send GMP message to hub via `lz_send`
- [ ] Emit `FulfillmentSucceeded` or `FulfillmentFailed` events
- [ ] Test all validation scenarios

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 4.**

---

### Commit 4: Implement InflowEscrowGMP program (SVM)

**Files:**

- `intent-frameworks/svm/programs/escrow-gmp/src/lib.rs`
- `intent-frameworks/svm/programs/escrow-gmp/tests/escrow_tests.rs`

**Tasks:**

- [ ] Implement LayerZero OApp pattern in native Solana Rust
- [ ] Implement `lz_receive` for intent requirements from hub
- [ ] **Idempotency check**: Before storing, check if requirements already exist for intent_id + step number
- [ ] **If requirements already exist → ignore duplicate message (idempotent)**
- [ ] **If requirements don't exist → store requirements** (mapped by intent_id + step number)
- [ ] Implement `create_escrow_with_validation` - validates requirements exist and match escrow details
- [ ] Implement `lz_receive` for fulfillment proof from hub
- [ ] Implement automatic escrow release on fulfillment proof receipt
- [ ] Send `EscrowConfirmation` message back to hub on creation
- [ ] Test all escrow scenarios

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 5.**

---

### Commit 5: Integrate GMP into MVM hub intent contract

**Files:**

- `intent-frameworks/mvm/sources/intent_gmp.move`
- `intent-frameworks/mvm/tests/intent_gmp_tests.move`

**Tasks:**

- [ ] Add `send_intent_requirements()` - calls `lz_send()` on intent creation
- [ ] Add `send_fulfillment_proof()` - calls `lz_send()` on fulfillment
- [ ] Add `receive_escrow_confirmation()` - called by `lz_receive()`
- [ ] Gate fulfillment on escrow confirmation receipt (for inflow)
- [ ] Add `receive_fulfillment_proof()` - called by `lz_receive()` for outflow completion
- [ ] Test message encoding matches SVM schema
- [ ] Test fulfillment blocked without escrow confirmation
- [ ] Test state updates on GMP message receipt

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 6.**

---

### Commit 6: Implement LayerZero simulator in trusted-gmp

**Files:**

- `trusted-gmp/src/layerzero_simulator.rs`
- `trusted-gmp/src/main.rs`
- `trusted-gmp/tests/simulator_tests.rs`

**Tasks:**

- [ ] Add `LayerZeroSimulator` struct
- [ ] Watch for `MessageSent` events on MVM and SVM
- [ ] Deliver messages by calling `lzReceive` / `deliver_message`
- [ ] Support configurable chain RPCs and mock endpoints
- [ ] Integrate into trusted-gmp binary as `--mode simulator`
- [ ] Test event parsing and message delivery

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 7.**

---

### Commit 7: Add cross-chain E2E test: MVM ↔ SVM outflow

**Files:**

- `testing-infra/ci-e2e/e2e-tests-gmp/mvm-svm-outflow.sh`
- `testing-infra/ci-e2e/e2e-tests-gmp/test-helpers.sh`

**Tasks:**

- [ ] Set up test environment with mock endpoints on both chains
- [ ] Start LayerZero simulator in background
- [ ] Create intent on MVM hub
- [ ] Verify requirements message sent to SVM
- [ ] Solver validates on SVM
- [ ] Verify success message sent back to MVM
- [ ] Verify intent completes on MVM

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh

# Run GMP e2e test
./testing-infra/ci-e2e/e2e-tests-gmp/mvm-svm-outflow.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 8.**

---

### Commit 8: Add cross-chain E2E test: MVM ↔ SVM inflow

**Files:**

- `testing-infra/ci-e2e/e2e-tests-gmp/mvm-svm-inflow.sh`

**Tasks:**

- [ ] Create intent on MVM hub (inflow type)
- [ ] Verify requirements message sent to SVM
- [ ] Requester creates escrow on SVM
- [ ] Verify escrow confirmation sent back to MVM
- [ ] Solver fulfills on MVM hub
- [ ] Verify fulfillment proof sent to SVM
- [ ] Verify escrow releases on SVM

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh

# Run GMP e2e test
./testing-infra/ci-e2e/e2e-tests-gmp/mvm-svm-inflow.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 9.**

---

### Commit 9: Add deployment scripts and deploy to testnets

**Files:**

- `intent-frameworks/svm/scripts/deploy-outflow-validator.sh`
- `intent-frameworks/svm/scripts/deploy-escrow-gmp.sh`
- `intent-frameworks/svm/scripts/configure-trusted-remotes.sh`
- `docs/architecture/plan/gmp-testnet-deployment.md`

**Tasks:**

- [ ] Script to deploy OutflowValidator to Solana devnet
- [ ] Script to deploy InflowEscrowGMP to Solana devnet
- [ ] Script to configure trusted remotes (hub address via PDA)
- [ ] Deploy MVM GMP modules to Movement testnet
- [ ] Configure trusted remotes on MVM
- [ ] Document deployed program/module addresses
- [ ] Verify cross-chain flow works on testnets (with simulator)

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh

# Verify deployments
solana program show <OUTFLOW_VALIDATOR_PROGRAM_ID> --url devnet
```

> ⚠️ **CI e2e tests must pass before Phase 2 is complete.**

---

## Run All Tests

```bash
# Run all unit tests (includes coordinator, trusted-gmp, solver, MVM, EVM, SVM, frontend)
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI runs e2e tests automatically. All e2e tests must pass before merging.**

---

## Exit Criteria

- [ ] All 9 commits merged to feature branch
- [ ] SVM programs build and pass unit tests
- [ ] MVM modules build and pass unit tests
- [ ] LayerZero simulator works for MVM ↔ SVM
- [ ] Cross-chain E2E tests pass (outflow + inflow)
- [ ] Programs/modules deployed to testnets
- [ ] Documentation updated
