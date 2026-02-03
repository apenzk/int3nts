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
| `inflow_escrow_gmp` | Receives requirements, creates escrow, sends confirmation | MVM as connected chain (inflow) |
| `outflow_validator` | Receives requirements, validates solver fulfillment, sends proof | MVM as connected chain (outflow) |

### Tasks

- [ ] **Commit 1: Audit MVM connected chain modules**
  - Review `inflow_escrow_gmp.move` dependencies
  - Review `outflow_validator.move` dependencies
  - Identify shared code with hub modules
  - Document minimal required dependencies
  - Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

- [ ] **Commit 2: Split MVM package into three separate packages (REQUIRED)**
  - **NOTE: This is now REQUIRED, not optional.** The combined MVM package is 108KB, exceeding Movement's 60KB single-transaction limit. While `--chunked-publish` works as a temporary workaround, splitting into separate packages is the proper solution.
  - Create three packages with the following dependency structure:
    - `mvmt_intent_gmp` is the base layer (deploy first)
    - `mvmt_intent_hub` and `mvmt_intent_connected` both depend on `mvmt_intent_gmp`
  - **`mvmt_intent_gmp`** - GMP infrastructure (deploy to both hub and connected chains)
    - gmp_common (message encoding/decoding)
    - gmp_sender (outbound message sending)
    - native_gmp_endpoint (inbound message receiving)
  - **`mvmt_intent_hub`** - Hub-only modules (deploy to hub chain only)
    - fa_intent, fa_intent_with_oracle
    - fa_intent_inflow, fa_intent_outflow
    - intent_gmp_hub, solver_registry, intent_registry
    - Depends on: mvmt_intent_gmp
  - **`mvmt_intent_connected`** - Connected chain modules (deploy to connected MVM chains only)
    - outflow_validator, outflow_validator_impl
    - inflow_escrow_gmp
    - Depends on: mvmt_intent_gmp
  - Update deployment scripts:
    - Hub chain: deploy mvmt_intent_gmp first, then mvmt_intent_hub
    - Connected chain: deploy mvmt_intent_gmp first, then mvmt_intent_connected
  - Verify each package is under 60KB limit
  - Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

- [ ] **Commit 3: Minimize connected chain module dependencies**
  - Remove any hub-only dependencies from connected chain modules
  - Ensure connected chain modules only import what they need
  - Update tests to verify isolation
  - Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

**Files to analyze:**

- `intent-frameworks/mvm/sources/gmp/inflow_escrow_gmp.move`
- `intent-frameworks/mvm/sources/gmp/outflow_validator.move`
- `intent-frameworks/mvm/sources/gmp/gmp_common.move`
- `intent-frameworks/mvm/sources/gmp/native_gmp_endpoint.move`

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

- [ ] **Commit 4: Document current intent type differences**
  - List all fields in `FungibleAssetLimitOrder`
  - List all fields in `OracleGuardedLimitOrder`
  - Identify overlap and differences
  - Document security implications of each field
  - Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

- [ ] **Commit 5: Prototype conditional oracle approach**
  - Create test branch with `oracle_required` flag
  - Implement conditional check in finish functions
  - Write security tests (attempt bypass without flag)
  - Document findings
  - Run `/review-tests-new` then `/review-commit-tasks` then `/commit` to finalize

- [ ] **Commit 6: Write recommendation document**
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

## Run All Tests

```bash
# Run all unit tests (includes coordinator, trusted-gmp, solver, MVM, EVM, SVM, frontend)
./testing-infra/run-all-unit-tests.sh
```

> **Note:** This phase is primarily research/review. Code changes are exploratory and may not be merged.

---

## Deliverables

1. **Part A Deliverable:** Assessment of MVM connected chain module isolation
   - Dependency audit report
   - Recommendation on package structure
   - Refactored modules (if beneficial)

2. **Part B Deliverable:** Intent unification recommendation document
   - Current state analysis
   - Approach comparison (with code examples)
   - Security analysis
   - Final recommendation with rationale

---

## Exit Criteria

- [ ] Part A: MVM connected chain modules audited and documented
- [ ] Part A: Recommendation on package structure documented
- [ ] Part B: All three approaches analyzed with security implications
- [ ] Part B: Prototype of conditional oracle approach (test branch)
- [ ] Part B: Final recommendation document written
- [ ] All existing tests still pass (no regressions)
