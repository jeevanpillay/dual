---
date: 2026-02-15T16:30:00+08:00
researcher: Claude
git_commit: 27125d7
branch: main
repository: dual
topic: "v3 Architecture Rethink — Branch Model, Context-Aware CLI, Config Defaults, TUI, Tmux/Plugin System, User Workflows"
tags: [research, codebase, architecture, v3, branch-model, tui, tmux, zellij, plugins, cli, config, worktrees, user-workflows]
status: complete
last_updated: 2026-02-15
last_updated_by: Claude
---

# Research: v3 Architecture Rethink

**Date**: 2026-02-15T16:30:00+08:00
**Researcher**: Claude
**Git Commit**: 27125d7
**Branch**: main
**Repository**: dual

## Research Question

Complete documentation of the current Dual codebase to inform the next architecture. Covers three fundamental bugs (branch model, context detection, config defaults), the full tmux integration model, every user workflow for terminal multiplexer setups, TUI design requirements, and the need for a plugin-based architecture where tmux/zellij integration is the first plugin.

---

## Summary

Dual v2.0.0 is a Rust CLI (11 source files, ~3,600 LOC) that manages isolated dev workspaces via Docker containers + tmux sessions + shell command routing. The codebase has three fundamental bugs and several architectural gaps that need resolution before the next phase. This document maps the entire system as-is and documents every relevant user workflow pattern for the architectural rethink.

---

## Part 1: The Three Bugs

### Bug 1: Dual Branches Are Not Origin Branches

**What exists:**

`cmd_create()` at `src/main.rs:159-206` registers a workspace entry with `path: None`. When `cmd_launch()` runs, it delegates to `clone::clone_workspace()` at `src/clone.rs:27-74`, which runs:

```
git clone -b <branch> <url> <target_dir>
```

The `-b` flag at `src/clone.rs:54` requires the branch to exist in the remote repository. If the user creates a Dual-internal branch name (e.g., `feat/new-thing` that doesn't exist at origin), the clone fails:

```
fatal: Remote branch feat/new-thing not found in upstream origin
```

**The full chain today:**

1. `dual create myrepo feat/new-thing` — writes `WorkspaceEntry{repo:"myrepo", branch:"feat/new-thing", path:None}` to `~/.dual/workspaces.toml` (succeeds, no git validation)
2. `dual launch myrepo-feat__new-thing` — resolves entry, calls `clone_workspace()`
3. `clone_workspace()` runs `git clone -b feat/new-thing <url> <dir>` — **fails** because branch doesn't exist at origin
4. Workspace remains in state as a ghost entry (registered but unusable)

**Key code locations:**
- `src/main.rs:159-206` — `cmd_create()` does zero git validation
- `src/clone.rs:46-58` — `git clone -b` construction
- `src/clone.rs:54` — the `-b` flag that causes failure
- `src/state.rs:87-96` — `add_workspace()` accepts any branch name

**What the user expects:** `dual create` should treat branches as internal workspace concepts. A branch like `feat/new-thing` should be created locally from the current HEAD (like `git checkout -b`), not required to pre-exist at origin.

---

### Bug 2: Commands Don't Detect Current Repository Context

**What exists:**

The CLI at `src/cli.rs:24-30` requires explicit repo and branch args for `create`:

```rust
Create {
    repo: String,    // required positional
    branch: String,  // required positional
},
```

Two context-detection functions exist but are underutilized:

| Function | Location | Used By | Could Be Used By |
|----------|----------|---------|-------------------|
| `detect_git_repo()` | `src/main.rs:723-757` | `cmd_add()` only | `cmd_create()`, all commands |
| `detect_workspace()` | `src/main.rs:669-688` | `cmd_sync()` only | `cmd_launch()`, `cmd_destroy()`, `cmd_open()`, `cmd_urls()` |

**`detect_git_repo()`** runs three git commands:
1. `git rev-parse --show-toplevel` — get repo root
2. `git remote get-url origin` — get remote URL (falls back to path)
3. `git rev-parse --abbrev-ref HEAD` — get current branch

**`detect_workspace()`** matches current directory against all workspace entries by comparing git root to `st.workspace_dir(ws)`.

**The gap:** If a user is inside `~/code/myproject` (already added as `myproject/main`), running `dual create feat/auth` should auto-detect `myproject` as the repo. Currently requires `dual create myproject feat/auth`.

**Commands that require explicit workspace identifier but could auto-detect:**

| Command | Current | Could Be |
|---------|---------|----------|
| `dual create <repo> <branch>` | Both required | `dual create <branch>` if in repo dir |
| `dual launch <workspace>` | Required | Auto-detect if in workspace dir |
| `dual destroy <workspace>` | Required | Auto-detect if in workspace dir |
| `dual open [workspace]` | Optional (shows all) | Auto-detect current |
| `dual urls [workspace]` | Optional (shows all) | Auto-detect current |

---

### Bug 3: Default `.dual.toml` Is Confusing

**What exists:**

When `dual add` runs and no `.dual.toml` exists, it creates one at `src/main.rs:101-111`:

```rust
let hints = config::RepoHints::default();
config::write_hints(&repo_root, &hints)?;
```

`RepoHints::default()` at `src/config.rs:44-54` produces:

```rust
Self {
    image: "node:20".to_string(),
    ports: Vec::new(),
    setup: None,
    env: HashMap::new(),
    shared: None,
}
```

Serialized by `toml::to_string_pretty()` at `src/config.rs:80`, this produces:

```toml
image = "node:20"
ports = []

[env]
```

**What's confusing:**
- `ports = []` — empty array with no explanation of what ports are for
- `[env]` — empty section header with nothing in it, looks broken
- No comments — `toml::to_string_pretty()` cannot emit comments
- `setup` field is `None` so it's invisible (user doesn't know it exists)
- `[shared]` section is omitted via `skip_serializing_if = "Option::is_none"`
- No indication of what values are valid, what fields do, or examples
- `image = "node:20"` assumes Node.js — wrong for Python/Rust/Go projects

