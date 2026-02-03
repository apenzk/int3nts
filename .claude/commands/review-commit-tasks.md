# Review Commit Tasks

This command reviews each task checkbox item in the current commit section of a GMP plan file, verifies implementation in code, and presents a completion table for user approval before ticking boxes.

## When to use

Run this command **before** `/commit` when working on GMP plan commits. It ensures that:

1. Each task item is actually implemented in code
2. Nothing is marked done that isn't done
3. The user explicitly approves which boxes get ticked

## Input

The command expects you to identify which commit you're working on. It will:

1. Read the relevant plan file (e.g., `docs/architecture/plan/gmp-plan-execution-phase2.md`)
2. Find the current commit section
3. Extract all task items (`- [ ]` checkboxes)

## Steps

### Step 1: Identify the current commit

**ALWAYS use AskUserQuestion to ask the user which phase and commit they're working on:**

Use AskUserQuestion with options:

- Phase options: 1, 2, 3, 4, 5, 6
- Then ask which commit number within that phase

Example question flow:

1. "Which GMP plan phase are you working on?" → Options: Phase 1, Phase 2, Phase 3, Phase 4, Phase 5, Phase 6
2. "Which commit number in Phase N?" → Options based on commits in that phase file

After getting user input, read the plan file:

```text
docs/architecture/plan/gmp-plan-execution-phase{N}.md
```

### Step 2: Extract task items

Find the commit section (e.g., "### Commit 9: ...") and extract all task items:

- Lines starting with `- [ ]` (unchecked)
- Lines starting with `- [x]` (already checked)

### Step 3: Review each task against code

For each unchecked task item (`- [ ]`):

1. **Identify what to look for**: Parse the task description to understand what code/feature should exist
2. **Search the codebase**: Use Grep/Glob/Read to find evidence of implementation
3. **Assess completion**: Determine if the task is:
   - **DONE**: Fully implemented and working
   - **PARTIAL**: Started but incomplete
   - **STUBBED**: Interface/placeholder exists but not functional
   - **NOT DONE**: No evidence of implementation

### Step 4: Document evidence

For each task, record:

- **Status**: DONE / PARTIAL / STUBBED / NOT DONE
- **Evidence**: File paths, line numbers, function names that prove implementation
- **Missing**: What's still needed (if not DONE)

### Step 5: Generate completion table

Present a markdown table to the user:

```markdown
## Commit N Task Review

| # | Task | Status | Evidence | Missing |
|---|------|--------|----------|---------|
| 1 | Add NativeGmpRelay struct | DONE | native_gmp_relay.rs:139 | - |
| 2 | Watch MVM events | DONE | poll_mvm_events() L195 | - |
| 3 | Watch SVM events | STUBBED | poll_svm_events() logs debug only | Actual RPC polling |
| 4 | Deliver messages | PARTIAL | deliver_to_mvm() exists | No actual tx submission |
| ... | ... | ... | ... | ... |
```

### Step 6: Get user approval

After presenting the table, ask the user:

```text
Which tasks should be marked as done?

Options:
1. Mark all DONE tasks (N items)
2. Mark DONE + PARTIAL tasks (M items)
3. Let me specify which ones
4. Don't mark any - more work needed
```

### Step 7: Update the plan file (if approved)

Only after user approval, update the plan file to tick the approved boxes:

- Change `- [ ]` to `- [x]` for approved items
- Do NOT change items the user didn't approve

## Output format

### Summary section

```text
## Review Summary

**Phase:** 2
**Commit:** 9 - Implement native GMP relay in trusted-gmp
**Tasks reviewed:** 7
**Status breakdown:**
- DONE: 4
- PARTIAL: 2
- STUBBED: 1
- NOT DONE: 0
```

### Detailed table

See Step 5 format above.

### Recommendation

```text
## Recommendation

Based on the review:
- 4 tasks are fully implemented and can be marked done
- 2 tasks have partial implementation (user decision needed)
- 1 task is stubbed with placeholder code

Suggested action: Mark DONE tasks, document PARTIAL/STUBBED in commit message
```

## Important notes

- **Never auto-tick boxes** - always get user approval first
- **Be thorough** - search multiple locations, check tests too
- **Be honest** - if something is stubbed, call it out
- **Show evidence** - include file paths and line numbers
- **Document what's missing** - help the user understand gaps

## Status definitions

| Status | Definition | Can be ticked? |
|--------|------------|----------------|
| DONE | Fully implemented, tested, working | Yes |
| PARTIAL | Core functionality exists but incomplete | User decides |
| STUBBED | Interface/placeholder exists, logs "not implemented" or similar | User decides |
| NOT DONE | No evidence of implementation | No |

## Example session

```text
User: /review-commit-tasks

Claude: I'll review the tasks for the current commit. Which phase and commit are you working on?

User: Phase 2, Commit 9

Claude: [Reads plan file, extracts tasks, searches codebase]

## Commit 9 Task Review

| # | Task | Status | Evidence | Missing |
|---|------|--------|----------|---------|
| 1 | Add NativeGmpRelay struct | DONE | native_gmp_relay.rs:139 | - |
| 2 | Watch MVM events | DONE | poll_mvm_events() L195-262 | - |
| 3 | Watch SVM events | STUBBED | poll_svm_events() L265-283 | Only debug log, no RPC calls |
| 4 | Deliver messages | PARTIAL | deliver_to_mvm() L331-375 | Logs "Would submit" instead of actual tx |
| 5 | Configurable RPCs | DONE | NativeGmpRelayConfig L36-82 | - |
| 6 | Default mode | DONE | main.rs only runs relay | - |
| 7 | Unit tests | DONE | tests L437-500 | - |

## Summary

- **DONE:** 5 tasks
- **PARTIAL:** 1 task (deliver_message is stubbed)
- **STUBBED:** 1 task (SVM polling)

Which tasks should be marked as done?
```
