# Phase 6: Intent Unification Review (1-2 days)

**Status:** Not Started
**Depends On:** Phase 5
**Blocks:** None (Review Phase)

**Goal:** Assess architectural improvements to simplify the intent framework by (A) separating and minimizing MVM connected chain contracts, and (B) investigating whether hub intents can be unified into a single base type.

---

## Background

### Current State

The hub chain (MVM) uses two different intent types:

| Flow | Intent Type | Module | Security Gate |
|------|-------------|--------|---------------|
| **Inflow** | `FungibleAssetLimitOrder` | `fa_intent` | Escrow confirmation check in wrapper |
| **Outflow** | `OracleGuardedLimitOrder` | `fa_intent_with_oracle` | Oracle witness at type level |

### Why Two Types Exist

1. **Inflow**: Tokens locked on connected chain, desired on hub. Solver provides tokens on hub. No oracle needed - escrow confirmation via GMP is sufficient.

2. **Outflow**: Tokens locked on hub, desired on connected chain. Solver delivers on connected chain. Requires proof of delivery before releasing hub tokens.

### Security Consideration

`OracleGuardedLimitOrder` provides **defense-in-depth**: even if someone bypasses the wrapper function and calls lower-level `fa_intent_with_oracle` functions directly, they still need an oracle witness (or must use the `_for_gmp` variant which is only called after GMP proof check).

`FungibleAssetLimitOrder` does NOT have this protection - the security gate is only in the wrapper function.

---

## Part A: Separate MVM Connected Chain Contracts

### Objective

Minimize and isolate the MVM connected chain contracts (used when MVM acts as a connected chain, not the hub).

### Current MVM Connected Chain Modules

| Module | Purpose | Used By |
|--------|---------|---------|
| `inflow_escrow_gmp` | Receives requirements from hub, creates escrow on connected chain, sends confirmation to hub | MVM as connected chain (inflow) |
| `intent_outflow_validator` | Receives requirements from hub, validates solver fulfillment on connected chain, sends proof to hub | MVM as connected chain (outflow) |

### Tasks

- [ ] **Commit 1: Audit MVM connected chain modules**
  - Review `inflow_escrow_gmp.move` dependencies
  - Review `intent_outflow_validator.move` dependencies (currently named `outflow_validator.move`)
  - Identify shared code with hub modules
  - Document minimal required dependencies
  - Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

- [ ] **Commit 2: Split MVM package into three separate packages (REQUIRED)**
  - **NOTE: This is now REQUIRED, not optional.** The combined MVM package is 108KB, exceeding Movement's 60KB single-transaction limit. While `--chunked-publish` works as a temporary workaround, splitting into separate packages is the proper solution.
  - Create three packages with the following dependency structure:
    - `intent_gmp` is the base layer (deploy first)
    - `intent_hub` and `intent_connected` both depend on `intent_gmp`
  - **`intent_gmp`** - GMP infrastructure (deploy to both hub and connected chains)
    - gmp_common (message encoding/decoding)
    - gmp_sender (outbound message sending)
    - native_gmp_endpoint (inbound message receiving — currently shared, see note below)
  - **`intent_hub`** - Hub-only modules (deploy to hub chain only)
    - fa_intent, fa_intent_with_oracle
    - fa_intent_inflow, fa_intent_outflow
    - intent_gmp_hub, solver_registry, intent_registry
    - Hub-specific native_gmp_endpoint route_message (routes to intent_gmp_hub only)
    - Depends on: intent_gmp
  - **`intent_connected`** - Connected chain modules (deploy to connected MVM chains only)
    - intent_outflow_validator, intent_outflow_validator_impl
    - inflow_escrow_gmp
    - Connected-chain-specific native_gmp_endpoint route_message (routes to intent_outflow_validator_impl + inflow_escrow_gmp)
    - Depends on: intent_gmp
  - **NOTE:** Once split, remove the `is_initialized()` conditional routing in `native_gmp_endpoint::route_message`. Currently the hub and connected chain share one `native_gmp_endpoint` with conditional checks because both deploy the same code. After the split, each package gets its own routing with unconditional calls — no fallbacks, missing init is a hard failure.
  - Update deployment scripts:
    - Hub chain: deploy intent_gmp first, then intent_hub
    - Connected chain: deploy intent_gmp first, then intent_connected
  - Verify each package is under 60KB limit
  - Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

