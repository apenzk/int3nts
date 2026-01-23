# Phase 0: Verifier Separation (3-4 days)

**Status:** Not Started
**Depends On:** None
**Blocks:** Phase 1
**Purpose:** Separate the current verifier into two independent components: Coordinator (UX functions) and Trusted GMP (message relay), enabling incremental migration and cleaner architecture.

---

## Overview

Before migrating to real GMP protocols, we first separate the verifier into:

1. **Coordinator Service** - Handles UX functions (event monitoring, caching, API, negotiation) - NO KEYS, CANNOT STEAL FUNDS
2. **Trusted GMP Service** - Handles message relay (watches mock GMP endpoints, delivers messages) - REQUIRES FUNDED OPERATOR WALLET on each chain (private key in config, pays gas to call `lzReceive()`), CAN FORGE MESSAGES, CAN STEAL FUNDS

This separation:

- ✅ Coordinator cannot steal funds (no keys, read-only)
- ✅ Enables independent testing of each component
- ✅ Provides testing path (trusted GMP for local/CI, real GMP for production)
- ✅ Makes architecture cleaner (clear separation of concerns)
- ⚠️ Trusted GMP has same security risk as current verifier (can forge messages)

---

## Important: Incremental Migration

> ⚠️ **Verifier must keep working until Commit 3.** Commits 1 & 2 **COPY** files from verifier (not move). This ensures:
>
> - Existing shell scripts (`run-verifier-local.sh`, `run-solver-local.sh`, etc.) continue working
> - CI/CD pipelines remain functional
> - Safe rollback if issues discovered
>
> Only in Commit 3 do we delete verifier and update scripts.

**Shell scripts that depend on verifier:**

Testnet scripts:

- `testing-infra/testnet/run-verifier-local.sh` - runs verifier binary
- `testing-infra/testnet/run-solver-local.sh` - checks verifier health
- `testing-infra/testnet/check-testnet-preparedness.sh` - checks verifier config
- `testing-infra/testnet/create-intent.sh` - uses verifier for negotiation
- `testing-infra/testnet/verify-verifier-evm-address.sh` - verifies EVM address
- `testing-infra/testnet/deploy-to-*.sh` - reference verifier config

CI-E2E scripts (configure verifier):

- `testing-infra/ci-e2e/chain-hub/configure-verifier.sh`
- `testing-infra/ci-e2e/chain-connected-evm/configure-verifier.sh`
- `testing-infra/ci-e2e/chain-connected-svm/configure-verifier.sh`
- `testing-infra/ci-e2e/chain-connected-mvm/configure-verifier.sh`

CI-E2E scripts (start verifier):

- `testing-infra/ci-e2e/e2e-tests-evm/start-verifier.sh`
- `testing-infra/ci-e2e/e2e-tests-svm/start-verifier.sh`
- `testing-infra/ci-e2e/e2e-tests-mvm/start-verifier.sh`

CI-E2E scripts (run tests - build verifier):

- `testing-infra/ci-e2e/e2e-tests-evm/run-tests-outflow.sh`
- `testing-infra/ci-e2e/e2e-tests-evm/run-tests-inflow.sh`
- `testing-infra/ci-e2e/e2e-tests-svm/run-tests-outflow.sh`
- `testing-infra/ci-e2e/e2e-tests-svm/run-tests-inflow.sh`
- `testing-infra/ci-e2e/e2e-tests-mvm/run-tests-outflow.sh`
- `testing-infra/ci-e2e/e2e-tests-mvm/run-tests-inflow.sh`
- `testing-infra/ci-e2e/e2e-tests-mvm/verifier-rust-integration-tests.sh`

CI-E2E scripts (submit intents - verify_verifier_running):

- `testing-infra/ci-e2e/e2e-tests-*/inflow-submit-hub-intent.sh`
- `testing-infra/ci-e2e/e2e-tests-*/outflow-submit-hub-intent.sh`

CI-E2E scripts (other):

- `testing-infra/ci-e2e/chain-hub/deploy-contracts.sh` - initialize_verifier
- `testing-infra/ci-e2e/chain-connected-evm/deploy-contract.sh` - get_verifier_eth_address
- `testing-infra/ci-e2e/chain-connected-svm/deploy-contract.sh` - load_verifier_keys
- `testing-infra/ci-e2e/chain-*/cleanup.sh` - stop_verifier
- `testing-infra/ci-e2e/util.sh` - verifier helper functions

---

## Commits

### Commit 1: Extract Coordinator Service

**Files:**

