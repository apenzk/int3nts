---
name: review-tests-all
description: Review ALL test files in the entire codebase for correct comment/header format, check test coverage, and update EXTENSION-CHECKLIST.md for framework symmetry
disable-model-invocation: true
context: fork
agent: Explore
---

# Review Tests (All Files)

This skill reviews **ALL test files in the entire codebase** and performs four major tasks:

1. **Review test file format**: Check ALL test files for correct comment and header format
2. **Review test code quality**: Check for magic numbers, proper constants, and code quality
3. **Check test coverage**: Verify that any added code is well covered by tests
4. **Review extension tests**: Check framework symmetry (MVM/EVM/SVM) and update EXTENSION-CHECKLIST.md files

## Task 1: Review Test File Format (All Files)

### What to check

Review **ALL test files** in the codebase for compliance with the format rules in `docs/architecture/codestyle-testing.md`.

### Test format requirements

All test functions must follow the documentation and section header rules defined in `docs/architecture/codestyle-testing.md`:

- **Rule 10**: Test function documentation format (numbered, with "Verifies" and "Why")
- **Rule 11**: Test file section headers (when and how to use them)

**See the full standard**: `docs/architecture/codestyle-testing.md` (Rules 10-11)

### Steps for Task 1

1. Find ALL test files in the codebase:
   - `intent-frameworks/**/tests/**/*.rs`
   - `intent-frameworks/**/test/**/*.js`
   - `intent-frameworks/**/tests/**/*.move`
   - `coordinator/tests/**/*.rs`
   - `trusted-gmp/tests/**/*.rs`
   - `solver/tests/**/*.rs`
   - `frontend/src/**/*.test.ts`
   - `frontend/src/**/*.test.tsx`

2. For each test file:
   - Check each test function has proper documentation (Rule 10)
   - Check section headers are used correctly (Rule 11)
   - Report violations with file path and line number

3. Fix violations or report them clearly

## Task 2: Review Test Code Quality (All Files)

### What to check

Review **ALL test files** for code quality issues per `docs/architecture/codestyle-testing.md` (Rules 1-9).

### Magic numbers and constants (Rules 1-3, 6, 9)

Check for proper use of constants instead of magic numbers:

- **Rule 1 (Semantic Meaning)**: Each constant represents a distinct concept
  - ❌ Don't reuse constants for different purposes (e.g., don't use `DUMMY_ESCROW_ID_MVM` for transaction hashes)
  - ✅ Create separate constants for different concepts

- **Rule 2 (Naming Convention)**: Use descriptive constant names
  - `DUMMY_*_ADDR_*` for addresses (e.g., `DUMMY_SOLVER_ADDR_EVM`)
  - `DUMMY_*_ID_*` for IDs (e.g., `DUMMY_INTENT_ID`)
  - `DUMMY_TX_HASH` for transaction hashes
  - `DUMMY_EXPIRY` for timestamps

- **Rule 3 (Hex Patterns)**: Use unique repeating hex patterns (0x1111..., 0x2222..., 0x3333...)

- **Rule 6 (Adding Constants)**: Before adding new constants, check if existing ones can be reused

- **Rule 9 (Format Requirements)**: Match expected formats
  - EVM addresses: 20 bytes (42 chars with 0x)
  - MVM addresses: 32 bytes (66 chars with 0x)
  - Transaction hashes: 32 bytes (66 chars with 0x)

### Variable naming (Rule 4)

Check variable and parameter naming:

- Address variables must use `_addr` suffix (e.g., `solver_addr`, `requester_addr`)
- Solver-related variables use `solver_` prefix (e.g., `solver_registered_evm_addr`)
- Avoid generic names like `address` or `addr` without context

### Code quality (Rules 5, 7, 8)

- **Rule 5**: Remove unnecessary variable bindings
  - ❌ Don't use `let` when value is only used once
  - ✅ Inline values directly into function calls
  - ✅ Keep `let` when variable is used multiple times

- **Rule 7**: Test-specific identifiers
  - Use descriptive inline values with meaningful comments only
  - Don't create constants for one-off test cases
  - Only create constants for values used across multiple tests

- **Rule 8**: Use struct update syntax with default helper functions
  - Use `..function_name()` to fill remaining fields
  - Override only specific fields that differ
  - Create default helper functions to reduce duplication

### Steps for Task 2

1. For each test file:
   - Check for hardcoded hex values, addresses, IDs (should use constants)
   - Verify constant naming follows conventions
   - Check variable naming follows `_addr` suffix rule
   - Look for unnecessary `let` bindings
   - Check if struct update syntax is used appropriately

2. Report violations:
   - List magic numbers that should be constants
   - List poorly named variables
   - List unnecessary variable bindings
   - Suggest improvements

**See the full standard**: `docs/architecture/codestyle-testing.md` (Rules 1-9)


## Task 3: Check Test Coverage (All Code)

### What to check

