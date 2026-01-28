# Phase 3: EVM Expansion (3-4 days)

**Status:** Not Started
**Depends On:** Phase 2
**Blocks:** Phase 4

**Goal:** Add EVM connected chain support to the GMP system. After this phase, all three chain types (MVM hub, SVM, EVM) can communicate via GMP.

---

## Commits

> 📋 **Commit Conventions:** Before each commit, review `.claude/CLAUDE.md` and `.cursor/rules` for commit message format, test requirements, and coding standards.

### Commit 1: Implement MockLayerZeroEndpoint for EVM

**Files:**

- `intent-frameworks/evm/contracts/mocks/MockLayerZeroEndpoint.sol`
- `intent-frameworks/evm/test/MockLayerZeroEndpoint.test.js`

**Tasks:**

- [ ] Implement `send()` function that emits `MessageSent` event
- [ ] Implement `deliverMessage()` for simulator to inject messages
- [ ] Implement trusted remote verification
- [ ] Track message nonces
- [ ] Test send emits correct event
- [ ] Test deliverMessage calls receiver's lzReceive

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

- [ ] Inherit from LayerZero `OApp` base contract (or implement interface)
- [ ] Implement `lzReceive()` to receive intent requirements from hub
- [ ] **Idempotency check**: Before storing, check if requirements already exist for intent_id + step number
- [ ] **If requirements already exist → ignore duplicate message (idempotent)**
- [ ] **If requirements don't exist → store intent requirements** in mapping
- [ ] Implement `fulfillIntent(intent_id, token, amount)` for authorized solvers to call
- [ ] Pull tokens from authorized solver's wallet via `transferFrom()` (requires prior approval)
- [ ] Validate recipient, amount, token match stored requirements
- [ ] Validate solver matches authorized solver from stored requirements
- [ ] Forward tokens to user wallet
- [ ] Send GMP message to hub via `lzSend()`
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

- [ ] Inherit from LayerZero `OApp` base contract (or implement interface)
- [ ] Implement `lzReceive()` for intent requirements from hub
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

### Commit 4: Update LayerZero simulator to support EVM

**Files:**

- `trusted-gmp/src/layerzero_simulator.rs`
- `trusted-gmp/tests/simulator_evm_tests.rs`

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

### Commit 5: Add cross-chain E2E test: MVM ↔ EVM outflow

**Files:**

- `testing-infra/ci-e2e/e2e-tests-gmp/mvm-evm-outflow.sh`

**Tasks:**

- [ ] Set up test environment with mock endpoints on MVM and EVM
- [ ] Start LayerZero simulator in background
- [ ] Create intent on MVM hub
- [ ] Verify requirements message sent to EVM
- [ ] Solver validates on EVM (approve + fulfillIntent)
- [ ] Verify success message sent back to MVM
- [ ] Verify intent completes on MVM

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh

# Run GMP e2e test
./testing-infra/ci-e2e/e2e-tests-gmp/mvm-evm-outflow.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 6.**

---

### Commit 6: Add cross-chain E2E test: MVM ↔ EVM inflow

**Files:**

- `testing-infra/ci-e2e/e2e-tests-gmp/mvm-evm-inflow.sh`

**Tasks:**

- [ ] Create intent on MVM hub (inflow type)
- [ ] Verify requirements message sent to EVM
- [ ] Requester creates escrow on EVM
- [ ] Verify escrow confirmation sent back to MVM
- [ ] Solver fulfills on MVM hub
- [ ] Verify fulfillment proof sent to EVM
- [ ] Verify escrow releases on EVM

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
- [ ] Test cross-chain flow MVM ↔ EVM on testnets (with simulator)

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
- [ ] LayerZero simulator supports all three chain types (MVM, SVM, EVM)
- [ ] Cross-chain E2E tests pass (MVM ↔ EVM outflow + inflow)
- [ ] All three chains can send/receive GMP messages in test environment
- [ ] EVM contracts deployed to Base Sepolia
- [ ] Documentation updated
