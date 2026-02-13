# Phase 4: Integration & Documentation

**Status:** In Progress
**Depends On:** Phase 3
**Blocks:** None (Final Phase)

**What Phase 3 completed:** Readiness tracking for outflow intents (commit `f46eb3d`) - monitors IntentRequirementsReceived events and sets `ready_on_connected_chain` flag.

**Architecture principle:** The coordinator is the single API surface for frontends and solvers. Clients never poll integrated-gmp directly. Integrated-gmp is purely infrastructure (relay) — invisible to clients.

---

## Commits

> 📋 **Commit Conventions:** Before each commit, review `.claude/CLAUDE.md` and `.cursor/rules` for commit message format, test requirements, and coding standards.

### Commit 1: Strip integrated-gmp client-facing API down to relay-only

**Files:**

- `integrated-gmp/src/api/generic.rs` (existing - route definitions)
- `integrated-gmp/src/api/outflow_generic.rs` (remove)
- `integrated-gmp/src/api/outflow_mvm.rs` (remove)
- `integrated-gmp/src/api/outflow_evm.rs` (remove)
- `integrated-gmp/src/api/outflow_svm.rs` (remove)
- `integrated-gmp/src/api/inflow_generic.rs` (remove)

**Tasks:**

- [x] Remove all client-facing API endpoints:
  - `POST /validate-outflow-fulfillment` (solver validated tx hash — now done on-chain by validation contract)
  - `POST /validate-inflow-escrow` (escrow validation — now auto-releases via GMP FulfillmentProof)
  - `POST /approval` (signature generation — GMP message is the proof)
  - `GET /public-key` (frontend needed for intent creation — no signatures in GMP)
  - `GET /approved/:intent_id` (frontend polled approval status — coordinator provides this)
  - `GET /approvals` (listed all signatures — no signatures exist)
  - `GET /approvals/:escrow_id` (specific escrow signature — no signatures exist)
  - `GET /events` (coordinator has its own `/events`)
- [x] Keep only:
  - `GET /health` (ops monitoring of relay process)
- [x] Remove dead code: outflow validation logic, inflow validation logic, signature generation, transaction parsing
- [x] Update integrated-gmp tests to remove tests for deleted endpoints
- [x] Verify relay functionality still works (MessageSent watching + deliverMessage calls)

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **All unit tests must pass before proceeding to Commit 2.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 2: Remove integrated-gmp polling from frontend, use coordinator only

**Files:**

- `frontend/src/lib/coordinator.ts` (existing)
- `frontend/src/lib/types.ts` (existing)

**Tasks:**

- [x] Remove all direct integrated-gmp API calls from frontend:
  - Remove `/approved/:intentId` polling (outflow approval check)
  - Remove `/public-key` call (no longer needed — GMP replaces signatures)
  - Remove `/approvals/:escrowId` call (inflow approval check)
- [x] Replace outflow completion tracking: poll coordinator `GET /events` for intent fulfillment/completion status instead of integrated-gmp `/approved/:intentId`
- [x] Replace inflow escrow release tracking: poll coordinator `GET /events` for `EscrowReleased` event instead of integrated-gmp `/approvals/:escrowId`
- [x] Remove `integrated_gmp_public_key` parameter from outflow intent creation flow
- [x] Use `ready_on_connected_chain` flag from coordinator events to show GMP delivery status
- [x] Remove integrated-gmp base URL configuration from frontend

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **All unit tests must pass before proceeding to Commit 2.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 3: Remove integrated-gmp polling from solver, use coordinator only

**Files:**

- `solver/src/coordinator_gmp_client.rs` (existing)
- `solver/src/service/outflow.rs` (existing)
- `solver/src/service/inflow.rs` (existing)

**Tasks:**

- [x] Remove direct integrated-gmp API calls from solver:
  - Remove `POST /validate-outflow-fulfillment` call (no longer needed — validation contract sends GMP message directly)
  - Remove any `/approvals` polling
- [x] Replace outflow completion tracking: use coordinator `GET /events` to check hub intent release status
- [x] Replace inflow escrow release tracking: use coordinator `GET /events` to check `EscrowReleased` event
- [x] Use `ready_on_connected_chain` flag from coordinator events before calling validation contracts
- [x] Remove integrated-gmp base URL configuration from solver

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **All unit tests must pass before proceeding to Commit 4.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 4: Update deployment scripts for GMP (moved from Phase 2)

