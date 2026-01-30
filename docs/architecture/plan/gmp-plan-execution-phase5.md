# Phase 5: Integration & Documentation (1-2 days)

**Status:** Not Started
**Depends On:** Phase 4
**Blocks:** None (Final Phase)

---

## Commits

> 📋 **Commit Conventions:** Before each commit, review `.claude/CLAUDE.md` and `.cursor/rules` for commit message format, test requirements, and coding standards.

### Commit 1: Update frontend for GMP integration

**Files:**

- `frontend/src/config/gmp.ts`
- `frontend/src/components/IntentStatus.tsx`
- `frontend/src/tests/gmp.test.ts`

**Tasks:**

- [ ] Show GMP message status in intent details
- [ ] Update status tracking for GMP-based intents
- [ ] Display cross-chain message delivery progress
- [ ] Test UI renders correctly for GMP flows

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 2.**

---

### Commit 2: Update solver SDK for GMP integration

**Files:**

- `solver/src/gmp.rs`
- `solver/src/tests/gmp_tests.rs`

**Tasks:**

- [ ] Use validation contract for outflow intents
- [ ] Handle escrow creation for inflow intents
- [ ] Integrate with coordinator API for intent discovery
- [ ] Test fulfillment flows work correctly

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 3.**

---

### Commit 3: Add full cross-chain testnet integration test

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

> ⚠️ **CI e2e tests must pass before proceeding to Commit 4.**

---

### Commit 4: Add fee estimation and endpoint configuration

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

> ⚠️ **Documentation review before proceeding to Commit 5.**

---

### Commit 5: Add GMP integration documentation

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

> ⚠️ **CI e2e tests must pass before proceeding to Commit 6.**

---

### Commit 6: Final cleanup and verification

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

> ⚠️ **CI e2e tests must pass before Phase 5 is complete (6 commits total).**

---

## Run All Tests

```bash
# Run all unit tests (includes coordinator, trusted-gmp, solver, MVM, EVM, SVM, frontend)
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI runs e2e tests automatically. All e2e tests (MVM, EVM, SVM - inflow + outflow, plus GMP cross-chain tests) must pass before merging.**

---

## Documentation Update

At the end of Phase 5, update:

- [ ] `docs/gmp/architecture.md` - Complete GMP architecture documentation
- [ ] `docs/gmp/solver-guide.md` - Complete solver integration guide
- [ ] `docs/gmp/troubleshooting.md` - Common issues and solutions
- [ ] `README.md` - Update with new architecture diagram
- [ ] `CHANGELOG.md` - Document GMP integration milestone
- [ ] Review ALL conception documents for accuracy after full GMP migration
- [ ] Final audit: No references to monolithic signer; architecture is coordinator + trusted-gmp only

---

## Exit Criteria

- [ ] All 6 commits merged to feature branch
- [ ] Frontend shows GMP status correctly
- [ ] Solver uses validation contracts (GMP flow only)
- [ ] Full cross-chain testnet integration passes
- [ ] Documentation complete
- [ ] Fee analysis complete (deferred from Phase 1)
- [ ] Architecture confirmed: coordinator + trusted-gmp only (no monolithic signer)
- [ ] All conception documents reviewed and updated
