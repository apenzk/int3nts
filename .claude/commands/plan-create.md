---
description: Create a new implementation plan
---

# Create Plan

Create a new plan and save it to `docs/architecture/plan/plan.md`.

## Process

1. Discuss the goal and scope with the user
2. **Read the codebase** to understand the current state before proposing changes
3. Write the plan to `docs/architecture/plan/plan.md`

## Plan template

1. **Title** — short name for the effort
2. **Progress table** — stage/status tracker, updated as each stage completes
3. **Goal** — what we're trying to achieve and why
4. **Stage protocol (MUST follow for every stage)** — the workflow for completing each stage:
   1. Run the relevant tests (commands given per stage)
   2. Run `/review-me` and wait for review output
   3. **Ask the user: "Ready to commit?"**
   4. Only if the user says yes: run `/commit`
   5. Do not proceed to the next stage without user confirmation
5. **Stages** — ordered, each with:
   - **Scope** — which directories/components are touched (keep stages focused)
   - **Files to change** — specific files with concrete before/after descriptions
   - **Test command** — exact command to verify the stage
   - **End of stage** — `Run tests → /review-me → ask user → if yes, /commit.`
