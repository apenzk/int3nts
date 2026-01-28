# Phase 1: Research & Design (2-3 days)

**Status:** Not Started
**Depends On:** None
**Blocks:** Phase 2

**Goal:** Define the shared message format and interfaces that all chains will use. Research LZ integration for both Solana and Movement.

---

## Commits

> 📋 **Commit Conventions:** Before each commit, review `.claude/CLAUDE.md` and `.cursor/rules` for commit message format, test requirements, and coding standards.

### Commit 1: Design GMP integration into our architecture

**Files:**

- `docs/architecture/plan/gmp-architecture-integration.md`

**Tasks:**

- [x] **Message flow diagrams** - Document full flows for:
  - Outflow: Hub intent created → LZ send → connected chain receives → solver fulfills → LZ send → hub releases
  - Inflow: Hub intent created → LZ send → connected escrow created → LZ send → hub confirms → solver fulfills → LZ send → escrow releases
- [x] **Integration points** - Identify which existing contracts need GMP hooks:
  - MVM: `intent_as_escrow.move`, `fa_intent_outflow.move`, `fa_intent_inflow.move`
  - SVM: `intent_escrow` program (modify existing to add GMP support)
  - What triggers `lzSend()`? (contract logic on state change, not external caller)
- [x] **Trusted-GMP relay design** - How it works in local/CI:
  - Watches `MessageSent` events on local GMP endpoints
  - Calls `deliver_message()` / `lzReceive()` on destination chain
  - Needs funded operator wallet per chain
- [x] **Environment matrix** - What uses local vs LZ GMP endpoints:
  - Local/CI: Local GMP endpoints + Trusted-GMP relay
  - Testnet: LZ GMP endpoints everywhere
  - Mainnet: LZ GMP endpoints everywhere

**Test:**

```bash
# Documentation review - manual
```

> ⚠️ **Review complete before proceeding to Commit 2.**

---

### Commit 2: Research LZ integration for Solana and Movement

**Files:**

- `docs/architecture/plan/layerzero-solana-integration.md`
- `docs/architecture/plan/layerzero-movement-integration.md`

**Tasks:**

- [x] Research LZ's Solana integration (OApp pattern in native Rust)
- [x] Research LZ's Movement/Aptos integration (OApp pattern in Move)
- [x] Document endpoint addresses for Solana devnet/mainnet
- [x] Document endpoint addresses for Movement testnet/mainnet (or confirm LZ not yet available)
- [x] Document how message payloads are wrapped by LZ on each chain
- [x] Document nonce tracking differences between chains
- [x] Identify any chain-specific limitations or quirks

**Test:**

```bash
# Documentation review - manual
```

> ⚠️ **Review complete before proceeding to Commit 3.**

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

> ⚠️ **Review complete before proceeding to Commit 4.**

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
- [x] Define GMP endpoint addresses (LZ devnet, LZ mainnet, local)
- [x] Test encoding matches documented wire format exactly
- [x] Test decoding of known byte sequences

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI tests must pass before proceeding to Commit 5.**

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
- [x] Define GMP endpoint addresses (LZ testnet, LZ mainnet, local)
- [x] Test encoding matches documented wire format exactly
- [x] Test decoding of known byte sequences (same test vectors as SVM)

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI tests must pass before proceeding to Commit 6.**

---

### Commit 6: Add cross-chain encoding compatibility test

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

> ⚠️ **Both chains must produce identical encoding before proceeding to Commit 7.**

---

### Commit 7: Add outflow validator interface (SVM)

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

> ⚠️ **CI tests must pass before proceeding to Commit 8.**

---

### Commit 8: Add GMP support to intent_escrow (SVM)

**Files:**

- `intent-frameworks/svm/programs/intent-escrow/src/lib.rs` (modify existing)

**Tasks:**

- [ ] Add `lz_receive` instruction for receiving intent requirements
- [ ] Add on-chain validation in `create_escrow` against stored requirements
- [ ] Add `lz_receive` instruction for receiving fulfillment proof (auto-release)
- [ ] Remove signature verification in `claim`
- [ ] Add dependency on `gmp-common`

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI tests must pass before proceeding to Commit 9.**

---

### Commit 9: Add hub intent GMP interface (MVM)

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

> ⚠️ **CI tests must pass before proceeding to Commit 10.**

---

### Commit 10: Add local GMP endpoint interfaces (SVM + MVM)

**Files:**

- `intent-frameworks/svm/programs/local-gmp-endpoint/Cargo.toml`
- `intent-frameworks/svm/programs/local-gmp-endpoint/src/lib.rs` (interface only)
- `intent-frameworks/mvm/sources/gmp/local_gmp_endpoint.move` (interface only)

**Tasks:**

- [ ] SVM: Define `send` instruction signature (emits event)
- [ ] SVM: Define `deliver_message` instruction for trusted-GMP relay
- [ ] SVM: Add stub implementations
- [ ] MVM: Define `lz_send()` function signature
- [ ] MVM: Define `deliver_message()` entry function for trusted-GMP relay
- [ ] MVM: Add stub implementations

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI tests must pass before proceeding to Commit 11.**

---

### Commit 11: Add fee estimation and document endpoint configuration

**Files:**

- `docs/architecture/plan/gmp-endpoints.md`
- `docs/architecture/plan/gmp-fee-analysis.md`

**Tasks:**

- [ ] Document all GMP endpoint addresses (LZ for Solana and Movement, local for testing)
- [ ] Document environment configuration (local/CI uses local GMP endpoints, testnet and mainnet use LZ GMP endpoints)
- [ ] Estimate LZ message fees for each route
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

- [ ] All 11 commits merged to feature branch
- [ ] GMP architecture integration design reviewed
- [ ] Wire format spec documented and reviewed
- [ ] SVM message encoding matches spec (tested)
- [ ] MVM message encoding matches spec (tested)
- [ ] Cross-chain encoding test passes (both produce identical bytes)
- [ ] All interfaces defined for SVM and MVM
- [ ] Local GMP endpoint interfaces defined for both chains
- [ ] LZ research documented for both Solana and Movement
- [ ] Fee analysis complete
