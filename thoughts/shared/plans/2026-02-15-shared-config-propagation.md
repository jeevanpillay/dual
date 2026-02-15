# Shared Config Propagation Implementation Plan

## Overview

Implement shared configuration file propagation across Dual workspaces. When users define a `[shared]` section in `.dual.toml`, gitignored files (`.env`, `.vercel/`, etc.) are automatically managed via a central shared directory (`~/.dual/shared/{repo}/`). The main workspace symlinks to the shared dir; branch workspaces receive copies. Users run `dual sync` to refresh branch copies.

## Current State Analysis

- `.dual.toml` (`RepoHints`) supports `image`, `ports`, `setup`, `env` — no `[shared]` section
- `cmd_add()` creates `.dual.toml` with defaults, registers workspace with `path: Some(...)`
- `cmd_create()` registers branch workspace with `path: None`
- `cmd_launch()` clones branch workspace via `git clone`, loads hints, creates container
- No shared directory, no symlink infrastructure, no sync command exists
- `WorkspaceEntry` has no parent/source reference between workspaces

### Key Discoveries:
- `RepoHints` ignores unknown TOML fields (serde default), so adding `[shared]` is backwards-compatible (`src/config.rs:242-251`)
- `clone_workspace()` does pure `git clone` — gitignored files are lost (`src/clone.rs:27-74`)
- Container creation only uses `hints.image` — `hints.env` is parsed but dropped (`src/main.rs:224-230`)
- `detect_git_repo()` in `main.rs:511-545` can detect repo context from current directory
- State directory structure: `~/.dual/` already exists for `workspaces.toml`

## Desired End State

1. `.dual.toml` supports a `[shared]` section listing files/directories to propagate
2. `dual add` auto-moves listed files from main workspace to `~/.dual/shared/{repo}/`, creates symlinks back
3. `dual launch` (branch) copies listed files from `~/.dual/shared/{repo}/` into the cloned workspace
4. `dual sync` (in branch) re-copies from shared dir, refreshing stale files
5. `dual sync` (in main) prompts for confirmation, then force-syncs ALL branch workspaces
6. On Windows: copy instead of symlink everywhere (no symlinks at all)

### Verification:
- User adds repo with `[shared] files = [".vercel", ".env.local"]`
- `.vercel/` and `.env.local` are moved to `~/.dual/shared/{repo}/`, symlinks created in main workspace
- User creates and launches a branch → branch gets copies of `.vercel/` and `.env.local`
- User runs `vercel pull` in main → updates flow through symlink to shared dir
- User runs `dual sync` in branch → branch gets fresh copy of updated files
- Changes in branch `.env.local` do NOT affect shared dir or other branches

## What We're NOT Doing

- No auto-detection of common file patterns (`.env*`, `.vercel/`) — users MUST define `[shared]`
- No file watching daemon — sync is manual via `dual sync`
- No Docker volume mount changes — shared files are in the workspace dir (visible to bind mount)
- No `dual destroy` changes — shared dir persists independently of workspace lifecycle
- No `[env]` wiring (separate concern, already documented in env-vars research)

## Implementation Approach

Three phases: (1) config schema + shared module, (2) CLI integration, (3) tests. The shared module is pure filesystem logic with no Docker or git dependencies, making it independently testable.

---

## Phase 1: Config Schema + Shared Module

### Overview
Add `[shared]` support to `RepoHints` and create `src/shared.rs` with all filesystem logic for managing the shared directory.

### Changes Required:

#### 1. Add `SharedConfig` to config
**File**: `src/config.rs`
**Changes**: Add `SharedConfig` struct and optional field on `RepoHints`

```rust
/// Shared configuration file propagation settings.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct SharedConfig {
    /// Files and directories to share across workspaces.
    /// e.g. [".vercel", ".env.local", ".env"]
    #[serde(default)]
    pub files: Vec<String>,
}
```

Add to `RepoHints`:
```rust
/// Shared files to propagate across workspaces
#[serde(skip_serializing_if = "Option::is_none")]
pub shared: Option<SharedConfig>,
```

