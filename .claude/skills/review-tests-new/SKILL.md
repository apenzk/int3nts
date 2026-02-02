---
name: review-tests-new
description: Review ONLY new or modified test files for correct comment/header format, check test coverage, and update EXTENSION-CHECKLIST.md for framework symmetry
disable-model-invocation: true
context: fork
agent: Explore
---

# Review Tests (New/Modified Only)

This skill reviews **ONLY new or modified test files** and performs four major tasks:

1. **Review test file format**: Check new/modified test files for correct comment and header format
2. **Review test code quality**: Check for magic numbers, proper constants, and code quality
3. **Check test coverage**: Verify that any added code is well covered by tests
4. **Review extension tests**: Check framework symmetry (MVM/EVM/SVM) and update EXTENSION-CHECKLIST.md files

## Task 1: Review Test File Format (New/Modified Only)

### What to check

Review **ONLY new or modified test files** (use git diff to identify them) for compliance with the format rules in `docs/architecture/codestyle-testing.md`.

### Test format requirements

All test functions must follow the documentation and section header rules defined in `docs/architecture/codestyle-testing.md`:

- **Rule 10**: Test function documentation format (numbered, with "Verifies" and "Why")
- **Rule 11**: Test file section headers (when and how to use them)

**See the full standard**: `docs/architecture/codestyle-testing.md` (Rules 10-11)

### Steps for Task 1

1. **Extract only new/modified test functions from git diff:**

   ```bash
   # For newly added files
   git diff --name-only HEAD
   git diff --name-only origin/main...HEAD

   # For modified files, extract only the changed lines
   git diff HEAD
   git diff origin/main...HEAD
   ```

   **CRITICAL: Use `git diff` output to identify ONLY the new/modified test functions.**
   - For **new files**: Review all test functions
   - For **modified files**: Extract and review ONLY the test functions that appear in the diff (marked with `+` lines)
   - DO NOT read entire test files and review all functions
   - ONLY review test functions that are new or have been modified

   Look for test files in:
   - `intent-frameworks/**/tests/**/*.rs`
   - `intent-frameworks/**/test/**/*.js`
   - `intent-frameworks/**/tests/**/*.move`
   - `coordinator/tests/**/*.rs`
   - `trusted-gmp/tests/**/*.rs`
   - `solver/tests/**/*.rs`
   - `frontend/src/**/*.test.ts`
   - `frontend/src/**/*.test.tsx`

2. For each new/modified test function (from diff):
   - Check the function has proper documentation (Rule 10)
   - Check section headers are used correctly (Rule 11)
   - Report violations with file path and line number

3. Fix violations or report them clearly

## Task 2: Review Test Code Quality (New/Modified Only)

### What to check

Review **ONLY new or modified test files** for code quality issues per `docs/architecture/codestyle-testing.md` (Rules 1-9).

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

1. **Use git diff to identify ONLY new/modified code in test files:**

   ```bash
   git diff HEAD
   git diff origin/main...HEAD
   ```

   **CRITICAL: Review ONLY the lines marked with `+` in the diff.**
   - For **new files**: Review all code
   - For **modified files**: Review ONLY the added/changed lines (marked with `+`)
   - DO NOT read entire test files

2. For each new/modified code section (from diff):
   - Check for hardcoded hex values, addresses, IDs (should use constants)
   - Verify constant naming follows conventions
   - Check variable naming follows `_addr` suffix rule
   - Look for unnecessary `let` bindings
   - Check if struct update syntax is used appropriately

3. Report violations:
   - List magic numbers that should be constants
   - List poorly named variables
   - List unnecessary variable bindings
   - Suggest improvements

**See the full standard**: `docs/architecture/codestyle-testing.md` (Rules 1-9)

## Task 3: Check Test Coverage

### What to check

Any added or modified code must be well covered by tests. This includes:

- New functions/methods
- New modules/contracts/programs
- Modified business logic
- Edge cases and error conditions

### Steps for Task 3

1. Identify recently added or modified code:
   - Use git diff to find changes
   - Focus on implementation files (not just tests)
   - Compare against origin/main or HEAD

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

## Task 4: Review Extension Tests and Update Checklists

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

### Steps for Task 4

1. Read all 5 EXTENSION-CHECKLIST.md files

2. For new/modified tests only:
   - Check if they should be added to the checklist
   - Verify if corresponding tests exist across frameworks (MVM/EVM/SVM)
   - Check test numbering matches across frameworks

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

### Part 1: Test Format Issues (New/Modified Only)

- File path and line number
- Description of the issue
- Suggested fix

### Part 2: Test Code Quality Issues (New/Modified Only)

- Magic numbers that should be constants
- Poorly named variables (missing `_addr` suffix, etc.)
- Unnecessary variable bindings
- Missing struct update syntax opportunities
- Suggested improvements

### Part 3: Test Coverage Analysis (New/Modified Code Only)

- Functions/modules without adequate test coverage
- Types of tests missing (happy path, edge cases, errors)
- Suggested test cases to add
- Assessment of test quality for existing tests
- **Tests with TODOs, placeholders, or soft failures** - these MUST be fixed
- **Tests with fallbacks or graceful degradation** - these MUST fail hard instead
- **Tests with weak assertions** - these MUST use strict checks

### Part 4: Extension Checklist Updates (New/Modified Tests Only)

- Which checklists were updated
- Tests that changed status (⚠️ → ✅)
- Tests that are still missing
- Any symmetry issues found across frameworks

## Important notes

- Use `git diff` to identify new/modified files
- Read `docs/architecture/codestyle-testing.md` for complete format rules
- Check both test documentation and section headers
- Focus only on new/modified test files and code
- Verify test coverage for any new or modified code
- Any added code should be well covered by tests
- Update checklists in place - don't just report, make the changes

### Framework Extension Requirement

**CRITICAL: When adding tests for a new framework, placeholder test files must be created FIRST.**

See `docs/intent-frameworks/framework-extension-guide.md` - "Step 1: Create Placeholder Test Files" for the complete procedure.

**Why this matters for test review:**
- If a new test file doesn't match the expected baseline from existing frameworks, it indicates placeholders were not created first
- Test numbering misalignment is a sign that placeholders were skipped
- Missing tests should have been identified during placeholder creation, not during implementation

**During review, check:**

1. Does the new test file have the same test numbers as the reference framework?
2. Are any tests missing that should be present (or marked N/A)?
3. Are extra tests properly added at the end with new numbers and marked N/A in other frameworks?

If you find misalignment, refer the developer to the Framework Extension Guide for the proper placeholder-first approach.

### Test Ordering Requirement

**CRITICAL: Test order in framework test files MUST match the order in EXTENSION-CHECKLIST.md.**

The EXTENSION-CHECKLIST.md tables define the canonical ordering of tests. When reviewing framework test files:

1. **Test functions must appear in the same order as listed in the checklist**
   - Test #1 from checklist should be the first test function in the file
   - Test #2 from checklist should be the second test function
   - And so on...

2. **Why this matters:**
   - Consistent ordering makes it easy to verify coverage across frameworks
   - Developers can quickly find corresponding tests in different frameworks
   - Test numbers in the checklist serve as the source of truth for ordering

3. **During review, verify:**
   - Read the relevant EXTENSION-CHECKLIST.md section for the module being tested
   - Check that tests in the file appear in the same sequence as the checklist rows
   - Report any ordering violations with the expected order from the checklist

4. **How to fix ordering issues:**
   - Reorder test functions in the file to match the checklist
   - Do NOT change the checklist order to match the file
   - The checklist defines the order; the file must conform

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
