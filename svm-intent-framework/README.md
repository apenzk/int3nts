# SVM Intent Framework

Native Solana program for SVM escrows.

 **Full documentation: [docs/svm-intent-framework/](../docs/svm-intent-framework/README.md)**

## Quick Start

```bash
# Enter dev shell (includes Solana)
nix develop

# Build and test
cd svm-intent-framework
./scripts/test.sh
```

## Build Script

**Always use `./scripts/build.sh`** for building. The script handles all toolchain workarounds automatically:

| What it does | Why |
|--------------|-----|
| Auto-enters `nix develop` | Ensures correct environment |
| Uses `cargo build-sbf` | Native Solana build command |
| Downgrades `Cargo.lock` to v3 | Solana's Rust 1.84 can't read v4 |
| Pins `constant_time_eq` to v0.3.x | Avoids `edition2024` crates |

You can pass arguments through: `./scripts/build.sh --verifiable`

## ️ Toolchain Constraints

> **Design Decision**: This project intentionally constrains its dependency graph to remain compatible with Solana's pinned Rust toolchain (1.84.x). Newer crates with `edition2024` dependencies violate this constraint. This is not accidental tech debt—remove these workarounds when Solana bumps to Rust 1.85+.

**As of Jan 2026**, Solana's bundled Rust (1.84.0) has compatibility issues:

| Issue | Cause | Workaround |
|-------|-------|------------|
| `lock file version 4 requires -Znext-lockfile-bump` | System cargo 1.86+ creates v4 lockfiles | Keep `Cargo.lock` at version 3 |
| `feature edition2024 is required` | Some crates use `edition = "2024"` | Pin dependencies to avoid edition2024 crates |

### Manual Lockfile Regeneration

**Do NOT regenerate `Cargo.lock` blindly.** If you must:

```bash
cargo generate-lockfile
sed -i 's/version = 4/version = 3/' Cargo.lock  # GNU sed
# or: sed -i '' 's/version = 4/version = 3/' Cargo.lock  # macOS sed
```

Then verify no edition2024 crates snuck in:

```bash
grep -A1 'name = "constant_time_eq"' Cargo.lock  # Should show v0.3.x, not v0.4.x
```

### When to Remove These Workarounds

Check periodically and remove when **all** conditions are met:

- [ ] `solana --version` shows Rust ≥1.85 bundled
- [ ] Cargo.lock v4 is accepted by Solana's cargo
- [ ] No need to pin `constant_time_eq` or `blake3` versions

Then:

1. Remove dependency pinning from `scripts/build.sh`
2. Remove Cargo.lock version downgrade logic
3. Update this README