- [ ] **Commit 3: Rename SVM and EVM programs for consistency**
  - **SVM renames:**
    - Rename `native-gmp-endpoint` → `intent-gmp`
    - Rename `outflow-validator` → `intent-outflow-validator`
    - Final SVM structure (2 logical groups, 3 programs):
      - **`intent-gmp`** - GMP infrastructure
      - **`intent-connected`** = `intent-escrow` + `intent-outflow-validator` (2 programs, logically grouped)
    - Note: Unlike Move, Solana cannot bundle programs into packages - each is deployed separately
  - **EVM renames:**
    - Rename `NativeGmpEndpoint.sol` → `IntentGmp.sol`
    - Rename `OutflowValidator.sol` → `IntentOutflowValidator.sol`
  - Update all references in code, scripts, and tests
  - Verify each program/contract can be deployed independently
  - Document deployment order in scripts
  - Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

- [ ] **Commit 4: Minimize connected chain module dependencies**
  - Remove any hub-only dependencies from connected chain modules (MVM + SVM)
  - Ensure connected chain modules only import what they need
  - Update tests to verify isolation
  - Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

- [ ] **Commit 5: Auto-release escrow on FulfillmentProof receipt (GMP flow)**
  - Currently escrow release is two steps: (1) `receive_fulfillment_proof` marks `fulfilled=true`, (2) solver calls `release_escrow` separately
  - Collapse into single step: `receive_fulfillment_proof` also transfers tokens to the solver
  - The solver address is already in the GMP FulfillmentProof payload — no extra data needed
  - Changes:
    - `inflow_escrow_gmp.move`: `receive_fulfillment_proof` calls release logic internally after marking fulfilled
    - `inflow_escrow_gmp.move`: `release_escrow` entry function can be removed (or kept as manual fallback)
    - `solver/src/service/inflow.rs`: remove `release_mvm_gmp_escrow` polling loop and `release_escrow` call for MVM
    - `solver/src/chains/connected_mvm.rs`: remove `release_gmp_escrow` and `is_escrow_fulfilled` if no longer needed
    - Update E2E `wait-for-escrow-claim.sh` timeout if needed (release happens faster now)
    - Update all affected Move and Rust tests
  - Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

**Files to analyze:**

- `intent-frameworks/mvm/sources/gmp/inflow_escrow_gmp.move`
- `intent-frameworks/mvm/sources/gmp/outflow_validator.move` (rename to `intent_outflow_validator.move`)
- `intent-frameworks/mvm/sources/gmp/gmp_common.move`
- `intent-frameworks/mvm/sources/gmp/native_gmp_endpoint.move`
- `intent-frameworks/svm/programs/native-gmp-endpoint/` (rename to `intent-gmp/`)
- `intent-frameworks/svm/programs/intent_escrow/`
- `intent-frameworks/svm/programs/outflow-validator/` (rename to `intent-outflow-validator/`)
- `intent-frameworks/evm/contracts/gmp/NativeGmpEndpoint.sol` (rename to `IntentGmp.sol`)
- `intent-frameworks/evm/contracts/OutflowValidator.sol` (rename to `IntentOutflowValidator.sol`)

---

## Part B: Investigate Intent Unification

### Objective

Determine if hub intents can use a single base type while maintaining security guarantees.

### Research Questions

1. **Can `OracleGuardedLimitOrder` be the basis for both flows?**
   - Inflow: Oracle witness not required (escrow confirmation is sufficient)
   - Outflow: Oracle witness required (GMP proof of delivery)
   - Is there a way to make the oracle requirement conditional?

2. **What modifications would be needed to `fa_intent_with_oracle`?**
   - Add a "skip oracle" flag checked at type level?
   - Add different finish functions for different flows?
   - Security implications of each approach?

3. **What are the trade-offs?**
   - Code simplification vs security guarantees
   - Type-level safety vs runtime checks
   - Developer experience vs attack surface

### Potential Approaches

#### Approach 1: Conditional Oracle Requirement

Add a field to `OracleGuardedLimitOrder` that indicates whether oracle verification is required:

```move
struct OracleGuardedLimitOrder has store, drop {
    // ... existing fields ...
    oracle_required: bool,  // false for inflow, true for outflow
}
```

**Pros:**
- Single intent type for both flows
- Simpler mental model

**Cons:**
- Runtime check instead of type-level enforcement
- Must audit all code paths to ensure flag is respected
- Potential for misconfiguration