**Files:**

- `intent-frameworks/svm/scripts/deploy.sh` (updated to deploy all 3 programs)
- `intent-frameworks/svm/scripts/initialize-gmp.sh` (new — GMP endpoint, outflow validator, escrow GMP config, routing)
- `intent-frameworks/svm/scripts/README.md` (updated with deployment workflow and new scripts)
- `intent-frameworks/mvm/scripts/deploy-hub.sh` (new — deploy + initialize hub chain with GMP)
- `intent-frameworks/mvm/scripts/deploy-connected.sh` (new — deploy + initialize connected chain with GMP)
- `intent-frameworks/evm/scripts/deploy-gmp.js` (already complete — no changes needed)

**Tasks:**

- [x] Update SVM deployment scripts to include GMP programs (intent-outflow-validator, intent-escrow with GMP config)
- [x] Update MVM deployment scripts to include GMP modules
- [x] Update EVM deployment scripts to include GMP contracts (IntentGmp, IntentOutflowValidator, IntentInflowEscrow) and remote GMP endpoint configuration
- [x] Add remote GMP endpoint configuration to all deployment scripts (SVM, MVM, EVM)
- [x] Deploy updated contracts/modules/programs to testnets
- [x] Verify cross-chain flow works on testnets (with integrated GMP relay)

**Test:**

```bash
./testing-infra/run-all-unit-tests.sh

# Verify deployments
solana program show <INTENT_OUTFLOW_VALIDATOR_PROGRAM_ID> --url devnet
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 5.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 5: Update existing docs with GMP endpoint configuration

**Files:**

- `docs/architecture/architecture-component-mapping.md` (update component mapping for GMP modules)
- `docs/architecture/domain-boundaries-and-interfaces.md` (update interfaces for GMP)
- `docs/architecture/data-models.md` (add GMP message types)
- `docs/architecture/conception/architecture-diff.md` (update implementation status)

**Tasks:**

- [ ] Update architecture-component-mapping with GMP modules/contracts across all VMs
- [ ] Update domain-boundaries-and-interfaces with GMP send/receive interfaces
- [ ] Update data-models with GMP message types (IntentRequirements, EscrowConfirmation, FulfillmentProof)
- [ ] Update architecture-diff with current implementation status post-GMP

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

- [ ] Confirm architecture: coordinator + integrated-gmp only (no monolithic signer code or directory)
- [ ] Update CHANGELOG with GMP integration notes
- [ ] Update README with new architecture diagram
- [ ] Verify coordinator has no private keys (integrated-gmp requires operator wallet privkeys per chain)
- [ ] Final security review of coordinator + integrated-gmp

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh

# Architecture check: coordinator + integrated-gmp only (no monolithic signer directory)
test ! -d verifier && echo "OK: coordinator + integrated-gmp only"

# Coordinator must not reference private keys
grep -r "private_key\|secret_key\|signing_key" coordinator/ && exit 1 || echo "OK: coordinator has no keys"
```

> ⚠️ **CI e2e tests must pass before Phase 4 is complete (7 commits total).** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

## Run All Tests

```bash
# Run all unit tests (includes coordinator, integrated-gmp, solver, MVM, EVM, SVM, frontend)
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
- [ ] Final audit: No references to monolithic signer; architecture is coordinator + integrated-gmp only

---

## Exit Criteria

- [ ] All 7 commits merged to feature branch
- [ ] Integrated-gmp stripped to relay-only (no client-facing API besides /health)
- [ ] Frontend uses coordinator as single API (no direct integrated-gmp calls)
- [ ] Solver uses coordinator as single API (no direct integrated-gmp calls)
- [ ] Programs/modules/contracts deployed to testnets for all chains: SVM, MVM, EVM (Commit 4)
- [ ] Documentation complete
- [ ] Existing architecture docs updated with GMP endpoint configuration
- [ ] Architecture confirmed: coordinator + integrated-gmp only (no monolithic signer)
- [ ] All conception documents reviewed and updated
