# Phase 1: Research & Design (1-2 days)

**Status:** Complete (Commits 1-10)
**Depends On:** None
**Blocks:** Phase 2

**Goal:** Define the shared message format and interfaces that all chains will use. Research LZ v2 patterns as design reference for our integrated GMP interfaces.

---

## Commits

> 📋 **Commit Conventions:** Before each commit, review `.claude/CLAUDE.md` and `.cursor/rules` for commit message format, test requirements, and coding standards.

### Commit 1: Design GMP integration into our architecture

**Files:**

- `docs/architecture/plan/gmp-architecture-integration.md`

**Tasks:**

- [x] **Message flow diagrams** - Document full flows for:
  - Outflow: Hub intent created → GMP send → connected chain receives → solver fulfills → GMP send → hub releases
  - Inflow: Hub intent created → GMP send → connected escrow created → GMP send → hub confirms → solver fulfills → GMP send → escrow releases
- [x] **Integration points** - Identify which existing contracts need GMP hooks:
  - MVM: `intent_as_escrow.move`, `fa_intent_outflow.move`, `fa_intent_inflow.move`
  - SVM: `intent_escrow` program (modify existing to add GMP support)
  - What triggers `gmpSend()`? (contract logic on state change, not external caller)
- [x] **Integrated-GMP relay design** - How it works in local/CI:
  - Watches `MessageSent` events on integrated GMP endpoints
  - Calls `deliver_message()` / `gmpReceive()` on destination chain
  - Needs funded operator wallet per chain
- [x] **Environment matrix** - All environments use integrated GMP:
  - Local/CI: Integrated GMP endpoints + Integrated GMP relay
  - Testnet: Integrated GMP endpoints + Integrated GMP relay
  - Mainnet: Integrated GMP endpoints + Integrated GMP relay

**Test:**

```bash
# Documentation review - manual
```

> ⚠️ **Review complete before proceeding to Commit 2.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 2: Research LZ v2 patterns as design reference for integrated GMP interfaces

**Files:**

- `docs/architecture/plan/lz-svm-integration.md`
- `docs/architecture/plan/lz-mvm-integration.md`

**Tasks:**

- [x] Research LZ's Solana OApp pattern as reference for our SVM integrated GMP endpoint
- [x] Research LZ's Movement/Aptos OApp pattern as reference for our MVM integrated GMP endpoint
- [x] Document LZ endpoint addresses for reference (Solana devnet/mainnet)
- [x] Document LZ availability for Movement (not yet available — confirmed our integrated GMP approach)
- [x] Document how LZ wraps message payloads (informed our wire format design)
- [x] Document nonce tracking differences between chains
- [x] Identify chain-specific limitations or quirks

**Test:**

```bash
# Documentation review - manual
```

> ⚠️ **Review complete before proceeding to Commit 3.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 3: Define GMP message wire format specification

**Files:**

- `docs/architecture/plan/gmp-message-spec.md`

**Tasks:**

- [x] Define wire format for `IntentRequirements` message (hub → connected chain)
  - Fields: message_type, intent_id, recipient, amount, token, authorized_solver, expiry
  - Encoding: fixed-width fields, big-endian integers, 32-byte addresses
- [x] Define wire format for `EscrowConfirmation` message (connected chain → hub)
  - Fields: message_type, intent_id, escrow_id, amount, token, creator
- [x] Define wire format for `FulfillmentProof` message (hub → connected chain, or connected → hub)
  - Fields: message_type, intent_id, solver, amount, timestamp
- [x] Document byte layout for each message type
- [x] Document message_type discriminator bytes
- [x] Explain why this format was chosen (simplicity, no dependencies, easy to implement in Move/Rust/Solidity)

**Test:**

```bash
# Documentation review - manual
```

> ⚠️ **Review complete before proceeding to Commit 4.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 4: Add gmp-common crate with message encoding (SVM)

**Files:**

- `intent-frameworks/svm/programs/gmp-common/Cargo.toml`
- `intent-frameworks/svm/programs/gmp-common/src/lib.rs`
- `intent-frameworks/svm/programs/gmp-common/src/messages.rs`
- `intent-frameworks/svm/programs/gmp-common/src/endpoints.rs`
- `intent-frameworks/svm/programs/gmp-common/tests/message_tests.rs`

**Tasks:**

- [x] Create `gmp-common` library crate
- [x] Implement `IntentRequirements` encode/decode per wire format spec
- [x] Implement `EscrowConfirmation` encode/decode per wire format spec
- [x] Implement `FulfillmentProof` encode/decode per wire format spec
- [x] Define GMP endpoint addresses (integrated GMP endpoints per environment)
- [x] Test encoding matches documented wire format exactly
- [x] Test decoding of known byte sequences

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI tests must pass before proceeding to Commit 5.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 5: Add gmp-common module with message encoding (MVM)

**Files:**

- `intent-frameworks/mvm/sources/gmp_common/messages.move`
- `intent-frameworks/mvm/sources/gmp_common/endpoints.move`
- `intent-frameworks/mvm/tests/gmp_common_tests.move`

**Tasks:**

- [x] Create `gmp_common` module
- [x] Implement `IntentRequirements` encode/decode per wire format spec
- [x] Implement `EscrowConfirmation` encode/decode per wire format spec
- [x] Implement `FulfillmentProof` encode/decode per wire format spec
- [x] Define GMP endpoint addresses (integrated GMP endpoints per environment)
- [x] Test encoding matches documented wire format exactly
- [x] Test decoding of known byte sequences (same test vectors as SVM)

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI tests must pass before proceeding to Commit 6.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 6: Add cross-chain encoding compatibility test

