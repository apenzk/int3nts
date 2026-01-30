# Phase 3: EVM Expansion (1-2 days)

**Status:** Not Started
**Depends On:** Phase 2
**Blocks:** Phase 4

**Goal:** Add EVM connected chain support to the GMP system. After this phase, all three chain types (MVM hub, SVM, EVM) can communicate via GMP.

---

## Commits

> 📋 **Commit Conventions:** Before each commit, review `.claude/CLAUDE.md` and `.cursor/rules` for commit message format, test requirements, and coding standards.

### Commit 1: Implement native GMP endpoint for EVM

**Files:**

- `intent-frameworks/evm/contracts/gmp/NativeGmpEndpoint.sol`
- `intent-frameworks/evm/test/NativeGmpEndpoint.test.js`

**Tasks:**

- [ ] Implement `send()` function that emits `MessageSent` event
- [ ] Implement `deliverMessage()` for relay to inject messages
- [ ] Implement trusted remote verification
- [ ] Add relay authorization checks
- [ ] Track message nonces for replay protection
- [ ] Test send emits correct event
- [ ] Test deliverMessage calls receiver's receive handler

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 2.**

---

### Commit 2: Implement OutflowValidator contract (EVM)

**Files:**

- `intent-frameworks/evm/contracts/OutflowValidator.sol`
- `intent-frameworks/evm/test/OutflowValidator.test.js`

**Tasks:**

- [ ] Implement GMP receive handler interface
- [ ] Implement `receiveMessage()` to receive intent requirements from hub
- [ ] **Idempotency check**: Before storing, check if requirements already exist for intent_id + step number
- [ ] **If requirements already exist → ignore duplicate message (idempotent)**
- [ ] **If requirements don't exist → store intent requirements** in mapping
- [ ] Implement `fulfillIntent(intent_id, token, amount)` for authorized solvers to call
- [ ] Pull tokens from authorized solver's wallet via `transferFrom()` (requires prior approval)
- [ ] Validate recipient, amount, token match stored requirements
- [ ] Validate solver matches authorized solver from stored requirements
- [ ] Forward tokens to user wallet
- [ ] Send GMP message to hub via native GMP endpoint
- [ ] Test all validation scenarios
- [ ] Test `transferFrom()` fails without approval
- [ ] Test fulfillment fails with unauthorized solver

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 3.**

---

### Commit 3: Implement InflowEscrowGMP contract (EVM)

**Files:**

- `intent-frameworks/evm/contracts/InflowEscrowGMP.sol`
- `intent-frameworks/evm/test/InflowEscrowGMP.test.js`

**Tasks:**

- [ ] Implement GMP receive handler interface
- [ ] Implement `receiveMessage()` for intent requirements from hub
- [ ] **Idempotency check**: Before storing, check if requirements already exist for intent_id + step number
- [ ] **If requirements already exist → ignore duplicate message (idempotent)**
- [ ] **If requirements don't exist → store requirements**
- [ ] Implement `createEscrowWithValidation()` - validates requirements exist and match escrow details
- [ ] Implement automatic escrow release on fulfillment proof receipt
- [ ] Send `EscrowConfirmation` message back to hub on creation
- [ ] Test all escrow scenarios
- [ ] Test escrow creation reverts if requirements don't exist
- [ ] Test escrow creation reverts if requirements don't match

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 4.**

---

### Commit 4: Update native GMP relay to support EVM

**Files:**

- `trusted-gmp/src/native_gmp_relay.rs`
- `trusted-gmp/tests/relay_evm_tests.rs`

**Tasks:**

- [ ] Add EVM event parsing for `MessageSent`
- [ ] Add EVM message delivery via `deliverMessage()`
- [ ] Support EVM RPC configuration
- [ ] Test event parsing for EVM chains
- [ ] Test message delivery to EVM contracts

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 5.**

---

### Commit 5: Fix existing E2E tests for GMP: MVM ↔ EVM outflow

**Files:**

- `testing-infra/ci-e2e/e2e-tests-evm/` (update existing)
- `testing-infra/ci-e2e/e2e-tests-mvm/` (update existing)

**Tasks:**

- [ ] Update test environment to use native GMP endpoints on MVM and EVM
- [ ] Start native GMP relay in background during tests
- [ ] Update outflow test to use GMP flow (solver calls validation contract)
- [ ] Verify GMP messages are sent and received correctly
- [ ] Ensure existing test assertions still pass

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh

# Run GMP e2e test
./testing-infra/ci-e2e/e2e-tests-gmp/mvm-evm-outflow.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 6.**

---

### Commit 6: Fix existing E2E tests for GMP: MVM ↔ EVM inflow

**Files:**

- `testing-infra/ci-e2e/e2e-tests-evm/` (update existing)
- `testing-infra/ci-e2e/e2e-tests-mvm/` (update existing)

**Tasks:**

- [ ] Update inflow test to use GMP flow (escrow receives requirements via GMP)
- [ ] Verify escrow confirmation GMP message sent back to MVM
- [ ] Verify fulfillment proof GMP message sent to EVM
- [ ] Verify escrow auto-releases on fulfillment proof receipt
- [ ] Ensure existing test assertions still pass

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh

# Run GMP e2e test
./testing-infra/ci-e2e/e2e-tests-gmp/mvm-evm-inflow.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 7.**

---

### Commit 7: Deploy EVM contracts to Base Sepolia and verify

**Files:**

- `intent-frameworks/evm/scripts/deploy-gmp-contracts.js`
- `docs/architecture/plan/gmp-testnet-deployment.md` (update)

**Tasks:**

- [ ] Script to deploy OutflowValidator to Base Sepolia
- [ ] Script to deploy InflowEscrowGMP to Base Sepolia
- [ ] Configure trusted remotes (MVM hub address)
- [ ] Verify contracts on BaseScan
- [ ] Document deployed contract addresses
- [ ] Test cross-chain flow MVM ↔ EVM on testnets (with native GMP relay)

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh

# Verify deployment
npx hardhat verify --network base-sepolia <CONTRACT_ADDRESS>
```

> ⚠️ **CI e2e tests must pass before Phase 3 is complete.**

---

## Run All Tests

```bash
# Run all unit tests (includes coordinator, trusted-gmp, solver, MVM, EVM, SVM, frontend)
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI runs e2e tests automatically. All e2e tests (MVM, EVM, SVM - including GMP cross-chain tests) must pass before merging.**

---

## Exit Criteria

- [ ] All 7 commits merged to feature branch
- [ ] EVM contracts build and pass unit tests
- [ ] Native GMP relay supports all three chain types (MVM, SVM, EVM)
- [ ] Cross-chain E2E tests pass (MVM ↔ EVM outflow + inflow)
- [ ] All three chains can send/receive GMP messages in test environment
- [ ] EVM contracts deployed to Base Sepolia
- [ ] Documentation updated
