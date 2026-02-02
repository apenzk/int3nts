# Claude Code Instructions

**Import project README and overview, treat as if import is in the main CLAUDE.md file.**
@../README.md

**Import project rules and conventions, treat as if import is in the main CLAUDE.md file.**
@./.claude/rules.md

## Test Commands - CRITICAL

**CRITICAL: Running tests without `nix develop` wrapper WILL FAIL.**

**ALL test commands MUST use the exact format from root README.md "Testing" section.**

Pattern:
```bash
nix develop ./nix -c bash -c "<test command>"
```

**Do NOT run test commands directly. They will fail without the nix environment.**

## No Fallbacks Policy

**CRITICAL: No fallbacks, workarounds, or graceful degradation.** Code either works or it fails. Tests either pass or they fail. There are no temporary workarounds.

- **No silent fallbacks**: If a feature doesn't work, it must error explicitly
- **No try/catch swallowing errors**: Exceptions must propagate or be handled meaningfully
- **No default values hiding failures**: Missing data should cause failures, not silently use defaults
- **No "best effort" patterns**: Operations succeed fully or fail completely
- **No temporary workarounds**: Fix the root cause, don't patch around it
- **Tests must fail on errors**: Test assertions must be strict, not lenient
