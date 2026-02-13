# Framework Extension Guide

This guide explains how to add a new blockchain framework (e.g., MVM, EVM, SVM) to the Intent Framework while maintaining consistency and test coverage across all platforms.

## Overview

When adding a new framework, you must:

1. **Create placeholder test files first** - before implementing any tests, create empty test files with placeholder headers and test descriptions matching existing frameworks
2. **Replicate the core escrow functionality** from existing frameworks
3. **Maintain test alignment** - each test should have a corresponding test in the same position across all frameworks
4. **Use generic test descriptions** - avoid platform-specific terminology
5. **Document platform differences** - use N/A comments for tests that don't apply to your platform
6. **Follow consistent structure** - use the same test file organization and section headers

## Step 1: Create Placeholder Test Files

**CRITICAL: Before implementing any functionality, create empty test files with placeholders.**

This establishes the baseline of test names and numbers from the start, preventing misalignment.

### Procedure

1. **Choose a reference framework** (e.g., SVM if adding MVM tests)
2. **For each test file in the reference framework**, create an equivalent empty file in your new framework
3. **Copy all test headers and documentation** from the reference framework
4. **Add placeholder implementations** (empty test bodies or TODO markers)
5. **Mark N/A tests** with inline comments explaining why they don't apply

### Example: Creating Placeholder Test File

**Reference (SVM):** `intent-frameworks/svm/programs/intent_inflow_escrow/tests/gmp.rs`

**New (MVM):** `intent-frameworks/mvm/tests/intent_inflow_escrow_tests.move`

```move
#[test_only]
module mvmt_intent::intent_inflow_escrow_tests {
    // ... imports and helpers ...

    // ============================================================================
    // GMP CONFIG TESTS
    // ============================================================================

    /// 1. Test: SetGmpConfig creates/updates GMP configuration
    /// Verifies that admin can set GMP config with hub chain ID, hub GMP endpoint address, and endpoint.
    /// Why: GMP config is required for source validation in all GMP message handlers.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent)]
    fun test_set_gmp_config(aptos_framework: &signer, admin: &signer) {
        // TODO: Implement
        abort 999
    }

    /// 2. Test: SetGmpConfig rejects unauthorized caller
    /// Verifies that only admin can update GMP config after initial setup.
    /// Why: GMP config controls authorized sources - must be admin-only.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent)]
    fun test_set_gmp_config_rejects_unauthorized(aptos_framework: &signer, admin: &signer) {
        // TODO: Implement
        abort 999
    }

    // ============================================================================
    // LZ RECEIVE REQUIREMENTS TESTS
    // ============================================================================

    /// 3. Test: GmpReceiveRequirements stores intent requirements
    /// Verifies that requirements from hub are stored correctly.
    /// Why: Requirements must be stored before escrow can be created with validation.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent)]
    fun test_gmp_receive_requirements_stores_requirements(aptos_framework: &signer, admin: &signer) {
        // TODO: Implement
        abort 999
    }

    // ... continue for all tests 4-13 ...
}
```

### Benefits of Placeholder Files

1. **Clear baseline**: Everyone knows exactly which tests are expected and their numbering
2. **Prevents drift**: Tests can't be numbered incorrectly if placeholders exist first
3. **Easy tracking**: Reviewers can easily see which tests are implemented vs placeholders
4. **Forces alignment**: Developers must consciously decide if a test is N/A rather than accidentally omitting it

### Placeholder Markers

Use these patterns for placeholder tests:

**Move:**

```move
fun test_example() {
    // TODO: Implement
    abort 999
}
```

**Rust:**

```rust
#[tokio::test]
async fn test_example() {
    todo!("Implement test")
}
```

**JavaScript/TypeScript:**

```javascript
it("Should test example", async function () {
    // TODO: Implement
    throw new Error("Not implemented");
});
```

## Test Structure Requirements

### Test File Organization

