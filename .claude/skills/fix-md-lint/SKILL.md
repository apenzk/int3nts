---
name: fix-md-lint
description: Go through markdown files in the repo and fix linting errors (MD031, MD032, etc.)
disable-model-invocation: true
context: fork
agent: general-purpose
---

# Fix Markdown Linting Errors

This skill goes through markdown files in the repo one by one and fixes common linting errors.

## Common Linting Rules to Fix

### MD031: Fenced code blocks should be surrounded by blank lines

**Bad:** Code block immediately after text without blank line.

**Good:** Add blank line before and after code blocks.

### MD032: Lists should be surrounded by blank lines

**Bad:** List immediately after text without blank line.

**Good:** Add blank line before and after lists.

### MD024: Multiple headings with the same content

Each heading should be unique within a document. If you have duplicate headings, make them more specific.

### MD009: Trailing spaces

Remove trailing whitespace from lines.

### MD010: Hard tabs

Use spaces instead of tabs.

### MD012: Multiple consecutive blank lines

Use only single blank lines.

### MD022: Headings should be surrounded by blank lines

**Bad:** Heading immediately after text without blank line.

**Good:** Add blank line before and after headings.

### MD023: Headings must start at the beginning of the line

Don't indent headings.

### MD040: Fenced code blocks should have a language specified

**Bad:** Using just triple backticks without a language.

**Good:** Always specify a language after the opening backticks:

- `bash` for shell commands
- `text` for plain text output
- `markdown` for markdown examples
- `json` for JSON
- `typescript` or `ts` for TypeScript
- `javascript` or `js` for JavaScript
- `rust` for Rust
- `move` for Move
- `solidity` for Solidity

### MD047: Files should end with a single newline character

Ensure file ends with exactly one newline.

## Steps

1. Find all markdown files in the repo:

   ```bash
   find . -name "*.md" -type f | grep -v node_modules | grep -v build | grep -v target
   ```

2. For each markdown file:

   - Read the file
   - Check for linting issues (look for patterns that violate the rules above)
   - Fix the issues
   - Move to the next file

3. Focus on these patterns:

   - Code blocks not surrounded by blank lines → add blank lines
   - Code blocks without language specifier → add appropriate language
   - Lists not surrounded by blank lines → add blank lines
   - Headings not surrounded by blank lines → add blank lines
   - Multiple consecutive blank lines → reduce to single
   - Trailing whitespace → remove
   - Missing final newline → add

4. Process files one at a time to avoid overwhelming context

## Important Notes

- Skip `node_modules/`, `build/`, `target/`, and other generated directories
- Don't change the semantic meaning of content
- Only fix formatting issues
- Report which files were fixed and what changes were made