**Key code locations:**
- `src/config.rs:44-54` — `Default` impl
- `src/config.rs:78-83` — `write_hints()` with `toml::to_string_pretty`
- `src/main.rs:101-111` — creation trigger in `cmd_add()`

---

## Part 2: Current Architecture — Complete Module Map

### 2.1 Source Files Overview

| File | Lines | Purpose |
|------|-------|---------|
| `src/lib.rs` | 9 | Public module re-exports |
| `src/cli.rs` | 74 | Clap-based CLI definitions |
| `src/main.rs` | 924 | Entry point, command dispatch, all handlers |
| `src/config.rs` | 310 | Per-repo hints (`.dual.toml`), naming conventions |
| `src/state.rs` | 669 | Global workspace state with atomic writes + file locking |
| `src/clone.rs` | 180 | Git clone management |
| `src/container.rs` | 277 | Docker container lifecycle |
| `src/shell.rs` | 201 | Shell RC generation for command routing |
| `src/tmux.rs` | 225 | Tmux session management |
| `src/proxy.rs` | 357 | HTTP reverse proxy with subdomain routing |
| `src/shared.rs` | 352 | Config propagation across workspaces |
| **Total** | **~3,578** | |

### 2.2 Module Dependency Graph

```
main.rs (orchestration layer)
├── cli.rs        (command parsing — leaf)
├── config.rs     (naming + hints — used by almost everything)
├── state.rs      (workspace registry — depends on config)
├── clone.rs      (git operations — depends on config)
├── container.rs  (docker lifecycle — leaf)
├── shell.rs      (RC generation — leaf)
├── tmux.rs       (session lifecycle — depends on config)
├── proxy.rs      (reverse proxy — depends on config, container, state)
└── shared.rs     (file propagation — depends on config)
```

`config.rs` is the central hub — all modules depend on it for naming conventions.

### 2.3 Data Flow: `dual launch`

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

### 2.4 Two Workspace Types

