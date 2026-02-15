---
date: 2026-02-15T12:00:00+08:00
researcher: Claude
git_commit: c2c8192a469bd88c8a5beea67c9cdf5f2cbfb6ea
branch: main
repository: dual
topic: "Tmux layer embedding and plugin/hook architecture discussion"
tags: [research, codebase, tmux, hooks, plugins, composability, architecture]
status: complete
last_updated: 2026-02-15
last_updated_by: Claude
---

# Research: Tmux Layer Embedding and Plugin/Hook Architecture

**Date**: 2026-02-15
**Researcher**: Claude
**Git Commit**: c2c8192a469bd88c8a5beea67c9cdf5f2cbfb6ea
**Branch**: main
**Repository**: dual

## Research Question

How is the tmux layer currently embedded in Dual, and what does the current architecture look like as context for discussing a move toward a plugin/hook system for composability?

## Summary

The tmux module (`src/tmux.rs`) is a 237-line standalone module of free functions that directly invoke the `tmux` CLI via `std::process::Command`. It is hardwired into `main.rs` at three specific lifecycle points: **launch** (create + attach), **destroy** (kill session), and **status** (is_alive check). There are no traits, no abstractions, no event system, and no hook points anywhere in the codebase. Every module (container, clone, shell, tmux, proxy) follows the same pattern: free functions called directly from `main.rs` command handlers in a fixed sequential pipeline.

## Detailed Findings

### 1. The tmux Module (`src/tmux.rs`)

The tmux module is a collection of 9 public free functions and 1 private helper:

| Function | Lines | Purpose |
|----------|-------|---------|
| `is_available()` | 8-13 | Checks if tmux binary exists via `tmux -V` |
| `create_session()` | 20-46 | Creates detached session with optional shell RC sourcing |
| `attach()` | 49-64 | Attaches to existing session (uses `.status()` for interactive) |
| `detach()` | 67-69 | Detaches current client from session |
| `destroy()` | 72-74 | Kills a tmux session |
| `is_alive()` | 77-82 | Checks if session exists via `has-session` |
| `list_sessions()` | 86-102 | Lists all `dual-` prefixed sessions |
| `send_keys()` | 105-107 | Sends keystrokes to a session pane |
| `build_new_session_args()` | 110-119 | Builds CLI args for `new-session` (exposed for testing) |
| `session_name()` | 122-125 | Generates session name: `dual-{repo}-{branch}` |

Key characteristics:
- **No struct, no trait** — purely free functions operating on session name strings
- **Direct CLI invocation** — every function calls `Command::new("tmux")` directly
- **Session naming convention** — session names match container names (`dual-{repo}-{branch}`) via shared `config::encode_branch()`
- **Error type** — `TmuxError` enum with `NotFound` and `Failed` variants
- **Shell RC injection** — `create_session()` accepts an optional shell RC command that gets sent via `send_keys()` after session creation

### 2. How tmux Is Wired Into main.rs

The tmux module is called from three command handlers in `main.rs`:

#### Launch flow (`cmd_launch`, lines 52-134)
The launch pipeline is a fixed 5-step sequence:
```
clone → container create/start → write shell RC → tmux create_session → tmux attach
```

Specifically at `main.rs:118-132`:
1. Line 119: `tmux::is_alive(&session_name)` — check if session already exists
2. Line 121: `tmux::create_session(&session_name, &workspace_dir, Some(&source_cmd))` — create detached session with shell RC
3. Line 129: `tmux::attach(&session_name)` — attach to the session (blocks until detach)

#### Destroy flow (`cmd_destroy`, lines 157-218)
The destroy pipeline tears down in reverse:
```
tmux destroy → container stop → container remove → clone remove
```

At `main.rs:179-184`:
1. Line 179: `tmux::is_alive(&session_name)` — check if session exists
2. Line 181: `tmux::destroy(&session_name)` — kill session (warning on failure, not fatal)

