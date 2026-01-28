# Phase 1: Research & Design (2-3 days)

**Status:** Not Started
**Depends On:** None
**Blocks:** Phase 2

**Goal:** Define the shared message format and interfaces that all chains will use. Research LayerZero integration for both Solana and Movement.

---

## Commits

> 📋 **Commit Conventions:** Before each commit, review `.claude/CLAUDE.md` and `.cursor/rules` for commit message format, test requirements, and coding standards.

### Commit 1: Research LayerZero integration for Solana and Movement

**Files:**

- `docs/architecture/plan/layerzero-solana-integration.md`
- `docs/architecture/plan/layerzero-movement-integration.md`

**Tasks:**

- [ ] Research LayerZero's Solana integration (OApp pattern in native Rust)
- [ ] Research LayerZero's Movement/Aptos integration (OApp pattern in Move)
- [ ] Document endpoint addresses for Solana devnet/mainnet
- [ ] Document endpoint addresses for Movement testnet/mainnet (or confirm LZ not yet available)
- [ ] Document how message payloads are wrapped by LayerZero on each chain
- [ ] Document nonce tracking differences between chains
- [ ] Identify any chain-specific limitations or quirks

**Test:**

```bash
# Documentation review - manual
```

> ⚠️ **Review complete before proceeding to Commit 2.**

---

### Commit 2: Define GMP message wire format specification

**Files:**

- `docs/architecture/plan/gmp-message-spec.md`

**Tasks:**

- [ ] Define wire format for `IntentRequirements` message (hub → connected chain)
  - Fields: message_type, intent_id, recipient, amount, token, authorized_solver, expiry
  - Encoding: fixed-width fields, big-endian integers, 32-byte addresses
- [ ] Define wire format for `EscrowConfirmation` message (connected chain → hub)
  - Fields: message_type, intent_id, escrow_id, amount, token, creator
- [ ] Define wire format for `FulfillmentProof` message (hub → connected chain, or connected → hub)
  - Fields: message_type, intent_id, solver, amount, timestamp
- [ ] Document byte layout for each message type
- [ ] Document message_type discriminator bytes
- [ ] Explain why this format was chosen (simplicity, no dependencies, easy to implement in Move/Rust/Solidity)

**Test:**

```bash
# Documentation review - manual
```

> ⚠️ **Review complete before proceeding to Commit 3.**

---

### Commit 3: Add gmp-common crate with message encoding (SVM)

**Files:**

- `intent-frameworks/svm/programs/gmp-common/Cargo.toml`
- `intent-frameworks/svm/programs/gmp-common/src/lib.rs`
- `intent-frameworks/svm/programs/gmp-common/src/messages.rs`
- `intent-frameworks/svm/programs/gmp-common/src/endpoints.rs`
- `intent-frameworks/svm/programs/gmp-common/tests/message_tests.rs`

**Tasks:**

- [ ] Create `gmp-common` library crate
- [ ] Implement `IntentRequirements` encode/decode per wire format spec
- [ ] Implement `EscrowConfirmation` encode/decode per wire format spec
- [ ] Implement `FulfillmentProof` encode/decode per wire format spec
- [ ] Define LayerZero endpoint addresses (devnet, mainnet, mock)
- [ ] Test encoding matches documented wire format exactly
- [ ] Test decoding of known byte sequences

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI tests must pass before proceeding to Commit 4.**

---

### Commit 4: Add gmp-common module with message encoding (MVM)

**Files:**

- `intent-frameworks/mvm/sources/gmp_common/messages.move`
- `intent-frameworks/mvm/sources/gmp_common/endpoints.move`
- `intent-frameworks/mvm/tests/gmp_common_tests.move`

**Tasks:**

- [ ] Create `gmp_common` module
- [ ] Implement `IntentRequirements` encode/decode per wire format spec
- [ ] Implement `EscrowConfirmation` encode/decode per wire format spec
- [ ] Implement `FulfillmentProof` encode/decode per wire format spec
- [ ] Define LayerZero endpoint addresses (testnet, mainnet, mock)
- [ ] Test encoding matches documented wire format exactly
- [ ] Test decoding of known byte sequences (same test vectors as SVM)

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI tests must pass before proceeding to Commit 5.**

---

### Commit 5: Add cross-chain encoding compatibility test

**Files:**

- `testing-infra/gmp-encoding-test/test-vectors.json`
- `testing-infra/gmp-encoding-test/verify-svm.sh`
- `testing-infra/gmp-encoding-test/verify-mvm.sh`

**Tasks:**

