---
description: Review staged changes by asking critical questions before committing
---

# Review Me on These Changes

Interactive review session where Claude asks critical questions about your staged changes before you commit.

## Step 1: Analyze Staged Changes

Read what's being changed:

```bash
git status
git diff --cached --stat
git diff --cached
```

Understand:
- What files changed?
- What functionality is affected?
- Which frameworks (MVM/EVM/SVM)?
- Are tests involved?

## Step 2: Ask Critical Questions

Based on the changes, ask the user questions in these categories:

### Completeness
- Did you implement this across all relevant frameworks (MVM/EVM/SVM)?
- Did you update all EXTENSION-CHECKLIST.md files?
- Are there related files that should also be updated?

### Testing
- Did new functions or features in this commit receive sufficient tests?
- Where are the tests for this change?
- Did you test happy path, edge cases, and error conditions?
- Do tests follow format rules (Rule 10-11 from codestyle-testing.md)?
- Did you check for magic numbers and replace them with constants?
- Are tests hard failures (no TODOs, no fallbacks)?

### Code Quality
- Why did you make this change?
- What edge cases does this handle?
- What happens if [error condition X] occurs?
- Did you check for code duplication?
- Are variable names following conventions (_addr suffix, etc.)?

### Documentation
- Did you update relevant README files?
- Are public functions documented?
- Did you update architecture diagrams if needed?

### Symmetry (for framework changes)
- If you added this to MVM, does EVM need it? Does SVM?
- Are the implementations equivalent across frameworks?
- Did you verify test numbering matches across frameworks?

## Step 3: Evaluate Answers

After each answer:
- Point out gaps or issues
- Ask follow-up questions if answers are unclear
- Challenge assumptions
- Suggest improvements

## Step 4: Pass/Fail Decision

**Pass criteria:**
- All questions answered satisfactorily
- No obvious gaps in implementation
- Tests are comprehensive
- Documentation is updated
- Framework symmetry maintained (if applicable)

**Fail criteria:**
- New functions/features without tests
- Missing tests for changed functionality
- Incomplete implementation
- Framework asymmetry not justified
- Can't explain rationale for changes

## Output Format

Start with:
```
🔍 REVIEWING YOUR CHANGES

I found changes to:
- [list key files/areas]

Let me ask you some questions...
```

Then ask questions one category at a time, wait for answers.

End with either:
```
✅ PASS - These changes look good. Ready to commit.
```

or:

```
❌ FAIL - Fix these issues before committing:
- [list issues]
```

## Important Notes

- Be thorough but not pedantic
- Focus on high-impact issues
- Ask "why" not just "what"
- Challenge but don't block unnecessarily
- Consider project-specific patterns (No Fallbacks Policy, etc.)
