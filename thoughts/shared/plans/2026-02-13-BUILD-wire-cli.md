# Wire CLI Integration Plan

## Overview

Replace all CLI stub handlers in `main.rs` with real module calls, creating a working end-to-end flow: config loading → workspace management → container lifecycle → shell interception → tmux sessions.

## Current State Analysis

- 6 modules built with public APIs: cli, config, clone, container, shell, tmux
- `main.rs:23-50` has 5 stub handlers printing "not yet implemented"
- Container `create` doesn't set a keep-alive command (containers exit immediately)
- No workspace resolution helper (identifier → repo + branch)
- Shell RC delivery via `send_keys` won't work for multi-line content
- No `Launch` subcommand for starting a specific workspace

### Key Discoveries:
- `container::build_create_args` (container.rs:139-156) needs `sleep infinity` to keep containers alive
- `tmux::create_session` (tmux.rs:41-43) sends `shell_rc` directly via `send_keys` — works for single commands but not multi-line RC scripts
- All naming is consistent: `DualConfig::container_name()` = `tmux::session_name()` = `dual-{repo}-{encoded_branch}`

## Desired End State

All CLI commands wired to real implementations:
- `dual` → Load config, list workspaces with status
- `dual launch <workspace>` → Full workspace launch (clone → container → shell RC → tmux)
- `dual list` → Show all workspaces with live status (clone/container/tmux)
- `dual destroy <workspace>` → Full teardown (tmux → container → clone removal)
- `dual open` / `dual urls` → Stub with helpful message (needs proxy, Phase 3)

Verification: `cargo build && cargo test && cargo clippy && cargo fmt --check` all pass.

## What We're NOT Doing

- Fuzzy picker for `dual` (no args) — that's polish
- Reverse proxy / `dual open` / `dual urls` — that's the proxy module
- Auto image generation — using default `node:20` for MVP
- Zellij backend or BasicBackend fallback
- Error recovery or retry logic

## Implementation Approach

Four incremental phases, each testable independently.

## Phase 1: Fix Container Keep-Alive

### Overview
Add `sleep infinity` command to container creation so containers stay running for `docker exec`.

### Changes Required:

#### 1. Container module
**File**: `src/container.rs`
**Changes**: Add keep-alive command to `build_create_args` and update `create`

Add `"sleep"` and `"infinity"` after the image argument in `build_create_args`.

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` succeeds
- [x] `cargo test` passes (update container tests for new args)
- [x] `cargo clippy` clean

---

## Phase 2: Add Config Helpers + RC File Writing

### Overview
Add workspace resolution and RC file persistence so command handlers can look up workspaces and deliver shell configuration.

### Changes Required:

#### 1. Config module — workspace resolution
**File**: `src/config.rs`
**Changes**: Add methods to DualConfig:
- `resolve_workspace(&self, identifier: &str) -> Option<(&RepoConfig, String)>` — match workspace identifier against all configured repo/branch combos
- `all_workspaces(&self) -> Vec<(&RepoConfig, &str)>` — iterate all configured workspaces

#### 2. Shell module — RC file persistence
**File**: `src/shell.rs`
**Changes**: Add function:
- `write_rc_file(container_name: &str) -> Result<PathBuf, std::io::Error>` — write RC to `~/.config/dual/rc/{container_name}.sh` and return path

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` succeeds
- [x] `cargo test` passes (add tests for new helpers)
- [x] `cargo clippy` clean

---

## Phase 3: Add Launch Subcommand + Wire All Handlers

### Overview
Add `Launch` CLI subcommand and wire all command handlers to real module calls.

### Changes Required:

#### 1. CLI module — add Launch subcommand
**File**: `src/cli.rs`
**Changes**: Add `Launch` variant to Command enum with required workspace positional arg.

Also add hidden `ShellRc` variant for the `dual shell-rc <container>` mechanism (outputs RC content to stdout).

#### 2. Main module — wire all handlers
**File**: `src/main.rs`
**Changes**: Replace all stub handlers:

**`cmd_launch()` (no args)**:
- `config::load()` → display workspace list with status
- Show hint: "Use `dual launch <workspace>` to start a workspace"

**`cmd_launch_workspace(workspace: &str)`**:
1. `config::load()` → `config.resolve_workspace(workspace)`
2. `clone::clone_workspace()` if not exists
3. Check `container::status()` — create if Missing, start if Stopped
4. `shell::write_rc_file()` → get RC file path
5. `tmux::create_session()` with `source <rc_path>` as shell_rc if session not alive
6. `tmux::attach()`

**`cmd_list()`**:
1. `config::load()`
2. For each repo/branch: check clone exists, container status, tmux alive
3. Print formatted table

**`cmd_destroy(workspace)`**:
1. `config::load()` → resolve workspace
2. `tmux::destroy()` if alive
3. `container::stop()` + `container::destroy()` if exists
4. `clone::remove_workspace()`
5. Print confirmation

**`cmd_open()` / `cmd_urls()`**:
- Keep as informative stubs: "Reverse proxy not yet implemented. See SPEC.md Phase 3."

**`cmd_shell_rc(container_name)`**:
- `shell::generate_rc(container_name)` → print to stdout

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` succeeds
- [x] `cargo test` passes (update main.rs tests for new CLI variants)
- [x] `cargo clippy` clean
- [x] `cargo fmt --check` clean

---

## Phase 4: Cleanup and Final Verification

### Overview
Remove dead code warnings, ensure all tests pass, run full verification suite.

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` — dead_code warnings only (expected for public APIs)
- [x] `cargo test` — 48 tests pass
- [x] `cargo clippy` — clean (dead_code warnings only)
- [x] `cargo fmt --check` — clean
- [x] `dual --help` shows all commands including `launch`
- [x] `dual list` runs without panic (shows workspaces with config, error without)
- [x] `dual launch` without args shows error message

## Testing Strategy

### Unit Tests:
- Config: `resolve_workspace` finds correct repo/branch
- Config: `resolve_workspace` returns None for unknown workspace
- Config: `all_workspaces` iterates all combinations
- Shell: `write_rc_file` creates file with correct content
- Container: `build_create_args` includes `sleep infinity`
- CLI: `Launch` subcommand parses correctly

### Integration Tests:
- Deferred to end-to-end module (requires Docker + tmux)

## References

- Research: `thoughts/shared/research/2026-02-13-BUILD-wire-cli.md`
- SPEC.md: CLI Commands section, Command Routing section
- ARCHITECTURE.md: All 24 confirmed decisions