- [ ] Create test vectors JSON with known inputs and expected byte outputs
- [ ] Script to run SVM encoding and compare to expected bytes
- [ ] Script to run MVM encoding and compare to expected bytes
- [ ] Verify both chains produce identical bytes for same logical message
- [ ] Add to CI pipeline

**Test:**

```bash
./testing-infra/gmp-encoding-test/verify-svm.sh
./testing-infra/gmp-encoding-test/verify-mvm.sh
```

> ⚠️ **Both chains must produce identical encoding before proceeding to Commit 6.**

---

### Commit 6: Add outflow validator interface (SVM)

**Files:**

- `intent-frameworks/svm/programs/outflow-validator/Cargo.toml`
- `intent-frameworks/svm/programs/outflow-validator/src/lib.rs` (interface only - stub implementations)

**Tasks:**

- [ ] Create Cargo.toml with dependencies on `gmp-common`, `solana-program`
- [ ] Define `lz_receive` instruction for receiving intent requirements
- [ ] Define `fulfill_intent` instruction for authorized solvers
- [ ] Define `FulfillmentSucceeded`, `FulfillmentFailed` events
- [ ] Add stub implementations that return `Ok(())`

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI tests must pass before proceeding to Commit 7.**

---

### Commit 7: Add inflow escrow GMP interface (SVM)

**Files:**

- `intent-frameworks/svm/programs/escrow-gmp/Cargo.toml`
- `intent-frameworks/svm/programs/escrow-gmp/src/lib.rs` (interface only - stub implementations)

**Tasks:**

- [ ] Create Cargo.toml with dependencies on `gmp-common`, `solana-program`
- [ ] Define `lz_receive` instruction for intent requirements
- [ ] Define `create_escrow_with_validation` instruction
- [ ] Define `lz_receive` instruction for fulfillment proof
- [ ] Add stub implementations that return `Ok(())`

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI tests must pass before proceeding to Commit 8.**

---

### Commit 8: Add hub intent GMP interface (MVM)

**Files:**

- `intent-frameworks/mvm/sources/interfaces/intent_gmp.move`

**Tasks:**

- [ ] Define `send_intent_requirements()` function signature (GMP outbound)
- [ ] Define `receive_escrow_confirmation()` function signature (GMP inbound)
- [ ] Define `send_fulfillment_proof()` function signature (GMP outbound)
- [ ] Define `receive_fulfillment_proof()` function signature (GMP inbound)
- [ ] Add stub implementations

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI tests must pass before proceeding to Commit 9.**

---

### Commit 9: Add mock LayerZero endpoint interfaces (SVM + MVM)

**Files:**

- `intent-frameworks/svm/programs/mock-lz-endpoint/Cargo.toml`
- `intent-frameworks/svm/programs/mock-lz-endpoint/src/lib.rs` (interface only)
- `intent-frameworks/mvm/sources/mocks/mock_lz_endpoint.move` (interface only)

**Tasks:**

- [ ] SVM: Define `send` instruction signature (emits event)
- [ ] SVM: Define `deliver_message` instruction for simulator
- [ ] SVM: Add stub implementations
- [ ] MVM: Define `lz_send()` function signature
- [ ] MVM: Define `deliver_message()` entry function for simulator
- [ ] MVM: Add stub implementations

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI tests must pass before proceeding to Commit 10.**

---

### Commit 10: Add fee estimation and document endpoint configuration

**Files:**

- `docs/architecture/plan/gmp-endpoints.md`
- `docs/architecture/plan/gmp-fee-analysis.md`

**Tasks:**

- [ ] Document all LayerZero endpoint addresses (Solana, Movement, mock)
- [ ] Document environment configuration (local/CI uses mock, testnet uses mock for Movement + real for Solana, mainnet uses real everywhere)
- [ ] Estimate LayerZero message fees for each route
- [ ] Estimate on-chain validation gas costs
- [ ] Compare costs to current Trusted GMP system

**Test:**

```bash
# Documentation review - manual
```

> ⚠️ **Documentation complete before Phase 1 is complete.**

---

## Run All Tests

```bash
./testing-infra/run-all-unit-tests.sh
```

---

## Exit Criteria

- [ ] All 10 commits merged to feature branch
- [ ] Wire format spec documented and reviewed
- [ ] SVM message encoding matches spec (tested)
- [ ] MVM message encoding matches spec (tested)
- [ ] Cross-chain encoding test passes (both produce identical bytes)
- [ ] All interfaces defined for SVM and MVM
- [ ] Mock endpoint interfaces defined for both chains
- [ ] LayerZero research documented for both Solana and Movement
- [ ] Fee analysis complete
