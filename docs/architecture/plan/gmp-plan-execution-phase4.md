# Phase 4: Integration & Documentation

**Status:** Blocked - Waiting for Phase 3 API work
**Depends On:** Phase 3 (deferred API/WebSocket endpoints)
**Blocks:** None (Final Phase)

**Note:** Phase 4 Commits 1-2 are blocked because they require Phase 3's coordinator API endpoints (REST API, WebSocket) which were deferred until frontend is ready. Commits 3-7 (deployment, testing, documentation) can potentially proceed independently.

**What Phase 3 completed:** Readiness tracking for outflow intents (commit `f46eb3d`) - monitors IntentRequirementsReceived events and sets `ready_on_connected_chain` flag. This enables basic frontend coordination without needing the full API.

---

## Commits

> 📋 **Commit Conventions:** Before each commit, review `.claude/CLAUDE.md` and `.cursor/rules` for commit message format, test requirements, and coding standards.

### Commit 1: Update frontend for GMP integration (BLOCKED)

**Status:** Blocked - Requires Phase 4 coordinator API endpoints

**Files:**

- `frontend/src/lib/coordinator.ts` (existing)
- `frontend/src/lib/types.ts` (existing)
- `frontend/src/config/chains.ts` (existing)

**Tasks:**

- [ ] Show GMP message status in intent details
- [ ] Update status tracking for GMP-based intents
- [ ] Display cross-chain message delivery progress
- [ ] Test UI renders correctly for GMP flows

**Blocked by:** Deferred API work from Phase 3 - needs `GET /intents`, `GET /escrows` endpoints to fetch intent status

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 2.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 2: Update solver SDK for GMP integration (BLOCKED)

**Status:** Blocked - Requires Phase 4 coordinator API endpoints

**Files:**

- `solver/src/coordinator_gmp_client.rs` (existing)
- `solver/src/service/outflow.rs` (existing - GMP flow already implemented)
- `solver/src/service/inflow.rs` (existing - GMP flow already implemented)

**Tasks:**

- [ ] **Note:** GMP flow already implemented in Phase 2 (solver calls validation contracts)
- [ ] Add any remaining GMP status tracking
- [ ] Integrate with coordinator API for GMP message status
- [ ] Test fulfillment flows work correctly with GMP

**Blocked by:** Deferred API work from Phase 3 - needs coordinator API to query intent/escrow status

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 3.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 3: Update deployment scripts for GMP (moved from Phase 2)

**Files:**

- `intent-frameworks/svm/scripts/` (update existing deployment scripts)
- `intent-frameworks/mvm/scripts/` (update existing deployment scripts)

**Tasks:**

- [ ] Update SVM deployment scripts to include GMP programs (intent-outflow-validator, intent-escrow with GMP config)
- [ ] Update MVM deployment scripts to include GMP modules
- [ ] Add trusted remote configuration to deployment scripts
- [ ] Deploy updated contracts/modules to testnets
- [ ] Verify cross-chain flow works on testnets (with native GMP relay)

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh

# Verify deployments
solana program show <INTENT_OUTFLOW_VALIDATOR_PROGRAM_ID> --url devnet
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 4.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 4: Add full cross-chain testnet integration test

**Files:**

- `testing-infra/ci-e2e/e2e-tests-gmp/full-flow-testnet.sh`

**Tasks:**

- [ ] Test complete outflow: MVM testnet → SVM devnet
- [ ] Test complete inflow: MVM testnet ← SVM devnet
- [ ] Test complete outflow: MVM testnet → Base Sepolia
- [ ] Test complete inflow: MVM testnet ← Base Sepolia
- [ ] Verify all GMP messages delivered correctly

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 5.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 5: Add fee estimation and endpoint configuration

**Files:**

- `docs/architecture/plan/gmp-endpoints.md`
- `docs/architecture/plan/gmp-fee-analysis.md`

**Tasks:**

- [ ] Document all GMP endpoint addresses (LZ for Solana and Movement, local for testing)
- [ ] Document environment configuration (local/CI uses native GMP endpoints, testnet/mainnet use LZ)
- [ ] Estimate LZ message fees for each route
- [ ] Estimate on-chain validation gas costs
- [ ] Compare costs to current Trusted GMP system

**Test:**

```bash
# Documentation review - manual
```

> ⚠️ **Documentation review before proceeding to Commit 6.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 6: Add GMP integration documentation

**Files:**

- `docs/gmp/architecture.md`
- `docs/gmp/solver-guide.md`
- `docs/gmp/troubleshooting.md`

**Tasks:**

- [ ] Document GMP architecture and message flows
- [ ] Document solver integration guide
- [ ] Document common issues and troubleshooting steps
- [ ] Document testnet contract addresses

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh

# Documentation review - manual
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 7.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 7: Final cleanup and verification

**Files:**

- `CHANGELOG.md`
- `README.md` (update architecture section)

**Tasks:**

- [ ] Confirm architecture: coordinator + trusted-gmp only (no monolithic signer code or directory)
- [ ] Update CHANGELOG with GMP integration notes
- [ ] Update README with new architecture diagram
- [ ] Verify coordinator has no private keys (trusted-gmp requires operator wallet privkeys per chain)
- [ ] Final security review of coordinator + trusted-gmp

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh

# Architecture check: coordinator + trusted-gmp only (no monolithic signer directory)
test ! -d verifier && echo "OK: coordinator + trusted-gmp only"

# Coordinator must not reference private keys
grep -r "private_key\|secret_key\|signing_key" coordinator/ && exit 1 || echo "OK: coordinator has no keys"
```

> ⚠️ **CI e2e tests must pass before Phase 4 is complete (7 commits total).** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

## Run All Tests

```bash
# Run all unit tests (includes coordinator, trusted-gmp, solver, MVM, EVM, SVM, frontend)
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI runs e2e tests automatically. All e2e tests (MVM, EVM, SVM - inflow + outflow, plus GMP cross-chain tests) must pass before merging.**

---

## Documentation Update

At the end of Phase 4, update:

- [ ] `docs/gmp/architecture.md` - Complete GMP architecture documentation
- [ ] `docs/gmp/solver-guide.md` - Complete solver integration guide
- [ ] `docs/gmp/troubleshooting.md` - Common issues and solutions
- [ ] `README.md` - Update with new architecture diagram
- [ ] `CHANGELOG.md` - Document GMP integration milestone
- [ ] Review ALL conception documents for accuracy after full GMP migration
- [ ] Final audit: No references to monolithic signer; architecture is coordinator + trusted-gmp only

---

## Exit Criteria

- [ ] All 7 commits merged to feature branch
- [ ] Frontend shows GMP status correctly
- [ ] Solver uses validation contracts (GMP flow only)
- [ ] Programs/modules deployed to testnets (Commit 3)
- [ ] Full cross-chain testnet integration passes
- [ ] Documentation complete
- [ ] Fee analysis complete (deferred from Phase 1)
- [ ] Architecture confirmed: coordinator + trusted-gmp only (no monolithic signer)
- [ ] All conception documents reviewed and updated
