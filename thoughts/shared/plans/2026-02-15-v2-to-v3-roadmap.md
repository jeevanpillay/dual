# Dual v2.x → v3 Roadmap — Implementation Plan

## Overview

Incremental evolution from Dual v2.0.0 to v3.0.0. v2.x releases fix bugs, complete unimplemented features, and improve CLI ergonomics — all within the current architecture. v3.0.0 is the architectural rewrite (multiplexer trait, TUI, plugin system). This plan covers Linear project cleanup, every v2.x change, and v3 scoping.

## Current State Analysis

**v2.0.0 is shipped** (commit `3bb1635`). Working features:
- CLI: add, create, launch, list, destroy, open, urls, sync, proxy, shell-rc
- State management with atomic writes + file locking
- Git clone lifecycle
- Docker container lifecycle (create, start, stop, remove, status)
- Shell command routing via RC file generation
- Tmux session management
- Reverse proxy with subdomain routing
- Shared config propagation across workspaces
- Production readiness (thiserror, atomic state, tracing)

**Linear is stale**: Project still called "Dual v0", several shipped features show as "In Progress"

### Key Discoveries:
- `src/clone.rs:54` — `-b` flag requires branch to exist at origin (Bug 1)
- `src/main.rs:669-688` — `detect_workspace()` exists but only used by `cmd_sync()` (Bug 2)
- `src/config.rs:78-83` — `write_hints()` uses `toml::to_string_pretty()` which can't emit comments (Bug 3)
- `src/main.rs:262` — `env` HashMap loaded then dropped, never passed to container
- `src/config.rs:29` — `setup` field exists in schema, never executed
- `src/shell.rs:2-4` — `CONTAINER_COMMANDS` is hardcoded, not configurable
- `src/container.rs:156-157` — Anonymous `node_modules` volume is hardcoded
- `src/tmux.rs:49-64` — `attach()` doesn't check `$TMUX` env var, creates nested sessions

## Desired End State

After v2.2.0:
- All three bugs fixed (branch model, context detection, config defaults)
- `env` and `setup` fields fully functional
- Container commands configurable per-project
- Anonymous volumes configurable per-project
- Context-aware CLI (auto-detect repo from cwd)
- Tmux nested session handling
- Linear project cleaned up with proper milestones
- Clear path to v3.0.0

After v3.0.0:
- Multiplexer trait with TmuxBackend (and ZellijBackend stub)
- TUI as primary interface (`dual` with no args)
- Session layout configuration
- Plugin system for extensibility

## What We're NOT Doing

- **Not rewriting the module structure for v2.x** — v2.x fixes work within the current `main.rs`-orchestrated architecture
- **Not adding TUI in v2.x** — TUI requires ratatui dependency and architectural changes (v3)
- **Not abstracting Docker behind a trait in v2.x** — Podman support is v3+
- **Not building a full plugin system in v2.x** — `.dual.toml` field additions are sufficient
- **Not implementing ZellijBackend in v2.x** — multiplexer trait is v3
- **Not adding session layout config in v2.x** — requires multiplexer trait (v3)

## What Should Be Removed

1. **Hardcoded `node_modules` volume** — Replace with configurable `anonymous_volumes` in `.dual.toml`
2. **Stale Linear issues** — Close shipped work, mark duplicates
3. **"Dual v0" project name** — Rename to "Dual"
4. **Dead code paths** — `env` loading that goes nowhere (`main.rs:262`)

---

## Phase 0: Linear Project Cleanup

### Overview
Update the Linear project to reflect v2.0.0 reality. Rename project, close shipped issues, create new milestones for v2.1, v2.2, and v3.0.

### Changes Required:

#### 1. Rename Project
- Rename "Dual v0" → "Dual"
- Update description to reflect v2.0.0 is shipped
- Update summary

#### 2. Close Shipped Issues (mark as Done)
These features exist in the v2.0.0 codebase and should be marked Done:

| Issue | Feature | Evidence |
|-------|---------|----------|
| DUAL-19 | Command routing & shell integration | `src/shell.rs` — RC generation shipped |
| DUAL-20 | Multi-port reverse proxy | `src/proxy.rs` — proxy shipped |
| DUAL-18 | Container lifecycle management | `src/container.rs` — container module shipped |
| DUAL-23 | Full lifecycle orchestration | `src/main.rs:209-339` — `cmd_launch()` pipeline works |
| DUAL-27 | End-to-end MVP smoke test | `tests/e2e.rs` — E2E tests exist |
| DUAL-22 | Project service and port configuration | `src/config.rs` — ports field in RepoHints |

#### 3. Update Phase 1 Milestone
Phase 1 (Docker Layer) is effectively complete — all modules exist and work:
- Container lifecycle ✅
- Command routing ✅
- Reverse proxy ✅
- Port configuration ✅

Mark Phase 1 as complete or update its description.

#### 4. Create New Milestones

| Milestone | Version | Description |
|-----------|---------|-------------|
| v2.1 — Bug Fixes & Completions | v2.1.0 | Fix 3 fundamental bugs, wire env/setup, configurable commands/volumes |
| v2.2 — Context Awareness & UX | v2.2.0 | Context-aware CLI, tmux nested detection, improved list output |
| v3.0 — Architecture Rewrite | v3.0.0 | Multiplexer trait, TUI, session layouts, plugin system |

#### 5. Create New Issues (see Phases 1-3 below for details)

### Success Criteria:

#### Automated Verification:
- N/A (Linear-only changes)

#### Manual Verification:
- [x] "Dual v0" project renamed to "Dual"
- [x] All 6 shipped issues marked as Done
- [x] Phase 1 milestone updated
- [x] 3 new milestones created
- [x] New issues created for v2.1, v2.2, v3.0 work

---

## Phase 1: v2.1.0 — Bug Fixes & Completions

### Overview
Fix the three fundamental bugs and complete unimplemented features. All changes are within existing modules — no new modules, no new dependencies.

### Changes Required:

#### 1. Fix Branch Model (Bug 1)
**Files**: `src/clone.rs`, `src/main.rs`

**Current behavior**: `git clone -b <branch>` fails when branch doesn't exist at origin.

**New behavior**: Clone from local main workspace using `--local`, then `git checkout -b <new-branch>`.

**Implementation**:

In `src/clone.rs`, change `clone_workspace()`:
```rust
// Instead of:
//   git clone -b <branch> <url> <target_dir>
// Do:
//   1. Find local main workspace path for this repo
//   2. git clone --local <main_workspace_path> <target_dir>
//   3. cd <target_dir> && git checkout -b <branch>
```

In `src/main.rs`, update `cmd_create()`:
- Look up the main workspace (the one with `path: Some(...)`) for the given repo
- Pass its path to a new `clone_from_local()` function
- If no main workspace exists, fall back to `git clone <url>` then `git checkout -b`

New function signature in `clone.rs`:
```rust
pub fn clone_from_local(
    main_workspace_path: &Path,
    target_dir: &Path,
    new_branch: &str,
) -> Result<(), CloneError>
```

**Tests to add**:
- `test_clone_from_local_args` — verify git clone --local + git checkout -b args
- `test_clone_fallback_to_remote` — verify fallback when no local main exists
- Update existing `test_clone_workspace_args` — adjust for new behavior

#### 2. Fix Default `.dual.toml` (Bug 3)
**File**: `src/config.rs`

**Current behavior**: `toml::to_string_pretty()` generates bare TOML without comments.

**New behavior**: Hand-write a commented template string instead of serializing.

**Implementation**:

Replace `write_hints()` to write a hand-crafted template:
```rust
pub fn write_default_hints(repo_root: &Path) -> Result<(), ConfigError> {
    let template = r#"# Dual workspace configuration
# See: https://github.com/jeevanpillay/dual

# Docker image for the container runtime
image = "node:20"

# Ports your dev server uses (for reverse proxy routing)
# Example: ports = [3000, 3001]
# ports = []

# Shell command to run after container creation (e.g., dependency install)
# Example: setup = "pnpm install"
# setup = ""

# Environment variables passed to the container
# Example:
# [env]
# NODE_ENV = "development"

# Commands to route to the container (in addition to defaults)
# Default: npm, npx, pnpm, node, python, python3, pip, pip3, curl, make
# Example: extra_commands = ["cargo", "go", "ruby"]
# extra_commands = []

# Directories to isolate with anonymous Docker volumes
# These directories get their own volume so they don't sync between host/container
# Example: anonymous_volumes = ["node_modules", ".next", "target"]
# anonymous_volumes = ["node_modules"]

# Files to share across all workspaces of this repo
# These are gitignored files that should be available in every branch workspace
# [shared]
# files = [".env.local", ".vercel"]
"#;
    let hints_path = repo_root.join(".dual.toml");
    std::fs::write(&hints_path, template)?;
    Ok(())
}
```

Update `cmd_add()` in `src/main.rs:101-111` to call `write_default_hints()` instead of `write_hints(&RepoHints::default())`.

Keep `write_hints()` for programmatic updates — it serializes actual `RepoHints` structs.

#### 3. Wire `env` Vars to Container Creation
**Files**: `src/container.rs`, `src/main.rs`

**Current behavior**: `env` HashMap loaded at `main.rs:262` then dropped. `container::create()` takes only `(name, dir, image)`.

**Implementation**:

Update `container::create()` signature:
```rust
pub fn create(
    name: &str,
    workspace_dir: &Path,
    image: &str,
    env: &HashMap<String, String>,
    anonymous_volumes: &[String],
) -> Result<(), ContainerError>
```

In `build_create_args()`, add env vars:
```rust
for (key, value) in env {
    args.push("-e".to_string());
    args.push(format!("{key}={value}"));
}
```

Replace hardcoded `node_modules` volume with configurable `anonymous_volumes`:
```rust
for vol in anonymous_volumes {
    args.push("-v".to_string());
    args.push(format!("/workspace/{vol}"));
}
```

Update `cmd_launch()` in `main.rs` to pass `hints.env` and `hints.anonymous_volumes` to `container::create()`.

#### 4. Wire `setup` Command Execution
**Files**: `src/container.rs`, `src/main.rs`

**Current behavior**: `setup` field exists in `RepoHints` but is never executed.

**Implementation**:

Add to `container.rs`:
```rust
pub fn exec_setup(name: &str, setup_cmd: &str) -> Result<(), ContainerError> {
    // docker exec <name> sh -c "<setup_cmd>"
}
```

In `cmd_launch()`, after `container::start()` and before tmux session creation:
```rust
if let Some(setup) = &hints.setup {
    info!("Running setup: {setup}");
    container::exec_setup(&container_name, setup)?;
}
```

#### 5. Configurable Container Commands
**Files**: `src/config.rs`, `src/shell.rs`, `src/main.rs`

**Current behavior**: `CONTAINER_COMMANDS` at `shell.rs:2-4` is a hardcoded `&[&str]`.

**Implementation**:

Add to `RepoHints` in `config.rs`:
```rust
#[serde(default)]
pub extra_commands: Vec<String>,

#[serde(default)]
pub anonymous_volumes: Vec<String>,
```

Update `Default` impl:
```rust
extra_commands: Vec::new(),
anonymous_volumes: vec!["node_modules".to_string()],
```

Update `shell.rs` to accept additional commands:
```rust
pub fn write_rc_file(
    container_name: &str,
    extra_commands: &[String],
) -> Result<PathBuf, ShellError>
```

