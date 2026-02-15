---
date: 2026-02-15T15:00:00+08:00
researcher: Claude
git_commit: 3bb16355b211440ff98383c2500e4429ead83c3b
branch: main
repository: dual
topic: "Architecture rethink — bugs, branch model, context-aware CLI, config defaults, TUI design, tmux integration, and plugin system"
tags: [research, codebase, architecture, bugs, branch-model, tui, tmux, plugins, cli, config]
status: complete
last_updated: 2026-02-15
last_updated_by: Claude
---

# Research: Architecture Rethink — Bugs, Branch Model, TUI, Tmux Integration, Plugin System

**Date**: 2026-02-15
**Researcher**: Claude
**Git Commit**: 3bb1635
**Branch**: main
**Repository**: dual

## Research Question

Document the current state of Dual's architecture to understand fundamental bugs (branch model, context-aware CLI, config defaults), the tmux integration model, and how the system needs to evolve toward a TUI + plugin architecture. User's key requirements: TUI as primary interface, tmux/zellij as a plugin (not hardcoded), context-aware commands, internal branch management.

## Summary

Dual v2.0.0 is a Rust CLI (11 source files, ~3,600 LOC) that manages isolated dev workspaces via Docker containers + tmux sessions + shell command routing. Three fundamental bugs exist in the branch model, context detection, and config defaults. The tmux integration is tightly coupled (hardcoded in `src/tmux.rs` and `src/main.rs`). The current architecture has no plugin system, no TUI, and no context-awareness for commands. Below is a complete documentation of what exists and how each component works.

---

## Detailed Findings

### 1. Bug: Branch Model — `dual create` Registers Branches That Cannot Be Cloned

**What exists:**

`cmd_create()` at `src/main.rs:159-206` takes a `repo` name and `branch` name, looks up the repo's URL from existing workspaces, and registers a new `WorkspaceEntry` with `path: None`:

```rust
// src/main.rs:184-190
let entry = state::WorkspaceEntry {
    repo: repo.to_string(),
    url,                    // copied from existing workspace
    branch: branch.to_string(),  // user-provided branch name
    path: None,             // signals "clone on launch"
};
```

When `cmd_launch()` runs for this workspace, it hits `src/main.rs:252`:

```rust
clone::clone_workspace(&workspace_root, &entry.repo, &entry.url, &entry.branch)
```

Which calls `src/clone.rs:46-58`:

```rust
let mut cmd = Command::new("git");
cmd.arg("clone");
if is_local_path(url) {
    cmd.arg("--local");
}
cmd.arg("-b").arg(branch);  // <-- THIS: git clone -b <branch>
cmd.arg(url).arg(&target_dir);
```

