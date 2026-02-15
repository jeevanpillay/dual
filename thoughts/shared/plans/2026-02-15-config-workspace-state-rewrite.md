# Config & Workspace State Architecture Rewrite

## Overview

Rewrite Dual's configuration architecture from a single `dual.toml` file to a split model: **per-repo `.dual.toml`** for runtime hints (image, ports, setup, env) and **centralized `~/.dual/workspaces.toml`** for workspace state (which repos/branches the developer is actively working on). This enables dynamic workspace creation via CLI commands instead of manual config editing.

Mental model: "Repos declare what they need. Dual tracks what you're working on."

## Current State Analysis

Today, `dual.toml` is the sole source of truth for workspace definitions. It declares repos with names, URLs, branches, and ports. There is no dynamic workspace creation — to add a new branch workspace, you must edit `dual.toml`. The config file is both the declaration and the registry.

### Key Discoveries:
- `src/config.rs:7-32` — `DualConfig` and `RepoConfig` structs define the current schema
- `src/config.rs:63-73` — `resolve_workspace()` iterates repos x branches to match identifiers
- `src/config.rs:76-84` — `all_workspaces()` returns cartesian product of repos x branches
- `src/config.rs:103-118` — Config discovery searches `./dual.toml` then `~/.config/dual/dual.toml`
- `src/container.rs:6` — Docker image hardcoded to `node:20`
- `src/container.rs:23-48` — `create()` takes `&DualConfig` to derive workspace_dir and container_name
- `src/proxy.rs:27-54` — `ProxyState::from_config()` iterates all workspaces and reads `repo.ports`
- `src/clone.rs:40-87` — `clone_workspace()` takes `&DualConfig` for workspace_dir computation
- `src/main.rs` — Every command handler loads config via `config::load()` (7 occurrences)
- No `dual add`, `dual create`, or `dual switch` commands exist in `src/cli.rs:1-52`

## Desired End State

After this plan is complete:

1. **No more `dual.toml`** — The old config format is removed entirely (clean break)
2. **`~/.dual/workspaces.toml`** exists as centralized workspace state, tracking which repos/branches the developer is working on
3. **`.dual.toml`** in repo roots provides runtime hints (image, ports, setup, env) — team-shared
4. **`dual add`** (no args, run inside a repo) detects git remote URL and current branch, registers the **existing directory** as a workspace with an explicit path, reads/prompts for `.dual.toml` if missing
5. **`dual create <repo> <branch>`** adds a new branch workspace to state (no explicit path — cloned to workspace_root on launch)
6. **`dual destroy <workspace>`** tears down resources AND removes from state
7. **All existing commands** (`launch`, `list`, `proxy`, `urls`, `open`) work with the new architecture
8. **All tests pass** — unit tests, integration tests, e2e tests

### Verification:
- `cargo test` passes (all unit tests)
- `cargo clippy` clean
- `cargo build` succeeds
- `dual add <local-fixture-repo>` creates state file and workspace entry
- `dual create <repo> <branch>` adds workspace to state
- `dual list` shows workspaces from state file
- `dual launch <workspace>` uses hints for container image
- `dual destroy <workspace>` removes from state
- `dual proxy` reads ports from per-repo `.dual.toml`

## What We're NOT Doing

- **No `dual switch` command** — Can be added later; `dual launch` already handles attachment
- **No remote state sync** — State is machine-local; cross-machine portability via `dual add` in each repo
- **No `.dual.toml` reconciliation** — If repo's `.dual.toml` changes, next launch picks it up naturally (it's read fresh each time)
- **No workspace groups or profiles** — Single flat list of workspaces
- **No `dual add <url>`** — User must clone repos manually first, then run `dual add` from inside the repo

## Implementation Approach

The rewrite proceeds bottom-up: new data models first, then update consumers, then add new CLI commands, then update existing commands. Each phase produces a compilable, testable state.

### Module Structure (new/changed):

