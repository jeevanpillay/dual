---
date_validated: 2026-02-13T06:05:00+08:00
research_doc: thoughts/shared/research/2026-02-13-ARCH-e2e-test-isolation.md
verdict: confirmed_with_caveats
---

# Validation Report: E2E Test Isolation via UUID Naming + RAII Cleanup

## Verdict: CONFIRMED WITH CAVEATS

UUID-namespaced resources reliably prevent cross-test contamination. RAII/Drop guards provide cleanup for the most common failure mode (test assertion failures via panic). The SIGKILL gap is real but manageable through prefix-based sweep cleanup.

## Hypothesis Tested

**Original hypothesis**: UUID naming + RAII cleanup prevents cross-test contamination

**What we empirically tested**: UUID naming compatibility with Docker/tmux, Drop semantics, parallel test isolation

## Test Results Summary

| Test | Expected | Actual | Verdict |
|------|----------|--------|---------|
| Docker UUID naming | Works within 63-char limit | 46-char name works, chars allowed | ✓ |
| tmux UUID naming | Works with no length limit | UUID chars pass through unchanged | ✓ |
| Duplicate name rejection | Docker/tmux reject duplicates | Both reject with errors (fail-fast) | ✓ |
| Drop on panic (unwind) | Destructors run | Confirmed by Rust spec | ✓ |
| Drop on SIGKILL | Destructors DON'T run | Confirmed — uncatchable signal | ✗ (known gap) |
| Drop on SIGTERM | Destructors DON'T run (default) | Confirmed — needs signal handler | ✗ (mitigable) |
| Parallel test safety | UUID prevents collisions | cargo test parallel + unique names = safe | ✓ |

## Detailed Analysis

### UUID Naming
Docker container names accept `[a-zA-Z0-9_.-]` with 63-char max. A full UUID name (`dual-test-{uuid}`) is 46 chars — well within the limit. tmux has no length limit and UUID characters pass unchanged.

Both Docker and tmux reject duplicate names with errors, providing fail-fast behavior rather than silent corruption.

### RAII/Drop Guarantees
Drop fires on panic with the default `unwind` strategy (covers test assertion failures). Drop does NOT fire on: SIGKILL, SIGTERM (default handler), `process::exit()`, `process::abort()`, `panic = "abort"`, `mem::forget()`, double panic.

### Parallel Test Safety
`cargo test` runs tests in parallel by default. UUID-namespaced resources ensure no collisions. The `dual-test-{uuid}` prefix is distinct from production's `dual-{repo}-{branch}` pattern.

## Caveats (Modification Needed)

The claim "Drop guards guarantee cleanup even on panic" is accurate for the `unwind` strategy, but must be supplemented with:

1. **Prefix-based cleanup sweep**: Run before/after test suite to catch orphaned resources from SIGKILL/crash
2. **Ensure `panic = "unwind"`**: Must not set `panic = "abort"` in test profile
3. **Idempotent test setup**: Each test should attempt cleanup at start AND end
4. **CI post-step cleanup**: GitHub Actions workflow should include cleanup step

## Evidence Summary

| Category | Count | Summary |
|----------|-------|---------|
| For hypothesis | 5 | UUID naming works, Drop fires on panic, parallel safe, fail-fast duplicates, prefix isolation |
| Against hypothesis | 2 | Drop doesn't fire on SIGKILL/SIGTERM (mitigable with cleanup sweep) |

## Conclusion

The claim is validated with caveats. UUID naming + Drop cleanup covers the primary failure mode (test assertion panics). The SIGKILL gap requires supplemental cleanup mechanisms (prefix sweep, CI post-step), which are straightforward to implement. The existing codebase's container listing already uses prefix filtering, making sweep cleanup architecturally natural.