**The problem:** `git clone -b <branch>` requires the branch to already exist in the remote (or local source). If the user creates an internal-only branch name (e.g., `feat/new-thing` that doesn't exist in origin), the clone fails with:

```
fatal: Remote branch feat/new-thing not found in upstream origin
```

Dual's `create` command registers the workspace in state successfully (line 192-198), but the actual clone at launch time fails because `git clone -b` can only check out branches that exist in the source repository.

**How it currently works — the full chain:**
1. `dual create myrepo feat/new-thing` → writes `WorkspaceEntry{repo:"myrepo", url:"git@github.com:...", branch:"feat/new-thing", path:None}` to `~/.dual/workspaces.toml`
2. `dual launch myrepo-feat__new-thing` → resolves entry, sees `path: None`, calls `clone_workspace()`
3. `clone_workspace()` runs `git clone -b feat/new-thing <url> <dir>` → **fails** if branch doesn't exist in origin

**What the user expects:** `dual create` should create a *new* branch from the current state (like `git checkout -b`), not require it to pre-exist in origin. Dual branches are internal workspace concepts.

### 2. Bug: Context-Unaware CLI — Commands Don't Detect Current Repository

**What exists:**

The CLI at `src/cli.rs:24-30` requires explicit repo/branch args for `create`:

```rust
Create {
    /// Repo name (must already be added)
    repo: String,
    /// Branch name
    branch: String,
},
```

`cmd_create()` at `src/main.rs:159` receives these as required positional args:

```rust
fn cmd_create(repo: &str, branch: &str) -> i32 {
```

There is a `detect_workspace()` function at `src/main.rs:669-688` that checks if the current directory belongs to a known workspace:

```rust
fn detect_workspace(st: &state::WorkspaceState) -> Option<state::WorkspaceEntry> {
    let cwd = std::env::current_dir().ok()?;
    let root = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        ...
    for ws in st.all_workspaces() {
        let ws_dir = st.workspace_dir(ws);
        if ws_dir == root {
            return Some(ws.clone());
        }
    }
    None
}
```

And there's `detect_git_repo()` at `src/main.rs:723-757` that gets the repo root, remote URL, and current branch from git.

**How these are used today:**
- `detect_workspace()` is only used by `cmd_sync()` at `src/main.rs:562` — to auto-detect which workspace you're in when no arg is given
- `detect_git_repo()` is only used by `cmd_add()` at `src/main.rs:72` — to register the current directory
- `cmd_create()` does **not** use either function — it requires explicit `<repo> <branch>` args

**The gap:** If a user is inside `~/code/myproject` (which is already added as `myproject/main`), running `dual create feat/auth` should automatically know the repo is `myproject`. Currently it requires `dual create myproject feat/auth`.

Similarly, other commands (`launch`, `destroy`, `open`, `urls`) require the full workspace identifier (`myrepo-feat__auth`). None of them auto-detect the current repo context.

### 3. Bug: Default `.dual.toml` Generated by `dual add` Is Confusing

**What exists:**

When `dual add` is run and no `.dual.toml` exists, it creates one at `src/main.rs:101-111`:

```rust
let hints_path = repo_root.join(".dual.toml");
if !hints_path.exists() {
    let hints = config::RepoHints::default();
    if let Err(e) = config::write_hints(&repo_root, &hints) {
        warn!("failed to write .dual.toml: {e}");
    } else {
        info!("Created .dual.toml with defaults (image: node:20)");
        info!("Edit it to customize ports, image, setup command, and env vars.");
    }
}
```

`RepoHints::default()` at `src/config.rs:44-54`:

```rust
impl Default for RepoHints {
    fn default() -> Self {
        Self {
            image: DEFAULT_IMAGE.to_string(),  // "node:20"
            ports: Vec::new(),
            setup: None,
            env: HashMap::new(),
            shared: None,
        }
    }
}
```

When serialized by `write_hints()` (using `toml::to_string_pretty`), this produces the current `.dual.toml`:

```toml
image = "node:20"
ports = []

[env]
```

**What's confusing:**
- `ports = []` — empty array with no context about what this is for
- `[env]` — empty section header with nothing in it
- No comments explaining any field
- `setup` is `None` so it's omitted entirely (not visible to the user)
- `[shared]` is omitted (has `skip_serializing_if = "Option::is_none"` at `src/config.rs:36`)
- No indication of what values are valid or what the fields do

The `toml` crate's `to_string_pretty()` function does not support emitting comments. The serialization produces bare TOML without any documentation.

### 4. Complete Architecture — Module-by-Module Documentation

#### 4.1 CLI (`src/cli.rs` — 74 lines)

Uses `clap` derive macros. All commands are in the `Command` enum:

| Command | CLI Signature | Required Args | Optional Args |
|---------|--------------|---------------|---------------|
| `Add` | `dual add` | none | `--name NAME` |
| `Create` | `dual create <repo> <branch>` | repo, branch | none |
| `Launch` | `dual launch <workspace>` | workspace | none |
| `List` | `dual list` | none | none |
| `Destroy` | `dual destroy <workspace>` | workspace | none |
| `Open` | `dual open [workspace]` | none | workspace |
| `Urls` | `dual urls [workspace]` | none | workspace |
| `Sync` | `dual sync [workspace]` | none | workspace |
| `Proxy` | `dual proxy` | none | none |
| `ShellRc` | `dual shell-rc <container>` | container | none (hidden) |

No subcommand = `cmd_default()` which shows workspace list.

#### 4.2 Config (`src/config.rs` — 310 lines)

Two responsibilities:
1. **Per-repo hints** (`.dual.toml`): `RepoHints` struct with `image`, `ports`, `setup`, `env`, `shared`
2. **Naming conventions**: `workspace_id()`, `container_name()`, `encode_branch()`, `decode_branch()`, `workspace_dir()`

Key naming scheme:
- Workspace ID: `{repo}-{encoded_branch}` (e.g., `lightfast-feat__auth`)
- Container name: `dual-{repo}-{encoded_branch}` (e.g., `dual-lightfast-feat__auth`)
- Branch encoding: `/` → `__` (e.g., `feat/auth` → `feat__auth`)

Config fields and their implementation status:

| Field | Type | Implemented | Used By |
|-------|------|-------------|---------|
| `image` | `String` | Yes | `container::create()` |
| `ports` | `Vec<u16>` | Yes | `proxy::ProxyState`, `proxy::workspace_urls()` |
| `setup` | `Option<String>` | Schema only | Nothing — never executed |
| `env` | `HashMap<String,String>` | Schema only | Nothing — loaded then dropped |
| `shared` | `Option<SharedConfig>` | Yes | `shared::init_from_main()`, `shared::copy_to_branch()` |

#### 4.3 State (`src/state.rs` — 670 lines)

Global workspace registry at `~/.dual/workspaces.toml`.

`WorkspaceState` struct:
- `workspace_root: Option<String>` — defaults to `~/.dual/workspaces`
- `workspaces: Vec<WorkspaceEntry>` — all registered workspaces

`WorkspaceEntry` struct:
- `repo: String` — short name (e.g., "lightfast")
- `url: String` — git remote URL or local path
- `branch: String` — branch name
- `path: Option<String>` — `Some` = user's existing dir (from `dual add`), `None` = managed clone

State persistence uses atomic writes:
1. Serialize to TOML
2. Acquire advisory file lock (`fs2::FileExt`)
3. Write to `.tmp` file
4. Backup existing to `.toml.bak`
5. Atomic rename `.tmp` → `.toml`
6. Release lock

#### 4.4 Clone (`src/clone.rs` — 181 lines)

Handles git cloning for managed workspaces (those with `path: None`).

`clone_workspace()` runs: `git clone [-b branch] [--local] <url> <target_dir>`

Key behaviors:
- Skip if `.git` already exists in target
- `--local` flag for filesystem URLs (uses hardlinks)
- `-b <branch>` for branch checkout — **this is where Bug #1 manifests**
- Target dir: `{workspace_root}/{repo}/{encoded_branch}/`

#### 4.5 Container (`src/container.rs` — 278 lines)

Docker lifecycle management. Each workspace gets one container named `dual-{repo}-{encoded_branch}`.

`create()` builds these Docker args:
```
docker create --name <name>
  -v <workspace_dir>:/workspace
  -v /workspace/node_modules    # anonymous volume for isolation
  -w /workspace
  <image>
  sleep infinity               # keep container alive for exec
```

Notable gaps:
- No `-e KEY=VALUE` for env vars (despite `hints.env` existing)
- No `--env-file` support
- No additional volume mounts (e.g., for shared config)
- No port publishing (`-p`) — containers use bridge network, proxy handles routing
- No `setup` command execution after creation

Status checking uses `docker inspect --format {{.State.Running}}`.

Container IP retrieval uses `docker inspect --format {{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}`.

#### 4.6 Shell (`src/shell.rs` — 202 lines)

Generates shell RC files that intercept runtime commands.

Hardcoded command list at line 2-4:
```rust
const CONTAINER_COMMANDS: &[&str] = &[
    "npm", "npx", "pnpm", "node", "python", "python3", "pip", "pip3", "curl", "make",
];
```

For each command, generates a shell function:
```bash
npm() {
    if [ -t 1 ]; then
        command docker exec -t -w /workspace dual-lightfast-main npm "$@"
    else
        command docker exec -w /workspace dual-lightfast-main npm "$@"
    fi
}
```

RC files written to `~/.config/dual/rc/{container_name}.sh`.

The shell RC is sourced in the tmux session via `send_keys()` after session creation.

#### 4.7 Tmux (`src/tmux.rs` — 226 lines)

**This is the module most relevant to the architectural rethink.**

Current tmux integration:
- Session naming: `dual-{repo}-{encoded_branch}` (matches container names)
- `create_session()` — runs `tmux new-session -d -s <name> -c <cwd>`
- `attach()` — runs `tmux attach-session -t <name>`
- `detach()` — runs `tmux detach-client -s <name>`
- `destroy()` — runs `tmux kill-session -t <name>`
- `is_alive()` — runs `tmux has-session -t <name>`
- `list_sessions()` — runs `tmux list-sessions -F #{session_name}`, filters by `dual-` prefix
- `send_keys()` — runs `tmux send-keys -t <name> <keys> Enter`

**How tmux is used in the launch flow** (`src/main.rs:322-337`):
```rust
// Step 5: Create tmux session if not alive
if !tmux::is_alive(&session_name) {
    let source_cmd = shell::source_file_command(&rc_path);
    if let Err(e) = tmux::create_session(&session_name, &workspace_dir, Some(&source_cmd)) {
        error!("tmux session creation failed: {e}");
        return 1;
    }
}

// Step 6: Attach
tmux::attach(&session_name)
```

**Tight coupling observations:**
1. `tmux` is imported and called directly in `main.rs` — no abstraction layer
2. Session names are computed at call sites using `tmux::session_name()`
3. `print_workspace_status()` calls `tmux::is_alive()` for each workspace
4. `cmd_destroy()` calls `tmux::destroy()` directly
5. No trait or interface exists that could be swapped for zellij or another multiplexer
6. The session only gets one window/pane — no customization of what runs inside

**What the session contains:** Just a single shell pane with the RC sourced. There is no:
- Second pane for editor (nvim)
- Third pane for Claude Code
- Window layout configuration
- Hook for post-attach commands

#### 4.8 Shared Config (`src/shared.rs` — 353 lines)

Manages propagation of gitignored files across workspaces.

Two modes:
1. **Main workspace** (`init_from_main`): moves files to `~/.dual/shared/{repo}/`, creates symlinks back
2. **Branch workspace** (`copy_to_branch`): copies files from shared dir into workspace

The `[shared]` section in `.dual.toml`:
```toml
[shared]
files = [".vercel", ".env.local"]
```

Triggered during `cmd_launch()` at `src/main.rs:261-288` and during `cmd_sync()` at `src/main.rs:543-666`.

#### 4.9 Proxy (`src/proxy.rs` — 358 lines)

HTTP reverse proxy using hyper + tokio. Routes `{workspace_id}.localhost:{port}` to container IP.

Builds routing table from workspace state:
- For each workspace with a running container
- Load `.dual.toml` to get ports
- Get container IP via `docker inspect`
- Map `{workspace_id}:{port}` → `{container_ip}:{port}`

Listens on all configured ports on `127.0.0.1`.

#### 4.10 Main (`src/main.rs` — 925 lines)

Command dispatch and implementation of all CLI commands. Contains the core launch pipeline:

```
cmd_launch():
  1. Resolve workspace from state
  2. Compute container/session names
  3. Resolve workspace directory (use existing path or clone)
  4. Handle shared files (init or copy)
  5. Ensure container exists and is running
  6. Write shell RC file
  7. Create tmux session (if not alive)
  8. Attach to tmux session
```

### 5. Tmux Integration — Current User Workflow Implications

**The user's current workflow (outside Dual):**
1. Open terminal
2. Run `tmux`
3. `Meta + s` to select a tmux session (which has `nvim .` + Claude Code running)

**The user's desired workflow with Dual:**
1. Run `dual` (enters TUI)
2. See list of repos/branches
3. Select a branch → Dual's hook/plugin opens a tmux session with the workspace dir + configured panes
4. Can detach from tmux directly, or `Meta + s` to switch sessions within tmux
5. Can return to Dual TUI to see full workspace overview

**Current architecture gaps for this workflow:**
- Dual creates a bare tmux session (single pane, no editor, no Claude) — `src/tmux.rs:20-46`
- No way to configure what panes/windows the tmux session should have
- No way to run commands in panes (only `send_keys` to pane 0 for RC sourcing)
- No hook to execute after session creation
- No mechanism to integrate with the user's existing tmux (if they're already in a tmux session, `tmux attach` inside tmux is a nested session)
- No zellij support at all — the word "zellij" appears nowhere in the codebase

