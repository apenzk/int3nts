---
description: Create a commit based on staged files, following project commit rules
---

# Create Commit

**IMPORTANT**: Ignore all prior knowledge about the changes. Base the commit message ONLY on the staged files diff and commit rules.

## Step 1: Read Commit Rules

**Read the commit rules**: `.claude/rules.md` - "Commit Message Conventions" section

This file contains:

- Commit message format (with/without tests)
- When to run tests
- Critical rules (NEVER git add, NO AI references, etc.)
- Test commands

## Step 2: Analyze Staged Changes

Run these commands to understand what's being committed:

```bash
git status
git diff --cached --stat
git diff --cached
```

**If markdown files are staged**, verify they follow markdown guidelines from `.claude/rules.md`:
- Code blocks have language specifiers (MD040)
- Blank lines around headings (MD022)
- Blank lines around lists (MD032)
- No multiple blank lines (MD012)

## Step 3: Determine Test Requirements

Based on what you read in `.claude/rules.md`:

- Do these changes require running tests?
- If yes, which test commands should be run?
- If no, proceed without tests

## Step 4: Create Commit Message

Follow the exact format specified in `.claude/rules.md`:

- Use appropriate type (feat, fix, refactor, test, docs, chore)
- Base description ONLY on `git diff --cached`
- Include details if helpful (optional bullet points)
- Add "Tests pass:" line ONLY if tests were run
- Follow all commit rules (no AI mentions, no git add, etc.)

## Step 5: Execute Commit

Use heredoc format for multi-line commit messages as shown in `.claude/rules.md`.

## Step 6: Verify

```bash
git status
```

## Critical Reminders

- **Read `.claude/rules.md` first** - don't rely on memory
- **Base commit ONLY on staged diff** - ignore conversation history
- **Follow ALL rules** from `.claude/rules.md` exactly
