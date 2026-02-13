---
date: 2026-02-13T06:00:00+08:00
researcher: Claude
git_commit: 08598f0ccb984e1dd284eb7f76d0db95987f1421
branch: feature/build-loop-pipeline
repository: dual
hypothesis: "A git init'd temp directory serves as test repo without network dependency"
tags: [experiment, research, git, clone, local, testing, e2e]
status: research_complete
last_updated: 2026-02-13
last_updated_by: Claude
---

# Research: Local Fixture Repo for E2E Tests

**Date**: 2026-02-13
**Researcher**: Claude
**Git Commit**: 08598f0
**Branch**: feature/build-loop-pipeline
**Repository**: dual

## Hypothesis

E2E tests can use `git init` + commit in /tmp as the "remote" repo, cloned via `--local` flag, avoiding any network dependency. Local clones are fast enough for test iteration.

## Why This Matters

E2E tests must not depend on GitHub/network availability. Network clones take 1-5 seconds and introduce flakiness. Local clones provide deterministic, fast, offline test execution.

## What We're Testing

- **Primary claim**: `git clone --local /tmp/fixture` works and is fast
- **Success criteria**: Clone succeeds, hardlinks work, <100ms, Dual's clone module handles it
- **Scope boundary**: Clone mechanism feasibility, not test fixture design

## Feasibility Assessment

### Git Local Clone Mechanics

`git clone --local` optimizes by using **hardlinks** for objects in `.git/objects/`. Source and destination must be on the same filesystem.

When you omit `--local` for a bare path (starting with `/`), git auto-detects it's local and applies the optimization automatically. The `--local` flag is redundant but communicates intent.

| Method | Works? | Timing | Notes |
|--------|--------|--------|-------|
| `git clone --local /tmp/...` | Yes | ~64ms | Hardlinks objects |
| `git clone /tmp/...` (auto-detect) | Yes | ~65ms | Auto-detects local |
| `git clone file:///tmp/...` | Yes | ~65ms | No hardlinks (copies) |

### Local Probe Results

5 runs with a 20+ file fixture repo:

```
Run 1: 64.1ms
Run 2: 62.2ms
Run 3: 65.9ms
Run 4: 64.6ms
Run 5: 63.7ms
Average: ~64ms
```

Higher than the claimed ~10ms but still sub-100ms and negligible vs network clones (1-5 seconds).

### Hardlink Verification

Inode analysis confirmed hardlinks work: fixture repo and clone share identical inodes for pack files. Link count of `2` on these files confirms hardlinking.

On macOS, `/tmp` symlinks to `/private/tmp`, and `mktemp` creates under `/private/var/folders/...` — all on the same APFS data volume, so hardlinks work.

### Dual Clone Module Compatibility

The clone module at `src/clone.rs` **already handles local paths correctly**:

```rust
// src/clone.rs:7-13
pub fn is_local_path(url: &str) -> bool {
    if url.contains("://") { return false; }
    url.starts_with('/') || url.starts_with("./") || url.starts_with("../") || url.starts_with("~/")
}
```

`/tmp/fixture-repo` starts with `/` → `is_local_path()` returns `true` → `--local` flag added.

The `clone_workspace` function (lines 40-87) then uses this to build the correct git clone command.

**No URL validation rejects /tmp paths.** The config module's `validate` function only checks that `name` and `url` are non-empty strings.

### Config Format Compatibility

A `dual.toml` with local paths is fully valid:

```toml
workspace_root = "/tmp/test-workspaces"

[[repos]]
name = "test-app"
url = "/tmp/fixture-repo"
branches = ["main"]
```

The `config::parse()` function can parse from a TOML string directly, useful for programmatic test setup.

### CI Environment

- `/tmp` always available on GitHub Actions (standard Linux filesystem)
- `mktemp` available (GNU coreutils, pre-installed)
- `$RUNNER_TEMP` points to `/home/runner/work/_temp` (cleaned per job)

## Evidence Assessment

### Supporting Evidence
- Local clone works from /tmp paths (local probe, 5 runs)
- Hardlinks confirmed via inode analysis
- Dual's `is_local_path()` correctly detects `/tmp/...` paths
- Existing unit tests already test local path detection
- Branch-specific cloning works (`git clone --local -b feat/auth`)
- `/tmp` and `mktemp` available in GitHub Actions

### Contradicting Evidence
- Clone timing is ~64ms not ~10ms (still acceptable, sub-100ms)

## References

- `src/clone.rs:7-13` — `is_local_path()` function
- `src/clone.rs:40-87` — `clone_workspace()` function
- `src/clone.rs:151-159` — existing unit tests for local path detection
- `src/config.rs:138-153` — `validate()` function (no URL format validation)

## Probing Log

```bash
# Create fixture repo
FIXTURE_DIR=$(mktemp -d)
cd "$FIXTURE_DIR"
git init && git config user.email "test@test.com" && git config user.name "Test"
echo '{"name": "test-app"}' > package.json
git add -A && git commit -m "initial"

# Clone it
CLONE_DIR=$(mktemp -d)
time git clone --local "$FIXTURE_DIR" "$CLONE_DIR/test-clone"
# real 0m0.064s

# Verify
ls -la "$CLONE_DIR/test-clone/"    # files present
cat "$CLONE_DIR/test-clone/package.json"  # correct content
cd "$CLONE_DIR/test-clone" && git log --oneline  # correct history

# Cleanup
rm -rf "$FIXTURE_DIR" "$CLONE_DIR"
```

## Assumptions Made

- Tests create fixture repos on the same filesystem as clone destination (required for hardlinks)
- The ~64ms clone time is acceptable for E2E tests (vs claimed ~10ms)
- Dual's config accepts local paths (verified through code analysis)
