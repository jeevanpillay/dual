---
date: 2026-02-15T03:59:05Z
researcher: Claude
git_commit: c2c8192a469bd88c8a5beea67c9cdf5f2cbfb6ea
branch: main
repository: dual
topic: "dual.toml configuration and workspace state architecture - current implementation"
tags: [research, codebase, config, dual-toml, workspace, branch-management, worktree-analogy]
status: complete
last_updated: 2026-02-15
last_updated_by: Claude
---

# Research: dual.toml Configuration and Workspace State Architecture

**Date**: 2026-02-15T03:59:05Z
**Researcher**: Claude
**Git Commit**: c2c8192a469bd88c8a5beea67c9cdf5f2cbfb6ea
**Branch**: main
**Repository**: dual

## Research Question

How does dual.toml configuration and workspace state management currently work? Specifically: what fields exist in dual.toml, where does it live, how are workspaces/branches created and tracked, and what is the relationship between the config file and runtime workspace state?

## Summary

Today, `dual.toml` is the **sole source of truth** for workspace definitions. It declares repos with their names, URLs, branches, and ports. There is no dynamic workspace creation — to add a new branch workspace, you must edit `dual.toml` first, then launch. All workspace state (clones, containers, tmux sessions) is derived from config at runtime through naming conventions. There is no separate workspace state file; the config file is both the declaration and the registry.

## Detailed Findings

### 1. dual.toml Schema and Structure

**File**: `src/config.rs:7-32`

The config has two levels:

```rust
pub struct DualConfig {
    pub workspace_root: Option<String>,  // line 10, defaults to ~/dual-workspaces
    pub repos: Vec<RepoConfig>,          // line 14, defaults to empty vec
}

pub struct RepoConfig {
    pub name: String,          // line 20, required - short name like "lightfast"
    pub url: String,           // line 23, required - git URL or local path
    pub branches: Vec<String>, // line 26, optional - defaults to empty vec
    pub ports: Vec<u16>,       // line 30, optional - defaults to empty vec
}
```

Current `dual.toml` at repo root:
```toml
[[repos]]
name = "dual"
url = "git@github.com:jeevanpillay/dual.git"
branches = ["main"]
```

Test fixture example (`src/config.rs:223-235`):
```toml
workspace_root = "~/my-workspaces"

[[repos]]
name = "lightfast"
url = "git@github.com:org/lightfast.git"
branches = ["main", "feat/auth", "fix/memory-leak"]

[[repos]]
name = "agent-os"
url = "/local/path/to/agent-os"
branches = ["main", "v2-rewrite"]
```

### 2. Config File Discovery

**File**: `src/config.rs:103-118, 155-167`

Discovery searches two locations in order:
1. **Current directory**: `./dual.toml` (line 159)
2. **User config directory**: `~/.config/dual/dual.toml` (lines 162-163)

First file found wins. No environment variable override exists.

```rust
fn discovery_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    paths.push(PathBuf::from(CONFIG_FILENAME));  // "dual.toml"
    if let Some(config_dir) = dirs::config_dir() {
        paths.push(config_dir.join("dual").join(CONFIG_FILENAME));
    }
    paths
}
```

### 3. Workspace Identity System

**File**: `src/config.rs:48-99`

All workspace resources derive their identity from `{repo.name}` + `{branch}`:

| Resource | Pattern | Example |
|----------|---------|---------|
| Workspace ID | `{repo}-{encoded_branch}` | `lightfast-feat__auth` |
| Directory | `{workspace_root}/{repo}/{encoded_branch}/` | `~/dual-workspaces/lightfast/feat__auth/` |
| Container | `dual-{repo}-{encoded_branch}` | `dual-lightfast-feat__auth` |
| Tmux session | `dual-{repo}-{encoded_branch}` | `dual-lightfast-feat__auth` |
| Shell RC file | `~/.config/dual/rc/{container_name}.sh` | `~/.config/dual/rc/dual-lightfast-feat__auth.sh` |
| Proxy subdomain | `{repo}-{encoded_branch}.localhost:{port}` | `lightfast-feat__auth.localhost:3000` |

Branch encoding (`src/config.rs:90-92`): `/` is replaced with `__` for filesystem safety.

### 4. How Workspaces Are Created (Launch Flow)

**File**: `src/main.rs:52-135`

The `dual launch <workspace-id>` command orchestrates five steps:

1. **Resolve** (line 61-72): Looks up workspace identifier against `config.repos × repo.branches`. Fails if not found in config.
2. **Clone** (line 78-84): Calls `clone::clone_workspace()` — idempotent, skips if `.git` exists. Full `git clone -b {branch}` to workspace directory.
3. **Container** (line 87-107): Checks status via `docker inspect`, creates/starts as needed. Uses hardcoded `node:20` image (`src/container.rs:6`). No image field in config.
4. **Shell RC** (line 110-116): Writes shell functions that wrap `docker exec` for transparent command routing.
5. **Tmux** (line 119-132): Creates detached session, sources RC, attaches.

### 5. How Workspaces Are Enumerated

**File**: `src/config.rs:75-84`

```rust
pub fn all_workspaces(&self) -> Vec<(&RepoConfig, &str)> {
    let mut result = Vec::new();
    for repo in &self.repos {
        for branch in &repo.branches {
            result.push((repo, branch.as_str()));
        }
    }
    result
}
```

This is the cartesian product `repos × branches`. Used by:
- Default command (`src/main.rs:38`) — shows workspace status
- List command (`src/main.rs:147`) — displays all workspaces
- Proxy (`src/proxy.rs:30`) — builds routing table
- URLs command (`src/proxy.rs:280`) — generates URLs

### 6. How Workspaces Are Resolved from CLI Input

**File**: `src/config.rs:63-73`

```rust
pub fn resolve_workspace(&self, identifier: &str) -> Option<(&RepoConfig, String)> {
    for repo in &self.repos {
        for branch in &repo.branches {
            let name = format!("{}-{}", repo.name, encode_branch(branch));
            if name == identifier {
                return Some((repo, branch.clone()));
            }
        }
    }
    None
}
```

Iterates all repos × branches, constructs the expected identifier, and compares. Returns `None` if no match — meaning **you cannot launch a workspace that isn't declared in dual.toml**.

### 7. What "State" Exists Beyond Config

The system has no dedicated state file. Runtime state is distributed across:

| State | Location | Checked via |
|-------|----------|-------------|
| Clone exists | `{workspace_dir}/.git` | `clone::workspace_exists()` at `src/clone.rs:15-19` |
| Container status | Docker daemon | `container::status()` at `src/container.rs:89-105` |
| Tmux session | tmux server | `tmux::is_alive()` at `src/tmux.rs:76-82` |
| Shell RC | `~/.config/dual/rc/*.sh` | `shell::write_rc_file()` at `src/shell.rs:84-96` |

Status display (`src/main.rs:330-351`) queries all three independently:
- `● attached` — container running + tmux alive
- `● running` — container running, no tmux
- `○ stopped` — container stopped or missing, clone exists
- `◌ lazy` — nothing created yet (declared in config but never launched)

### 8. Destroy Flow

**File**: `src/main.rs:158-218`

Tears down in reverse order: tmux → container → clone. Best-effort: failures are warnings, not errors.

### 9. Container Configuration

**File**: `src/container.rs:6-48, 159-179`

- Default image: `node:20` (hardcoded constant at line 6)
- `create()` accepts `image: Option<&str>` but callers always pass `None` (line 90 in `main.rs`)
- Container args built at line 159-179:
  - Bind mount: `{workspace_dir}:/workspace`
  - Anonymous volume: `/workspace/node_modules` (isolated)
  - Working directory: `/workspace`
  - Entrypoint: `sleep infinity`

### 10. CLI Commands Available

**File**: `src/cli.rs:1-52`

| Command | Arguments | Description |
|---------|-----------|-------------|
| (default) | none | Show all workspaces with status |
| `launch` | `<workspace>` | Create/start workspace and attach |
| `list` | none | List all workspaces with status |
| `destroy` | `<workspace>` | Tear down workspace resources |
| `open` | `[workspace]` | Open workspace URLs in browser |
| `urls` | `[workspace]` | Display workspace URLs |
| `proxy` | none | Start reverse proxy server |
| `shell-rc` | `<container>` | (hidden) Print shell RC for eval |

**No command exists to add/create a branch or workspace dynamically.**

### 11. The Current Branch Addition Workflow

To add a new workspace today:

1. **Edit `dual.toml`** — manually add branch to the `branches` array:
   ```toml
   [[repos]]
   name = "lightfast"
   url = "git@github.com:org/lightfast.git"
   branches = ["main", "feat/auth", "feat/new-feature"]  # ← add here
   ```