#### Approach 2: Separate Finish Functions (Current)

Keep separate types but investigate if they can share more code:

```move
// For inflow (no oracle)
finish_fa_receiving_session_with_event()

// For outflow (oracle required)
finish_fa_receiving_session_with_oracle()
finish_fa_receiving_session_for_gmp()
```

**Pros:**
- Type-level enforcement
- Clear separation of concerns
- Defense-in-depth

**Cons:**
- Two intent types to maintain
- More code duplication

#### Approach 3: Generic Intent with Pluggable Validation

Create a generic intent type with pluggable validation:

```move
struct GenericLimitOrder<V: store + drop> has store, drop {
    // ... common fields ...
    validator: V,  // NoValidator, OracleValidator, GmpValidator, etc.
}
```

**Pros:**
- Maximum flexibility
- Type-safe validation
- Extensible

**Cons:**
- More complex implementation
- Higher learning curve
- May be over-engineered for current needs

### Tasks

- [ ] **Commit 6: Document current intent type differences**
  - List all fields in `FungibleAssetLimitOrder`
  - List all fields in `OracleGuardedLimitOrder`
  - Identify overlap and differences
  - Document security implications of each field
  - Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

- [ ] **Commit 7: Prototype conditional oracle approach**
  - Create test branch with `oracle_required` flag
  - Implement conditional check in finish functions
  - Write security tests (attempt bypass without flag)
  - Document findings
  - Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

- [ ] **Commit 8: Write recommendation document**
  - Compare approaches with concrete code examples
  - Security analysis of each approach
  - Recommendation with rationale
  - Migration path if unification is recommended
  - Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

**Files to analyze:**

- `intent-frameworks/mvm/sources/fa_intent.move`
- `intent-frameworks/mvm/sources/fa_intent_with_oracle.move`
- `intent-frameworks/mvm/sources/fa_intent_inflow.move`
- `intent-frameworks/mvm/sources/fa_intent_outflow.move`

---

## Part C: SVM Build Performance

### Objective

The SVM Docker build is slow. Research bottlenecks and identify optimization opportunities.

### Current Bottlenecks (Suspected)

1. **Solana CLI downloaded fresh every Docker run** (~200MB+)
2. **Platform-tools downloaded for each cargo build-sbf call**
3. **Toolchain re-registration happening 3 times** due to cargo-build-sbf bug workaround
4. **No cargo cache between Docker runs**

### Tasks

- [ ] **Commit 9: Profile SVM Docker build and document bottlenecks**
  - Time each phase (Solana install, platform-tools download, compilation)
  - Measure download sizes
  - Identify what's re-downloaded vs cached
  - Document findings
  - Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

- [ ] **Commit 10: Implement SVM build optimizations**
  - Based on profiling results, implement improvements:
    - Pre-built Docker image with Solana CLI?
    - Volume mounts for caches (~/.cache/solana, cargo registry)?
    - Single cargo build for all programs?
    - Fix root cause of toolchain bug instead of workaround?
  - Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

**Files to analyze:**

- `intent-frameworks/svm/scripts/build-with-docker.sh`
- `intent-frameworks/svm/scripts/build.sh`

---

## Run All Tests

```bash
# Run all unit tests (includes coordinator, trusted-gmp, solver, MVM, EVM, SVM, frontend)
./testing-infra/run-all-unit-tests.sh
```

> **Note:** This phase is primarily research/review. Code changes are exploratory and may not be merged.

---

## Deliverables

1. **Part A Deliverable:** Assessment of MVM and SVM connected chain module isolation
   - Dependency audit report
   - Recommendation on package structure (MVM + SVM)
   - Refactored modules/programs (if beneficial)

2. **Part B Deliverable:** Intent unification recommendation document
   - Current state analysis
   - Approach comparison (with code examples)
   - Security analysis
   - Final recommendation with rationale

---

## Exit Criteria

- [ ] Part A: MVM and SVM connected chain modules audited and documented
- [ ] Part A: MVM package split into 3 packages (intent_gmp, intent_hub, intent_connected)
- [ ] Part A: SVM program structure documented (gmp + connected logical grouping)
- [ ] Part A: Recommendation on package structure documented
- [ ] Part B: All three approaches analyzed with security implications
- [ ] Part B: Prototype of conditional oracle approach (test branch)
- [ ] Part B: Final recommendation document written
- [ ] All existing tests still pass (no regressions)