| Type | Created By | `path` Field | Clone Behavior | Destroy Behavior |
|------|-----------|-------------|----------------|------------------|
| Main | `dual add` | `Some("/absolute/path")` | No clone needed | Does NOT remove directory |
| Branch | `dual create` | `None` | `git clone -b` into managed dir | Removes clone directory |

### 2.5 Naming Conventions

All naming flows through `src/config.rs`:

| Thing | Pattern | Example |
|-------|---------|---------|
| Workspace ID | `{repo}-{encoded_branch}` | `lightfast-feat__auth` |
| Container name | `dual-{repo}-{encoded_branch}` | `dual-lightfast-feat__auth` |
| Tmux session | `dual-{repo}-{encoded_branch}` | `dual-lightfast-feat__auth` |
| Directory | `{workspace_root}/{repo}/{encoded_branch}/` | `~/.dual/workspaces/lightfast/feat__auth/` |
| Branch encoding | `/` → `__` | `feat/auth` → `feat__auth` |

Container names and tmux session names are **identical** (verified by test at `src/tmux.rs:200-212`).

### 2.6 Unimplemented Schema Fields

| Field | Location | Status |
|-------|----------|--------|
| `setup` | `config.rs:29` | Schema exists, never executed |
| `env` | `config.rs:33` | Loaded then dropped at `main.rs:262` — never passed to container |
| Auto-detection | SPEC lines 163-169 | `.nvmrc`, `.node-version`, etc. — not implemented |

---

## Part 3: Tmux Integration — Current State

### 3.1 All Tmux Functions

| Function | Location | Command | Purpose |
|----------|----------|---------|---------|
| `is_available()` | `tmux.rs:8-13` | `tmux -V` | Check if tmux is installed |
| `create_session()` | `tmux.rs:20-46` | `tmux new-session -d -s {name} -c {cwd}` | Create detached session |
| `attach()` | `tmux.rs:49-64` | `tmux attach-session -t {name}` | Attach terminal to session |
| `detach()` | `tmux.rs:67-69` | `tmux detach-client -s {name}` | Detach from session |
| `destroy()` | `tmux.rs:72-74` | `tmux kill-session -t {name}` | Kill session |
| `is_alive()` | `tmux.rs:77-82` | `tmux has-session -t {name}` | Check existence |
| `list_sessions()` | `tmux.rs:86-102` | `tmux list-sessions -F #{session_name}` | List dual-managed sessions |
| `send_keys()` | `tmux.rs:105-107` | `tmux send-keys -t {name} {keys} Enter` | Send command to session |
| `session_name()` | `tmux.rs:122-125` | N/A | Generate `dual-{repo}-{encoded_branch}` |

### 3.2 Session Creation — What Gets Created

`build_new_session_args()` at `tmux.rs:110-119` produces only:

```
tmux new-session -d -s dual-lightfast-main -c /path/to/workspace
```

That's it. One window, one pane, default shell, no layout. After creation, `send_keys()` sources the shell RC file. The session contains:

- Single shell pane
- Shell RC sourced (command routing active)
- Working directory set to workspace
- **No** nvim pane
- **No** Claude Code pane
- **No** custom layout
- **No** additional windows

### 3.3 Integration Points in main.rs

| Location | Function | Tmux Call |
|----------|----------|-----------|
| `main.rs:233` | `cmd_launch()` | `tmux::session_name()` |
| `main.rs:323` | `cmd_launch()` | `tmux::is_alive()` |
| `main.rs:325` | `cmd_launch()` | `tmux::create_session()` |
| `main.rs:333` | `cmd_launch()` | `tmux::attach()` |
| `main.rs:381` | `cmd_destroy()` | `tmux::session_name()` |
| `main.rs:384` | `cmd_destroy()` | `tmux::is_alive()` |
| `main.rs:386` | `cmd_destroy()` | `tmux::destroy()` |
| `main.rs:696` | `print_workspace_status()` | `tmux::session_name()` |
| `main.rs:708` | `print_workspace_status()` | `tmux::is_alive()` |

### 3.4 Tight Coupling

