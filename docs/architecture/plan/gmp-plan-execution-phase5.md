# Phase 5: Integration & Documentation (2-3 days)

**Status:** Not Started
**Depends On:** Phase 4
**Blocks:** None (Final Phase)

---

## Commits

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
nix develop ./nix -c bash -c "cd frontend && npm test -- --grep 'gmp'"
```

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
nix develop ./nix -c bash -c "cd solver && cargo test -- --test gmp_tests"
```

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
nix develop ./nix -c bash -c "./testing-infra/ci-e2e/e2e-tests-gmp/full-flow-testnet.sh"
```

---

### Commit 4: Add GMP integration documentation

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
# Documentation review - no automated test
# Manual: Review documentation for completeness
```

---

### Commit 5: Final cleanup and verification

**Files:**

- `CHANGELOG.md`
- `README.md` (update architecture section)

**Tasks:**

- [ ] Verify old verifier code is completely removed (done in Phase 0)
- [ ] Update CHANGELOG with GMP integration notes
- [ ] Update README with new architecture diagram
- [ ] Verify coordinator has no private keys (trusted-gmp requires operator wallet privkeys per chain)
- [ ] Final security review of coordinator + trusted-gmp

**Test:**

```bash
# Verify no verifier directory exists
test ! -d verifier && echo "Old verifier removed"

# Verify coordinator has no private key references (trusted-gmp requires operator wallet privkeys)
grep -r "private_key\|secret_key\|signing_key" coordinator/ && exit 1 || echo "Coordinator has no keys"

# All tests pass
nix develop ./nix -c bash -c "cd coordinator && cargo test"
nix develop ./nix -c bash -c "cd trusted-gmp && cargo test"
```

---

## Run All Tests

```bash
# Coordinator tests
nix develop ./nix -c bash -c "cd coordinator && cargo test"

# Trusted GMP tests
nix develop ./nix -c bash -c "cd trusted-gmp && cargo test"

# Frontend GMP tests
nix develop ./nix -c bash -c "cd frontend && npm test -- --grep 'gmp'"

# Solver GMP tests
nix develop ./nix -c bash -c "cd solver && cargo test -- --test gmp_tests"

# Full testnet integration (requires deployed contracts)
nix develop ./nix -c bash -c "./testing-infra/ci-e2e/e2e-tests-gmp/full-flow-testnet.sh"
```

---

## Documentation Update

At the end of Phase 5, update:

- [ ] `docs/gmp/architecture.md` - Complete GMP architecture documentation
- [ ] `docs/gmp/solver-guide.md` - Complete solver integration guide
- [ ] `docs/gmp/troubleshooting.md` - Common issues and solutions
- [ ] `README.md` - Update with new architecture diagram
- [ ] `CHANGELOG.md` - Document GMP integration milestone
- [ ] Review ALL conception documents for accuracy after full GMP migration
- [ ] Final audit: Check if any files still reference old verifier architecture and update them

---

## Exit Criteria

- [ ] All 5 commits merged to feature branch
- [ ] Frontend shows GMP status correctly
- [ ] Solver uses validation contracts (GMP flow only)
- [ ] Full cross-chain testnet integration passes
- [ ] Documentation complete
- [ ] Old verifier completely removed (no legacy code)
- [ ] All conception documents reviewed and updated
