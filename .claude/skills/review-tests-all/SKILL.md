---
name: review-tests-all
description: Review ALL test files in the entire codebase for correct comment/header format, check test coverage, and update EXTENSION-CHECKLIST.md for framework symmetry
disable-model-invocation: true
context: fork
agent: Explore
---

# Review Tests (All Files)

This skill is identical to `review-tests-new` except it reviews **ALL test files** instead of only new/modified ones.

## Reference

See `.claude/skills/review-tests-new/SKILL.md` for the complete rules and requirements. All rules apply here.

## Key Differences from review-tests-new

| Aspect | review-tests-new | review-tests-all |
| --- | --- | --- |
| Scope | Only new/modified test files (from git diff) | ALL test files in codebase |
| How to find files | `git diff --name-only` | Glob patterns (see below) |
| Use case | Quick feedback on recent changes | Comprehensive codebase audit |
| Performance | Fast | May take significant time |

## Test File Locations

Find ALL test files using these patterns:

- `intent-frameworks/**/tests/**/*.rs`
- `intent-frameworks/**/test/**/*.js`
- `intent-frameworks/**/tests/**/*.move`
- `coordinator/tests/**/*.rs`
- `trusted-gmp/tests/**/*.rs`
- `solver/tests/**/*.rs`
- `frontend/src/**/*.test.ts`
- `frontend/src/**/*.test.tsx`

## Steps

1. **Find all test files** using the glob patterns above
2. **For each test file**, apply all checks from review-tests-new:
   - Task 1: Test file format (Rules 10-11)
   - Task 2: Test code quality (Rules 1-9)
   - Task 3: Test coverage
   - Task 4: Extension checklist updates
3. **Report with summary statistics** (total files checked, total violations found)

## Output Format

Same as review-tests-new, but include:

- Summary statistics (total files, violations per category)
- Overall symmetry score across MVM/EVM/SVM

## Performance Note

Since this reviews ALL test files, it may take significant time and generate a large report. Consider using `/review-tests-new` for faster feedback on recent changes.