1. `tmux` is imported and called directly — no trait, no abstraction
2. Session names computed at call sites using `tmux::session_name()`
3. Status display hardcodes `tmux::is_alive()` checks
4. `cmd_destroy()` calls `tmux::destroy()` directly
5. No backend swap mechanism for zellij or anything else
6. The word "zellij" appears nowhere in the codebase

### 3.5 Edge Cases Not Handled

| Scenario | What Happens | Expected |
|----------|-------------|----------|
| User already in tmux | `tmux attach` creates nested tmux | Should detect `$TMUX` env var |
| Detach from session | Must use native `Ctrl+b d` | No Dual-level detach command |
| Crash recovery | Tmux sessions survive, Dual can re-attach | Works accidentally via `is_alive()` check |
| Session layout customization | Not possible | Users want nvim + Claude + shell panes |
| Zellij user | No support | Should work via plugin/backend swap |

---

## Part 4: User Workflow Mapping — Tmux Setup Patterns

### 4.1 The User's Current Workflow (Without Dual)

```
1. Open terminal emulator (Ghostty/iTerm/etc.)
2. Run `tmux`
3. Press Meta+s to select a tmux session
4. Session has: nvim . + Claude Code already running
5. Work in that session
6. Meta+s to switch to another session
```

**Key observations:**
- User creates tmux sessions manually (or via scripts)
- Each session is pre-configured with specific panes/windows
- Switching is via tmux's native session picker (`choose-tree` or custom binding)
- The session is the "workspace" — it has the tools the user needs

### 4.2 The User's Desired Workflow (With Dual)

```
1. Run `dual` (enters TUI)
2. See tree view: repos → branches
3. Select a branch
4. Hook/plugin opens tmux session with:
   - Correct working directory
   - Configured panes (nvim, Claude, shell, etc.)
5. Work in tmux session
6. To switch: either
   a. Detach tmux, return to Dual TUI, pick another
   b. Meta+s within tmux to switch sessions directly
7. If user runs `dual` from inside tmux: should work (show TUI, allow navigation)
```

### 4.3 Common Tmux Setup Patterns to Support

**Pattern A: Simple Shell**
```
┌─────────────────────────────────┐
│ $ (shell with command routing)  │
│                                 │
│                                 │
│                                 │
└─────────────────────────────────┘
```
Current Dual behavior. Single pane, shell RC sourced.

**Pattern B: Editor + Shell**
```
┌──────────────────┬──────────────┐
│                  │              │
│  nvim .          │  $ (shell)   │
│                  │              │
│                  │              │
└──────────────────┴──────────────┘
```
Most common developer setup. Editor takes 60-70% width, shell on right.

**Pattern C: Editor + Claude + Shell**
```
┌──────────────────┬──────────────┐
│                  │  claude      │
│  nvim .          ├──────────────┤
│                  │  $ (shell)   │
│                  │              │
└──────────────────┴──────────────┘
```
The user's described setup. Editor left, Claude top-right, shell bottom-right.

**Pattern D: Editor + Multiple Shells**
```
┌──────────────────┬──────────────┐
│                  │  $ (shell 1) │
│  nvim .          ├──────────────┤
│                  │  $ (shell 2) │
│                  ├──────────────┤
│                  │  $ (logs)    │
└──────────────────┴──────────────┘
```
Power user setup. Multiple shells for different tasks.

**Pattern E: Multi-Window**
```
Window 1: Editor (nvim .)
Window 2: Shell (git, CLI operations)
Window 3: Claude Code
Window 4: Logs / monitoring
```
Some users prefer windows over panes.

**Pattern F: User's Existing Tmux Config**
```
User already has a tmux.conf with:
- Custom prefix key
- Custom pane layouts
- Plugins (tpm, tmux-resurrect, etc.)
- Status bar customization
- Key bindings for window/pane management
```
Dual must not conflict with or override the user's tmux configuration.

### 4.4 Tmux Launch Scenarios

**Scenario 1: User is NOT in tmux, runs `dual`**
```
Terminal → dual TUI → user selects branch → Dual creates/attaches tmux session
```
This is the clean case. Dual creates the session and attaches.