Update `Default` impl to include `shared: None`.

#### 2. Add `shared_dir` helper to config
**File**: `src/config.rs`
**Changes**: Add function to compute shared directory path

```rust
/// Get the shared config directory for a repo: ~/.dual/shared/{repo}/
pub fn shared_dir(repo: &str) -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".dual").join("shared").join(repo))
}
```

#### 3. Create shared module
**File**: `src/shared.rs` (new)
**Changes**: Core logic for shared file management

```rust
use std::path::{Path, PathBuf};

/// Ensure the shared directory exists for a repo.
pub fn ensure_shared_dir(repo: &str) -> Result<PathBuf, SharedError> {
    let dir = crate::config::shared_dir(repo).ok_or(SharedError::NoHomeDir)?;
    std::fs::create_dir_all(&dir).map_err(|e| SharedError::Filesystem(dir.clone(), e))?;
    Ok(dir)
}

/// Initialize shared directory from the main workspace.
///
/// For each file in `files`:
/// - If file exists in workspace and is NOT already a symlink to shared dir:
///   move it to shared dir, create symlink back (Unix) or copy back (Windows)
/// - If file is already a symlink to shared dir: skip
/// - If file doesn't exist in workspace: skip
///
/// Returns list of files that were moved.
pub fn init_from_main(
    workspace_dir: &Path,
    shared_dir: &Path,
    files: &[String],
) -> Result<Vec<String>, SharedError>

/// Copy shared files into a branch workspace.
///
/// For each file in `files`:
/// - If file exists in shared dir: copy to workspace (overwrite if exists)
/// - If file doesn't exist in shared dir: skip
///
/// Returns list of files that were copied.
pub fn copy_to_branch(
    workspace_dir: &Path,
    shared_dir: &Path,
    files: &[String],
) -> Result<Vec<String>, SharedError>
```

**Key implementation details for `init_from_main`**:

```rust
for file in files {
    let src = workspace_dir.join(file);
    let dst = shared_dir.join(file);

    if !src.exists() {
        continue; // File not present yet, skip
    }

    // Check if already a symlink pointing to shared dir
    if src.symlink_metadata().map(|m| m.is_symlink()).unwrap_or(false) {
        if let Ok(target) = std::fs::read_link(&src) {
            if target == dst {
                continue; // Already set up correctly
            }
        }
    }

    // Ensure parent dir exists in shared
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Move file/dir to shared dir
    // Use copy + remove for cross-device moves (rename fails across filesystems)
    copy_recursive(&src, &dst)?;
    remove_recursive(&src)?;

    // Create symlink back (Unix) or copy back (Windows)
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&dst, &src)?;
    }
    #[cfg(windows)]
    {
        copy_recursive(&dst, &src)?;
    }
}
```

**Helper functions** (private in `src/shared.rs`):

```rust
/// Recursively copy a file or directory.
fn copy_recursive(src: &Path, dst: &Path) -> Result<(), SharedError>

/// Remove a file or directory.
fn remove_recursive(src: &Path) -> Result<(), SharedError>
```

`copy_recursive` logic:
- If `src` is a file: `std::fs::copy(src, dst)`
- If `src` is a directory: create dst dir, iterate entries, recurse

**Error type**:
```rust
#[derive(Debug)]
pub enum SharedError {
    NoHomeDir,
    Filesystem(PathBuf, std::io::Error),
}
```