- `coordinator/src/main.rs`
- `coordinator/src/config/` (copied from `verifier/src/config/`, keys removed)
- `coordinator/src/monitor/` (copied from `verifier/src/monitor/`)
- `coordinator/src/api/` (copied from `verifier/src/api/`, validation endpoints removed)
- `coordinator/src/storage/` (copied from `verifier/src/storage/`)
- `coordinator/Cargo.toml`

> ⚠️ **COPY, not move.** Verifier must keep working. Use `cp -r` not `git mv`.

**Tasks:**

- [ ] Create new `coordinator/` crate
- [ ] Copy event monitoring logic from verifier (no validation, just monitoring)
- [ ] Copy REST API from verifier (no signature endpoints, just read-only)
- [ ] Copy event caching/storage from verifier
- [ ] Remove all cryptographic operations (no keys, no signing)
- [ ] Remove all validation logic (contracts will handle this)
- [ ] Keep negotiation API (application logic, not security-critical)
- [ ] Update configuration to remove key-related settings
- [ ] Test coordinator can monitor events and serve API without keys
- [ ] Verify verifier still builds and works (scripts unchanged)

**Test:**

```bash
# Run all unit tests (verifier still exists at this point)
./testing-infra/run-all-unit-tests.sh

# Verify coordinator builds and tests pass
nix develop ./nix -c bash -c "cd coordinator && cargo build && cargo test"
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 2.** E2E tests run in CI, not locally.

---

### Commit 2: Extract Trusted GMP Service

**Files:**

- `trusted-gmp/src/main.rs`
- `trusted-gmp/src/config/` (new config with operator wallet keys)
- `trusted-gmp/src/monitor/gmp_events.rs`
- `trusted-gmp/src/delivery/` (message delivery logic)
- `trusted-gmp/Cargo.toml`

> ⚠️ **Verifier still exists.** This commit creates trusted-gmp alongside verifier.

**Tasks:**

- [ ] Create new `trusted-gmp/` crate
- [ ] Implement mock GMP endpoint event monitoring (watches `MessageSent` events)
- [ ] Implement message delivery logic (calls `lzReceive()` on destination contracts)
- [ ] Support configurable chain connections (MVM, EVM, SVM)
- [ ] Support message routing (source chain → destination chain)
- [ ] No validation logic (contracts validate)
- [ ] Configure operator wallet private key for each chain in config (MVM account, EVM EOA, SVM keypair) - these wallets must be funded with native tokens to pay gas for `lzReceive()` calls - can forge messages, same risk as verifier
- [ ] Add configuration for trusted mode (which chains to connect)
- [ ] Test message delivery works end-to-end
- [ ] Verify verifier still builds and works (scripts unchanged)

**Test:**

```bash
# Run all unit tests (verifier still exists at this point)
./testing-infra/run-all-unit-tests.sh

# Verify trusted-gmp builds and tests pass
nix develop ./nix -c bash -c "cd trusted-gmp && cargo build && cargo test"

# Verify coordinator still works
nix develop ./nix -c bash -c "cd coordinator && cargo build && cargo test"
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 3.** E2E tests run in CI, not locally.

---

### Commit 3: Remove Old Verifier, Update Scripts

**Files:**

- `verifier/` (DELETED - old verifier is completely removed)
- `coordinator/src/main.rs` (standalone service)
- `trusted-gmp/src/main.rs` (standalone service)
- All scripts listed in "Shell scripts that depend on verifier" section above

> ⚠️ **This is the breaking change commit.** After this, verifier no longer exists.

**Tasks:**

- [ ] Delete the old `verifier/` crate entirely

Testnet scripts:

- [ ] Rename `run-verifier-local.sh` → `run-coordinator-local.sh`
- [ ] Update `run-solver-local.sh` to check coordinator health (same API, different service name)
- [ ] Update `check-testnet-preparedness.sh` to check coordinator config
- [ ] Update `create-intent.sh` to use coordinator
- [ ] Update `verify-verifier-evm-address.sh` or remove if no longer needed
- [ ] Update `deploy-to-*.sh` scripts to reference coordinator config

CI-E2E scripts:

- [ ] Rename `configure-verifier.sh` → `configure-coordinator.sh` (all chains)
- [ ] Rename `start-verifier.sh` → `start-coordinator.sh` (all test suites)
- [ ] Update `run-tests-*.sh` to build coordinator instead of verifier
- [ ] Update `*-submit-hub-intent.sh` to use `verify_coordinator_running`
- [ ] Update `deploy-contracts.sh` and `deploy-contract.sh` scripts
- [ ] Update `cleanup.sh` scripts to use `stop_coordinator`
- [ ] Update `util.sh` helper functions (rename verifier → coordinator)

CI/CD:

- [ ] Update CI/CD to deploy coordinator + trusted-gmp instead of verifier