**Scenario 2: User IS in tmux session A, runs `dual`**
```
tmux session A → dual TUI → user selects branch
  Option a: Create new tmux session B, switch to it (tmux switch-client)
  Option b: Create new tmux window in session A
  Option c: Detach from A, attach to new session B (nested tmux issue)
```
Currently Dual does option (c) which creates nested tmux — bad UX.

**Scenario 3: User has existing tmux sessions, opens terminal, runs `dual`**
```
Terminal → dual TUI → sees branches with [running] status
  Some branches have active tmux sessions from previous work
  User selects one → Dual attaches to existing session
```
Currently works because `is_alive()` check skips creation.

**Scenario 4: User switches between branches rapidly**
```
dual TUI → branch A → work → Meta+key → dual TUI → branch B → work → Meta+key → dual TUI
```
This requires the TUI to persist and resume quickly. Currently not possible (no TUI exists).

**Scenario 5: User runs `dual` from a workspace directory (already in a branch)**
```
~/code/myproject (main branch) → dual → should show TUI with myproject highlighted
~/code/.dual/workspaces/myproject/feat__auth → dual → should highlight feat/auth
```
Context-aware TUI entry point.

**Scenario 6: Multiple terminals open, each in different branches**
```
Terminal 1: tmux session → lightfast/main
Terminal 2: tmux session → lightfast/feat-auth
Terminal 3: dual TUI (overview)
```
All should coexist. TUI should show accurate real-time status.

### 4.5 Zellij Equivalents

For users who use zellij instead of tmux:

| Tmux Concept | Zellij Equivalent |
|-------------|-------------------|
| Session | Session |
| Window | Tab |
| Pane | Pane |
| `tmux attach-session` | `zellij attach` |
| `tmux new-session` | `zellij --session` |
| `tmux kill-session` | `zellij kill-session` |
| `tmux has-session` | Check session existence |
| `tmux send-keys` | `zellij action write` |
| `tmux list-sessions` | `zellij list-sessions` |

The backend trait should abstract these into a common interface.

---

## Part 5: TUI Design — What's Needed

### 5.1 Current Interface

CLI-only. No TUI library in dependencies. Output via `tracing::info!()` with Unicode status symbols:
- `●` (green) = attached
- `●` (blue) = running
- `○` (yellow) = stopped
- `◌` (dim) = lazy

The SPEC (Phase 4, line 397) defers TUI to "Polish" phase.

### 5.2 TUI Requirements (from user's description)

1. Run `dual` → shows TUI
2. Tree view: repos at root, branches nested underneath
3. Selecting a branch opens/attaches to session
4. Must work from inside tmux (no nested issues)
5. Must coexist with user's existing tmux (`Meta+s` switching)
6. Quick return path from session back to TUI

### 5.3 Three TUI Models (from existing research)

**Model 1: Full Takeover**
- `dual` takes over terminal with alternate screen
- Tree view + preview pane
- Enter → exits TUI, attaches tmux session
- Meta-key → detaches tmux, returns to TUI

**Model 2: Overlay/Popup**
- Quick picker overlay (like tmux choose-tree)
- Appears on meta-key, dismisses on selection
- No persistent dashboard

**Model 3: Hybrid (recommended in existing research)**
- `dual` → full TUI home screen
- Select branch → transitions to tmux session
- Within tmux, meta-key → lightweight overlay picker
- Esc → returns to TUI home

### 5.4 TUI ↔ Tmux Transition Challenge

The critical design problem. Three options from existing research:

**Option A: Dual Runs Inside Tmux**
```
Outer tmux session (dual-control)
├── Window 0: Dual TUI
├── Window 1: lightfast-main session
├── Window 2: lightfast-feat-auth session
└── Window 3: agent-os-main session
```
Pros: Instant switching via tmux window switch. Cons: Nested tmux if user already uses tmux.

