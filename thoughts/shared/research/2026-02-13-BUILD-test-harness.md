---
date: 2026-02-13
researcher: Claude
git_commit: d28fd7d
branch: feature/build-loop-pipeline
repository: dual
topic: "E2E Test Harness Infrastructure"
tags: [research, codebase, test-harness, e2e, raii, integration-tests]
status: complete
last_updated: 2026-02-13
last_updated_by: Claude
---

# Research: E2E Test Harness Infrastructure

**Date**: 2026-02-13
**Researcher**: Claude
**Git Commit**: d28fd7d
**Branch**: feature/build-loop-pipeline
**Repository**: dual

## Research Question

What infrastructure is needed to build an RAII test harness for E2E tests that manages Docker containers, tmux sessions, and temp directories with proper cleanup?

## Summary

The test harness requires a structural change to the crate: converting from a pure binary to a library + binary so integration tests in `tests/` can import module APIs. The harness itself is a `TestFixture` struct with RAII `Drop` that cleans up Docker containers, tmux sessions, and temp directories. UUID-based naming (`dual-test-{uuid}`) prevents cross-test contamination. A prefix-based cleanup sweep handles the SIGKILL gap.

## Detailed Findings

### Binary Crate Restructuring Required

**Current state**: `src/main.rs` declares all modules via `mod` statements. This makes the project a binary crate only. Integration tests in `tests/` cannot import from binary crates — Rust's test framework only supports importing from library crates.

**Required change**: Create `src/lib.rs` that declares and re-exports all modules. Have `src/main.rs` use `dual::*` to import from the library crate. This is the standard Rust pattern for projects that need both a binary and integration tests.

**Files affected**:
- New: `src/lib.rs` — module declarations and public exports
- Modified: `src/main.rs` — remove `mod` declarations, add `use dual::*`

### Module Public APIs for Test Harness

The test harness wraps these module functions:

| Module | Create | Destroy | Status Check |
|--------|--------|---------|--------------|
| container | `container::create(config, repo, branch, image)` | `container::stop(name)` + `container::destroy(name)` | `container::status(name)` |
| tmux | `tmux::create_session(name, cwd, shell_rc)` | `tmux::destroy(name)` | `tmux::is_alive(name)` |
| clone | `clone::clone_workspace(config, repo, url, branch)` | `clone::remove_workspace(config, repo, branch)` | `clone::workspace_exists(config, repo, branch)` |
| config | `config::parse(toml_str)` | N/A | N/A |

### UUID Naming Strategy

- **Pattern**: `dual-test-{uuid}` (e.g., `dual-test-cd2d0cb2-10ac-4dbc-bc51-43630e49f9e1`)
- **Docker limit**: 63 characters — pattern uses 46 chars (safe)
- **tmux limit**: None — no constraint
- **Prefix**: `dual-test-` distinguishes test resources from production `dual-` resources
- **Collision prevention**: UUID v4 provides 2^122 unique values

### RAII Drop Behavior

- Drop fires on: normal scope exit, panic with unwind (Rust default, covers test assertion failures)
- Drop does NOT fire on: SIGKILL, SIGTERM (default handler), `std::process::abort()`, `panic = "abort"`
- **Rust test default**: `panic = "unwind"` — Drop IS guaranteed for test assertion failures
- **Defense-in-depth**: Prefix-based cleanup sweep for SIGKILL gap

### Cleanup Strategy

**Container cleanup**: `docker rm -f {name}` — force-removes even running containers (one command vs stop+rm)
**tmux cleanup**: `tmux kill-session -t {name}` — kills session and all processes
**Temp dir cleanup**: `std::fs::remove_dir_all(path)` — recursive deletion
**Cleanup sweep**: Remove all `dual-test-*` prefixed containers and sessions

### Dependencies Needed

- `uuid` (dev-dependency) — UUID v4 generation for test naming
- No other new dependencies required

## Code References

- `src/main.rs:1-7` — Current mod declarations (need to move to lib.rs)
- `src/container.rs:23-48` — Container creation
- `src/container.rs:56-63` — Container stop + destroy
- `src/container.rs:89-105` — Container status check
- `src/tmux.rs:20-46` — Session creation
- `src/tmux.rs:72-74` — Session destroy
- `src/tmux.rs:77-82` — Session alive check
- `src/config.rs:131-136` — `parse()` function for test configs

## Architecture Constraints

From validated ARCHITECTURE.md decisions:
- **e2e-test-isolation** (CONFIRMED WITH CAVEATS): UUID naming + RAII Drop + prefix cleanup
- **e2e-local-fixture-repo** (CONFIRMED): Local git repos in /tmp for fixtures
- **e2e-ci-environment** (CONFIRMED): Docker pre-installed, tmux installable via apt