**Files:**

- `intent-frameworks/common/testing/gmp-encoding-test-vectors.json`
- `intent-frameworks/svm/programs/gmp-common/tests/gmp_common_tests.rs` (tests 36-40)
- `intent-frameworks/mvm/tests/gmp_common_tests.move` (tests 36-40)

**Tasks:**

- [x] Create test vectors JSON with known inputs and expected byte outputs
- [x] Add SVM unit tests (36-40) that verify encoding matches expected bytes
- [x] Add MVM unit tests (36-40) that verify encoding matches expected bytes
- [x] Verify both chains produce identical bytes for same logical message (via shared test vectors)
- [x] Tests run as part of CI pipeline (`./testing-infra/run-all-unit-tests.sh`)

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **Both chains must produce identical encoding before proceeding to Commit 7.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 7: Add outflow validator interface (SVM)

**Files:**

- `intent-frameworks/svm/programs/outflow-validator/Cargo.toml`
- `intent-frameworks/svm/programs/outflow-validator/src/lib.rs`
- `intent-frameworks/svm/programs/outflow-validator/src/instruction.rs`
- `intent-frameworks/svm/programs/outflow-validator/src/processor.rs`
- `intent-frameworks/svm/programs/outflow-validator/src/state.rs`
- `intent-frameworks/svm/programs/outflow-validator/src/error.rs`
- `intent-frameworks/svm/programs/outflow-validator/src/events.rs`
- `intent-frameworks/svm/programs/outflow-validator/src/entrypoint.rs`

**Tasks:**

- [x] Create Cargo.toml with dependencies on `gmp-common`, `solana-program`
- [x] Define `gmp_receive` instruction for receiving intent requirements
- [x] Define `fulfill_intent` instruction for authorized solvers
- [x] Define `FulfillmentSucceeded`, `FulfillmentFailed` events
- [x] Add stub implementations that return `Ok(())`

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI tests must pass before proceeding to Commit 8.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 8: Add GMP support to intent_escrow (SVM)

**Files:**

- `intent-frameworks/svm/programs/intent_escrow/src/lib.rs` (modify existing)

**Tasks:**

- [x] Add `gmp_receive` instruction for receiving intent requirements
- [x] Add on-chain validation in `create_escrow` against stored requirements
- [x] Add `gmp_receive` instruction for receiving fulfillment proof (auto-release)
- [x] Remove signature verification in `claim`
- [x] Add dependency on `gmp-common`

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI tests must pass before proceeding to Commit 9.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 9: Add intent GMP interface (MVM)

**Files:**

- `intent-frameworks/mvm/sources/interfaces/intent_gmp_hub.move`
- `intent-frameworks/mvm/sources/interfaces/outflow_validator.move`
- `intent-frameworks/mvm/tests/intent_gmp_tests.move`
- `intent-frameworks/mvm/tests/interface_tests.move`

**Tasks:**

Hub functions (MVM as hub):

- [x] Define `send_intent_requirements()` function signature (GMP outbound)
- [x] Define `receive_escrow_confirmation()` function signature (GMP inbound)
- [x] Define `send_fulfillment_proof()` function signature (GMP outbound)
- [x] Define `receive_fulfillment_proof()` function signature (GMP inbound)

Connected chain functions (MVM as connected chain):

- [x] Define `receive_intent_requirements()` function signature (GMP inbound, mirrors SVM outflow-validator)
- [x] Add stub implementations
- [x] Add unit tests for all functions

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI tests must pass before proceeding to Commit 10.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 10: Add integrated GMP endpoint interfaces (SVM + MVM)

**Files:**

- `intent-frameworks/svm/programs/integrated-gmp-endpoint/Cargo.toml`
- `intent-frameworks/svm/programs/integrated-gmp-endpoint/src/lib.rs` (interface only)
- `intent-frameworks/mvm/sources/gmp/intent_gmp.move` (interface only)

**Tasks:**

- [x] SVM: Define `send` instruction signature (emits event)
- [x] SVM: Define `deliver_message` instruction for integrated-gmp relay
- [x] SVM: Add stub implementations
- [x] MVM: Define `gmp_send()` function signature
- [x] MVM: Define `deliver_message()` entry function for integrated-gmp relay
- [x] MVM: Add stub implementations

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **Phase 1 complete after Commit 10.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### ~~Commit 11: Add fee estimation and document endpoint configuration~~ (Deferred)

> **Moved to Phase 4, Commit 5.** Fee estimation deferred. All environments use integrated GMP endpoints (no third-party GMP fees). Commit 5 covers integrated GMP relay configuration and operational costs.

---

## Run All Tests

```bash
./testing-infra/run-all-unit-tests.sh
```

---

## Exit Criteria

- [x] All 10 commits merged to feature branch
- [x] GMP architecture integration design reviewed
- [x] Wire format spec documented and reviewed
- [x] SVM message encoding matches spec (tested)
- [x] MVM message encoding matches spec (tested)
- [x] Cross-chain encoding test passes (both produce identical bytes)
- [x] All interfaces defined for SVM and MVM
- [x] Integrated GMP endpoint interfaces defined for both chains
- [x] LZ v2 patterns researched as design reference for both Solana and Movement
- ~~[ ] Fee analysis complete~~ (Moved to Phase 4, Commit 5)