#### Status display (`print_workspace_status`, lines 331-351)
At `main.rs:335-339`:
1. Line 335: `tmux::session_name(&repo.name, branch)` — generate session name
2. Line 339: `tmux::is_alive(&session_name)` — check liveness for status icon
3. Lines 341-347: Status icon logic combines container status + tmux alive state

### 3. Coupling Points Between tmux and Other Modules

#### tmux ↔ config
- `tmux::session_name()` calls `crate::config::encode_branch()` (line 123-124)
- Session names follow the same `dual-{repo}-{branch}` convention as container names
- Unit test at line 213-224 explicitly verifies `session_name() == DualConfig::container_name()`

#### tmux ↔ shell
- `shell::source_file_command()` generates the RC source command passed to `tmux::create_session()` as the shell_rc parameter
- `shell::source_command()` is documented as "Useful for injecting into tmux pane creation" (shell.rs:77-78)
- The RC file contains shell function wrappers that intercept runtime commands (`npm`, `node`, etc.) and route them to Docker containers

#### tmux ↔ container
- No direct code dependency between tmux.rs and container.rs
- Coupled only through naming convention (both use `dual-{repo}-{branch}`)
- In main.rs, container must be running before tmux session is created (container provides the Docker target for shell interception)

### 4. All Other Modules Follow the Same Pattern

Every module in the codebase uses the same architecture:

| Module | Lines | Pattern |
|--------|-------|---------|
| `container.rs` | 306 | Free functions, `Command::new("docker")`, `ContainerError` enum |
| `clone.rs` | 223 | Free functions, `Command::new("git")`, `CloneError` enum |
| `shell.rs` | 201 | Free functions, string generation, `RouteTarget` enum |
| `proxy.rs` | 357 | `ProxyState` struct with `impl` block, `hyper`/`tokio` for HTTP |
| `tmux.rs` | 237 | Free functions, `Command::new("tmux")`, `TmuxError` enum |
| `config.rs` | 381 | `DualConfig`/`RepoConfig` structs, TOML parsing |
| `cli.rs` | 52 | `Cli`/`Command` structs, clap derive macros |

**No traits exist anywhere in the codebase.** No dynamic dispatch (`dyn Trait`), no generics with trait bounds, no plugin interfaces.

### 5. The Fixed Pipeline in main.rs

The `cmd_launch()` function (main.rs:52-134) executes a rigid sequential pipeline:

```
Step 1: config::load() + resolve_workspace()
Step 2: clone::clone_workspace()
Step 3: container::create() + container::start()
Step 4: shell::write_rc_file()
Step 5: tmux::create_session() + tmux::attach()
```

There are no pre/post hooks, no event emissions, no callbacks, and no extension points between these steps. Each step either succeeds and proceeds to the next, or prints an error and returns exit code 1.

The `cmd_destroy()` function (main.rs:157-218) has the reverse pipeline:

```
Step 1: tmux::destroy()
Step 2: container::stop() + container::destroy()
Step 3: clone::remove_workspace()
```

### 6. Existing "Hook" Precedent

The only hook-like mechanism in the project is in `.claude/hooks/architecture-loop-hook.sh` — a Claude Code stop hook configured in `.claude/settings.json`. This is external to Dual's Rust codebase and operates at the Claude Code tool level, not within Dual itself.

### 7. Architecture Validation Context

From `thoughts/ARCHITECTURE.md`, two validated claims are relevant:

- **tmux-backend-viable** (claim #20): "tmux handles 100+ concurrent sessions. Designed for this exact use case." — CONFIRMED
- **progressive-enhancement** (claim #24): "If user doesn't have tmux, Dual still works. Core functionality (containers, routing, proxy) preserved. Only UI degradation (no panes)." — CONFIRMED

The progressive-enhancement claim validates that tmux was always architecturally intended to be optional/separable from core functionality.

### 8. E2E Test Surface for tmux

Two e2e tests in `tests/e2e.rs` exercise tmux:

- `tmux_session_lifecycle` (lines 310-334): create → is_alive → destroy → !is_alive
- `tmux_send_keys` (lines 336-370): create → send_keys → poll for marker file

Both use the `TestFixture` harness which provides RAII cleanup via `register_tmux_session()` (in `tests/harness/mod.rs:72`).

## Code References

- `src/tmux.rs:1-237` — Complete tmux module (9 public functions, TmuxError enum, 5 unit tests)
- `src/main.rs:8` — `use dual::tmux;` import
- `src/main.rs:75` — `tmux::session_name()` call in launch
- `src/main.rs:118-132` — tmux create + attach in launch pipeline
- `src/main.rs:176-184` — tmux destroy in destroy pipeline
- `src/main.rs:335-347` — tmux status check in workspace listing
- `src/lib.rs:7` — `pub mod tmux;` declaration
- `src/shell.rs:77-80` — `source_command()` documented for tmux injection
- `src/shell.rs:99-101` — `source_file_command()` used in launch flow
- `tests/e2e.rs:308-370` — tmux e2e tests (lifecycle + send_keys)
- `tests/harness/mod.rs:72` — `register_tmux_session()` RAII cleanup

## Architecture Documentation

### Current Module Dependency Graph
```
main.rs
  ├── config (load, resolve, naming)
  ├── clone (clone_workspace, workspace_exists, remove_workspace)
  ├── container (create, start, stop, destroy, status)
  ├── shell (write_rc_file, source_file_command)
  ├── tmux (session_name, is_alive, create_session, attach, destroy)
  └── proxy (start, workspace_urls)
```

All arrows point one direction: main.rs → modules. Modules have minimal cross-dependencies (only tmux → config for `encode_branch`, and shell generates content consumed by tmux via main.rs).

### Pipeline Lifecycle Events (implicit, not codified)
```
LAUNCH: workspace_resolved → clone_complete → container_started → shell_rc_written → session_created → session_attached
DESTROY: session_destroyed → container_stopped → container_removed → clone_removed
STATUS: per-workspace: clone_check + container_status + tmux_is_alive
```

These lifecycle events exist as implicit sequential steps in main.rs but are not modeled as a formal event system.

## Historical Context (from thoughts/)

- `thoughts/shared/plans/2026-02-13-BUILD-tmux.md` — Implementation plan for tmux module
- `thoughts/shared/research/2026-02-13-BUILD-tmux.md` — Research on tmux requirements and API
- `thoughts/BUILD.md` — Shows tmux as module #6 of 14 in the MVP build sequence
- `thoughts/ARCHITECTURE.md` — Validates tmux as optional (progressive-enhancement claim #24)
- `thoughts/shared/research/2026-02-15-config-workspace-state-architecture.md` — Recent research on config/workspace/state architecture

## Related Research

- `thoughts/shared/research/2026-02-13-BUILD-shell.md` — Shell interception (generates RC consumed by tmux)
- `thoughts/shared/research/2026-02-13-BUILD-wire-cli.md` — CLI wiring (how modules connect to commands)
- `thoughts/shared/research/2026-02-13-BUILD-container.md` — Container lifecycle (runs before tmux in pipeline)

## Open Questions

1. What specific lifecycle events should emit hooks? (e.g., pre_launch, post_clone, pre_attach, post_destroy)
2. Should the hook system be synchronous (blocking pipeline) or asynchronous (fire-and-forget)?
3. Should hooks be configured in dual.toml, in a separate hooks config, or discovered from a directory?
4. What is the interface for a hook consumer — shell scripts, WASM plugins, Rust trait objects, or HTTP webhooks?
5. Should tmux become one of several possible "session backend" plugins, or remain a first-party module that's just decoupled via hooks?
6. How does this interact with the progressive-enhancement claim — if tmux is a plugin, what's the "no plugin" baseline?
