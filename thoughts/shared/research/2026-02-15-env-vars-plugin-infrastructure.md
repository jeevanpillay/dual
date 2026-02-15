---
date: 2026-02-15T05:46:09Z
researcher: jeevan
git_commit: b96a25d
branch: main
repository: dual
topic: "Environment variables, platform config files, and plugin/extensibility infrastructure"
tags: [research, codebase, config, env-vars, plugins, extensibility, workspace-provisioning]
status: complete
last_updated: 2026-02-15
last_updated_by: jeevan
---

# Research: Environment Variables, Platform Config Files, and Plugin Infrastructure

**Date**: 2026-02-15T05:46:09Z
**Researcher**: jeevan
**Git Commit**: b96a25d
**Branch**: main
**Repository**: dual

## Research Question

How does Dual currently handle environment variables and platform-specific config files (e.g. `.vercel`)? Is the `.dual.toml` `[env]` section sufficient, or does the system need a more generalized extensibility mechanism (plugin infrastructure) for users with different configuration needs?

## Summary

Dual has a **schema-defined but unimplemented** environment variable system. The `RepoHints` struct in `.dual.toml` includes an `[env]` table (`HashMap<String, String>`), but this data is loaded and never passed to Docker containers or shell wrappers. There is **no plugin system, hook mechanism, or extensibility infrastructure** of any kind. The codebase is entirely procedural with hardcoded command lists and fixed Docker arguments. The SPEC describes auto-detecting `.env` / `.env.local` files for container provisioning, but this is not implemented. No references to `.vercel`, `vercel.json`, or any platform-specific configuration exist anywhere in the codebase.

## Detailed Findings

### 1. Environment Variables: Schema Exists, Wiring Does Not

**The `[env]` field in RepoHints** (`src/config.rs:22-24`):
```rust
/// Environment variables for the container
#[serde(default)]
pub env: HashMap<String, String>,
```

Users can write this in `.dual.toml`:
```toml
[env]
NODE_ENV = "development"
DATABASE_URL = "postgres://localhost/dev"
```

**Where it gets loaded** (`src/main.rs:224`):
```rust
let hints = config::load_hints(&workspace_dir).unwrap_or_default();
```

**Where it gets dropped** (`src/main.rs:230`):
```rust
container::create(&container_name, &workspace_dir, &hints.image)?;
// Only hints.image is used. hints.env is never passed.
```

**Container creation accepts no env vars** (`src/container.rs:20`):
```rust
pub fn create(name: &str, workspace_dir: &Path, image: &str) -> Result<String, ContainerError>
```

**Docker args have no `-e` flags** (`src/container.rs:147-167`):
The `build_create_args()` function builds: `--name`, `-v` (bind mount), `-v` (node_modules volume), `-w`, image, `sleep infinity`. No environment variable injection.

**Shell wrappers also have no env forwarding** (`src/shell.rs:63-74`):
Generated docker exec commands are: `docker exec -t -w /workspace {container} {cmd} "$@"`. No `-e KEY=VAL` flags.

### 2. The SPEC's Vision vs Current Reality

**SPEC line 168** describes auto-detection:

| Detects | Provisions in Container |
|---|---|
| `.env` / `.env.local` | Environment variables |

This auto-detection is not implemented. The current system requires manual `[env]` entries in `.dual.toml`.

**SPEC line 24** states: "DO NOT require Claude to check environment variables" — meaning env vars should be transparently available inside the container without the AI agent needing to know about the routing.

### 3. No Plugin System, No Hook Mechanism, No Extensibility

The codebase has zero extensibility infrastructure:

- **No traits for behavior injection** — All functions are concrete implementations
- **No lifecycle hooks** — No `on_workspace_create`, `before_container_start`, `after_launch` callbacks
- **No plugin discovery or loading** — No dynamic dispatch, no registry pattern
- **No middleware chain** — Proxy, shell, and container modules are fixed pipelines
- **No observer pattern** — State mutations (`add_workspace`, `remove_workspace`) have no event emission
- **No command extension** — Container commands are a hardcoded constant array (`src/shell.rs:2-4`)

### 4. What Configuration CAN Do Today

The `.dual.toml` file provides **data, not behavior**:

| Field | Status | Used By |
|---|---|---|
| `image` | Implemented | `container::create()` at `src/main.rs:230` |
| `ports` | Implemented | `proxy::ProxyState::from_state()` at `src/proxy.rs:45` |
| `setup` | Schema only | Never executed (field exists at `src/config.rs:20`) |
| `env` | Schema only | Never passed to container or shell |

### 5. Platform-Specific Config Files

**No references exist in the codebase for**:
- `.vercel` / `vercel.json`
- `.netlify` / `netlify.toml`
- `fly.toml`
- `.env.production`, `.env.staging`
- `wrangler.toml` (Cloudflare Workers)
- Any secret management system (Vault, AWS SSM, 1Password CLI)

**The only platform-specific detection in the SPEC** (lines 163-169):
- `.nvmrc` / `.node-version` → Node.js version
- `pnpm-lock.yaml` → pnpm
- `pyproject.toml` → Python
- `docker-compose.yml` → Service definitions
- `.env` / `.env.local` → Environment variables
- `turbo.json` / `nx.json` → Monorepo task runner