2. **Run `dual launch lightfast-feat__new-feature`** — creates clone, container, tmux
3. **Workspace appears in `dual list`** — even before launching (status: `◌ lazy`)

There is no `dual create` or `dual branch` command. The config file IS the workspace registry.

## Code References

- `src/config.rs:7-32` — DualConfig and RepoConfig struct definitions
- `src/config.rs:34-84` — Workspace path/name/resolution methods
- `src/config.rs:87-99` — Branch name encoding/decoding
- `src/config.rs:103-118` — Config loading with discovery
- `src/config.rs:138-153` — Config validation
- `src/config.rs:155-167` — Discovery path construction
- `src/cli.rs:1-52` — CLI command definitions
- `src/main.rs:28-49` — Default command handler
- `src/main.rs:52-135` — Launch command handler (workspace creation flow)
- `src/main.rs:138-155` — List command handler
- `src/main.rs:158-218` — Destroy command handler
- `src/main.rs:330-351` — Workspace status display
- `src/clone.rs:15-19` — Workspace existence check
- `src/clone.rs:40-87` — Git clone creation
- `src/container.rs:6` — Default image constant
- `src/container.rs:23-48` — Container creation
- `src/container.rs:89-105` — Container status check
- `src/container.rs:159-179` — Docker create argument building
- `src/shell.rs:37-55` — Shell RC generation
- `src/shell.rs:84-96` — RC file persistence
- `src/tmux.rs:20-46` — Tmux session creation
- `src/tmux.rs:122-125` — Session name generation
- `src/proxy.rs:26-54` — Proxy route building from config

## Architecture Documentation

### Config-Driven, Stateless Architecture

The current architecture is fundamentally **config-driven with stateless resolution**:
- `dual.toml` is the single source of truth for what workspaces SHOULD exist
- Runtime state (clones, containers, tmux) is checked on-demand via system queries
- No database, no state file, no registry — just config + naming conventions
- Workspace identity is deterministically derived from `(repo.name, branch)` tuple

### Dependency Flow

```
dual.toml → config::load() → DualConfig
                                  ↓
                        all_workspaces() / resolve_workspace()
                                  ↓
                     workspace_dir() / container_name()
                                  ↓
            ┌─────────────┬──────────────┬─────────────┐
            │             │              │             │
      clone::clone    container::    tmux::create   proxy::start
                      create          session
```

### Key Design Decisions in Current Code

1. **Full clones, not worktrees** — Each workspace is an independent `git clone` (confirmed in `thoughts/shared/research/2026-02-13-BUILD-clone.md`)
2. **Lazy creation** — Workspaces declared in config are only materialized on first launch
3. **Idempotent operations** — All create operations check for existence first
4. **Naming convention as state** — No need for a state registry because names are deterministic
5. **Config is the registry** — Adding a workspace means editing the config file

## Historical Context (from thoughts/)

- `thoughts/ARCHITECTURE.md` — Main architecture validation document confirming full-clone approach
- `thoughts/BUILD.md` — Build progress tracker showing module implementation status
- `thoughts/shared/research/2026-02-13-BUILD-config.md` — Research on config module design, including discovery logic
- `thoughts/shared/plans/2026-02-13-BUILD-config.md` — Implementation plan for config with TOML parsing
- `thoughts/shared/research/2026-02-13-BUILD-clone.md` — Research on full git clone strategy, explicitly rejecting worktrees
- `thoughts/shared/research/2026-02-13-BUILD-cli.md` — Research on CLI command structure
- `thoughts/shared/plans/2026-02-13-BUILD-wire-cli.md` — Plan for wiring CLI to all modules

## Related Research

- `thoughts/shared/research/2026-02-13-BUILD-config.md`
- `thoughts/shared/research/2026-02-13-BUILD-clone.md`
- `thoughts/shared/research/2026-02-13-BUILD-cli.md`
- `thoughts/shared/research/2026-02-13-BUILD-container.md`

## Open Questions

- How should workspace state (active branches) be separated from project config (docker image, ports)?
- What should the `~/.dual/` state directory structure look like?
- Should `dual.toml` in a repo become minimal (just docker image / runtime hints)?
- How should `dual create <branch>` dynamically register a workspace without editing config?
- What happens to workspace state when a user moves between machines (portability)?
