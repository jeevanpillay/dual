# End-to-End Verification Plan

## Overview

Verify the full dual workspace flow works end-to-end with real Docker and tmux. This is a validation module — it tests the wired CLI, not new code.

## Prerequisites

- Docker 29.2.0 available
- tmux 3.5a available
- dual binary built at target/debug/dual

## Test Setup

Create `/tmp/dual-e2e/dual.toml`:
```toml
workspace_root = "/tmp/dual-e2e-workspaces"

[[repos]]
name = "dual-test"
url = "/Users/jeevanpillay/Code/@jeevanpillaystudios/dual"
branches = ["main"]
```

## Phase 1: List (Lazy State)

Run `dual list` from /tmp/dual-e2e. Expect:
```
  dual-test-main                 ◌ lazy
```

### Success Criteria:
- [x] Output shows "lazy" status
- [x] No errors

## Phase 2: Launch Workspace

Run `dual launch dual-test-main` from /tmp/dual-e2e.

### Success Criteria:
- [x] Clone directory exists with .git
- [x] Container is running (docker inspect shows `true`)
- [x] Shell RC file exists at ~/Library/Application Support/dual/rc/
- [x] tmux session is alive
- [x] tmux attach fails with "not a terminal" (expected in non-interactive shell)

## Phase 3: List (Running State)

Run `dual list` from /tmp/dual-e2e. Expect:
```
  dual-test-main                 ● attached
```

### Success Criteria:
- [x] Output shows "attached" status (container running + tmux alive)

## Phase 4: Verify Container Exec

Run `docker exec dual-dual-test-main ls /workspace` to verify bind mount works.

### Success Criteria:
- [x] Can see repo files inside container (Cargo.toml visible)

## Phase 5: Verify Shell RC

Check that the RC file intercepts commands correctly.

### Success Criteria:
- [x] RC file contains DUAL_CONTAINER export
- [x] RC file contains function wrappers for npm, pnpm, etc.

## Phase 6: Destroy Workspace

Run `dual destroy dual-test-main` from /tmp/dual-e2e.

### Success Criteria:
- [x] tmux session destroyed
- [x] Container stopped and removed
- [x] Clone directory removed
- [x] Exit code 0

## Phase 7: List (Back to Lazy)

Run `dual list` from /tmp/dual-e2e. Expect "lazy" again.

### Success Criteria:
- [x] Output shows "lazy" status

## Cleanup

- [x] Removed /tmp/dual-e2e and /tmp/dual-e2e-workspaces