**Option B: Dual Manages Terminal Directly**
```
Terminal
├── State A: Dual TUI (alternate screen)
└── State B: tmux attach (tmux controls terminal)
    └── Meta-key → tmux detach → back to State A
```
Pros: Clean separation, no nested tmux. Cons: Terminal state transitions, potential flicker.

**Option C: Dual Replaces Tmux**
```
Terminal
└── Dual TUI (full control)
    ├── Tree view
    └── Embedded PTY terminal pane
```
Pros: Complete control. Cons: Massive effort, reimplementing terminal emulation.

### 5.5 Ratatui Stack (from existing research)

```toml
ratatui = "0.29"          # TUI framework
crossterm = "0.28"        # Terminal backend
tui-tree-widget = "0.22"  # Tree view for repos/branches
tui-input = "0.11"        # Text input for search/filter
```

---

## Part 6: Plugin System — Current State (None) and Requirements

### 6.1 Current Extensibility

The codebase has **zero extensibility infrastructure**:

- No traits for behavior injection
- No lifecycle hooks
- No plugin discovery or loading
- No middleware chain
- No observer pattern
- No command extension mechanism
- No configuration-driven behavior

The only "extension point" is `.dual.toml` data fields.

### 6.2 What Needs to Be Pluggable

| Component | Why | Current State |
|-----------|-----|---------------|
| Terminal multiplexer | tmux vs zellij vs none | Hardcoded tmux in `src/tmux.rs` |
| Session layout | Per-project pane configuration | Single pane, no config |
| Container runtime | Docker vs Podman vs none | Hardcoded docker in `src/container.rs` |
| Command routing | Which commands go to container | Hardcoded list in `src/shell.rs:2-4` |
| Post-launch hooks | Run commands after workspace setup | `setup` field exists but never executes |
| Env var sources | `.env`, Vercel, 1Password | `env` field exists but never wires |

### 6.3 The SPEC's Backend Contract

SPEC.md lines 269-294 describe a backend interface:

| Method | Signature |
|--------|-----------|
| `create_session` | `(workspace_id, processes[])` |
| `attach` | `(session_handle)` |
| `detach` | `(session_handle)` |
| `destroy` | `(session_handle)` |
| `is_alive` | `(session_handle) → bool` |
| `list_sessions` | `() → session_handle[]` |

Three planned backends:
- `TmuxBackend` (default)
- `ZellijBackend` (future)
- `BasicBackend` (fallback — no multiplexer)

None of these are implemented as traits. The tmux module is a flat collection of functions.

### 6.4 Shell Command List

The container command list at `src/shell.rs:2-4` is hardcoded:

```rust
const CONTAINER_COMMANDS: &[&str] = &[
    "npm", "npx", "pnpm", "node", "python", "python3", "pip", "pip3", "curl", "make",
];
```

No mechanism to add project-specific commands. A Go project would need `go`, a Rust project would need `cargo`, etc.

---

## Part 7: State and Config Systems

### 7.1 Global State (`~/.dual/workspaces.toml`)

```rust
// src/state.rs:14-39
pub struct WorkspaceState {
    pub workspace_root: Option<String>,  // Default: ~/.dual/workspaces
    pub workspaces: Vec<WorkspaceEntry>,
}

pub struct WorkspaceEntry {
    pub repo: String,           // "lightfast"
    pub url: String,            // git URL or local path
    pub branch: String,         // "feat/auth"
    pub path: Option<String>,   // Some = user dir, None = managed clone
}
```

Atomic writes with advisory file locking (`fs2::FileExt`).

No concept of:
- Branch parent (which branch was this created from?)
- Branch type (local vs remote)
- Workspace status (only computed dynamically from docker/tmux)
- Creation timestamp
- Last accessed timestamp

### 7.2 Per-Repo Hints (`.dual.toml`)

```rust
// src/config.rs:18-38
pub struct RepoHints {
    pub image: String,                    // Docker image
    pub ports: Vec<u16>,                  // Exposed ports
    pub setup: Option<String>,            // Setup command (UNIMPLEMENTED)
    pub env: HashMap<String, String>,     // Env vars (UNIMPLEMENTED)
    pub shared: Option<SharedConfig>,     // Shared file config
}
```

