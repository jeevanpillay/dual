---
date: 2026-02-13T00:00:00+00:00
researcher: Claude
git_commit: 8864b9f
branch: feature/build-loop-pipeline
repository: dual
topic: "Wire CLI stub handlers to real module calls"
tags: [research, build, integration, wire-cli]
status: complete
last_updated: 2026-02-13
last_updated_by: Claude
---

# Research: Wire CLI Integration

**Date**: 2026-02-13
**Researcher**: Claude
**Git Commit**: 8864b9f
**Branch**: feature/build-loop-pipeline

## Research Question

What changes are needed to wire the CLI stub handlers in main.rs to the real module implementations?

## Summary

All 6 MVP modules (cli, config, clone, container, shell, tmux) are built as independent units with comprehensive tests. The CLI handlers in `main.rs:23-50` are all stubs printing "not yet implemented". Wiring requires: (1) fixing container keep-alive, (2) adding config workspace resolution helpers, (3) adding shell RC file persistence, (4) orchestrating module calls in each command handler.

## Detailed Findings

### Current Stub Handlers (main.rs:23-50)

Five handlers exist as stubs:
- `cmd_launch()` (line 23) — no-arg default
- `cmd_list()` (line 27) — list workspaces
- `cmd_destroy(Option<String>)` (line 31) — teardown
- `cmd_open(Option<String>)` (line 38) — browser open
- `cmd_urls(Option<String>)` (line 45) — show URLs

### Module Public APIs Available

**config** (`src/config.rs`):
- `load() -> Result<DualConfig, ConfigError>` (line 73) — auto-discovers config
- `DualConfig::workspace_dir(&self, repo, branch) -> PathBuf` (line 46)
- `DualConfig::container_name(repo, branch) -> String` (line 52)
- `encode_branch(branch) -> String` (line 60)

**clone** (`src/clone.rs`):
- `clone_workspace(config, repo, url, branch) -> Result<PathBuf, CloneError>` (line 40)
- `workspace_exists(config, repo, branch) -> bool` (line 16)
- `remove_workspace(config, repo, branch) -> Result<(), CloneError>` (line 90)

**container** (`src/container.rs`):
- `create(config, repo, branch, image) -> Result<String, ContainerError>` (line 23)
- `start(name) -> Result<(), ContainerError>` (line 51)
- `stop(name) -> Result<(), ContainerError>` (line 56)
- `destroy(name) -> Result<(), ContainerError>` (line 61)
- `status(name) -> ContainerStatus` (line 89)
- `list_all() -> Vec<(String, bool)>` (line 108)

**shell** (`src/shell.rs`):
- `generate_rc(container_name) -> String` (line 37)
- `source_command(container_name) -> String` (line 78)

**tmux** (`src/tmux.rs`):
- `create_session(name, cwd, shell_rc) -> Result<(), TmuxError>` (line 20)
- `attach(name) -> Result<(), TmuxError>` (line 49)
- `destroy(name) -> Result<(), TmuxError>` (line 72)
- `is_alive(name) -> bool` (line 77)
- `list_sessions() -> Vec<String>` (line 86)
- `session_name(repo, branch) -> String` (line 122)

### Issues Discovered

1. **Container keep-alive missing**: `build_create_args` (container.rs:139-156) doesn't set a command. `docker create node:20` without a command runs the default `node` entrypoint which exits immediately without TTY. Need to add `sleep infinity` to keep container alive for `docker exec`.

2. **No workspace resolution**: Given a workspace identifier like "lightfast-main", there's no helper to resolve it back to repo="lightfast" + branch="main". Need to match against configured repos.

3. **Shell RC delivery**: `tmux::create_session` sends `shell_rc` via `send_keys`, which types multi-line content into terminal. Won't work for the full RC script. Need to write RC to a file and source it instead.

4. **No Launch subcommand**: The CLI has no way to launch a specific workspace. Need to add a `Launch` variant or modify default behavior.

### Architecture Constraints

- shell-interception (CONFIRMED WITH CAVEATS): Functions work for interactive + Claude Code. Needs RC injection.
- docker-exec-basic (CONFIRMED): Exit codes preserved. CWD requires `-w`.
- tmux-backend-viable (CONFIRMED): 100% feasible for session management.
- container-network-isolation (CONFIRMED): Bridge mode (default) provides isolation.

## Code References

- `src/main.rs:23-50` — Stub handlers to replace
- `src/container.rs:139-156` — build_create_args missing keep-alive command
- `src/shell.rs:37-55` — RC generation
- `src/tmux.rs:20-46` — Session creation with RC injection
- `src/config.rs:52-54` — Container name generation

## Open Questions

None — all questions answered through code analysis.