Any added or modified code must be well covered by tests. This includes:
- New functions/methods
- New modules/contracts/programs
- Modified business logic
- Edge cases and error conditions

### Steps for Task 2

1. Identify recently added or modified code:
   - Use git diff to find changes
   - Look at commit history if available
   - Focus on implementation files (not just tests)

2. For each new/modified function or module:
   - Find corresponding test files
   - Verify tests exist for:
     - Happy path / normal operation
     - Edge cases (boundary values, empty inputs, max values)
     - Error conditions (invalid inputs, unauthorized access, state violations)
     - Integration scenarios (if applicable)
   - Check that tests follow the format rules from Task 1

3. Report coverage gaps:
   - List functions/modules without adequate test coverage
   - Specify what types of tests are missing (happy path, edge cases, errors)
   - Suggest test cases that should be added

4. Verify test quality:
   - Tests actually exercise the code path
   - Tests have meaningful assertions (not just checking for no panic)
   - Tests are properly documented with "Verifies" and "Why"
   - **Tests must be hard failures - NO TODOs, NO "implement later", NO placeholders**
   - **Tests must crash and error on failures - NO fallbacks, NO graceful degradation**
   - **Tests must have strict assertions - NO lenient checks, NO silent passes**

## Task 4: Review Extension Tests and Update Checklists (All Tests)

### EXTENSION-CHECKLIST.md files

There are 5 checklist files that track test symmetry across frameworks (MVM/EVM/SVM):

1. `intent-frameworks/EXTENSION-CHECKLIST.md`
2. `coordinator/tests/EXTENSION-CHECKLIST.md`
3. `frontend/src/EXTENSION-CHECKLIST.md`
4. `solver/tests/EXTENSION-CHECKLIST.md`
5. `trusted-gmp/tests/EXTENSION-CHECKLIST.md`

### What to check

Each checklist has tables showing which tests are:
- ✅ = Implemented
- ⚠️ = Not yet implemented
- N/A = Not applicable to platform

### Steps for Task 3

1. Read all 5 EXTENSION-CHECKLIST.md files

2. For each checklist:
   - Extract test file paths and test names from the tables
   - Verify the actual test files match the checklist status
   - Check if tests marked ✅ actually exist
   - Check if tests marked ⚠️ should now be ✅
   - Verify test numbering matches across frameworks

3. Update checklists:
   - Change ⚠️ to ✅ if tests are now implemented
   - Add new tests if they exist but aren't in the checklist
   - Ensure test numbers are sequential and match across frameworks

4. Report findings:
   - List tests that were updated
   - List tests that are missing
   - List any inconsistencies found

## Output format

Provide a clear report with:

### Part 1: Test Format Issues (All Files)
- File path and line number
- Description of the issue
- Suggested fix
- Summary statistics (total files checked, total violations found)

### Part 2: Test Code Quality Issues (All Files)
- Magic numbers that should be constants
- Poorly named variables (missing `_addr` suffix, etc.)
- Unnecessary variable bindings
- Missing struct update syntax opportunities
- Suggested improvements

### Part 3: Test Coverage Analysis (All Code)
- Functions/modules without adequate test coverage
- Types of tests missing (happy path, edge cases, errors)
- Suggested test cases to add
- Assessment of test quality for existing tests
- **Tests with TODOs, placeholders, or soft failures** - these MUST be fixed
- **Tests with fallbacks or graceful degradation** - these MUST fail hard instead
- **Tests with weak assertions** - these MUST use strict checks

### Part 4: Extension Checklist Updates (All Tests)
- Which checklists were updated
- Tests that changed status (⚠️ → ✅)
- Tests that are still missing
- Any symmetry issues found across frameworks
- Overall symmetry score across MVM/EVM/SVM

## Important notes

- Read `docs/architecture/codestyle-testing.md` for complete format rules
- Check both test documentation and section headers
- Be thorough - check **ALL test files**, not just extension tests
- Verify test coverage for any new or modified code
- Any added code should be well covered by tests
- Update checklists in place - don't just report, make the changes
- This is a comprehensive review - expect it to take time and find many issues

### Critical: No Fallbacks Policy

**TESTS MUST FAIL HARD. NO EXCEPTIONS.**

- ❌ NO TODO comments or "implement later" placeholders
- ❌ NO fallback values or default behavior on errors
- ❌ NO graceful degradation or error swallowing
- ❌ NO lenient assertions (e.g., "just check it doesn't panic")
- ❌ NO try/catch blocks that hide failures
- ✅ Tests either PASS with strict assertions or FAIL with clear errors
- ✅ Missing functionality = test fails (not skipped or todo)
- ✅ Errors must propagate and cause test failure
- ✅ Assertions must be strict and explicit

## Performance considerations

Since this reviews ALL test files, it may:
- Take significant time to complete
- Generate a large report
- Find many violations if the codebase is not yet fully compliant

Consider using `/review-tests-new` for faster feedback on recent changes.