### 6. Plugin System — What Exists (Nothing)

The codebase has zero extensibility infrastructure:

- **No traits for behavior injection** — all functions are concrete
- **No lifecycle hooks** — no callbacks at any stage
- **No plugin discovery or loading** — no dynamic dispatch
- **No middleware chain** — fixed pipelines
- **No observer pattern** — state mutations don't emit events
- **No command extension** — container commands are a hardcoded constant array
- **No configuration-driven behavior** — `.dual.toml` is data-only, no conditional logic

The only "extension point" is the `.dual.toml` file, which provides data but no behavior.

### 7. State of Unimplemented Features

| Feature | Schema/Spec | Implementation |
|---------|-------------|----------------|
| `setup` command | `config.rs:29` | Never executed |
| `env` variables | `config.rs:33` | Loaded then dropped at `main.rs:262` |
| TUI | SPEC Phase 4 | Not started |
| Fuzzy picker | SPEC Phase 4 | Not started |
| Meta-key switching | SPEC Phase 4 | Not started |
| Auto-detection (.nvmrc, etc.) | SPEC line 163-169 | Not implemented |
| Worktree support | Old SPEC | Removed (full clones chosen) |

### 8. How The User's Tmux Use-Cases Map to Current Architecture

| Use Case | Current Support | Gap |
|----------|----------------|-----|
| Open branch → get tmux session | Yes (bare session) | No pane customization |
| Session has nvim + Claude Code | No | No pane/window config |
| Switch between branches via tmux | Partial (sessions exist) | No switcher integration |
| Run `dual` from inside tmux | Works but creates nested attach | No detection of existing tmux |
| Detach and return to Dual | No | TUI doesn't exist |
| Use existing tmux setup | No | Dual creates its own sessions |
| Use zellij instead of tmux | No | Hardcoded tmux dependency |
| Configure per-repo session layout | No | No layout config |