Each framework should have the following test files, matching the order and structure of existing frameworks:

1. **initialization** - Basic setup and escrow creation
2. **deposit** - Escrow creation and deposit functionality
3. **claim** - Claiming escrow funds with integrated-gmp signatures
4. **cancel** - Cancellation and refund functionality
5. **expiry** - Expiry timestamp handling and expired escrow behavior
6. **cross-chain** - Intent ID conversion and cross-chain compatibility
7. **edge-cases** - Boundary values, concurrent operations, gas/compute limits
8. **error-conditions** - Error handling and validation
9. **integration** - Full lifecycle workflows
10. **scripts** - Utility script testing (if applicable)

### Section Headers

See [Test File Section Headers](../../architecture/codestyle-testing.md#11-test-file-section-headers) in the coding guide for section header formatting guidelines.

### Test Documentation Format

See [Test Function Documentation](../../architecture/codestyle-testing.md#10-test-function-documentation) in the coding guide for the required `Test:` / `Verifies` / `Why:` format.

### Test Descriptions

**Use generic, platform-appropriate terminology:**

✅ **Good:**

- "Verifies that escrows cannot be created with zero amount"
- "Verifies that the program handles boundary intent ID values correctly"
- "Verifies that escrow creation fails if requester has insufficient tokens"

❌ **Bad:**

- "Verifies that createEscrow reverts when ERC20 allowance is insufficient" (too EVM-specific)
- "Verifies that intent IDs from Aptos hex format can be converted to EVM uint256" (mentions other platforms)
- "Verifies that the contract handles boundary values" (use "program" for SVM, "contract" for EVM)

### Test Order and Numbering

**Maintain the exact same test order and numbering across all frameworks.** This ensures:

- Easy comparison between frameworks
- Consistent test numbering
- Clear alignment of functionality

**Numbering format:**

- Each test should be numbered: `1. Test:`, `2. Test:`, etc.
- Numbers must match across all frameworks at the same position
- If a test is N/A for a framework, it still gets the same number with an N/A comment

## Test Alignment Reference

> **⚠️ IMPORTANT: When adding a new framework, ensure maximal completeness by implementing all tests listed in the respective completeness files below.**

These lists track alignment status by component category. The detailed test lists have been split into separate files located in their respective test directories for easier access during development.

All tests listed are VM-specific; generic tests are intentionally excluded because they are not relevant when integrating a new VM.

### VM Intent Framework

Escrow test alignment for VM intent framework contracts:

- See [`intent-frameworks/extension-checklist.md`](../../intent-frameworks/extension-checklist.md)

### Integrated GMP

Test alignment for the integrated-gmp service:

- See [`integrated-gmp/tests/extension-checklist.md`](../../integrated-gmp/tests/extension-checklist.md)

### Solver

Test alignment for the solver:

- See [`solver/tests/extension-checklist.md`](../../solver/tests/extension-checklist.md)

### Frontend

Test alignment for the frontend:

- See [`frontend/src/extension-checklist.md`](../../frontend/src/extension-checklist.md)

## Handling Platform Differences

### N/A Comments for Platform-Specific Tests

When a test from another framework doesn't apply to your platform, add a comment-only entry in the same position:

**In SVM (for EVM-specific tests):**

```rust
/// Test: Insufficient Allowance Rejection
/// Verifies that createEscrow reverts when token allowance is insufficient.
/// Why: Token transfers require explicit approval. Insufficient allowance must be rejected to prevent failed transfers.
///
/// NOTE: N/A for SVM - SPL tokens don't use approve/allowance pattern
// EVM: intent-frameworks/evm/test/error-conditions.test.js - "Should revert with insufficient ERC20 allowance"
```

**In EVM (for SVM-specific tests):**

```javascript
/// Test: Zero Solver Address Rejection
/// Verifies that escrows cannot be created with zero/default solver address.
/// Why: A valid solver must be specified for claims.
///
/// NOTE: N/A for EVM - Solidity address type cannot be zero by default, and require() checks prevent zero addresses
// SVM: intent-frameworks/svm/programs/intent_inflow_escrow/tests/error_conditions.rs - "test_reject_zero_solver_address"
```

### Platform-Specific Tests

If your platform has tests that don't exist in other frameworks, add them to the end of the test list in the checklist (with the next sequential number), and add N/A comment blocks in other frameworks.

**Example: MVM has manual release tests 16-19 that don't apply to SVM**

**Step 1: Add to checklist** (`intent-frameworks/extension-checklist.md`)

```markdown
| 16 | test_release_escrow_succeeds_after_fulfillment | ✅ | ⚠️ | N/A |
| 17 | test_release_escrow_rejects_without_fulfillment | ✅ | ⚠️ | N/A |
| 18 | test_release_escrow_rejects_unauthorized_solver | ✅ | ⚠️ | N/A |
| 19 | test_release_escrow_rejects_double_release | ✅ | ⚠️ | N/A |
```

**Step 2: Implement in MVM** (`intent-frameworks/mvm/tests/intent_inflow_escrow_tests.move`)

```move
/// 16. Test: Release escrow succeeds after fulfillment (MVM-specific)
/// Verifies that the solver can successfully claim escrowed tokens after receiving a fulfillment proof from the hub.
/// Why: This is the final step in the inflow intent lifecycle. The solver must receive payment after fulfilling the intent on the hub.
/// Note: MVM requires manual release call. SVM auto-releases in test 6.
#[test(aptos_framework = @0x1, admin = @mvmt_intent, solver = @0x456)]
fun test_release_escrow_succeeds_after_fulfillment(...) {
    // ... test implementation
}
```

**Step 3: Add N/A comments in SVM** (`intent-frameworks/svm/programs/intent_inflow_escrow/tests/gmp.rs`)

```rust
// ============================================================================
// MVM-SPECIFIC TESTS (N/A for SVM)
// ============================================================================
//
// 16. test_release_escrow_succeeds_after_fulfillment - N/A
//     Why: MVM uses two-step fulfillment: (1) receive proof marks fulfilled,
//     (2) manual release transfers tokens. SVM auto-releases tokens in test 6
//     when fulfillment proof is received - no separate release step exists.
//
// 17. test_release_escrow_rejects_without_fulfillment - N/A
//     Why: MVM tests that manual release requires fulfillment first. SVM doesn't
//     have a separate release instruction - release happens automatically in
//     test_gmp_receive_fulfillment_proof_releases_escrow (test 6).
//
// 18. test_release_escrow_rejects_unauthorized_solver - N/A
//     Why: MVM tests solver authorization during manual release. SVM validates
//     solver during GmpReceiveFulfillmentProof and auto-releases to the correct
//     solver immediately (covered in test 6).
//
// 19. test_release_escrow_rejects_double_release - N/A
//     Why: MVM tests that manual release can't happen twice. SVM auto-releases
//     once in test 6, and the escrow is marked claimed. Double fulfillment is
//     rejected in test 8 (test_gmp_receive_fulfillment_proof_rejects_already_fulfilled).
```

**Critical Rule:** When adding a new test to one framework, you **must**:

1. Add it to the checklist with the next sequential number
2. Mark it N/A for frameworks where it doesn't apply
3. Add comment blocks in those frameworks explaining WHY it's N/A

### Adding New Tests to Existing Frameworks

**If you add a new test to a framework (with a new number):**

1. **Add the test** at the appropriate position in your framework's test file
2. **Number it** according to its position in the sequence
3. **Add N/A descriptions** in all other frameworks at the **exact same index/position**

**Example:** If you add a new test as "12. Test: New Feature Validation" in the EVM framework:

**In EVM (error-conditions.test.js):**

```javascript
/// 12. Test: New Feature Validation
/// Verifies that new feature works correctly.
/// Why: Ensures the new feature behaves as expected.
it("Should validate new feature", async function () {
  // ... test implementation
});
```

**In SVM (error_conditions.rs) - at the same position:**

```rust
/// 12. Test: New Feature Validation
/// Verifies that new feature works correctly.
/// Why: Ensures the new feature behaves as expected.
///
/// NOTE: N/A for SVM - [Clear explanation of why this test doesn't apply to SVM]
// EVM: intent-frameworks/evm/test/error-conditions.test.js - "Should validate new feature"
```

**Key points:**

- The test number must match across all frameworks
- N/A comments must be at the same index/position as the actual test
- The N/A comment must clearly explain why the test doesn't apply to that framework
- Include a reference to where the actual test is implemented

## Code Comments

### Avoid Historical Change Comments

❌ **Bad:**

```rust
let amount = 100_000u64; // Reduced to allow 6 escrows with initial 1M tokens
```

✅ **Good:**

```rust
let amount = 100_000u64; // Amount chosen to allow 6 escrows within test token budget
```

**Rule:** Comments should describe the current state and purpose, not what was changed from a previous version.

## Verification Checklist

When adding a new framework, verify:

- [ ] All core tests are implemented or have N/A comments
- [ ] Test order matches existing frameworks exactly
- [ ] Test numbers match across all frameworks at the same positions
- [ ] Test descriptions use generic, platform-appropriate terminology
- [ ] Section headers are used consistently (only where appropriate)
- [ ] Platform-specific tests are documented with N/A comments in other frameworks at the same index
- [ ] Code comments describe current state, not historical changes
- [ ] Test file names match the pattern: `[category].test.js` (EVM) or `[category].rs` (SVM)

When adding a new test to an existing framework:

- [ ] Test is numbered according to its position
- [ ] N/A descriptions are added in all other frameworks at the exact same index/position
- [ ] N/A comments clearly explain why the test doesn't apply to that framework
- [ ] Reference to the actual test implementation is included in N/A comments

## High-Level Integration Checklist

### Solver Integration

- [ ] Define chain type identifiers and config surface for the new framework
- [ ] Add connected-chain client (RPC, signing, key handling)
- [ ] Implement inflow fulfillment transaction building
- [ ] Implement outflow fulfillment transaction building
- [ ] Route fulfillment selection based on chain type
- [ ] Add retry/error handling consistent with existing chains
- [ ] Add unit tests for the new chain client
- [ ] Add integration tests for inflow/outflow fulfillment
- [ ] Update env/config defaults and testnet scripts
- [ ] Document solver setup and required env vars

### Integrated GMP Integration

- [ ] Define chain type identifiers and config surface for the new framework
- [ ] Add RPC client for chain queries and validation
- [ ] Implement inflow validation for escrow + intent matching
- [ ] Implement outflow validation for fulfillment transactions
- [ ] Extend monitors to parse escrow events
- [ ] Extend monitors to parse fulfillment events
- [ ] Ensure signature generation covers the new chain type
- [ ] Update API serialization/deserialization for new chain type
- [ ] Add unit tests for validation helpers
- [ ] Add monitoring tests for escrow + fulfillment ingestion
- [ ] Document integrated-gmp setup and required env vars

### Frontend Integration

- [ ] Add wallet adapter/provider for new chain
- [ ] Add chain config (RPC, program/contract IDs, chain IDs)
- [ ] Add token config (decimals, metadata addresses, native token handling)
- [ ] Add balance fetching for native + token assets
- [ ] Add escrow instruction builders/transaction helpers
- [ ] Wire inflow intent creation for new chain
- [ ] Wire outflow intent creation for new chain
- [ ] Handle chain-specific address formatting and conversions
- [ ] Add UI states for wallet connection + transaction status
- [ ] Add unit tests for helpers and wallet connectors
- [ ] Document frontend env vars and wallet prerequisites