Merge `CONTAINER_COMMANDS` with `extra_commands` when generating RC.

#### 6. Update `cmd_launch()` Pipeline
**File**: `src/main.rs`

Wire all the new parameters through the launch pipeline:
```
load hints → pass hints.env + hints.anonymous_volumes to container::create()
           → pass hints.setup to container::exec_setup()
           → pass hints.extra_commands to shell::write_rc_file()
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo test` — all existing tests pass
- [x] `cargo clippy` — no warnings
- [x] `cargo fmt --check` — properly formatted
- [x] New tests for `clone_from_local()` pass
- [x] New tests for `write_default_hints()` pass
- [x] New tests for env var args in container create pass
- [x] New tests for extra_commands in shell RC generation pass
- [x] New tests for anonymous_volumes in container create pass

#### Manual Verification:
- [ ] `dual create myrepo feat/new-thing` creates a workspace that clones from local main and creates a new branch
- [ ] `dual add` in a new repo creates a well-commented `.dual.toml`
- [ ] Setting `env` vars in `.dual.toml` → they appear in `docker exec env` inside the container
- [ ] Setting `setup = "pnpm install"` → runs on first launch
- [ ] Setting `extra_commands = ["cargo"]` → `cargo build` routes to container
- [ ] Setting `anonymous_volumes = ["node_modules", ".next"]` → both get anonymous volumes

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 2: v2.2.0 — Context Awareness & UX

### Overview
Make the CLI context-aware so commands auto-detect the current repo/workspace from the working directory. Fix tmux nested session handling.

### Changes Required:

#### 1. Context-Aware `dual create`
**Files**: `src/cli.rs`, `src/main.rs`

Make `repo` optional in `Create` command:
```rust
Create {
    /// Branch name
    branch: String,
    /// Repo name (auto-detected if omitted)
    #[arg(long)]
    repo: Option<String>,
},
```

In `cmd_create()`, if `repo` is `None`:
1. Call `detect_git_repo()` to get the current repo info
2. Look up the repo name from state by matching the git remote URL
3. If found, use it. If not, error with a helpful message.

This means: `dual create feat/auth` works when inside a repo directory.

#### 2. Context-Aware `launch`, `destroy`, `open`, `urls`
**Files**: `src/cli.rs`, `src/main.rs`

Make the workspace arg optional for these commands:
```rust
Launch {
    /// Workspace identifier (auto-detected if omitted)
    workspace: Option<String>,
},
Destroy {
    workspace: Option<String>,
},
```

When workspace is `None`, call `detect_workspace()` to find the current workspace.

#### 3. Tmux Nested Session Detection
**File**: `src/tmux.rs`

Add detection in `attach()`:
```rust
pub fn attach(session_name: &str) -> Result<(), TmuxError> {
    if std::env::var("TMUX").is_ok() {
        // Already in tmux — use switch-client instead of attach
        // tmux switch-client -t <session_name>
    } else {
        // Not in tmux — use attach-session
        // tmux attach-session -t <session_name>
    }
}
```

#### 4. Improve `dual list` Output
**File**: `src/main.rs`

Group workspaces by repo in the list output:
```
lightfast
  main          ● running    (container: up, tmux: attached)
  feat/auth     ○ stopped    (container: stopped, tmux: none)
  feat/billing  ◌ lazy       (not cloned yet)

agent-os
  main          ● running    (container: up, tmux: background)
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo test` — all tests pass
- [x] `cargo clippy` — no warnings
- [x] New tests for context detection in create pass
- [x] New tests for optional workspace args pass
- [x] New tests for `$TMUX` detection pass
- [ ] New tests for grouped list output pass (output uses println! directly — verified by inspection)

#### Manual Verification:
- [ ] `cd ~/code/myproject && dual create feat/auth` auto-detects repo
- [ ] `cd ~/code/myproject && dual launch` auto-detects workspace
- [ ] Running `dual launch` from inside tmux uses `switch-client` (no nesting)
- [ ] `dual list` shows grouped output by repo

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 3: v3.0.0 — Architecture Rewrite (Scoping Only)