Written to workspace root as `.dual.toml`. Committed to git.

### 7.3 Shared Config (`~/.dual/shared/{repo}/`)

Managed by `src/shared.rs`. Two modes:
- Main workspace: moves files to shared dir, creates symlinks back (Unix) or copies (Windows)
- Branch workspace: copies files from shared dir into workspace

Configured via `[shared]` section in `.dual.toml`:
```toml
[shared]
files = [".vercel", ".env.local"]
```

---

## Part 8: Container and Proxy Systems

### 8.1 Docker Integration (`src/container.rs`)

Container create args at `src/container.rs:147-167`:
```
docker create --name dual-lightfast-main
  -v /path/to/workspace:/workspace
  -v /workspace/node_modules    # anonymous volume
  -w /workspace
  node:20
  sleep infinity
```

Notable gaps:
- No `-e KEY=VALUE` for env vars
- No `--env-file` support
- No additional volume mounts
- No port publishing (`-p`) — proxy handles routing
- No `setup` command execution

### 8.2 Reverse Proxy (`src/proxy.rs`)

HTTP reverse proxy using hyper + tokio. Subdomain-based routing:
```
lightfast-main.localhost:3000 → container IP:3000
```

Builds routing table from workspace state + running containers + port config.

### 8.3 Shell Command Routing (`src/shell.rs`)

Generates shell functions that intercept commands:
```bash
npm() {
    if [ -t 1 ]; then
        command docker exec -t -w /workspace dual-lightfast-main npm "$@"
    else
        command docker exec -w /workspace dual-lightfast-main npm "$@"
    fi
}
```

Written to `~/.config/dual/rc/{container_name}.sh`, sourced in tmux session.

---

## Part 9: Test Infrastructure

99 unit tests across all modules + integration tests:

| Module | Tests | Focus |
|--------|-------|-------|
| `state.rs` | 24 | Parsing, validation, CRUD operations |
| `config.rs` | 18 | Parsing, encoding, file I/O |
| `main.rs` | 16 | CLI parsing |
| `shared.rs` | 13 | Init, copy, overwrite scenarios |
| `shell.rs` | 10 | Classification, RC generation |
| `tmux.rs` | 6 | Session naming, argument building |
| `proxy.rs` | 5 | Subdomain extraction, routing |
| `container.rs` | 4 | Argument building |
| `clone.rs` | 3 | Path detection, argument building |

Integration tests in `tests/e2e.rs` (marked `#[ignore]`, require Docker + tmux):
- `tmux_session_lifecycle` — create → alive → destroy
- `tmux_send_keys` — send command → poll for result

CI: `.github/workflows/test.yml` installs Docker + tmux in GitHub Actions.

---

## Part 10: Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| `clap` | 4 | CLI argument parsing |
| `dirs` | 6 | Home directory detection |
| `fs2` | 0.4 | Advisory file locking |
| `http-body-util` | 0.1 | HTTP body utilities for proxy |
| `hyper` | 1 | HTTP server/client for proxy |
| `hyper-util` | 0.1 | Hyper tokio integration |
| `serde` | 1 | TOML serialization |
| `thiserror` | 2 | Error type derivation |
| `tokio` | 1 | Async runtime for proxy |
| `toml` | 0.8 | TOML parsing |
| `tracing` | 0.1 | Structured logging |
| `tracing-subscriber` | 0.3 | Log initialization |

No TUI dependencies yet. Adding TUI would require `ratatui` + `crossterm`.

---

## Part 11: Historical Context (from thoughts/)

### Architecture Decisions

- `thoughts/ARCHITECTURE.md` — 27/27 SPEC claims validated via hypothesis-driven experiments
- `thoughts/BUILD.md` — MVP 14/14 modules complete
- `thoughts/shared/research/2026-02-13-BUILD-clone.md` — Worktrees rejected for lock contention; full clones chosen
- `thoughts/shared/research/2026-02-13-BUILD-tmux.md` — Tmux module research, session lifecycle design

