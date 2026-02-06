# Phase 3: EVM Expansion & Cross-Chain Architecture Alignment

**Status:** In Progress (Commits 1-5 Complete, Commits 6-10 Pending)
**Depends On:** Phase 2
**Blocks:** Phase 4

**Goal:** Add EVM connected chain support, separate MVM packages for hub vs connected chain deployment, align naming conventions across all three VMs, optimize the GMP escrow release flow, investigate intent type unification, and optimize SVM build performance.

---

## Commits

> 📋 **Commit Conventions:** Before each commit, review `.claude/CLAUDE.md` and `.cursor/rules` for commit message format, test requirements, and coding standards.

### Commit 1: Audit MVM connected chain modules ✅

- Review `intent_inflow_escrow.move` dependencies
- Review `intent_outflow_validator.move` dependencies
- Identify shared code with hub modules
- Document minimal required dependencies
- See: `gmp-phase6-audit-mvm-connected-chain.md`

---

### Commit 2: Split MVM package into three separate packages ✅

- Created three packages:
  - **`intent-gmp`** (8KB bytecode, 16KB deploy) - gmp_common, gmp_sender, gmp_intent_state, gmp_endpoints
  - **`intent-hub`** (35KB bytecode, 75KB deploy) - All core intent modules + hub-specific intent_gmp
  - **`intent-connected`** (14KB bytecode, 14KB deploy) - intent_outflow_validator, intent_inflow_escrow + connected-specific intent_gmp
- Removed `is_initialized()` conditional routing - missing init is now a hard failure
- Updated deployment scripts (hub deploys intent-gmp then intent-hub with `--chunked-publish`)
- **Note:** intent-hub still exceeds 60KB (75KB) and requires `--chunked-publish`
- All 164 MVM tests passing across 3 packages

---

### Commit 3: Rename SVM programs for consistency ✅

- **SVM renames completed:**
  - Renamed `native-gmp-endpoint` → `intent-gmp`
  - Renamed `outflow-validator` → `intent-outflow-validator`
  - Final SVM structure (2 logical groups, 3 programs):
    - **`intent-gmp`** - GMP infrastructure
    - **`intent-connected`** = `intent-escrow` + `intent-outflow-validator` (2 programs, logically grouped)
  - Note: Unlike Move, Solana cannot bundle programs into packages - each is deployed separately
- **EVM:** NativeGmpEndpoint.sol and OutflowValidator.sol do not exist yet (skipped)
- Updated Cargo.toml, Rust imports, build.sh, test.sh

---

### Commit 4: Align EVM architecture with MVM/SVM patterns ✅

Created EVM contracts following the same three-package structure as MVM and SVM, plus native GMP relay support and E2E test updates.

**EVM contracts created:**

- `IntentGmp.sol` - GMP infrastructure (like MVM intent-gmp, SVM intent-gmp)
  - `send()` function emits `MessageSent` event
  - `deliverMessage()` for relay to inject messages
  - Trusted remote verification and relay authorization
  - Message nonce tracking for replay protection
- `IntentOutflowValidator.sol` - Outflow validation (like MVM/SVM intent-outflow-validator)
  - `receiveMessage()` receives intent requirements from hub (idempotent)
  - `fulfillIntent()` for authorized solvers (pulls tokens via `transferFrom`, validates, forwards, sends GMP)
- `IntentInflowEscrow.sol` - Escrow for inflow (like SVM intent-inflow-escrow)
  - `receiveMessage()` for intent requirements from hub (idempotent)
  - `createEscrowWithValidation()` validates requirements match escrow details
  - Auto-release on fulfillment proof receipt
  - Sends `EscrowConfirmation` back to hub on creation
- `gmp-common/Messages.sol` + `gmp-common/Endpoints.sol` - Shared message encoding/decoding

**Native GMP relay extended for EVM:**

- EVM event parsing for `MessageSent` in `native_gmp_relay.rs`
- EVM message delivery via `deliverMessage()`
- EVM chain config in relay (similar to MVM/SVM)

**Solver + E2E updates:**

- `solver/src/chains/connected_evm.rs` - added `fulfill_outflow_via_gmp()`
- `solver/src/service/outflow.rs` - EVM uses GMP flow
- E2E deployment scripts deploy IntentGmp + IntentInflowEscrow + IntentOutflowValidator
- E2E inflow and outflow tests updated for GMP flow

**Tests:** EVM 161 unit tests, Solver 149, Trusted-GMP 191

---

### Commit 5: Auto-release escrow on FulfillmentProof receipt (GMP flow) ✅

- Collapsed two-step release into single step matching SVM behavior
- Changes made:
  - `intent_inflow_escrow.move`: `receive_fulfillment_proof` now transfers tokens to solver and marks both fulfilled+released
  - `intent_inflow_escrow.move`: `release_escrow` kept as manual fallback
  - `solver/src/service/inflow.rs`: `release_mvm_gmp_escrow` now polls `is_escrow_released` (no manual release call)
  - `solver/src/chains/connected_mvm.rs`: replaced `is_escrow_fulfilled` with `is_escrow_released`, marked `release_gmp_escrow` as dead code
  - Updated 5 Move tests to reflect auto-release behavior
  - E2E tests already poll `is_released` - no changes needed (release happens faster now)

---

### Commit 6: Document current intent type differences

Investigate whether hub intents can be unified into a single base type while maintaining security guarantees.

