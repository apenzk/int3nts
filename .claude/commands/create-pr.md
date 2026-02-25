---
description: Create a pull request using the project PR template
---

# Create Pull Request

Write a PR description draft to `pr.md` in the project root for the user to review and iterate on. Once approved, update the existing PR (or create one if none exists) and delete `pr.md`.

**PR target provided**: $ARGUMENTS

## Step 1: Gather Context

Run these commands in parallel to understand the current state:

```bash
# Current branch
git branch --show-current

# Check if branch tracks a remote and is pushed
git status -sb

# Recent commits on this branch vs main
git log main..HEAD --oneline

# Full diff against main
git diff main...HEAD --stat

# Find existing PR for this branch
gh pr list --head "$(git branch --show-current)" --json number,title,url --jq '.[] | "PR #\(.number): \(.title) - \(.url)"'
```

If there are uncommitted changes, **STOP** and inform the user they should commit first.

Note the existing PR number if one exists.

## Step 2: Determine Base Branch and Title

- **Base branch**: Use `main` unless `$ARGUMENTS` specifies a different target
- **PR title**: Derive from the branch commits. Keep under 70 characters. Use conventional commit style (e.g., `feat:`, `fix:`, `chore:`)

## Step 3: Analyze Changes

Read the full diff to understand what changed:

```bash
git diff main...HEAD
git log main..HEAD --format="%h %s%n%b"
```

Categorize the changes to fill the template:

- What types of changes are included? (bug fix, feature, docs, refactor, breaking, tests, CI/CD)
- Is there a summary description?
- Are there breaking changes?
- Are there related issues?

## Step 4: Write Draft to `pr.md`

Read the PR template at `.github/pull_request_template.md`.

Write the filled-in template to `pr.md` in the project root. The first line of `pr.md` must be the PR title as a markdown heading (e.g., `# feat: add cancellation policies`). Everything after is the PR body.

Fill in every section of the template:

- **Summary**: 2-4 sentences describing the PR
- **Types of Changes**: Check the relevant boxes with `[x]`
- **Description**: Detailed description organized by component/area. Use bullet points.
- **Breaking Changes**: List them or remove the section entirely if none
- **Related Issues**: Link issues or remove the section entirely if none
- **Checklist**: Check items that apply based on the actual changes

**Rules for filling the template:**

- Remove HTML comments (`<!-- ... -->`)
- Remove sections that don't apply (e.g., "Breaking Changes" if none, "Related Issues" if none)
- Be specific and factual — base content ONLY on the actual diff
- Do NOT mention AI/Claude/LLM in the PR body

## Step 5: Ask User to Review

Tell the user that the draft is at `pr.md` and ask them to review it. They can:

- Ask for changes (you will edit `pr.md`)
- Edit `pr.md` directly themselves
- Approve and proceed

**Do NOT proceed until the user explicitly approves.**

## Step 6: Update or Create PR

Once the user approves:

1. Read `pr.md` to get the final content
2. Extract the title from the first `#` heading line
3. Use everything after the title line as the PR body

**If a PR already exists** (detected in Step 1), update it:

```bash
gh pr edit <PR_NUMBER> --title "<title>" --body "$(cat <<'EOF'
<body content>
EOF
)"
```

**If no PR exists**, create one:

```bash
gh pr create --title "<title>" --base <base-branch> --body "$(cat <<'EOF'
<body content>
EOF
)"
```

## Step 7: Clean Up and Report

After the PR is updated/created successfully:

1. Delete `pr.md`:

```bash
rm pr.md
```

2. Display the result:

```text
PR updated/created: <url>
Title: <title>
Base: <base> <- <head>
```
