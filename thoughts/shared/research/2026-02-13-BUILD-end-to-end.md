---
date: 2026-02-13T00:00:00+00:00
researcher: Claude
git_commit: 8864b9f
branch: feature/build-loop-pipeline
repository: dual
topic: "End-to-end verification of dual workspace flow"
tags: [research, build, integration, end-to-end]
status: complete
last_updated: 2026-02-13
last_updated_by: Claude
---

# Research: End-to-End Verification

## Research Question

Can the wired CLI successfully launch, list, and destroy a workspace using real Docker and tmux?

## Summary

Docker 29.2.0 and tmux 3.5a are available. The test will use the dual repository itself as the test repo (local clone), creating a workspace that:
1. Clones the repo to ~/dual-workspaces/dual-test/main
2. Creates a Docker container with node:20 + sleep infinity
3. Writes shell RC to ~/.config/dual/rc/
4. Creates a tmux session with shell interceptors sourced
5. Verifies container is running, tmux session is alive
6. Destroys everything cleanly

## Test Strategy

Use a temporary `dual.toml` in /tmp to avoid polluting the project directory. Use the dual repo itself as the test repo (local path, fast --local clone).

### Test Cases:
1. `dual list` shows workspace as "lazy"
2. `dual launch dual-test-main` creates clone + container + tmux session
3. `dual list` shows workspace as "running"
4. `dual destroy dual-test-main` tears everything down
5. `dual list` shows workspace as "lazy" again

### Cleanup:
- Remove test clone directory
- Remove test container
- Kill test tmux session
- Remove temp config