**Background:**

The hub chain (MVM) uses two different intent types:

| Flow | Intent Type | Module | Security Gate |
|------|-------------|--------|---------------|
| **Inflow** | `FungibleAssetLimitOrder` | `fa_intent` | Escrow confirmation check in wrapper |
| **Outflow** | `OracleGuardedLimitOrder` | `fa_intent_with_oracle` | Oracle witness at type level |

`OracleGuardedLimitOrder` provides **defense-in-depth**: even if someone bypasses the wrapper function and calls lower-level `fa_intent_with_oracle` functions directly, they still need an oracle witness. `FungibleAssetLimitOrder` does NOT have this protection.

**Potential approaches:**

1. **Conditional Oracle Requirement** - Add `oracle_required: bool` field (simpler, but runtime check)
2. **Separate Finish Functions (Current)** - Keep separate types, share more code (type-safe, more duplication)
3. **Generic Intent with Pluggable Validation** - `GenericLimitOrder<V>` (flexible, but complex)

**Tasks:**

- [ ] List all fields in `FungibleAssetLimitOrder`
- [ ] List all fields in `OracleGuardedLimitOrder`
- [ ] Identify overlap and differences
- [ ] Document security implications of each field
- [ ] Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

**Files to analyze:**

- `intent-frameworks/mvm/intent-hub/sources/fa_intent.move`
- `intent-frameworks/mvm/intent-hub/sources/fa_intent_with_oracle.move`
- `intent-frameworks/mvm/intent-hub/sources/fa_intent_inflow.move`
- `intent-frameworks/mvm/intent-hub/sources/fa_intent_outflow.move`

---

### Commit 7: Prototype conditional oracle approach

- [ ] Create test branch with `oracle_required` flag
- [ ] Implement conditional check in finish functions
- [ ] Write security tests (attempt bypass without flag)
- [ ] Document findings
- [ ] Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

---

### Commit 8: Write intent unification recommendation document

- [ ] Compare approaches with concrete code examples
- [ ] Security analysis of each approach
- [ ] Recommendation with rationale
- [ ] Migration path if unification is recommended
- [ ] Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

---

### Commit 9: Profile SVM Docker build and document bottlenecks

The SVM Docker build is slow. Suspected bottlenecks:

1. **Solana CLI downloaded fresh every Docker run** (~200MB+)
2. **Platform-tools downloaded for each cargo build-sbf call**
3. **Toolchain re-registration happening 3 times** due to cargo-build-sbf bug workaround
4. **No cargo cache between Docker runs**

**Tasks:**

- [ ] Time each phase (Solana install, platform-tools download, compilation)
- [ ] Measure download sizes
- [ ] Identify what's re-downloaded vs cached
- [ ] Document findings
- [ ] Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

**Files to analyze:**

- `intent-frameworks/svm/scripts/build-with-docker.sh`
- `intent-frameworks/svm/scripts/build.sh`

---

### Commit 10: Implement SVM build optimizations

- [ ] Based on profiling results, implement improvements:
  - Pre-built Docker image with Solana CLI?
  - Volume mounts for caches (~/.cache/solana, cargo registry)?
  - Single cargo build for all programs?
  - Fix root cause of toolchain bug instead of workaround?
- [ ] Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

---

## Key Files

- `intent-frameworks/evm/contracts/IntentGmp.sol`
- `intent-frameworks/evm/contracts/IntentInflowEscrow.sol`
- `intent-frameworks/evm/contracts/IntentOutflowValidator.sol`
- `intent-frameworks/evm/contracts/gmp-common/Messages.sol`
- `intent-frameworks/evm/contracts/gmp-common/Endpoints.sol`
- `intent-frameworks/mvm/intent-gmp/` (MVM GMP package)
- `intent-frameworks/mvm/intent-hub/` (MVM hub package)
- `intent-frameworks/mvm/intent-connected/` (MVM connected chain package)
- `intent-frameworks/svm/programs/intent-gmp/` (renamed from native-gmp-endpoint)
- `intent-frameworks/svm/programs/intent_escrow/`
- `intent-frameworks/svm/programs/intent-outflow-validator/` (renamed from outflow-validator)
- `trusted-gmp/src/native_gmp_relay.rs`

---

## Run All Tests

```bash
# Run all unit tests (includes coordinator, trusted-gmp, solver, MVM, EVM, SVM, frontend)
./testing-infra/run-all-unit-tests.sh
```

---

## Exit Criteria

- [x] MVM connected chain modules audited and documented
- [x] MVM package split into 3 packages (intent-gmp, intent-hub, intent-connected)
- [x] SVM programs renamed for consistency (intent-gmp, intent-outflow-validator)
- [x] EVM contracts created with consistent naming (IntentGmp, IntentInflowEscrow, IntentOutflowValidator)
- [x] Native GMP relay supports all three chain types (MVM, SVM, EVM)
- [x] Cross-chain E2E tests pass (MVM ↔ EVM outflow + inflow)
- [x] MVM escrow auto-releases on FulfillmentProof (matches SVM behavior)
- [ ] Intent unification: all three approaches analyzed with security implications
- [ ] Intent unification: prototype of conditional oracle approach
- [ ] Intent unification: final recommendation document written
- [ ] SVM build: Docker build profiled and bottlenecks documented
- [ ] SVM build: optimizations implemented (if beneficial)
- [x] All existing tests still pass (no regressions)
