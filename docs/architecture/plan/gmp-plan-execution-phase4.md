# Phase 4: Coordinator GMP Integration (2 days)

**Status:** Not Started
**Depends On:** Phase 3
**Blocks:** Phase 5

**Note:** Coordinator service is already extracted in Phase 0. This phase focuses on integrating GMP message tracking and status updates into the coordinator.

---

## Commits

### Commit 1: Add GMP message tracking to coordinator

**Files:**

- `coordinator/Cargo.toml`
- `coordinator/src/main.rs`
- `coordinator/src/lib.rs`
- `coordinator/src/db/mod.rs`
- `coordinator/src/db/models.rs`
- `coordinator/migrations/001_initial.sql`

**Tasks:**

- [ ] Create `coordinator/` as standalone Rust crate (like `verifier/` and `solver/`)
- [ ] Initialize new Rust crate for coordinator service
- [ ] Add dependencies: axum, sqlx, tokio, ethers, aptos-sdk
- [ ] Define `intents` table (id, status, requester, requirements, timestamps)
- [ ] Define `escrows` table (id, intent_id, status, chain_id, timestamps)
- [ ] Define `fulfillments` table (id, intent_id, solver, timestamps)
- [ ] Define `gmp_messages` table (id, source_chain, dest_chain, payload, status)
- [ ] Add SQLx models and basic queries

**Test:**

```bash
# Build coordinator (no database required for build)
nix develop ./nix -c bash -c "cd coordinator && cargo build"

# Database tests (requires PostgreSQL - skip in CI if unavailable)
# nix develop ./nix -c bash -c "cd coordinator && sqlx database create && sqlx migrate run && cargo test -- --test db_tests"
```

---

### Commit 2: Add chain event listeners

**Files:**

- `coordinator/src/listeners/mod.rs`
- `coordinator/src/listeners/mvm.rs`
- `coordinator/src/listeners/evm.rs`
- `coordinator/src/listeners/svm.rs`
- `coordinator/src/tests/listener_tests.rs`

**Tasks:**

- [ ] Implement MVM event listener (IntentCreated, IntentFulfilled, etc.)
- [ ] Implement EVM event listener (EscrowCreated, ValidationSucceeded, etc.)
- [ ] Implement SVM event listener (account changes)
- [ ] Store events in database as they arrive
- [ ] Test event parsing for each chain

**Test:**

```bash
nix develop ./nix -c bash -c "cd coordinator && cargo test -- --test listener_tests"
```

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
nix develop ./nix -c bash -c "cd coordinator && cargo test -- --test api_tests"
```

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
nix develop ./nix -c bash -c "cd coordinator && cargo test -- --test websocket_tests"
```

---

### Commit 5: Update frontend to use coordinator API

**Files:**

- `frontend/src/services/coordinator.ts`
- `frontend/src/hooks/useIntents.ts`
- `frontend/src/hooks/useEscrows.ts`
- `frontend/src/tests/coordinator.test.ts`

**Tasks:**

- [ ] Create coordinator API client
- [ ] Replace verifier API calls with coordinator API calls
- [ ] Add WebSocket connection for real-time updates
- [ ] Update UI to show GMP message status
- [ ] Test API client with mocked responses

**Test:**

```bash
nix develop ./nix -c bash -c "cd frontend && npm test -- --grep 'coordinator'"
```

---

### Commit 6: Update solver SDK to use coordinator API

**Files:**

- `solver/src/coordinator_client.rs`
- `solver/src/tests/coordinator_client_tests.rs`

**Tasks:**

- [ ] Create coordinator API client for solver
- [ ] Replace verifier API calls with coordinator API calls
- [ ] Add intent discovery via coordinator
- [ ] Add escrow status polling
- [ ] **Add validation contract discovery** - query coordinator API for validation contract addresses on each connected chain
- [ ] **Support GMP flow** - update solver to call validation contract functions instead of arbitrary transfers
- [ ] **Add approval step** - solver must approve validation contract before calling fulfillment function
- [ ] Test API client with mocked responses

**Test:**

```bash
nix develop ./nix -c bash -c "cd solver && cargo test -- --test coordinator_client_tests"
```

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
docker-compose up -d coordinator
curl http://localhost:8080/health
docker-compose down
```

---

## Run All Tests

```bash
# Coordinator unit tests
nix develop ./nix -c bash -c "cd coordinator && cargo test"

# Frontend integration tests
nix develop ./nix -c bash -c "cd frontend && npm test -- --grep 'coordinator'"

# Solver integration tests
nix develop ./nix -c bash -c "cd solver && cargo test -- --test coordinator_client_tests"

# Docker smoke test
docker-compose up -d coordinator && curl http://localhost:8080/health && docker-compose down
```

---

## Documentation Update

At the end of Phase 4, update:

- [ ] `docs/coordinator/README.md` - Coordinator service overview
- [ ] `docs/coordinator/api-reference.md` - Full API documentation
- [ ] `docs/solver/migration-guide.md` - How solvers migrate to GMP flow
- [ ] `docker-compose.yml` - Document coordinator + PostgreSQL setup
- [ ] Review conception documents for accuracy after changes
- [ ] Check if other files reference old verifier API and update them

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
