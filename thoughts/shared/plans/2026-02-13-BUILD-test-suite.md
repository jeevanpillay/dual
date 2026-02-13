# Test Suite Implementation Plan

## Overview

Create E2E integration tests that exercise the full Dual workspace lifecycle with real Docker containers and tmux sessions. Tests are mapped from the 27 validated architecture claims.

## Current State Analysis

- Test harness (`tests/harness/mod.rs`) provides RAII TestFixture
- Test fixtures (`tests/fixtures/mod.rs`) create local git repos
- All MVP modules built and unit-tested (65 tests)
- Architecture fully validated (27/27 claims confirmed)

## Desired End State

A `tests/e2e.rs` integration test file with tests covering:
1. Clone: local clone creates workspace, filesystem layout correct
2. Container: full lifecycle (create → start → exec → stop → destroy)
3. Shell: command classification and RC generation
4. Bind mount: host file edits visible in container
5. Network isolation: 2 containers bind same port without conflict
6. Exit codes: preserved through docker exec

All tests use RAII cleanup, UUID naming, and run with real Docker/tmux.

## What We're NOT Doing

- NOT testing reverse proxy (requires async HTTP server, complex setup)
- NOT testing tmux attach (requires TTY, not available in CI/test)
- NOT testing shell interception end-to-end (requires sourcing RC in running shell)
- NOT modifying src/ modules

## Phase 1: Create E2E Test File

### Tests:

1. **clone_creates_workspace** — git clone --local creates workspace dir
2. **clone_filesystem_layout** — workspace dir matches expected layout
3. **container_lifecycle** — create → start → status → stop → destroy
4. **container_exec_returns_output** — docker exec runs command and gets output
5. **container_exec_exit_codes** — exit codes preserved (0, 1, 42)
6. **bind_mount_host_to_container** — file written on host visible in container
7. **network_isolation_same_port** — 2 containers bind :3000 simultaneously
8. **tmux_session_lifecycle** — create → is_alive → destroy

### Success Criteria:

#### Automated Verification:
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes (unit tests, no Docker needed)
- [ ] `cargo test --test e2e` passes (requires Docker + tmux)
- [ ] `cargo clippy` clean
- [ ] `cargo fmt --check` clean

## References

- Architecture: thoughts/ARCHITECTURE.md (27 validated claims)
- Test harness: tests/harness/mod.rs
- Test fixtures: tests/fixtures/mod.rs