### Overview
This phase is scoped but NOT detailed for implementation yet. It defines what v3 contains so the v2.x work doesn't accidentally paint us into a corner.

### 3.1 Multiplexer Trait

Extract `src/tmux.rs` functions into a trait:
```rust
pub trait MultiplexerBackend {
    fn create_session(&self, name: &str, cwd: &Path, init_cmd: Option<&str>) -> Result<()>;
    fn attach(&self, name: &str) -> Result<()>;
    fn detach(&self, name: &str) -> Result<()>;
    fn destroy(&self, name: &str) -> Result<()>;
    fn is_alive(&self, name: &str) -> bool;
    fn list_sessions(&self) -> Vec<String>;
    fn send_keys(&self, name: &str, keys: &str) -> Result<()>;
    fn session_name(&self, repo: &str, branch: &str) -> String;
}
```

Implement `TmuxBackend`, stub `ZellijBackend`, add `BasicBackend` (no multiplexer).

### 3.2 TUI

Add ratatui + crossterm dependencies. New `src/tui/` module:
- `mod.rs` — TUI app state and event loop
- `tree.rs` — repo/branch tree view
- `status.rs` — workspace status panel

`dual` (no args) launches TUI. `dual list` becomes the non-interactive fallback.

### 3.3 Session Layout Configuration

Add to `.dual.toml`:
```toml
[layout]
type = "editor-claude-shell"  # or custom

[[layout.panes]]
command = "nvim ."
size = "60%"

[[layout.panes]]
command = "claude"
size = "20%"

[[layout.panes]]
command = ""  # shell
size = "20%"
```

### 3.4 Plugin System (Future)

Design TBD. Likely TOML-declared hooks + shell commands:
```toml
[hooks]
post_launch = "pnpm install && pnpm dev"
pre_destroy = "pnpm build"
```

### What v3 Requires That v2.x Must NOT Break:
- `config.rs` naming conventions must remain stable
- `WorkspaceEntry` struct can add fields but not change existing ones
- `container.rs` function signatures can evolve (already changing in v2.1)
- `.dual.toml` format must remain backward-compatible (new fields with defaults)

---

## Linear Issue Plan

### Issues to Create for v2.1.0 Milestone:

| Title | Priority | Labels | Description |
|-------|----------|--------|-------------|
| Fix branch model: clone from local main + git checkout -b | Urgent | Core | Bug 1 fix. Change `clone.rs` to clone from local main workspace using `--local`, then `git checkout -b` for new branches |
| Fix default .dual.toml: hand-write commented template | High | Core, DX | Bug 3 fix. Replace `toml::to_string_pretty()` with hand-crafted commented template in `write_default_hints()` |
| Wire env vars to container creation | High | Docker, Core | Pass `hints.env` HashMap as `-e KEY=VALUE` args to `docker create` |
| Wire setup command execution | High | Docker, Core | Execute `hints.setup` via `docker exec` after container start |
| Configurable container commands via .dual.toml | High | Core, DX | Add `extra_commands` field to RepoHints, merge with default CONTAINER_COMMANDS |
| Configurable anonymous volumes via .dual.toml | High | Docker, Core | Replace hardcoded `node_modules` volume with `anonymous_volumes` field |

### Issues to Create for v2.2.0 Milestone:

| Title | Priority | Labels | Description |
|-------|----------|--------|-------------|
| Context-aware `dual create`: auto-detect repo from cwd | High | Core, DX | Bug 2 fix. Make repo arg optional, use `detect_git_repo()` when omitted |
| Context-aware launch/destroy/open/urls | High | Core, DX | Make workspace arg optional, use `detect_workspace()` when omitted |
| Detect $TMUX and use switch-client | High | Tmux, DX | Check `$TMUX` env var in `attach()`, use `switch-client` instead of nested attach |
| Improve `dual list` with grouped repo output | Medium | DX | Group workspaces by repo, show container + tmux status per workspace |