Unit test script:

- [ ] Update `testing-infra/run-all-unit-tests.sh`:
  - Replace "Verifier" section with "Coordinator" section (`cd coordinator && cargo test`)
  - Add "Trusted GMP" section (`cd trusted-gmp && cargo test`)
  - Update summary table to show Coordinator and Trusted GMP instead of Verifier

**Test:**

```bash
# Verify old verifier is removed
test ! -d verifier && echo "Old verifier removed"

# Run all unit tests (now uses coordinator + trusted-gmp instead of verifier)
./testing-infra/run-all-unit-tests.sh

# Test updated testnet scripts work
./testing-infra/testnet/run-coordinator-local.sh --help
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 4.** E2E tests run in CI, not locally.

---

### Commit 4: Integration Tests for New Architecture

**Files:**

- `testing-infra/ci-e2e/phase0-tests/coordinator_tests.rs`
- `testing-infra/ci-e2e/phase0-tests/trusted_gmp_tests.rs`
- `testing-infra/ci-e2e/phase0-tests/integration_tests.rs`

**Tasks:**

- [ ] Test coordinator can monitor events independently
- [ ] Test coordinator API works without keys
- [ ] Test trusted GMP can relay messages end-to-end
- [ ] Test coordinator + trusted GMP work together for full flow
- [ ] Verify coordinator has no private keys (trusted-gmp requires operator wallet privkeys per chain)

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh

# Run Phase 0 integration tests
nix develop ./nix -c bash -c "cd testing-infra/ci-e2e/phase0-tests && cargo test"
```

> ⚠️ **CI e2e tests must pass before Phase 0 is complete.** E2E tests run in CI, not locally.

---

## Success Criteria

✅ **Coordinator Service:**

- Monitors events across chains (no keys needed)
- Serves REST API (read-only, no signature endpoints)
- Handles negotiation routing (application logic)
- No security-critical functions

✅ **Trusted GMP Service:**

- Watches mock GMP endpoint events
- Delivers messages to destination contracts
- No validation logic (contracts validate)
- ⚠️ Requires operator wallet private key per chain (configured, funded with native tokens for gas) to submit `lzReceive()` calls - can forge messages, same risk as verifier

✅ **Old Verifier:**

- Completely removed (no legacy code)
- No off-chain validation logic

✅ **System:**

- New architecture fully operational
- Clear separation of concerns
- Coordinator cannot steal funds (no keys)
- Trusted GMP can steal funds (same as before) - only used for testing
- Ready for Phase 1 (on-chain validation migration)

---

## Exit Criteria

- [ ] All 4 commits merged to feature branch
- [ ] Coordinator builds and tests pass
- [ ] Trusted GMP builds and tests pass
- [ ] Old verifier directory deleted
- [ ] Integration tests pass
- [ ] Documentation updated

---

## Benefits of Phase 0

1. **Clean Break** - Old verifier completely removed, no legacy code
2. **Reduced Risk** - Separating components reduces blast radius of changes
3. **Clear Architecture** - Coordinator and trusted GMP have distinct roles
4. **Testing** - Can test each component in isolation
5. **Coordinator Safety** - Coordinator has no keys, cannot steal funds
6. **Production Path** - Trusted GMP for local/CI, real GMP for production

> ⚠️ **Note:** Trusted GMP requires operator wallet private keys (configured per chain, funded with native tokens for gas) to submit `lzReceive()` calls. It can forge messages (same security risk as current verifier). The security improvement only applies in production when using real LayerZero.

---

## Documentation Update

At the end of Phase 0, update:

- [ ] `README.md` - Update architecture section to reflect coordinator + trusted-gmp split
- [ ] `docs/architecture/architecture-component-mapping.md` - **PRIMARY UPDATE**: Update mermaid diagrams to show Coordinator + Trusted GMP instead of Verification Domain. Update "Verification Domain" section to split into "Coordinator Service" and "Trusted GMP Service" with their respective components
- [ ] `docs/architecture/domain-boundaries-and-interfaces.md` - Update "Verification: Boundaries and Interfaces" section to reflect the split
- [ ] `docs/architecture/README.md` - Update "Trusted Verifier" reference to "Coordinator + Trusted GMP"
- [ ] `docs/operations/` - Document how to run coordinator and trusted-gmp services separately
- [ ] Review conception documents for accuracy after changes
- [ ] Check if other files reference old verifier architecture and update them (grep for `verifier/src`)

---

## Next Steps

After Phase 0 completes:

- **Phase 1**: Research & Design (can now focus on on-chain validation, knowing coordinator/trusted-gmp are separated)
- **Phase 2+**: Implement GMP contracts (can use trusted GMP for testing)
