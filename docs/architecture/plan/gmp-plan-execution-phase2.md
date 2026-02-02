# Phase 2: SVM + MVM Core Implementation (2-3 days)

**Status:** Not Started
**Depends On:** Phase 1
**Blocks:** Phase 3

**Goal:** Build GMP support for both SVM (connected chain) and MVM (hub) together so we can test real cross-chain messaging from the start.

---

## Commits

> 📋 **Commit Conventions:** Before each commit, review `.claude/CLAUDE.md` and `.cursor/rules` for commit message format, test requirements, and coding standards.

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

> ⚠️ **CI e2e tests must pass before proceeding to Commit 2.**

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

> ⚠️ **CI e2e tests must pass before proceeding to Commit 3.**

---

### Commit 3: Implement OutflowValidator program (SVM)

**Files:**

- `intent-frameworks/svm/programs/outflow-validator/src/lib.rs` (already exists from Phase 1, implement)
- `intent-frameworks/svm/programs/outflow-validator/tests/validator_tests.rs`

**Tasks:**

- [ ] Implement GMP receive handler for native GMP endpoint
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

- [ ] Implement GMP receive handler for native GMP endpoint
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

### Commit 6: Implement native GMP relay in trusted-gmp

**Files:**

- `trusted-gmp/src/native_gmp_relay.rs`
- `trusted-gmp/src/main.rs`
- `trusted-gmp/tests/relay_tests.rs`

**Tasks:**

- [ ] Add `NativeGmpRelay` struct
- [ ] Watch for `MessageSent` events on MVM and SVM native GMP endpoints
- [ ] Deliver messages by calling `deliver_message` on destination chain
- [ ] Support configurable chain RPCs and endpoint addresses
- [ ] Integrate into trusted-gmp binary as `--mode native-gmp-relay`
- [ ] Test event parsing and message delivery

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 7.**

---

### Commit 7: Fix existing E2E tests for GMP: MVM ↔ MVM

**Files:**

- `testing-infra/ci-e2e/e2e-tests-mvm/` (update existing)

**Tasks:**

- [ ] Update MVM e2e test environment to use native GMP endpoints
- [ ] Start native GMP relay in background during tests
- [ ] Update `run-tests-outflow.sh` to use GMP flow
- [ ] Update `run-tests-inflow.sh` to use GMP flow
- [ ] Verify GMP messages are sent and received correctly (MVM hub ↔ MVM connected)
- [ ] Ensure existing test assertions still pass

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh

# Run MVM e2e tests
./testing-infra/ci-e2e/e2e-tests-mvm/run-tests-outflow.sh
./testing-infra/ci-e2e/e2e-tests-mvm/run-tests-inflow.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 8.**

---

### Commit 8: Fix existing E2E tests for GMP: MVM ↔ SVM outflow

**Files:**

- `testing-infra/ci-e2e/e2e-tests-svm/` (update existing)

**Tasks:**

- [ ] Update SVM e2e test environment to use native GMP endpoints
- [ ] Start native GMP relay in background during tests
- [ ] Update outflow test to use GMP flow (solver calls validation contract)
- [ ] Verify GMP messages are sent and received correctly
- [ ] Ensure existing test assertions still pass

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh

# Run SVM e2e tests
./testing-infra/ci-e2e/e2e-tests-svm/run-tests-outflow.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 9.**

---

### Commit 9: Fix existing E2E tests for GMP: MVM ↔ SVM inflow

**Files:**

- `testing-infra/ci-e2e/e2e-tests-svm/` (update existing)

**Tasks:**

- [ ] Update inflow test to use GMP flow (escrow receives requirements via GMP)
- [ ] Verify escrow confirmation GMP message sent back to MVM
- [ ] Verify fulfillment proof GMP message sent to SVM
- [ ] Verify escrow auto-releases on fulfillment proof receipt
- [ ] Ensure existing test assertions still pass

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh

# Run SVM e2e tests
./testing-infra/ci-e2e/e2e-tests-svm/run-tests-inflow.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 10.**

---

### Commit 10: Update existing deployment scripts for GMP

**Files:**

- `intent-frameworks/svm/scripts/` (update existing deployment scripts)
- `intent-frameworks/mvm/scripts/` (update existing deployment scripts)

**Tasks:**

- [ ] Update SVM deployment scripts to include GMP programs (OutflowValidator, InflowEscrowGMP)
- [ ] Update MVM deployment scripts to include GMP modules
- [ ] Add trusted remote configuration to deployment scripts
- [ ] Deploy updated contracts/modules to testnets
- [ ] Verify cross-chain flow works on testnets (with native GMP relay)

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

- [ ] All 10 commits merged to feature branch
- [ ] SVM programs build and pass unit tests
- [ ] MVM modules build and pass unit tests
- [ ] Native GMP relay works for MVM ↔ SVM
- [ ] Cross-chain E2E tests pass (outflow + inflow)
- [ ] Programs/modules deployed to testnets
- [ ] Documentation updated
