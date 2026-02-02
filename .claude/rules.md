# Intent Framework Project Rules

## Commit Message Conventions

After completing a subtask or task, create a commit with the changes.

**If tests were run:**

```bash
git commit -m "<type of change>: <description>

- <more detailed points if needed (optional)>
- <more detailed points if needed (optional)>

Tests pass: Coordinator <number>, Trusted-GMP <number>, Solver <number>, MVM <amount>, EVM <amount>, SVM <number>, Frontend <number>"
```

**If tests were NOT run (e.g., project setup, docs only, no test-affecting changes):**

```bash
git commit -m "<type of change>: <description>

- <more detailed points if needed (optional)>
- <more detailed points if needed (optional)>"
```

**For multi-line commit messages, use heredoc format:**

```bash
git commit -m "$(cat <<'EOF'
<type>: <description>

- <detail 1 if needed>
- <detail 2 if needed>
EOF
)"
```

**Note**: Use `EOF` as the heredoc delimiter (standard convention).

**Commit Rules:**

- **ALWAYS commit after completing each subtask or task** - ensures incremental progress is saved
- **CRITICAL: NEVER run `git add` or `git add -A`** - files must already be staged by the user
- **Run tests before committing** ONLY if changes affect existing test code (e.g., adding new tests, modifying code that has tests)
- **Do NOT run tests for:** project setup, documentation-only changes, configuration files, or other non-code changes
- **If sandbox prevents test execution**, ask user for help or skip tests (don't include "Tests pass:" line)
- **Only include test results** in commit message if tests were actually run, in the format: `Tests pass: Coordinator <number>, Trusted-GMP <number>, Solver <number>, MVM <amount>, EVM <amount>, SVM <number>, Frontend <number>`
- **If tests were NOT run**, omit the "Tests pass:" line entirely from the commit message
- **Display test summary table** after running tests using the commands in the next subsection, showing passed/total for each category (Coordinator, Trusted-GMP, Solver, MVM, EVM, SVM, Frontend)
- Follow conventional commit format (e.g., `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`)
- Keep commit messages brief and professional
- Do NOT mention subtask or task IDs in commit messages
- Do NOT advertise AI tools in commits - no AI/Claude/LLM references, "Co-Authored-By" phrases, or similar

### Test Commands

**CRITICAL: NEVER run tests directly with `cargo test`, `npm test`, or `movement move test`. ALWAYS use the project scripts.**

Run all tests with summary:

```bash
./testing-infra/run-all-unit-tests.sh
```

Run individual component tests (from project root):

```bash
# SVM (Solana) - ALWAYS use this, never cargo test directly
cd intent-frameworks/svm && ./scripts/test.sh

# MVM (Movement)
cd intent-frameworks/mvm && movement move test --dev --named-addresses mvmt_intent=0x123

# EVM
cd intent-frameworks/evm && npm test

# Rust services
cd coordinator && cargo test --quiet
cd trusted-gmp && cargo test --quiet
cd solver && cargo test --quiet

# Frontend
cd frontend && npm test
```

## Documentation

See `docs/docs-guide.md` for documentation organization.

### Markdown Style Guidelines

**IMPORTANT:** All markdown files must follow these formatting rules:

- **Blank lines around headings**: There must be a blank line before and after all headings (MD022)
- **Blank lines around lists**: There must be a blank line before bullet lists and numbered lists (MD032)
- **No multiple blank lines**: Use only one blank line between sections (MD012)

These rules ensure consistent formatting and prevent linting errors. Always check linting errors before committing documentation changes.

--------------------------------------------------------------------------------

## Project Overview

This is a cross-chain intent framework enabling conditional asset transfers across blockchain networks. The system supports both **inflow** (tokens locked on connected chain, desired on hub) and **outflow** (tokens locked on hub, desired on connected chain) flows.

--------------------------------------------------------------------------------

## Code Organization

- **MVM/Move**: `docs/intent-frameworks/mvm/README.md`
- **EVM/Solidity**: `docs/intent-frameworks/evm/README.md`
- **SVM/Solana**: `docs/intent-frameworks/svm/README.md`
- **Rust services**: `docs/coordinator/`, `docs/trusted-gmp/`, `docs/solver/`
- **Testing**: `docs/testing-infra/README.md`
- **Coding standards**: `docs/codestandards.md`

--------------------------------------------------------------------------------

## Documentation Standards

- **Location**: `docs/` directory
- **Structure**: Each major component has its own README
- **API docs**: Use code comments that can be extracted to API reference
- **Architecture docs**: Use mermaid diagrams for visual representation

## Testing Standards

- **Test documentation format**: See `docs/architecture/codestyle-testing.md` for required `Test:` / `Verifies` / `Why:` format
- **Move tests**: Place in `intent-frameworks/mvm/tests/` with `*_tests.move` naming
- **Solidity tests**: Place in `intent-frameworks/evm/test/` with `*.test.js` naming
- **E2E tests**: Use shell scripts in `testing-infra/ci-e2e/e2e-tests-*/`
- **Test isolation**: Each test should be independent and clean up after itself
- **Balance checks**: Use granular balance display functions appropriate for the chain being tested

### Cross-Chain Test Consistency

When adding tests for shared modules (e.g., `gmp_common`/`gmp-common`):

- **Number tests sequentially**: Use numbered comments (e.g., `//1. Test:`, `//2. Test:`) to track test coverage
- **Mirror tests across frameworks**: If a test exists in MVM, add the equivalent in SVM (and EVM when applicable)
- **Use identical test vectors**: Same inputs must produce same expected outputs across all chains
- **Update test-vectors.json**: Cross-chain encoding tests reference `intent-frameworks/common/testing/gmp-encoding-test-vectors.json`
- **Keep test counts in sync**: MVM and SVM should have matching test numbers for shared functionality
- **Update EXTENSION-CHECKLIST.md**: Each component has an EXTENSION-CHECKLIST.md tracking test status. See `docs/intent-frameworks/framework-extension-guide.md` for the full reference

--------------------------------------------------------------------------------

## Common Patterns

### Error Handling

- Move: Use `abort` with error codes defined in module
- Solidity: Use `require()` with descriptive error messages
- Rust: Use `anyhow::Result` and propagate errors with context
- Shell scripts: Use `set -e` and check return codes
