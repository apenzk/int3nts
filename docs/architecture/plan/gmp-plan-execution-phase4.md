# Phase 4: Coordinator GMP Integration (1 day)

**Status:** Not Started
**Depends On:** Phase 3
**Blocks:** Phase 5

**Note:** Coordinator service already exists. This phase adds GMP message tracking and status updates to the coordinator.

---

## Commits

> 📋 **Commit Conventions:** Before each commit, review `.claude/CLAUDE.md` and `.cursor/rules` for commit message format, test requirements, and coding standards.

### Commit 1: Add GMP message tracking to coordinator

**Files:**

- `coordinator/Cargo.toml`
- `coordinator/src/main.rs`
- `coordinator/src/lib.rs`
- `coordinator/src/storage/mod.rs` (existing)
- `coordinator/src/storage/models.rs` (new or extend existing)

**Tasks:**

- [ ] Coordinator crate already exists; add GMP message tracking
- [ ] Extend existing storage module with GMP message tracking
- [ ] Add `gmp_messages` tracking (source_chain, dest_chain, payload, status)
- [ ] Add intent/escrow status tracking for GMP flows

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 2.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 2: Add chain event listeners

**Files:**

- `coordinator/src/monitor/mod.rs` (existing)
- `coordinator/src/monitor/hub_mvm.rs` (existing)
- `coordinator/src/monitor/inflow_mvm.rs` (existing)
- `coordinator/src/monitor/inflow_svm.rs` (existing)
- `coordinator/src/monitor/inflow_generic.rs` (existing)
- `coordinator/src/monitor/outflow_generic.rs` (existing)

**Tasks:**

- [ ] Extend existing monitors to track GMP message events
- [ ] Add GMP MessageSent/MessageDelivered event parsing
- [ ] Store GMP events in coordinator state
- [ ] Test GMP event parsing for each chain

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 3.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 3: Add REST API endpoints

**Files:**

- `coordinator/src/api/mod.rs`
- `coordinator/src/api/intents.rs`
- `coordinator/src/api/escrows.rs`
- `coordinator/src/api/health.rs`
- `coordinator/src/tests/api_tests.rs`

**Tasks:**

- [ ] `GET /intents` - list intents with filters
- [ ] `GET /intents/:id` - get intent details
- [ ] `GET /escrows` - list escrows with filters
- [ ] `GET /escrows/:id` - get escrow details
- [ ] `GET /validation-contracts` - discover validation contract addresses on each connected chain (returns chain_id => contract_address mapping)
- [ ] `GET /health` - health check endpoint
- [ ] Add pagination support
- [ ] Test all API endpoints

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 4.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 4: Add WebSocket subscription support

**Files:**

- `coordinator/src/api/websocket.rs`
- `coordinator/src/tests/websocket_tests.rs`

**Tasks:**

- [ ] Implement WebSocket upgrade handler
- [ ] Support subscribing to intent updates
- [ ] Support subscribing to escrow updates
- [ ] Broadcast events to subscribers on state change
- [ ] Test WebSocket subscription flow

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 5.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 5: Update frontend to use coordinator API

**Files:**

- `frontend/src/lib/coordinator.ts` (existing)
- `frontend/src/lib/types.ts` (existing)

**Tasks:**

- [ ] Extend existing coordinator client with GMP status endpoints
- [ ] Add GMP message status display to UI
- [ ] Test coordinator client with GMP flows

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 6.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 6: Update solver SDK to use coordinator API

**Files:**

- `solver/src/coordinator_gmp_client.rs` (existing)
- `solver/src/service/outflow.rs` (existing - already has GMP flow)
- `solver/src/service/inflow.rs` (existing - already has GMP flow)
- `solver/src/chains/connected_mvm.rs` (existing - has fulfill_outflow_via_gmp)
- `solver/src/chains/connected_svm.rs` (existing - has fulfill_outflow_via_gmp)

**Tasks:**

- [ ] Extend coordinator_gmp_client with GMP status tracking
- [ ] Add validation contract address discovery via coordinator API
- [ ] **Note:** GMP flow already implemented in Phase 2 (solver calls validation contracts)
- [ ] Test coordinator client with GMP flows

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI e2e tests must pass before proceeding to Commit 7.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

### Commit 7: Add Docker configuration and documentation

**Files:**

- `coordinator/Dockerfile`
- `docker-compose.yml` (update)
- `docs/coordinator/README.md`
- `docs/coordinator/api-reference.md`

**Tasks:**

- [ ] Create Dockerfile for coordinator service
- [ ] Add PostgreSQL service to docker-compose
- [ ] Add coordinator service to docker-compose
- [ ] Configure networking between services
- [ ] Document all API endpoints
- [ ] Document WebSocket protocol
- [ ] Document deployment instructions
- [ ] **Create solver migration guide** - document how solvers transition from current flow (arbitrary transfer + signature) to new GMP flow (approve + call validation contract)
- [ ] **Document that old solvers cannot work** - old solvers must upgrade to new GMP flow (no backward compatibility)

**Test:**

```bash
# Run all unit tests
./testing-infra/run-all-unit-tests.sh

# Docker smoke test
docker-compose up -d coordinator
curl http://localhost:8080/health
docker-compose down
```

> ⚠️ **CI e2e tests must pass before Phase 4 is complete.** Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize.

---

## Run All Tests

```bash
# Run all unit tests (includes coordinator, trusted-gmp, solver, MVM, EVM, SVM, frontend)
./testing-infra/run-all-unit-tests.sh
```

> ⚠️ **CI runs e2e tests automatically. All e2e tests (MVM, EVM, SVM - inflow + outflow) must pass before merging.**

---

## Documentation Update

At the end of Phase 4, update:

- [ ] `docs/coordinator/README.md` - Coordinator service overview (if exists, or create)
- [ ] `docs/coordinator/api-reference.md` - Full API documentation
- [ ] `docs/solver/migration-guide.md` - How solvers migrate to GMP flow (note: GMP flow already implemented)
- [ ] Review conception documents for accuracy after changes

---

## Exit Criteria

- [ ] All 7 commits merged to feature branch
- [ ] Coordinator service runs and indexes events from all chains
- [ ] All API endpoints return correct data
- [ ] Documentation updated
- [ ] WebSocket subscriptions work
- [ ] Frontend uses coordinator API successfully
- [ ] Solver SDK uses coordinator API successfully
- [ ] Docker setup works for local development