#### 4. Register module
**File**: `src/lib.rs`
**Changes**: Add `pub mod shared;`

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` compiles successfully
- [x] `cargo clippy` passes with no warnings
- [x] `cargo fmt --check` passes
- [x] `cargo test` passes (existing tests unbroken)

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to Phase 2.

---

## Phase 2: CLI Integration

### Overview
Wire the shared module into `cmd_add`, `cmd_launch`, add the `Sync` CLI command, and implement `cmd_sync` with context detection.

### Changes Required:

#### 1. Add `Sync` command to CLI
**File**: `src/cli.rs`
**Changes**: Add variant to `Command` enum

```rust
/// Sync shared config files for current workspace
Sync {
    /// Workspace to sync (detected from current directory if omitted)
    workspace: Option<String>,
},
```

#### 2. Wire `cmd_sync` in main
**File**: `src/main.rs`
**Changes**: Add match arm and implement `cmd_sync`

Add to the match in `main()`:
```rust
Some(Command::Sync { workspace }) => cmd_sync(workspace),
```

Implement `cmd_sync`:
```rust
fn cmd_sync(workspace_arg: Option<String>) -> i32 {
    let st = match state::load() { ... };

    // Resolve which workspace we're syncing
    let entry = if let Some(ws) = workspace_arg {
        // Explicit workspace argument
        match st.resolve_workspace(&ws) {
            Some(e) => e.clone(),
            None => { eprintln!("error: unknown workspace '{ws}'"); return 1; }
        }
    } else {
        // Detect from current directory
        match detect_workspace(&st) {
            Some(e) => e.clone(),
            None => {
                eprintln!("error: not inside a dual workspace");
                eprintln!("Usage: dual sync [workspace]");
                return 1;
            }
        }
    };

    // Load hints
    let workspace_dir = st.workspace_dir(&entry);
    let hints = config::load_hints(&workspace_dir).unwrap_or_default();
    let shared_config = match &hints.shared {
        Some(s) if !s.files.is_empty() => s,
        _ => {
            eprintln!("error: no [shared] section in .dual.toml (or files list is empty)");
            return 1;
        }
    };

    let shared_dir = match shared::ensure_shared_dir(&entry.repo) {
        Ok(d) => d,
        Err(e) => { eprintln!("error: {e}"); return 1; }
    };

    let is_main = entry.path.is_some();

    if is_main {
        // Main workspace: init shared dir, then prompt to sync all branches
        match shared::init_from_main(&workspace_dir, &shared_dir, &shared_config.files) {
            Ok(moved) => {
                for f in &moved { println!("  moved {f} → shared/"); }
            }
            Err(e) => { eprintln!("error: {e}"); return 1; }
        }

        // Prompt to sync all branches
        let branches: Vec<_> = st.workspaces_for_repo(&entry.repo)
            .into_iter()
            .filter(|ws| ws.path.is_none())
            .collect();

        if branches.is_empty() {
            println!("No branch workspaces to sync.");
            return 0;
        }

        println!("\nSync shared files to ALL {} branch workspace(s)? [y/N]", branches.len());
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap_or(0);
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return 0;
        }

        for branch_entry in &branches {
            let branch_dir = st.workspace_dir(branch_entry);
            if !branch_dir.exists() {
                continue; // Not yet cloned
            }
            let ws_id = config::workspace_id(&branch_entry.repo, &branch_entry.branch);
            match shared::copy_to_branch(&branch_dir, &shared_dir, &shared_config.files) {
                Ok(copied) => {
                    println!("{ws_id}: synced {} file(s)", copied.len());
                }
                Err(e) => eprintln!("{ws_id}: error: {e}"),
            }
        }
    } else {
        // Branch workspace: copy from shared dir
        match shared::copy_to_branch(&workspace_dir, &shared_dir, &shared_config.files) {
            Ok(copied) => {
                if copied.is_empty() {
                    println!("No shared files available yet. Run `dual sync` in the main workspace first.");
                } else {
                    for f in &copied { println!("  synced {f}"); }
                }
            }
            Err(e) => { eprintln!("error: {e}"); return 1; }
        }
    }

    0
}
```

#### 3. Add `detect_workspace` helper
**File**: `src/main.rs`
**Changes**: Add function to find workspace from current directory

```rust
/// Detect which workspace the current directory belongs to.
fn detect_workspace(st: &state::WorkspaceState) -> Option<state::WorkspaceEntry> {
    let cwd = std::env::current_dir().ok()?;

    // Try git root first (handles being in subdirectories)
    let root = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string()))
        .unwrap_or(cwd);

    for ws in st.all_workspaces() {
        let ws_dir = st.workspace_dir(ws);
        if ws_dir == root {
            return Some(ws.clone());
        }
    }
    None
}
```

#### 4. Wire shared into `cmd_add`
**File**: `src/main.rs`
**Changes**: After `.dual.toml` handling (line ~98), add shared init

Insert after the `.dual.toml` creation block and before adding the workspace entry:

```rust
// Initialize shared directory if [shared] is configured
let hints = config::load_hints(&repo_root).unwrap_or_default();
if let Some(ref shared_config) = hints.shared {
    if !shared_config.files.is_empty() {
        match shared::ensure_shared_dir(&repo_name) {
            Ok(shared_dir) => {
                match shared::init_from_main(&repo_root, &shared_dir, &shared_config.files) {
                    Ok(moved) => {
                        for f in &moved {
                            println!("  shared: {f} → ~/.dual/shared/{repo_name}/");
                        }
                    }
                    Err(e) => eprintln!("warning: shared init failed: {e}"),
                }
            }
            Err(e) => eprintln!("warning: could not create shared directory: {e}"),
        }
    }
}
```

#### 5. Wire shared into `cmd_launch` (branch workspaces)
**File**: `src/main.rs`
**Changes**: After clone succeeds (line ~221), before container creation, add shared copy

Insert between workspace directory resolution and hint loading:

```rust
// For branch workspaces: copy shared files if [shared] is configured
if entry.path.is_none() {
    let hints = config::load_hints(&workspace_dir).unwrap_or_default();
    if let Some(ref shared_config) = hints.shared {
        if !shared_config.files.is_empty() {
            if let Ok(shared_dir) = shared::ensure_shared_dir(&entry.repo) {
                match shared::copy_to_branch(&workspace_dir, &shared_dir, &shared_config.files) {
                    Ok(copied) => {
                        for f in &copied {
                            println!("  shared: copied {f}");
                        }
                    }
                    Err(e) => eprintln!("warning: shared copy failed: {e}"),
                }
            }
        }
    }
}
```

#### 6. Wire shared into `cmd_launch` (main workspace re-init)
**File**: `src/main.rs`
**Changes**: For main workspaces (when `entry.path.is_some()`), re-run init to catch newly added files

Insert after workspace directory resolution for the `Some(path)` branch:

```rust
// For main workspace: ensure shared files are initialized
if entry.path.is_some() {
    let hints_check = config::load_hints(&workspace_dir).unwrap_or_default();
    if let Some(ref shared_config) = hints_check.shared {
        if !shared_config.files.is_empty() {
            if let Ok(shared_dir) = shared::ensure_shared_dir(&entry.repo) {
                match shared::init_from_main(&workspace_dir, &shared_dir, &shared_config.files) {
                    Ok(moved) => {
                        for f in &moved {
                            println!("  shared: {f} → ~/.dual/shared/{}/", entry.repo);
                        }
                    }
                    Err(e) => eprintln!("warning: shared init failed: {e}"),
                }
            }
        }
    }
}
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` compiles successfully
- [x] `cargo clippy` passes with no warnings
- [x] `cargo fmt --check` passes
- [x] `cargo test` passes (all existing + new tests)

#### Manual Verification:
- [ ] `dual add` with `[shared]` moves files and creates symlinks
- [ ] `dual launch` on branch copies shared files
- [ ] `dual sync` in branch refreshes files from shared dir
- [ ] `dual sync` in main prompts and syncs all branches
- [ ] `dual sync` without args detects workspace from current directory
- [ ] `dual sync lightfast-feat__auth` works with explicit argument
- [ ] Editing a file through the main workspace symlink updates the shared dir
- [ ] Editing a file in a branch workspace does NOT affect shared dir

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to Phase 3.

---

## Phase 3: Tests

### Overview
Add unit tests for the shared module and config changes. Integration tests for the full workflow.

### Changes Required:

#### 1. Config tests
**File**: `src/config.rs` (in `#[cfg(test)] mod tests`)