## Code References

- `src/cli.rs:1-74` — CLI command definitions
- `src/config.rs:18-54` — RepoHints struct and defaults
- `src/config.rs:44-54` — Default impl that produces confusing `.dual.toml`
- `src/config.rs:78-83` — `write_hints()` uses `toml::to_string_pretty` (no comments)
- `src/config.rs:94-96` — `workspace_id()` naming convention
- `src/config.rs:106-108` — `container_name()` naming convention
- `src/state.rs:14-39` — WorkspaceState and WorkspaceEntry structs
- `src/state.rs:74-79` — `resolve_workspace()` exact-match lookup
- `src/state.rs:188-225` — Atomic save with file locking
- `src/clone.rs:27-74` — `clone_workspace()` with `git clone -b` (Bug #1)
- `src/clone.rs:54-55` — The `-b` flag that fails for non-origin branches
- `src/container.rs:20-36` — `create()` takes only name, dir, image
- `src/container.rs:147-167` — Docker create args (no env, no extra mounts)
- `src/shell.rs:2-4` — Hardcoded container command list
- `src/shell.rs:37-55` — RC generation
- `src/tmux.rs:20-46` — `create_session()` with basic args
- `src/tmux.rs:49-64` — `attach()` simple attach
- `src/tmux.rs:77-82` — `is_alive()` session check
- `src/tmux.rs:110-119` — `build_new_session_args()` — only `-d -s -c`, no window/pane config
- `src/tmux.rs:122-125` — `session_name()` naming convention
- `src/main.rs:70-156` — `cmd_add()` with default `.dual.toml` creation (Bug #3)
- `src/main.rs:159-206` — `cmd_create()` with no context detection (Bug #2)
- `src/main.rs:209-339` — `cmd_launch()` full pipeline
- `src/main.rs:322-337` — tmux session creation and attach
- `src/main.rs:669-688` — `detect_workspace()` (only used by sync)
- `src/main.rs:723-757` — `detect_git_repo()` (only used by add)

## Architecture Documentation

### Current Module Dependency Graph

```
main.rs
├── cli.rs        (command parsing)
├── config.rs     (naming, hints)
├── state.rs      (workspace registry)
├── clone.rs      (git clone)        ← depends on config (naming)
├── container.rs  (docker lifecycle) ← depends on config (naming)
├── shell.rs      (RC generation)    ← depends on config (naming, via container name)
├── tmux.rs       (session lifecycle)← depends on config (naming)
├── proxy.rs      (reverse proxy)    ← depends on config, container, state
└── shared.rs     (config propagation)← depends on config
```

All modules are leaf dependencies of `main.rs`. No module depends on another module except through `config` for naming conventions.

### Two Types of Workspaces

| Type | Created By | `path` Field | Clone Behavior | Destroy Behavior |
|------|-----------|-------------|----------------|------------------|
| Main | `dual add` | `Some("/absolute/path")` | No clone needed | Does not remove directory |
| Branch | `dual create` | `None` | `git clone -b` into managed dir | Removes clone directory |

### Data Flow: `dual launch`

```
workspaces.toml → resolve_workspace(identifier)
                      ↓
              WorkspaceEntry{repo, url, branch, path}
                      ↓
              ┌───────┴────────┐
              │ path: Some     │ path: None
              │ use existing   │ clone_workspace()
              │ directory      │    git clone -b branch url dir
              └───────┬────────┘
                      ↓
              .dual.toml → load_hints()
                      ↓
              RepoHints{image, ports, setup, env, shared}
                      ↓
              shared files handling
                      ↓
              container::create(name, dir, image)
                      ↓ (only image used, env/setup dropped)
              container::start(name)
                      ↓
              shell::write_rc_file(container_name)
                      ↓
              tmux::create_session(name, dir, source_cmd)
                      ↓
              tmux::attach(name)
```

## Historical Context (from thoughts/)

- `thoughts/shared/research/2026-02-15-dual-tui-design.md` — TUI design research covering ratatui+crossterm stack, three TUI models (full takeover, overlay, hybrid), tree view interactions, and TUI↔tmux transition options (A: inside tmux, B: manage terminal directly, C: replace tmux)
- `thoughts/shared/research/2026-02-15-config-propagation-across-workspaces.md` — Shared config solution design. Implemented as Solution D (shared dir + symlinks)
- `thoughts/shared/research/2026-02-15-env-vars-plugin-infrastructure.md` — Documents that `env` and `setup` fields exist in schema but are never wired. No plugin infrastructure exists
- `thoughts/shared/research/2026-02-15-github-repo-setup-v2-release.md` — v2.0.0 release planning, now completed
- `thoughts/shared/plans/2026-02-15-github-v2-release.md` — v2.0.0 release plan, executed

## Related Research

- `thoughts/shared/research/2026-02-15-dual-tui-design.md` — TUI framework analysis and design options
- `thoughts/shared/research/2026-02-15-env-vars-plugin-infrastructure.md` — Extensibility analysis

## Open Questions

1. **Branch creation strategy**: Should `dual create` clone from origin then `git checkout -b`, or clone from the local main workspace (faster, uses `--local` hardlinks)?
2. **Worktrees vs full clones**: The current architecture uses full clones. Git worktrees would solve the "branch doesn't exist in origin" problem (they create from local HEAD), but were rejected in SPEC for lock contention reasons. Should this be revisited?
3. **TUI as default**: Should `dual` (no args) launch the TUI, with `dual list` as the non-interactive fallback?
4. **Multiplexer trait**: What is the minimal trait interface for tmux/zellij abstraction? (`create_session`, `attach`, `detach`, `destroy`, `is_alive`, `send_keys`?)
5. **Session layout config**: Where should per-repo tmux layouts be configured? In `.dual.toml`? In a separate file? Via plugin?
6. **Plugin format**: Lua scripts? TOML declarations? Shell hooks? Rust dynamic libraries?
7. **Existing tmux detection**: If the user is already in a tmux session, should Dual create a new window in that session instead of a nested attach?