### Previous Research (same day)

- `thoughts/shared/research/2026-02-15-dual-tui-design.md` — TUI framework analysis, three models, ratatui stack
- `thoughts/shared/research/2026-02-15-config-propagation-across-workspaces.md` — Shared config solution (implemented)
- `thoughts/shared/research/2026-02-15-env-vars-plugin-infrastructure.md` — Documents env/setup are unimplemented
- `thoughts/shared/research/2026-02-15-architecture-rethink-bugs-tui-plugins.md` — Earlier version of this research

### SPEC Vision

- SPEC.md lines 269-294: Backend trait contract (TmuxBackend, ZellijBackend, BasicBackend)
- SPEC.md lines 298-320: TUI with fuzzy picker, meta-key switching
- SPEC.md lines 349-357: Workspace states (ATTACHED, BACKGROUND, STOPPED, LAZY)
- SPEC.md line 397: Phase 4 "TUI with workspace sidebar showing live status"
- SPEC.md lines 113-132: Full clones not worktrees, with reasoning

---

## Code References

### Bug 1 — Branch Model
- `src/main.rs:159-206` — `cmd_create()` no git validation
- `src/clone.rs:46-58` — `git clone -b` construction
- `src/clone.rs:54` — the `-b` flag that requires remote branch
- `src/state.rs:87-96` — `add_workspace()` accepts any branch name

### Bug 2 — Context Detection
- `src/cli.rs:24-30` — `Create` requires explicit repo + branch
- `src/main.rs:669-688` — `detect_workspace()` only used by sync
- `src/main.rs:723-757` — `detect_git_repo()` only used by add

### Bug 3 — Config Defaults
- `src/config.rs:44-54` — `Default` impl
- `src/config.rs:78-83` — `write_hints()` with no comments
- `src/main.rs:101-111` — creation in `cmd_add()`

### Tmux
- `src/tmux.rs:20-46` — `create_session()` bare session
- `src/tmux.rs:49-64` — `attach()` simple attach
- `src/tmux.rs:110-119` — `build_new_session_args()` only `-d -s -c`
- `src/tmux.rs:122-125` — `session_name()` naming

### State
- `src/state.rs:14-39` — `WorkspaceState` and `WorkspaceEntry`
- `src/state.rs:74-79` — `resolve_workspace()` exact match
- `src/state.rs:188-225` — Atomic save with locking

### Config
- `src/config.rs:18-38` — `RepoHints` struct
- `src/config.rs:94-96` — `workspace_id()` naming
- `src/config.rs:113-115` — `encode_branch()` `/` → `__`

### Shell
- `src/shell.rs:2-4` — Hardcoded container commands
- `src/shell.rs:37-55` — RC generation
- `src/shell.rs:63-74` — Function template

### Container
- `src/container.rs:20-36` — `create()` takes only name, dir, image
- `src/container.rs:147-167` — Docker args (no env, no extra mounts)

---

## Open Questions

1. **Branch creation strategy**: Should `dual create` clone then `git checkout -b`, or clone from local main workspace (faster, `--local` hardlinks)?
2. **Worktrees revisited**: Full clones were chosen over worktrees for lock contention. Should this be revisited for internal-only branches?
3. **TUI default**: Should `dual` (no args) launch TUI, with `dual list` as non-interactive fallback?
4. **Multiplexer trait**: What's the minimal trait for tmux/zellij abstraction? Current functions map directly.
5. **Session layout config**: Where should per-repo layouts live? In `.dual.toml`? Separate file? Plugin config?
6. **Plugin format**: Lua? TOML declarations? Shell hooks? Rust dynamic libraries?
7. **Existing tmux detection**: If user is in tmux, should Dual use `tmux switch-client` instead of nested attach?
8. **TUI model**: Full takeover (Model 1), overlay (Model 2), or hybrid (Model 3)?
9. **Container runtime abstraction**: Should Docker be behind a trait too (for Podman, etc.)?
10. **Standalone vs tmux-aware**: Should Dual work without any multiplexer at all (BasicBackend from SPEC)?