```rust
#[test]
fn parse_hints_with_shared() {
    let toml = r#"
image = "node:20"

[shared]
files = [".vercel", ".env.local"]
"#;
    let hints = parse_hints(toml).unwrap();
    let shared = hints.shared.unwrap();
    assert_eq!(shared.files, vec![".vercel", ".env.local"]);
}

#[test]
fn parse_hints_without_shared() {
    let hints = parse_hints("image = \"node:20\"").unwrap();
    assert!(hints.shared.is_none());
}

#[test]
fn parse_hints_shared_empty_files() {
    let toml = r#"
[shared]
files = []
"#;
    let hints = parse_hints(toml).unwrap();
    let shared = hints.shared.unwrap();
    assert!(shared.files.is_empty());
}

#[test]
fn write_hints_without_shared_omits_section() {
    let hints = RepoHints::default();
    let toml_str = toml::to_string_pretty(&hints).unwrap();
    assert!(!toml_str.contains("[shared]"));
}

#[test]
fn write_hints_with_shared_includes_section() {
    let hints = RepoHints {
        shared: Some(SharedConfig { files: vec![".env".to_string()] }),
        ..Default::default()
    };
    let toml_str = toml::to_string_pretty(&hints).unwrap();
    assert!(toml_str.contains("[shared]"));
    assert!(toml_str.contains(".env"));
}
```

