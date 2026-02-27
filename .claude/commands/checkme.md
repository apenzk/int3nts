---
description: Self-review after completing a task — check for rule violations, silent fallbacks, test style issues, and extension-checklist drift
---

# Check Me — Post-Task Self-Review

Run this command after completing a task to catch violations before committing.

## Step 1: Identify What Changed

Determine all files created or modified in this session:

```bash
git diff --name-only HEAD
git diff --name-only origin/main...HEAD
git diff HEAD
git diff origin/main...HEAD
```

Separate changes into:

- **Source files**: `*.rs`, `*.move`, `*.sol`, `*.js`, `*.ts`, `*.tsx`
- **Test files**: Files in `tests/`, `test/`, or `*.test.*` patterns
- **Config/docs**: Everything else

## Step 2: No Fallbacks Policy Violations (CRITICAL)

**Reference**: `.claude/rules.md` — "No Fallbacks Policy" section and `CLAUDE.md` — "No Fallbacks Policy"

Scan ALL new/modified source AND test code for these violations:

### Silent fallback patterns to find

Search the diff output (lines with `+` prefix) for:

- `unwrap_or` / `unwrap_or_default` / `unwrap_or_else` — hiding missing data behind defaults
- `or_else` with a default value — masking errors
- `.ok()` — silently discarding errors
- `catch` blocks that swallow errors (empty or logging-only)
- `if let Some(x) = ...` without an `else` that errors — silently ignoring `None`
- `match` arms with `_ => {}` or `_ => ()` — swallowing unexpected variants
- Optional chaining (`?.`) in critical paths masking null/undefined
- `default()` / `Default::default()` in places where missing data should be an error
- `try`/`catch` that returns a fallback value instead of propagating
- TODO comments, FIXME, HACK, "implement later" placeholders

### For each match found, classify

- **VIOLATION**: The fallback hides a real failure. Must be removed.
- **ACCEPTABLE**: Rare — well-documented user-facing default, feature flag, or config with safe default. Must justify.

### Report format

For each match, present the classification to the user with the surrounding code context so they can see exactly what was found:

```text
FALLBACK ANALYSIS — N pattern(s) found

[VIOLATION] file/path.rs:42
  Pattern: unwrap_or_default()
  Context:
    let balance = get_balance().unwrap_or_default();
  Hides: Missing balance data silently becomes 0
  Fix: Use `?` or `.expect("balance must exist")`

[ACCEPTABLE] file/path.rs:88
  Pattern: unwrap_or
  Context:
    let theme = user_prefs.theme.unwrap_or("light".to_string());
  Justification: User-facing preference with documented default
```

**CRITICAL: Always present this analysis to the user explicitly.** Do not silently pass over matches. The user must see every match and its classification so they can confirm or override the judgment.

## Step 3: Test Style Violations

**Reference**: `docs/architecture/codestyle-testing.md` (Rules 1-11)

For ALL new/modified test files and test functions (from diff):

### Rule 10 — Test function documentation

Every test function must have:

```text
/// N. Test: [Test Name]
/// Verifies that [what the test does].
/// Why: [rationale].
```

Check:

- Sequential numbering present
- "Verifies that" phrasing used
- "Why:" line present with meaningful rationale
- Numbered comment matches the test's position in the file

### Rule 11 — Section headers

If the test file groups tests, verify headers use the correct format:

```text
// ============================================================================
// SECTION NAME
// ============================================================================
```

### Rules 1-9 — Code quality

- **Rule 1**: Constants represent distinct concepts (no reuse across different meanings)
- **Rule 2**: `DUMMY_*` naming convention followed
- **Rule 3**: Hex patterns use unique repeating digits (0x1111..., 0x2222...)
- **Rule 4**: Address variables use `_addr` suffix
- **Rule 5**: No unnecessary `let` bindings for single-use values
- **Rule 6**: No duplicate constants — reuse existing ones
- **Rule 7**: Test-specific values inlined, not made into constants
- **Rule 8**: Struct update syntax (`..default_fn()`) used to reduce duplication
- **Rule 9**: Correct byte lengths (EVM addr=42 chars, MVM addr=66 chars, tx hash=66 chars)

### Magic numbers in source code

Scan all new/modified source files (not just tests) for hardcoded numeric/hex literals that should be named constants. See `docs/architecture/codestyle-testing.md` for what counts vs. what is acceptable.

### Test assertion quality

- Tests must have strict assertions — no "just check it doesn't panic"
- Tests must fail on errors — no `unwrap_or`, no fallback values in assertions
- No TODO/placeholder test functions

## Step 4: Extension Checklist Violations

**Reference**: Extension checklist files track test symmetry across MVM/EVM/SVM.

### Checklist locations

1. `solver/tests/extension-checklist.md`
2. `coordinator/tests/extension-checklist.md`
3. `integrated-gmp/tests/extension-checklist.md`
4. `frontend/src/extension-checklist.md`
5. `intent-frameworks/extension-checklist.md`

### What to check

For each new/modified test file:

1. **Is the test in the relevant checklist?** If a test was added but the checklist wasn't updated, that's a violation.
2. **Is the checklist status correct?** New tests should be marked ✅, not left as ⚠️.
3. **Are test numbers sequential and matching?** Test order in the file must match the checklist order.
4. **Framework symmetry**: If a test was added for one VM, is it marked ⚠️ (not yet implemented) or N/A in other VMs?
5. **Were any tests removed?** If so, the checklist must be updated too.

### Report format

| Checklist | Issue | Fix needed |
|-----------|-------|------------|

## Step 5: Verdict

After completing all checks, output a final verdict:

### If violations found

```text
CHECKME FAILED — N violation(s) found

FALLBACK VIOLATIONS (Critical):
- [list each with file:line]

TEST STYLE VIOLATIONS:
- [list each with file:line and rule number]

EXTENSION CHECKLIST VIOLATIONS:
- [list each with checklist file and issue]

Fix these before committing.
```

### If clean

```text
CHECKME PASSED — No violations found

Checked:
- N source files for fallback patterns
- N test files for style compliance
- N extension checklists for drift
```

## Important Notes

- This is a SELF-review — Claude checks its own work from the current session
- Use `git diff` output to scope the review to only what changed
- Do NOT read entire files — focus on the diff
- Every fallback pattern is a violation unless explicitly justified
- Every test must have Rule 10 documentation — no exceptions
- Extension checklists must be updated whenever tests change — no exceptions
- Be strict. The purpose of this command is to catch mistakes before they reach commit.