None of these detection mechanisms are implemented.

### 6. Current Workspace Provisioning Flow

The full launch sequence (`src/main.rs:176-275`) is linear and fixed:

1. Resolve workspace entry from state
2. Compute container/tmux names
3. Clone repo if managed workspace (no path)
4. Load `.dual.toml` hints
5. Create Docker container (image only)
6. Start container
7. Write shell RC (hardcoded commands)
8. Create tmux session
9. Attach

There are no extension points in this pipeline. Every step calls a concrete function with fixed arguments.

### 7. The `dual.toml` vs `.dual.toml` Distinction

Two different files with similar names:
- **`dual.toml`** (root, no dot) — Workspace state file listing repos/branches. Currently at the project root: `[[repos]]` with name, url, branches.
- **`.dual.toml`** (dot prefix) — Per-workspace hints file in each workspace directory with image, ports, setup, env.

The root `dual.toml` is the old config format (pre-state-rewrite). The current `feature/config-workspace-state-rewrite` branch has recently split state from hints.

### 8. How Docker Supports Env Vars

Research documents (`thoughts/shared/research/2026-02-05-ARCH-docker-exec-basic.md`) confirm Docker supports:
- `docker create -e KEY=VALUE` — Set env at container creation
- `docker exec -e KEY=VALUE` — Set env per command execution
- `docker create --env-file .env` — Load from file

The plumbing exists in Docker; Dual just doesn't wire it through.

## Code References

- `src/config.rs:5` — `HINTS_FILENAME` constant (`.dual.toml`)
- `src/config.rs:8-25` — `RepoHints` struct with `env: HashMap<String, String>`
- `src/config.rs:31-40` — Default hints (empty env)
- `src/config.rs:44-56` — `load_hints()` reads `.dual.toml`
- `src/main.rs:224` — Hints loaded during launch
- `src/main.rs:230` — Only `hints.image` used; `hints.env` dropped
- `src/container.rs:20-36` — `create()` function, no env param
- `src/container.rs:147-167` — `build_create_args()`, no `-e` flags
- `src/shell.rs:2-4` — Hardcoded `CONTAINER_COMMANDS` array
- `src/shell.rs:63-74` — Shell wrapper template, no env forwarding
- `src/state.rs:11-35` — `WorkspaceState` and `WorkspaceEntry` structs
- `src/proxy.rs:45` — Ports read from hints for routing
- `SPEC.md:168` — `.env` / `.env.local` detection specified but unimplemented

## Architecture Documentation

### Current Extension Points (None Formal)

The only way to customize Dual's behavior today is through the `.dual.toml` data fields:
- Change the Docker image
- Declare ports for proxy routing
- Declare env vars and setup commands (not yet wired)

There is no mechanism to:
- Add custom file detection (e.g., find `.vercel` and do something with it)
- Hook into the workspace lifecycle
- Register additional commands for container routing
- Transform or inject configuration at specific pipeline stages
- Load external plugins or scripts

### Data Flow Gap

```
.dual.toml [env] table
    ↓
RepoHints.env (HashMap loaded in memory)
    ↓
cmd_launch() — hints loaded
    ↓
container::create() — only image passed, env ignored
    ↓
Container runs with NO user-defined env vars
```

## Historical Context (from thoughts/)

- `thoughts/shared/research/2026-02-13-BUILD-config.md` — Config module research. Defines `dual.toml` structure with repos, branches, optional image/routing overrides. No plugin discussion.
- `thoughts/shared/research/2026-02-13-BUILD-container.md` — Container module research. Documents Docker exec `-e KEY=VALUE` flag for env vars. MVP uses pre-existing base image.
- `thoughts/shared/research/2026-02-13-BUILD-shell.md` — Shell module research. Notes docker exec supports `-e KEY=VAL` for per-command env forwarding. Lists configurable per-project routing overrides (not implemented).
- `thoughts/shared/research/2026-02-05-ARCH-docker-exec-basic.md` — Architecture research confirming Docker's env var support and noting "Shell wrapper must forward environment variables explicitly."
- `thoughts/ARCHITECTURE.md` — Architecture validation. 27/27 claims validated. Notes credential separation (host vs container) but no env var management system.
- `thoughts/BUILD.md` — MVP build tracker. 14/14 modules complete. No plugin module planned or built.

## Open Questions

1. **Env var sources**: Beyond the `[env]` table in `.dual.toml`, what other sources should Dual consume? `.env` files, Vercel project settings, 1Password, AWS SSM?
2. **Scope of extensibility**: Is the need limited to "bring env vars into the container" or broader "run arbitrary setup logic per workspace"?
3. **Plugin vs configuration**: Should Dual detect files and act on them (plugin/detection system), or should users declare what they need in `.dual.toml` (configuration-driven)?
4. **Cross-workspace sharing**: Should env vars be defined per-workspace (`.dual.toml`), per-repo (shared across branches), or globally (`~/.dual/`)?
5. **Secrets vs config**: Environment variables often contain secrets. Should Dual have opinions about secret storage, or just pass through whatever the user provides?
6. **The `setup` command**: The existing but unimplemented `setup` field could serve as a minimal hook (run arbitrary shell after container creation). Is this sufficient extensibility?