### Issues to Create for v3.0.0 Milestone:

| Title | Priority | Labels | Description |
|-------|----------|--------|-------------|
| Multiplexer trait: extract tmux behind backend interface | High | Core, Tmux | Create `MultiplexerBackend` trait, implement `TmuxBackend` |
| TUI: ratatui-based workspace browser | High | Core, DX | Add ratatui/crossterm deps, implement tree view, `dual` → TUI |
| Session layout configuration | Medium | Core, Tmux | Add `[layout]` section to `.dual.toml` for pane/window config |
| ZellijBackend stub | Medium | Core | Implement `MultiplexerBackend` for zellij |
| BasicBackend: no-multiplexer fallback | Low | Core | Implement `MultiplexerBackend` that just runs in current terminal |

---

## Testing Strategy

### Unit Tests:
- Clone module: test local clone args, fallback behavior, checkout -b construction
- Config module: test template generation, extra_commands/anonymous_volumes parsing
- Container module: test env var args, anonymous volume args, setup exec args
- Shell module: test merged command lists (defaults + extras)
- Tmux module: test `$TMUX` detection, switch-client vs attach args
- CLI module: test optional args parsing

### Integration Tests:
- Full launch pipeline with env vars → verify env inside container
- Full launch pipeline with setup command → verify setup ran
- Clone from local → verify new branch exists
- Context detection → verify auto-detect from cwd

### Manual Testing Steps:
1. Create a workspace with `dual create myrepo feat/new-thing` where `feat/new-thing` doesn't exist at origin
2. Verify `.dual.toml` has helpful comments when created by `dual add`
3. Add env vars to `.dual.toml`, launch, verify inside container
4. Set `setup = "echo hello > /tmp/setup-ran"`, launch, verify file exists
5. Run `dual create feat/test` from inside a repo directory (no repo arg)
6. Run `dual launch` from inside tmux — verify no nesting

## Performance Considerations

- Clone from local with `--local` uses hardlinks — significantly faster than remote clone
- `detect_workspace()` and `detect_git_repo()` run git commands — acceptable for interactive CLI, but TUI (v3) should cache results
- Anonymous volumes list is typically small (1-3 items) — no performance concern

## Migration Notes

### v2.0.0 → v2.1.0
- **`.dual.toml` backward compatible**: New fields (`extra_commands`, `anonymous_volumes`) have defaults via `#[serde(default)]`
- **Existing workspaces**: Already-cloned workspaces are unaffected. Only new `dual create` uses the new clone strategy
- **Existing containers**: Must be recreated to pick up env vars/anonymous volumes. `dual destroy` + `dual launch` handles this

### v2.1.0 → v2.2.0
- **CLI breaking change**: `dual create` positional args change from `<repo> <branch>` to `<branch> [--repo REPO]`. This is a minor breaking change — document in changelog
- **Optional workspace args**: Previously required args become optional. Fully backward compatible (explicit args still work)

### v2.2.0 → v3.0.0
- **TUI as default**: `dual` (no args) changes from showing list to launching TUI. `dual list` remains as non-interactive fallback
- **Multiplexer trait**: Internal refactor, no user-facing change unless using zellij

## References

- Research: `thoughts/shared/research/2026-02-15-v3-architecture-rethink.md`
- Earlier research: `thoughts/shared/research/2026-02-15-architecture-rethink-bugs-tui-plugins.md`
- TUI design: `thoughts/shared/research/2026-02-15-dual-tui-design.md`
- SPEC.md: Backend trait contract (lines 269-294), TUI (lines 298-320, 397)
- Linear project: https://linear.app/jps0000-dual/project/dual-v0-7f8871a38577
