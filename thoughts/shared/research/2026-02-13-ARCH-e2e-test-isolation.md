---
date: 2026-02-13T06:00:00+08:00
researcher: Claude
git_commit: 08598f0ccb984e1dd284eb7f76d0db95987f1421
branch: feature/build-loop-pipeline
repository: dual
hypothesis: "UUID-namespaced resources + RAII cleanup prevents cross-test contamination in parallel E2E tests"
tags: [experiment, research, rust, drop, raii, docker, tmux, testing]
status: research_complete
last_updated: 2026-02-13
last_updated_by: Claude
---

# Research: E2E Test Isolation via UUID Naming + RAII Cleanup

**Date**: 2026-02-13
**Researcher**: Claude
**Git Commit**: 08598f0
**Branch**: feature/build-loop-pipeline
**Repository**: dual

## Hypothesis

Each E2E test can use UUID-namespaced resources (containers, tmux sessions, temp dirs) that don't collide with parallel tests, and Rust's Drop guards guarantee cleanup even on panic.

## Why This Matters

Tests must be parallelizable (`cargo test` runs in parallel by default) and leave no orphaned containers/sessions that could interfere with subsequent runs or user workspaces.

## What We're Testing

- **Primary claim**: UUID naming prevents collisions; Drop guarantees cleanup
- **Success criteria**: Parallel tests don't interfere; resources cleaned up after panic
- **Scope boundary**: Test isolation mechanisms, not test framework design

## Feasibility Assessment

### Rust Drop Semantics

| Scenario | Drop Runs? |
|---|---|
| Normal scope exit | YES |
| Panic (unwind strategy) | YES |
| Panic (abort strategy) | **NO** |
| Double panic during unwinding | **NO** (aborts) |
| `std::process::exit()` | **NO** |
| `std::process::abort()` | **NO** |
| `core::mem::forget()` | **NO** |
| SIGKILL | **NO** (uncatchable) |
| SIGTERM (default handler) | **NO** |

**Key insight**: For test assertion failures (which use `panic!`), Drop DOES fire with the default `unwind` strategy. This covers the most common E2E test failure mode.

### Docker Container Naming

- Allowed characters: `a-z`, `A-Z`, `0-9`, `-`, `_`, `.`
- Max length: 63 characters (RFC 1123)
- UUID-based name like `dual-test-cd2d0cb2-10ac-4dbc-bc51-43630e49f9e1` = 46 chars (within limit)
- Docker rejects duplicate names (fail-fast, not silent corruption)

**Local probe**: Successfully created container `dual-test-58fdaac4` with 8-char UUID prefix.

### tmux Session Naming

- Colons and periods silently replaced with underscores
- All UUID characters (`[0-9a-f-]`) pass through unchanged
- No defined maximum length (confirmed by tmux maintainer, tested 210 chars)
- tmux rejects duplicate session names

**Local probe**: Successfully created and destroyed session `dual-test-f2845ce4`.

### Parallel Test Safety

`cargo test` runs tests in parallel by default. UUID-namespaced resources prevent collisions. Both Docker and tmux reject duplicate names, providing fail-fast behavior rather than silent corruption.

### Existing Codebase State

- No existing `impl Drop` in the codebase — RAII cleanup is greenfield
- Current naming: `dual-{repo}-{encoded_branch}` (deterministic) — test containers with `dual-test-{uuid}` prefix won't collide with production naming pattern
- Container listing uses `--filter "name=dual-"` prefix — test containers visible but distinguishable by `dual-test-` prefix
- `shell::write_rc_file` test uses fixed name and manual cleanup — not UUID-based

### Constraints & Limitations

1. **SIGKILL gap**: Drop does not fire on SIGKILL. Orphaned resources possible.
2. **panic=abort**: If `Cargo.toml` sets `panic = "abort"` for test profiles, Drop won't fire on assertion failures. Must use default `panic = "unwind"`.
3. **Test prefix visibility**: `dual-test-*` containers appear in `container::list_all()` results (uses `name=dual-` filter). Minor concern for production/test interference.

### Mitigations for SIGKILL Gap

1. **Prefix-based cleanup sweep**: Run `docker rm -f $(docker ps -aq --filter "name=dual-test-")` and `tmux kill-session -t dual-test-*` before/after test suite
2. **Idempotent test setup**: Each test attempts cleanup at start AND end
3. **CI post-step**: Add cleanup step in GitHub Actions workflow
4. **Container TTL**: Create containers with `--stop-timeout` for time-based defense

## Evidence Assessment

### Supporting Evidence
- UUID naming verified working for both Docker and tmux (local probes)
- Drop fires on panic with unwind strategy (Rust Reference)
- Docker/tmux reject duplicate names (fail-fast behavior)
- `cargo test` parallel execution is standard

### Contradicting Evidence
- Drop does NOT fire on SIGKILL, SIGTERM, process::exit() — gap exists but is mitigable

## References

- [Drop trait - Rust](https://doc.rust-lang.org/std/ops/trait.Drop.html)
- [Unwinding - Rustonomicon](https://doc.rust-lang.org/nomicon/unwinding.html)
- [Rust Reference - Destructors](https://doc.rust-lang.org/reference/destructors.html)
- [Docker Container Naming](https://www.codestudy.net/blog/docker-restrictions-regarding-naming-container/)
- [tmux session names - GitHub #3113](https://github.com/tmux/tmux/issues/3113)
- [Controlling How Tests Are Run - Rust Book](https://doc.rust-lang.org/book/ch11-02-running-tests.html)

## Probing Log

```bash
# UUID generation
uuidgen | tr '[:upper:]' '[:lower:]'  # dec8472a-6a44-42e3-a39b-3c074522ccce

# Docker UUID container
docker run -d --name "dual-test-58fdaac4" alpine sleep 10  # SUCCESS
docker ps --format '{{.Names}}' | grep dual-test            # dual-test-58fdaac4
docker rm -f dual-test-58fdaac4                              # SUCCESS

# tmux UUID session
tmux new-session -d -s "dual-test-f2845ce4"  # SUCCESS
tmux list-sessions                            # dual-test-f2845ce4: ...
tmux kill-session -t "dual-test-f2845ce4"     # SUCCESS

# Full UUID name length
echo -n "dual-test-cd2d0cb2-10ac-4dbc-bc51-43630e49f9e1" | wc -c  # 46 chars
```

## Assumptions Made

- Test profile uses `panic = "unwind"` (Rust default)
- Tests use `dual-test-{uuid}` prefix, not `dual-{repo}-{branch}` pattern
- Cleanup sweep acceptable as defense-in-depth for SIGKILL gap