#### 2. Shared module tests
**File**: `src/shared.rs` (in `#[cfg(test)] mod tests`)

```rust
#[test]
fn init_from_main_moves_file_and_creates_symlink() {
    // Create temp dirs for workspace and shared
    // Create a file in workspace
    // Call init_from_main
    // Assert: file exists in shared dir
    // Assert: symlink exists in workspace pointing to shared dir (Unix)
    // Assert: file content matches
}

#[test]
fn init_from_main_moves_directory() {
    // Create temp dirs
    // Create a directory with files in workspace
    // Call init_from_main
    // Assert: directory exists in shared dir with all files
    // Assert: symlink exists in workspace (Unix)
}

#[test]
fn init_from_main_skips_missing_files() {
    // Create temp dirs, no files
    // Call init_from_main with ["nonexistent"]
    // Assert: returns Ok with empty vec
}

#[test]
fn init_from_main_skips_existing_symlink() {
    // Create temp dirs
    // Create file in shared dir
    // Create symlink in workspace → shared dir
    // Call init_from_main
    // Assert: returns Ok with empty vec (nothing moved)
}

#[test]
fn copy_to_branch_copies_file() {
    // Create temp dirs for branch workspace and shared
    // Create a file in shared dir
    // Call copy_to_branch
    // Assert: file exists in workspace
    // Assert: file is a regular file (not symlink)
    // Assert: content matches
}

#[test]
fn copy_to_branch_copies_directory() {
    // Create a directory with files in shared dir
    // Call copy_to_branch
    // Assert: directory and files exist in workspace
}

#[test]
fn copy_to_branch_overwrites_existing() {
    // Create file with content "old" in workspace
    // Create file with content "new" in shared dir
    // Call copy_to_branch
    // Assert: workspace file now contains "new"
}

#[test]
fn copy_to_branch_skips_missing_shared_files() {
    // Shared dir is empty
    // Call copy_to_branch with ["nonexistent"]
    // Assert: Ok with empty vec
}

#[test]
fn shared_dir_path_format() {
    let dir = crate::config::shared_dir("lightfast").unwrap();
    assert!(dir.to_string_lossy().contains(".dual/shared/lightfast"));
}

#[cfg(unix)]
#[test]
fn init_from_main_creates_symlink_on_unix() {
    // Verify the symlink target is correct
    // Use std::fs::read_link to check
}

#[cfg(windows)]
#[test]
fn init_from_main_creates_copy_on_windows() {
    // Verify a real file (not symlink) exists after init
}
```

#### 3. CLI test
**File**: `src/main.rs` (in `#[cfg(test)] mod tests`)

