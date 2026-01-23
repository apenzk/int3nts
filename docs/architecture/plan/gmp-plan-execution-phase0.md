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

## Commits

### Commit 1: Extract Coordinator Service

**Files:**

- `coordinator/src/main.rs`
- `coordinator/src/monitor/` (moved from `verifier/src/monitor/`)
- `coordinator/src/api/` (moved from `verifier/src/api/`)
- `coordinator/src/storage/` (moved from `verifier/src/storage/`)
- `coordinator/Cargo.toml`

**Tasks:**

- [ ] Create new `coordinator/` crate
- [ ] Move event monitoring logic from verifier (no validation, just monitoring)
- [ ] Move REST API from verifier (no signature endpoints, just read-only)
- [ ] Move event caching/storage from verifier
- [ ] Remove all cryptographic operations (no keys, no signing)
- [ ] Remove all validation logic (contracts will handle this)
- [ ] Keep negotiation API (application logic, not security-critical)
- [ ] Update configuration to remove key-related settings
- [ ] Test coordinator can monitor events and serve API without keys

**Test:**

```bash
# Build coordinator
nix develop ./nix -c bash -c "cd coordinator && cargo build"

# Test coordinator API (should work without keys)
nix develop ./nix -c bash -c "cd coordinator && cargo test"
```

---

### Commit 2: Extract Trusted GMP Service

**Files:**

- `trusted-gmp/src/main.rs`
- `trusted-gmp/src/monitor/gmp_events.rs`
- `trusted-gmp/src/delivery/` (message delivery logic)
- `trusted-gmp/Cargo.toml`

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

**Test:**

```bash
# Build trusted-gmp
nix develop ./nix -c bash -c "cd trusted-gmp && cargo build"

# Test message delivery
nix develop ./nix -c bash -c "cd trusted-gmp && cargo test"
```

---

### Commit 3: Remove Old Verifier, Keep Coordinator + Trusted GMP Only

**Files:**

- `verifier/` (DELETED - old verifier is completely removed)
- `coordinator/src/main.rs` (standalone service)
- `trusted-gmp/src/main.rs` (standalone service)

**Tasks:**

- [ ] Delete the old `verifier/` crate entirely
- [ ] Coordinator runs as standalone service (no verifier dependency)
- [ ] Trusted GMP runs as standalone service (no verifier dependency)
- [ ] Remove all validation logic (contracts handle this now)
- [ ] Remove all signature generation (GMP handles authorization now)
- [ ] Remove all private key configuration
- [ ] Update CI/CD to deploy coordinator + trusted-gmp instead of verifier

**Test:**

```bash
# Verify old verifier is removed
test ! -d verifier && echo "Old verifier removed"

# Test coordinator standalone
nix develop ./nix -c bash -c "cd coordinator && cargo test"

# Test trusted-gmp standalone
nix develop ./nix -c bash -c "cd trusted-gmp && cargo test"
```

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
# Run all Phase 0 integration tests
nix develop ./nix -c bash -c "cd testing-infra/ci-e2e/phase0-tests && cargo test"
```

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
