---
description: Check CI test results for a PR and investigate failures
---

# Check CI for PR

Investigate CI test results for a given PR number.

**PR number provided**: $ARGUMENTS

## Step 1: Determine PR Number

**If a PR number is provided in $ARGUMENTS**, use that PR number.

**If no PR number is provided**, auto-detect from the current branch:

```bash
# Get current branch name
git branch --show-current

# Find PR associated with current branch
gh pr list --head "$(git branch --show-current)" --json number,title,url,state --jq '.[] | "PR #\(.number): \(.title)\nURL: \(.url)\nState: \(.state)"'
```

If no PR is found for the current branch, ask the user for a PR number.

## Step 2: Show PR Info

Display basic PR information:

```bash
gh pr view <PR_NUMBER> --json number,title,url,headRefName,baseRefName,state --jq '"PR #\(.number): \(.title)\nBranch: \(.headRefName) → \(.baseRefName)\nState: \(.state)\nURL: \(.url)"'
```

Present as:

```text
PR #<number>: <title>
Branch: <head> → <base>
State: <open/closed/merged>
URL: <url>
```

## Step 3: List All CI Checks

Run:

```bash
gh pr checks <PR_NUMBER>
```

Present the results in a clear format:

```text
📊 CI STATUS FOR PR #<number>

✅ PASSING:
- <job name> (<duration>)
- ...

❌ FAILING:
- <job name> (<duration>)
- ...

⏳ PENDING:
- <job name>
- ...
```

## Step 4: Ask User Which Check to Investigate

Use AskUserQuestion to ask which check(s) the user wants to investigate. Include options for:

- Each failing check (prioritize these)
- Each passing check
- "All failing checks"
- Option to provide a custom job name

## Step 5: Fetch Logs for Selected Check

For the selected check(s), fetch the logs:

```bash
# Get the run ID and job ID from the check URL
# URL format: https://github.com/<org>/<repo>/actions/runs/<run_id>/job/<job_id>

# View failed steps only (preferred for failures)
gh run view <run_id> --job <job_id> --log-failed

# If user wants full logs
gh run view <run_id> --job <job_id> --log
```

## Step 6: Analyze and Present Findings

After fetching logs:

1. **Identify the failure point** - What step failed? What was the error message?
2. **Extract relevant context** - Show the key error lines, not the entire log
3. **Suggest possible causes** - Based on error patterns:
   - Timeout issues
   - Test failures (show which tests)
   - Build failures (show compiler errors)
   - Infrastructure issues (Docker, network, etc.)

Present findings as:

```text
🔍 ANALYSIS: <job name>

❌ Failed at: <step name>

Error:
<relevant error excerpt>

Possible causes:
- <cause 1>
- <cause 2>

Suggested next steps:
- <action 1>
- <action 2>
```

## Step 7: Offer Follow-up Actions

Ask if the user wants to:

- Investigate another check
- See more context from the logs
- Compare with a last successful run
- View the full log output

## Tips for Log Analysis

- E2E test failures often have container logs - look for Docker output
- Rust test failures show the test name and assertion that failed
- Movement/Aptos errors often include transaction hashes
- Look for "error:", "FAILED", "panic", or "Error:" patterns
- Timeout failures may need infrastructure investigation

## Important Notes

- Parse job IDs from the check URLs automatically
- Truncate very long logs - show most relevant parts
- For "All failing checks", summarize each briefly first, then offer deep dives