```
src/
  state.rs      # NEW: WorkspaceState, WorkspaceEntry — centralized state management
  config.rs     # REWRITTEN: workspace identity functions + RepoHints
  cli.rs        # UPDATED: Add, Create commands
  main.rs       # UPDATED: new + updated command handlers
  clone.rs      # UPDATED: takes workspace_root path instead of &DualConfig
  container.rs  # UPDATED: takes image string directly (no more &DualConfig)
  proxy.rs      # UPDATED: uses state + hints
  shell.rs      # UNCHANGED
  tmux.rs       # UNCHANGED
  lib.rs        # UPDATED: add `pub mod state;`
```

---

## Phase 1: State Module

### Overview
Create `src/state.rs` — the centralized workspace state manager that reads/writes `~/.dual/workspaces.toml`.

### Changes Required:

#### 1. New file: `src/state.rs`

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const STATE_DIR: &str = "dual";
const STATE_FILENAME: &str = "workspaces.toml";
const DEFAULT_WORKSPACE_ROOT: &str = "dual-workspaces";

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct WorkspaceState {
    /// Root directory for all workspace clones (default: ~/dual-workspaces)
    pub workspace_root: Option<String>,

    /// Active workspace entries
    #[serde(default)]
    pub workspaces: Vec<WorkspaceEntry>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct WorkspaceEntry {
    /// Short repo name (e.g. "lightfast")
    pub repo: String,

    /// Git URL or local path
    pub url: String,

    /// Branch name (e.g. "main", "feat/auth")
    pub branch: String,

    /// Explicit path to workspace directory (for `dual add` — user's existing clone).
    /// If None, workspace lives at {workspace_root}/{repo}/{encoded_branch}/ and
    /// will be cloned on first launch.
    pub path: Option<String>,
}
```

**Methods on `WorkspaceState`:**

```rust
impl WorkspaceState {
    /// Create a new empty state.
    pub fn new() -> Self { ... }

    /// Resolve the workspace root directory as an absolute path.
    pub fn workspace_root(&self) -> PathBuf { ... }

    /// Find a workspace entry by identifier (e.g. "lightfast-main").
    pub fn resolve_workspace(&self, identifier: &str) -> Option<&WorkspaceEntry> { ... }

    /// Get all workspace entries.
    pub fn all_workspaces(&self) -> &[WorkspaceEntry] { ... }

    /// Add a workspace entry. Returns Err if duplicate exists.
    pub fn add_workspace(&mut self, entry: WorkspaceEntry) -> Result<(), StateError> { ... }

    /// Remove a workspace entry by repo + branch. Returns true if found.
    pub fn remove_workspace(&mut self, repo: &str, branch: &str) -> bool { ... }

    /// Find all workspaces for a given repo name.
    pub fn workspaces_for_repo(&self, repo: &str) -> Vec<&WorkspaceEntry> { ... }

    /// Check if a workspace entry exists for repo + branch.
    pub fn has_workspace(&self, repo: &str, branch: &str) -> bool { ... }

    /// Get the workspace directory for an entry.
    /// Uses explicit path if set, otherwise computes from workspace_root.
    pub fn workspace_dir(&self, entry: &WorkspaceEntry) -> PathBuf { ... }
}
```

**Module-level functions:**

```rust
/// Get the state file path: ~/.dual/workspaces.toml
pub fn state_path() -> Option<PathBuf> { ... }

/// Load state from default location. Returns empty state if file doesn't exist.
pub fn load() -> Result<WorkspaceState, StateError> { ... }

/// Save state to default location. Creates directory if needed.
pub fn save(state: &WorkspaceState) -> Result<(), StateError> { ... }

/// Parse state from TOML string (for testing).
pub fn parse(toml_str: &str) -> Result<WorkspaceState, StateError> { ... }

/// Validate state entries.
fn validate(state: &WorkspaceState) -> Result<(), StateError> { ... }
```

**`StateError` enum:**

```rust
#[derive(Debug)]
pub enum StateError {
    NoHomeDir,
    ReadError(PathBuf, std::io::Error),
    WriteError(PathBuf, std::io::Error),
    ParseError(PathBuf, toml::de::Error),
    SerializeError(toml::ser::Error),
    Validation(String),
    DuplicateWorkspace(String, String), // repo, branch
}
```

**Key behaviors:**
- `load()` returns an empty `WorkspaceState` if the file doesn't exist (not an error — first-time use)
- `save()` creates `~/.dual/` directory if it doesn't exist
- `resolve_workspace()` uses `config::workspace_id()` to match identifiers
- `add_workspace()` rejects duplicates (same repo + branch)
- Workspace root defaults to `~/dual-workspaces` (same as current)

#### 2. Update `src/lib.rs`

Add `pub mod state;` to module exports.

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` compiles successfully
- [x] `cargo test` — all existing tests still pass (state.rs is additive)
- [x] `cargo clippy` — no warnings
- [x] New unit tests in `state.rs` pass:
  - Parse empty state
  - Parse state with workspaces
  - Serialize/deserialize roundtrip
  - Add workspace
  - Add duplicate workspace → error
  - Remove workspace
  - Resolve workspace by identifier
  - All workspaces iteration
  - Workspaces for repo filtering
  - State path construction
  - Empty state when file doesn't exist

---

## Phase 2: Rewrite Config Module

### Overview
Rewrite `src/config.rs` to contain workspace identity functions and `RepoHints` (per-repo `.dual.toml` loading). Remove `DualConfig`, `RepoConfig`, and all old loading logic.

### Changes Required:

#### 1. Rewrite `src/config.rs`

**Keep (unchanged):**
- `encode_branch(branch: &str) -> String` — `/` → `__`
- `decode_branch(encoded: &str) -> String` — `__` → `/`

**Keep (signature change):**
- `container_name(repo: &str, branch: &str) -> String` — change from method on `DualConfig` to module function
- `workspace_dir(workspace_root: &Path, repo: &str, branch: &str) -> PathBuf` — takes `workspace_root` as first arg instead of `&self`

**Add new:**
```rust
/// Compute the workspace identifier from repo + branch.
/// e.g. ("lightfast", "feat/auth") → "lightfast-feat__auth"
pub fn workspace_id(repo: &str, branch: &str) -> String {
    format!("{}-{}", repo, encode_branch(branch))
}
```

**Add `RepoHints`:**
```rust
const HINTS_FILENAME: &str = ".dual.toml";
const DEFAULT_IMAGE: &str = "node:20";

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct RepoHints {
    /// Docker image to use for containers (default: "node:20")
    #[serde(default = "default_image")]
    pub image: String,

    /// Ports that services bind to inside the container
    #[serde(default)]
    pub ports: Vec<u16>,

    /// Setup command to run after container creation (e.g. "pnpm install")
    pub setup: Option<String>,

    /// Environment variables for the container
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

fn default_image() -> String {
    DEFAULT_IMAGE.to_string()
}

impl Default for RepoHints {
    fn default() -> Self {
        Self {
            image: DEFAULT_IMAGE.to_string(),
            ports: Vec::new(),
            setup: None,
            env: std::collections::HashMap::new(),
        }
    }
}
```

**Add hints loading:**
```rust
/// Load RepoHints from a workspace directory's .dual.toml.
/// Returns default hints if the file doesn't exist.
pub fn load_hints(workspace_dir: &Path) -> Result<RepoHints, HintsError> { ... }

/// Write RepoHints to a workspace directory's .dual.toml.
pub fn write_hints(workspace_dir: &Path, hints: &RepoHints) -> Result<(), HintsError> { ... }

/// Parse hints from TOML string (for testing).
pub fn parse_hints(toml_str: &str) -> Result<RepoHints, HintsError> { ... }
```

**Remove:**
- `DualConfig` struct
- `RepoConfig` struct
- `ConfigError` enum
- `load()`, `load_from()`, `parse()`, `validate()` functions
- `discovery_paths()` function
- `CONFIG_FILENAME` constant
- `shellexpand()` function (move to `state.rs` as needed)

**`HintsError` enum:**
```rust
#[derive(Debug)]
pub enum HintsError {
    ReadError(PathBuf, std::io::Error),
    WriteError(PathBuf, std::io::Error),
    ParseError(PathBuf, toml::de::Error),
    SerializeError(toml::ser::Error),
}
```

#### 2. Update Cargo.toml

Add `serde` Serialize feature (already has Deserialize). The existing `serde = { version = "1", features = ["derive"] }` already includes what we need since `derive` enables both. No change needed here unless we need `dialoguer` for prompting (Phase 4).

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` compiles (note: consumer modules will have compile errors — we fix in Phase 3)
- [x] Unit tests for config.rs pass:
  - `encode_branch` / `decode_branch` (unchanged)
  - `container_name` as module function
  - `workspace_dir` with explicit workspace_root
  - `workspace_id` formatting
  - Parse hints TOML
  - Parse hints with all fields
  - Parse hints with missing fields (defaults)
  - Default hints
  - Load hints from file
  - Load hints from missing file (returns defaults)
  - Write hints to file

**Implementation Note**: Phase 2 and Phase 3 must be implemented together for the code to compile, since removing `DualConfig` breaks all consumers. They are separated here for clarity but should be implemented as a single commit.

---

## Phase 3: Update Consumer Modules

### Overview
Update clone.rs, container.rs, and proxy.rs to work with the new state + hints architecture instead of `&DualConfig`.

### Changes Required:

#### 1. Update `src/clone.rs`

Change all functions to take `workspace_root: &Path` instead of `config: &DualConfig`:

```rust
// BEFORE:
pub fn workspace_exists(config: &DualConfig, repo: &str, branch: &str) -> bool {
    let dir = config.workspace_dir(repo, branch);
    dir.join(".git").exists()
}

// AFTER:
pub fn workspace_exists(workspace_root: &Path, repo: &str, branch: &str) -> bool {
    let dir = crate::config::workspace_dir(workspace_root, repo, branch);
    dir.join(".git").exists()
}
```

Functions to update:
- `workspace_exists(workspace_root: &Path, repo, branch)` — was `(config: &DualConfig, repo, branch)`
- `clone_workspace(workspace_root: &Path, repo, url, branch)` — was `(config: &DualConfig, repo, url, branch)`
- `remove_workspace(workspace_root: &Path, repo, branch)` — was `(config: &DualConfig, repo, branch)`
- `list_existing_workspaces()` — remove entirely (was only used for iterating RepoConfig.branches, no longer applicable)

Remove `use crate::config::{DualConfig, RepoConfig};` import, add `use crate::config;`.

Update tests in `clone.rs` to use `workspace_root` path directly instead of parsing a DualConfig.

#### 2. Update `src/container.rs`

Change `create()` to take concrete values instead of `&DualConfig`:

```rust
// BEFORE:
pub fn create(config: &DualConfig, repo: &str, branch: &str, image: Option<&str>) -> Result<String, ContainerError>

// AFTER:
pub fn create(name: &str, workspace_dir: &Path, image: &str) -> Result<String, ContainerError>
```

The caller now computes the container name and workspace directory before calling `create()`. The image is always provided (resolved from hints or default by the caller).

Remove `use crate::config::DualConfig;` import.

Remove `DEFAULT_IMAGE` constant (moved to config.rs as part of `RepoHints` defaults).

#### 3. Update `src/proxy.rs`

Change `ProxyState::from_config()` and related functions:

```rust
// BEFORE:
pub fn from_config(config: &DualConfig) -> Self

// AFTER:
use crate::config;
use crate::state::WorkspaceState;

pub fn from_state(state: &WorkspaceState) -> Self {
    let workspace_root = state.workspace_root();
    let mut port_routes: HashMap<u16, RouteMap> = HashMap::new();

    for entry in state.all_workspaces() {
        let container_name = config::container_name(&entry.repo, &entry.branch);

        // Only include running containers
        match container::status(&container_name) {
            ContainerStatus::Running => {}
            _ => continue,
        }

        let ip = match container::get_ip(&container_name) {
            Some(ip) => ip,
            None => continue,
        };

        // Load hints to get ports
        let ws_dir = config::workspace_dir(&workspace_root, &entry.repo, &entry.branch);
        let hints = config::load_hints(&ws_dir).unwrap_or_default();

        let workspace_id = config::workspace_id(&entry.repo, &entry.branch);
        for port in &hints.ports {
            let subdomain = format!("{workspace_id}.localhost:{port}");
            port_routes
                .entry(*port)
                .or_default()
                .insert(subdomain, format!("{ip}:{port}"));
        }
    }

    Self { routes: port_routes }
}
```

Similarly update:
- `start(state: &WorkspaceState)` — was `start(config: &DualConfig)`
- `workspace_urls(state: &WorkspaceState)` — was `workspace_urls(config: &DualConfig)`

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` compiles successfully
- [x] `cargo clippy` — no warnings
- [x] `cargo test` — unit tests in clone.rs, container.rs, proxy.rs pass
- [x] Updated tests use new function signatures

---

## Phase 4: New CLI Commands

### Overview
Add `dual add <url>` and `dual create <repo> <branch>` commands with interactive prompting.

### Changes Required:

#### 1. Add `dialoguer` dependency to `Cargo.toml`

```toml
[dependencies]
dialoguer = "0.11"
```

#### 2. Update `src/cli.rs`

Add new command variants:

```rust
#[derive(Subcommand)]
pub enum Command {
    /// Register the current repo as a dual workspace
    Add {
        /// Short name for the repo (derived from directory name if omitted)
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Create a new branch workspace for an existing repo
    Create {
        /// Repo name (must already be added)
        repo: String,

        /// Branch name
        branch: String,
    },

    // ... existing commands unchanged ...
}
```

#### 3. Add command handlers in `src/main.rs`

**`cmd_add(name)`:**
```
1. Detect current directory and verify it's a git repo:
   - Run `git rev-parse --show-toplevel` → repo root path
   - Run `git remote get-url origin` → git URL
   - Run `git rev-parse --abbrev-ref HEAD` → current branch
2. Derive repo name: use --name arg, or derive from directory name
3. Load state (or create empty)
4. Check if repo+branch already exists → error if so
5. Try to load .dual.toml from repo root
6. If .dual.toml doesn't exist:
   a. Prompt for Docker image (default: node:20)
   b. Prompt for ports (comma-separated, default: none)
   c. Prompt for setup command (default: none)
   d. Write .dual.toml to repo root
7. Add WorkspaceEntry { repo, url, branch, path: Some(repo_root) } to state
8. Save state
9. Print success message with workspace ID
```

**Git detection helpers:**
```rust
/// Detect git repo info from the current directory.
fn detect_git_repo() -> Result<(PathBuf, String, String), String> {
    // Get repo root
    let root_output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|_| "git not found")?;
    if !root_output.status.success() {
        return Err("not inside a git repository".to_string());
    }
    let root = PathBuf::from(String::from_utf8_lossy(&root_output.stdout).trim());

    // Get remote URL
    let url_output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .map_err(|_| "git not found")?;
    let url = if url_output.status.success() {
        String::from_utf8_lossy(&url_output.stdout).trim().to_string()
    } else {
        root.to_string_lossy().to_string() // local-only repo, use path as URL
    };

    // Get current branch
    let branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|_| "git not found")?;
    let branch = String::from_utf8_lossy(&branch_output.stdout).trim().to_string();

    Ok((root, url, branch))
}
```

**Prompting flow (using dialoguer):**
```rust
use dialoguer::Input;

fn prompt_hints() -> RepoHints {
    let image: String = Input::new()
        .with_prompt("Docker image")
        .default("node:20".into())
        .interact_text()
        .unwrap();

    let ports_str: String = Input::new()
        .with_prompt("Ports (comma-separated, or empty)")
        .default("".into())
        .allow_empty(true)
        .interact_text()
        .unwrap();

    let ports: Vec<u16> = ports_str
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let setup: String = Input::new()
        .with_prompt("Setup command (or empty)")
        .default("".into())
        .allow_empty(true)
        .interact_text()
        .unwrap();

    RepoHints {
        image,
        ports,
        setup: if setup.is_empty() { None } else { Some(setup) },
        env: std::collections::HashMap::new(),
    }
}
```

**Repo name derivation:**
```rust
/// Derive a short repo name from a directory path.
/// "/Users/jeevan/code/lightfast" → "lightfast"
fn derive_repo_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repo")
        .to_string()
}
```

**`cmd_create(repo, branch)`:**
```
1. Load state
2. Find an existing workspace for this repo → error if none (repo not added)
3. Check if repo+branch already exists → error if so
4. Get URL from existing workspace entry
5. Add new WorkspaceEntry { repo, url, branch, path: None }
6. Save state
7. Print success: "Created workspace {repo}-{branch}. Use `dual launch {id}` to start."
```

#### 4. Wire new commands in `main.rs`

Add to the match block:
```rust
Some(Command::Add { name }) => cmd_add(name.as_deref()),
Some(Command::Create { repo, branch }) => cmd_create(&repo, &branch),
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` compiles successfully
- [x] `cargo clippy` — no warnings
- [x] `cargo test` — all tests pass
- [x] Unit tests for `derive_repo_name()`:
  - SSH URL → repo name
  - HTTPS URL → repo name
  - Local path → repo name
  - URL with .git suffix → stripped

#### Manual Verification:
- [ ] `dual add /path/to/local/repo` works with interactive prompts
- [ ] `dual add --name myrepo /path/to/repo` uses provided name
- [ ] `dual create myrepo feat/auth` adds workspace to state
- [ ] `dual create nonexistent feat/auth` errors with clear message
- [ ] `dual list` shows newly added/created workspaces

**Implementation Note**: After completing this phase, pause for manual verification that `dual add` and `dual create` work correctly with interactive prompting before proceeding.

---

## Phase 5: Update Existing Commands

### Overview
Rewrite all existing command handlers to use `state::load()` + `config::load_hints()` instead of `config::load()`.

### Changes Required:

#### 1. Update `cmd_default()` (main.rs)

```rust
fn cmd_default() -> i32 {
    let state = match state::load() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!("\nRun `dual add <url>` to get started.");
            return 1;
        }
    };

    let workspaces = state.all_workspaces();
    if workspaces.is_empty() {
        println!("No workspaces. Run `dual add <url>` to add a repo.");
        return 0;
    }

    println!("Workspaces:\n");
    print_workspace_status(&state);
    println!("\nUse `dual launch <workspace>` to start a workspace.");
    println!("Use `dual add <url>` to add a new repo.");
    0
}
```

#### 2. Update `cmd_launch()` (main.rs)

```rust
fn cmd_launch(workspace: &str) -> i32 {
    let state = match state::load() { ... };

    let entry = match state.resolve_workspace(workspace) {
        Some(e) => e,
        None => {
            eprintln!("error: unknown workspace '{workspace}'");
            eprintln!("\nConfigured workspaces:");
            for ws in state.all_workspaces() {
                eprintln!("  {}", config::workspace_id(&ws.repo, &ws.branch));
            }
            return 1;
        }
    };

    let workspace_root = state.workspace_root();
    let container_name = config::container_name(&entry.repo, &entry.branch);
    let session_name = tmux::session_name(&entry.repo, &entry.branch);

    // Step 1: Resolve workspace directory
    // If entry has explicit path (from `dual add`), use it directly.
    // Otherwise, clone to workspace_root.
    let workspace_dir = if let Some(ref path) = entry.path {
        let dir = PathBuf::from(shellexpand(path));
        if !dir.join(".git").exists() {
            eprintln!("error: workspace path {} does not contain a git repo", dir.display());
            return 1;
        }
        dir
    } else {
        match clone::clone_workspace(&workspace_root, &entry.repo, &entry.url, &entry.branch) {
            Ok(dir) => dir,
            Err(e) => { eprintln!("error: clone failed: {e}"); return 1; }
        }
    };

    // Step 2: Load hints for image
    let hints = config::load_hints(&workspace_dir).unwrap_or_default();

    // Step 3: Container (using hints.image)
    match container::status(&container_name) {
        ContainerStatus::Missing => {
            if let Err(e) = container::create(&container_name, &workspace_dir, &hints.image) { ... }
            // ... start
        }
        // ... Stopped, Running cases unchanged
    }

    // Steps 4-5: Shell RC, Tmux — unchanged
}
```

#### 3. Update `cmd_list()` (main.rs)

Replace `config::load()` with `state::load()`. Update `print_workspace_status()` to take `&WorkspaceState`.

#### 4. Update `cmd_destroy()` (main.rs)

```rust
fn cmd_destroy(workspace: &str) -> i32 {
    let mut state = match state::load() { ... };

    let entry = match state.resolve_workspace(workspace) {
        Some(e) => e.clone(), // clone to avoid borrow issues
        None => { eprintln!("error: unknown workspace"); return 1; }
    };

    let workspace_root = state.workspace_root();
    let container_name = config::container_name(&entry.repo, &entry.branch);
    let session_name = tmux::session_name(&entry.repo, &entry.branch);

    // Tear down: tmux → container → clone (unchanged logic)
    // ...

    // Remove from state
    state.remove_workspace(&entry.repo, &entry.branch);
    if let Err(e) = state::save(&state) {
        eprintln!("warning: failed to save state: {e}");
    }

    println!("Workspace '{workspace}' destroyed.");
    0
}
```

#### 5. Update `cmd_proxy()`, `cmd_urls()`, `cmd_open()` (main.rs)

Replace `config::load()` with `state::load()`. Pass `&state` to proxy functions.

#### 6. Update `print_workspace_status()` (main.rs)

```rust
fn print_workspace_status(state: &state::WorkspaceState) {
    let workspace_root = state.workspace_root();
    for ws in state.all_workspaces() {
        let ws_id = config::workspace_id(&ws.repo, &ws.branch);
        let container_name = config::container_name(&ws.repo, &ws.branch);
        let session_name = tmux::session_name(&ws.repo, &ws.branch);

        let clone_exists = clone::workspace_exists(&workspace_root, &ws.repo, &ws.branch);
        let container_st = container::status(&container_name);
        let tmux_alive = tmux::is_alive(&session_name);

        // Same status display logic
    }
}
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` compiles successfully
- [x] `cargo clippy` — no warnings
- [x] `cargo test` — all tests pass
- [x] CLI parser tests pass for all existing commands

#### Manual Verification:
- [ ] `dual` (no args) shows workspaces from state file
- [ ] `dual list` shows workspace status
- [ ] `dual launch <workspace>` uses hints for container image
- [ ] `dual destroy <workspace>` removes from state file
- [ ] `dual proxy` starts with ports from per-repo .dual.toml
- [ ] `dual urls` shows correct URLs based on hints

**Implementation Note**: After completing this phase, pause for manual verification of all existing commands working with the new architecture before proceeding.

---

## Phase 6: Update Tests

### Overview
Update all test infrastructure — unit tests, integration tests, e2e tests, and test fixtures — to use the new state + hints architecture.

### Changes Required:

#### 1. Update `src/config.rs` unit tests

Rewrite tests for the new module shape:
- Keep `encode_branch` / `decode_branch` tests (unchanged)
- Update `container_name` tests (now a module function, not method)
- Update `workspace_dir` tests (now takes `workspace_root: &Path`)
- Add `workspace_id` tests
- Add `RepoHints` parsing tests
- Add hints load/write tests
- Remove all `DualConfig` / `RepoConfig` tests

#### 2. Update `src/state.rs` unit tests

Should already exist from Phase 1. Ensure coverage:
- Parse empty state → empty workspaces
- Parse with entries
- Serialize/deserialize roundtrip
- Add/remove workspace
- Resolve workspace
- Duplicate detection
- Validation

#### 3. Update `tests/harness/mod.rs`

Replace `test_config()` helper:

```rust
// BEFORE:
pub fn test_config(workspace_root: &Path, toml_extra: &str) -> dual::config::DualConfig { ... }

// AFTER:
pub fn test_state(workspace_root: &Path, workspaces: &[(& str, &str, &str)]) -> dual::state::WorkspaceState {
    let mut state = dual::state::WorkspaceState::new();
    state.workspace_root = Some(workspace_root.to_string_lossy().to_string());
    for (repo, url, branch) in workspaces {
        state.add_workspace(dual::state::WorkspaceEntry {
            repo: repo.to_string(),
            url: url.to_string(),
            branch: branch.to_string(),
        }).expect("test workspace should not duplicate");
    }
    state
}
```

Also add a helper to write `.dual.toml` hints into fixture workspaces:

```rust
pub fn write_test_hints(workspace_dir: &Path, hints: &dual::config::RepoHints) {
    dual::config::write_hints(workspace_dir, hints).expect("failed to write test hints");
}
```

#### 4. Update `tests/fixtures/mod.rs`

Update `fixture_config_toml()` → `fixture_state()`:

```rust
// Return a WorkspaceState instead of a TOML string
pub fn fixture_state(
    workspace_root: &Path,
    fixture_repo: &Path,
    repo_name: &str,
    branch: &str,
) -> dual::state::WorkspaceState {
    let mut state = dual::state::WorkspaceState::new();
    state.workspace_root = Some(workspace_root.to_string_lossy().to_string());
    state.add_workspace(dual::state::WorkspaceEntry {
        repo: repo_name.to_string(),
        url: fixture_repo.to_string_lossy().to_string(),
        branch: branch.to_string(),
    }).unwrap();
    state
}
```

Add helper to create `.dual.toml` in fixture repos:

```rust
pub fn create_fixture_hints(repo_dir: &Path, ports: &[u16]) {
    let hints = dual::config::RepoHints {
        image: "node:20".to_string(),
        ports: ports.to_vec(),
        setup: None,
        env: std::collections::HashMap::new(),
    };
    dual::config::write_hints(repo_dir, &hints).expect("failed to write fixture hints");
}
```

#### 5. Update `tests/e2e.rs`

Update all e2e tests to construct `WorkspaceState` instead of parsing `DualConfig`:

```rust
// BEFORE:
let toml_str = fixtures::fixture_config_toml(&workspace_root, &repo_dir, "test-app", "main", &[3000]);
let config = dual::config::parse(&toml_str).unwrap();
dual::clone::clone_workspace(&config, "test-app", &config.repos[0].url, "main")

// AFTER:
let state = fixtures::fixture_state(&workspace_root, &repo_dir, "test-app", "main");
let workspace_root_path = state.workspace_root();
dual::clone::clone_workspace(&workspace_root_path, "test-app", &repo_dir.to_string_lossy(), "main")
```

Update container tests to pass image directly:

```rust
// BEFORE:
container::create(&config, "test-app", "main", None)

// AFTER:
let ws_dir = dual::config::workspace_dir(&workspace_root, "test-app", "main");
container::create("dual-test-app-main", &ws_dir, "node:20")
```

#### 6. Update `tests/fixture_smoke.rs` and `tests/harness_smoke.rs`

Rewrite to use new state + hints APIs instead of DualConfig.

#### 7. Remove old `dual.toml` from repo root

Delete `./dual.toml` from the repository (clean break).

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` compiles successfully
- [x] `cargo test` — ALL unit tests pass
- [x] `cargo test --test e2e` — all non-ignored tests pass
- [x] `cargo test --test fixture_smoke` — all tests pass
- [x] `cargo test --test harness_smoke` — all tests pass
- [x] `cargo clippy` — no warnings
- [x] `cargo fmt --check` — no formatting issues

#### Manual Verification:
- [ ] `cargo test --test e2e -- --ignored` — Docker/tmux tests pass (if Docker available)
- [ ] Full workflow test: `dual add <fixture-repo>` → `dual create <repo> <branch>` → `dual launch` → `dual destroy`

---

## Testing Strategy

### Unit Tests:
- `state.rs`: TOML parsing, serialization, add/remove/resolve/filter operations
- `config.rs`: encode/decode branch, container_name, workspace_dir, workspace_id, hints parsing/writing
- `clone.rs`: updated function signatures with workspace_root path
- `container.rs`: updated create function with direct parameters
- `main.rs`: CLI parser tests for new Add/Create commands

### Integration Tests:
- Clone operations with new workspace_root parameter
- Container lifecycle with explicit image parameter
- Hints loading from real filesystem (.dual.toml)

### Edge Cases:
- Empty state file (first run)
- Missing ~/.dual directory
- `.dual.toml` doesn't exist in repo (defaults)
- `.dual.toml` has unknown fields (should be ignored by serde)
- Duplicate workspace detection
- Repo name derivation from various URL formats
- Branch names with slashes (`feat/auth` → `feat__auth`)

## Performance Considerations

- Hints are loaded from disk per-workspace when needed (proxy, launch). For `dual list`, no hints needed — only state + runtime status checks.
- State file is small (dozens of entries max). Read/write is negligible.
- No change to Docker/tmux operation performance.

## References

- Research: `thoughts/shared/research/2026-02-15-config-workspace-state-architecture.md`
- Architecture: `thoughts/shared/research/2026-02-15-architectural-approaches-evaluation.md`
- Current config module: `src/config.rs:1-382`
- Current main: `src/main.rs:1-446`
- Current CLI: `src/cli.rs:1-52`
- Build tracker: `thoughts/BUILD.md`
