# Framework Extension Guide

This guide explains how to add a new blockchain framework (e.g., MVM, EVM, SVM) to the Intent Framework while maintaining consistency and test coverage across all platforms.

## Overview

When adding a new framework, you must:

1. **Replicate the core escrow functionality** from existing frameworks
2. **Maintain test alignment** - each test should have a corresponding test in the same position across all frameworks
3. **Use generic test descriptions** - avoid platform-specific terminology
4. **Document platform differences** - use N/A comments for tests that don't apply to your platform
5. **Follow consistent structure** - use the same test file organization and section headers

## Test Structure Requirements

### Test File Organization

Each framework should have the following test files, matching the order and structure of existing frameworks:

1. **initialization** - Basic setup and escrow creation
2. **deposit** - Escrow creation and deposit functionality
3. **claim** - Claiming escrow funds with verifier signatures
4. **cancel** - Cancellation and refund functionality
5. **expiry** - Expiry timestamp handling and expired escrow behavior
6. **cross-chain** - Intent ID conversion and cross-chain compatibility
7. **edge-cases** - Boundary values, concurrent operations, gas/compute limits
8. **error-conditions** - Error handling and validation
9. **integration** - Full lifecycle workflows
10. **scripts** - Utility script testing (if applicable)

### Section Headers

See [Test File Section Headers](../../architecture/codestyle-testing.md#10-test-file-section-headers) in the coding guide for section header formatting guidelines.

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

**Test description format:**

```rust
/// Test: [Test Name]
/// Verifies that [what the test does].
/// Why: [rationale for why this test is important].
```

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

- See [`intent-frameworks/COMPLETENESS.md`](../../intent-frameworks/COMPLETENESS.md)

### Verifier

Test alignment for the verifier:

- See [`verifier/tests/COMPLETENESS.md`](../../verifier/tests/COMPLETENESS.md)

### Solver

Test alignment for the solver:

- See [`solver/tests/COMPLETENESS.md`](../../solver/tests/COMPLETENESS.md)

### Frontend

Test alignment for the frontend:

- See [`frontend/src/COMPLETENESS.md`](../../frontend/src/COMPLETENESS.md)

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
// SVM: intent-frameworks/svm/programs/intent_escrow/tests/error_conditions.rs - "test_reject_zero_solver_address"
```

### Platform-Specific Tests

If your platform has tests that don't exist in other frameworks, add them at the end of the appropriate test file (maintaining the numbered sequence):

**Example (SVM-specific tests in error_conditions.rs):**

```rust
/// 9. Test: Zero Solver Address Rejection
/// Verifies that escrows cannot be created with zero/default solver address.
/// Why: A valid solver must be specified for claims.
#[tokio::test]
async fn test_reject_zero_solver_address() {
    // ... test implementation
}
```

**Critical Rule:** When adding a new test to one framework, you **must** add a corresponding N/A comment description at the **same index/position** in all other frameworks, explaining why that test is not implemented.

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

### Verifier Integration

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
- [ ] Document verifier setup and required env vars

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