```rust
#[test]
fn sync_subcommand_no_args() {
    let cli = Cli::parse_from(["dual", "sync"]);
    if let Some(Command::Sync { workspace }) = cli.command {
        assert!(workspace.is_none());
    } else {
        panic!("expected Sync command");
    }
}

#[test]
fn sync_subcommand_with_workspace() {
    let cli = Cli::parse_from(["dual", "sync", "lightfast-feat__auth"]);
    if let Some(Command::Sync { workspace }) = cli.command {
        assert_eq!(workspace.as_deref(), Some("lightfast-feat__auth"));
    } else {
        panic!("expected Sync command");
    }
}
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo test` — all new tests pass
- [x] `cargo clippy` — no warnings
- [x] `cargo fmt --check` — formatted correctly
- [x] `cargo build --release` — release build succeeds

---

## Data Flow Summary

```
User edits .dual.toml:
  [shared]
  files = [".vercel", ".env.local"]

dual add (main workspace):
  .vercel/ exists in /Users/dev/myrepo/
    → mv to ~/.dual/shared/myrepo/.vercel/
    → symlink /Users/dev/myrepo/.vercel → ~/.dual/shared/myrepo/.vercel
  .env.local exists in /Users/dev/myrepo/
    → mv to ~/.dual/shared/myrepo/.env.local
    → symlink /Users/dev/myrepo/.env.local → ~/.dual/shared/myrepo/.env.local

dual launch myrepo-feat__auth (branch):
  git clone → ~/.dual/workspaces/myrepo/feat__auth/
  .dual.toml has [shared] → copy from shared dir:
    cp ~/.dual/shared/myrepo/.vercel → ~/.dual/workspaces/myrepo/feat__auth/.vercel
    cp ~/.dual/shared/myrepo/.env.local → ~/.dual/workspaces/myrepo/feat__auth/.env.local

User runs `vercel pull` in main workspace:
  Writes through symlink → updates ~/.dual/shared/myrepo/.vercel/

User runs `dual sync` in feat__auth branch:
  Re-copies from ~/.dual/shared/myrepo/ → overwrites branch copies

User runs `dual sync` in main:
  Prompt: "Sync to ALL 3 branch workspaces? [y/N]"
  If y: copies shared files to all branch workspace directories
```

## Edge Cases

1. **File doesn't exist yet**: User defines `.vercel` in `[shared]` before running `vercel pull`. Init skips it. When user later runs `vercel pull`, it writes through the symlink (if init ran after the file existed) OR writes to the workspace dir (if symlink doesn't exist yet). Running `dual sync` in main again will move the new file.

2. **Cross-filesystem move**: `std::fs::rename` fails across filesystems. Use copy + remove instead of rename.

3. **Nested directories**: `.vercel/` contains subdirectories. `copy_recursive` handles this.

4. **Windows**: No symlinks. `init_from_main` copies back instead of symlinking. This means main workspace changes don't auto-propagate to shared dir on Windows — user must `dual sync` in main to push changes.

5. **Branch created before shared dir populated**: `copy_to_branch` gracefully skips files not in shared dir. User runs `dual sync` later when shared dir has content.

6. **`.dual.toml` not in git**: `.dual.toml` is committed to git (not gitignored). Branch clones receive it. The `[shared]` section is available in all workspaces.

## Performance Considerations

- File operations are synchronous and sequential per workspace — acceptable for the small number of config files typically shared
- `copy_recursive` for directories like `.vercel/` may involve dozens of files but total size is small (< 1 MB typically)
- No filesystem watchers, no background processes

## References

- Research: `thoughts/shared/research/2026-02-15-config-propagation-across-workspaces.md`
- Related: `thoughts/shared/research/2026-02-15-env-vars-plugin-infrastructure.md`
- Config module: `src/config.rs:9-40` (RepoHints)
- Clone module: `src/clone.rs:27-74` (clone_workspace)
- State module: `src/state.rs:20-35` (WorkspaceEntry)
- Container module: `src/container.rs:147-167` (build_create_args)
