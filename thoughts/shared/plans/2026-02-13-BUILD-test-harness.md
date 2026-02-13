# Test Harness Implementation Plan

## Overview

Implement an RAII test harness for E2E integration tests. This requires restructuring the crate from a pure binary to library + binary, then creating a `TestFixture` struct that manages Docker containers, tmux sessions, and temp directories with automatic cleanup on Drop.

## Current State Analysis

- Binary-only crate: `src/main.rs` with `mod` declarations — integration tests cannot import
- No `tests/` directory exists
- No `[dev-dependencies]` in Cargo.toml
- 55 unit tests across 7 modules, all inline `#[cfg(test)]`
- All module APIs are `pub` and ready for external consumption

### Key Discoveries:
- Container cleanup needs `docker rm -f` for force removal (`src/container.rs:61-63`)
- Tmux cleanup needs `tmux kill-session -t` (`src/tmux.rs:72-74`)
- Config has `parse()` for string-based config creation (`src/config.rs:131-136`)
- UUID naming fits Docker 63-char limit (46 chars for `dual-test-{uuid}`)

## Desired End State

A `tests/harness/mod.rs` module providing:
1. `TestFixture` struct with RAII Drop cleanup
2. UUID-based resource naming (`dual-test-{uuid}`)
3. `cleanup_sweep()` function for prefix-based defense-in-depth
4. A passing smoke test verifying the harness works

Verification: `cargo build && cargo test && cargo clippy && cargo fmt --check` all pass.

## What We're NOT Doing

- NOT creating fixture repos (that's the test-fixture module)
- NOT writing E2E tests (that's the test-suite module)
- NOT creating CI pipeline (that's the ci-pipeline module)
- NOT modifying any existing module logic

## Implementation Approach

Create `lib.rs` to expose modules, update `main.rs` to use the library, add uuid dev-dependency, then build the test harness.

## Phase 1: Restructure Crate to Library + Binary

### Overview
Move module declarations from `main.rs` to a new `lib.rs` so integration tests can import module APIs.

### Changes Required:

#### 1. Create `src/lib.rs`
**File**: `src/lib.rs`
**Changes**: Declare and re-export all modules

```rust
pub mod cli;
pub mod clone;
pub mod config;
pub mod container;
pub mod proxy;
pub mod shell;
pub mod tmux;
```

#### 2. Update `src/main.rs`
**File**: `src/main.rs`
**Changes**: Remove `mod` declarations, use library imports

Replace:
```rust
mod cli;
mod clone;
mod config;
mod container;
mod proxy;
mod shell;
mod tmux;
```

With:
```rust
use dual::cli;
use dual::clone;
use dual::config;
use dual::container;
use dual::proxy;
use dual::shell;
use dual::tmux;
```

Also update the `use` statement:
Replace: `use cli::{Cli, Command};`
With: `use dual::cli::{Cli, Command};`

#### 3. Add dev-dependency
**File**: `Cargo.toml`
**Changes**: Add uuid crate for test naming

```toml
[dev-dependencies]
uuid = { version = "1", features = ["v4"] }
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` succeeds
- [x] `cargo test` passes (all 55 existing tests still pass)
- [x] `cargo clippy` clean
- [x] `cargo fmt --check` clean

---

## Phase 2: Create Test Harness

### Overview
Create `tests/harness/mod.rs` with TestFixture struct, RAII Drop, and cleanup sweep.

### Changes Required:

#### 1. Create test harness module
**File**: `tests/harness/mod.rs`
**Changes**: Full TestFixture implementation

```rust
use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;

/// RAII test fixture that manages Docker containers, tmux sessions, and temp directories.
///
/// Resources are cleaned up automatically on Drop — including on test assertion panics
/// (Rust default panic strategy is "unwind", which triggers Drop).
///
/// Naming convention: `dual-test-{uuid}` prevents cross-test contamination.
pub struct TestFixture {
    /// Unique identifier for this test run
    pub id: String,
    /// Short ID for display purposes
    pub short_id: String,
    /// Docker containers created by this fixture (cleaned up on Drop)
    containers: Vec<String>,
    /// Tmux sessions created by this fixture (cleaned up on Drop)
    tmux_sessions: Vec<String>,
    /// Temporary directories created by this fixture (cleaned up on Drop)
    temp_dirs: Vec<PathBuf>,
}

impl TestFixture {
    /// Create a new test fixture with a unique UUID-based identifier.
    pub fn new() -> Self {
        let id = Uuid::new_v4().to_string();
        let short_id = id[..8].to_string();
        Self {
            id,
            short_id,
            containers: Vec::new(),
            tmux_sessions: Vec::new(),
            temp_dirs: Vec::new(),
        }
    }

    /// Generate a unique container name for this test.
    /// Pattern: dual-test-{uuid}
    pub fn container_name(&self) -> String {
        format!("dual-test-{}", self.id)
    }

    /// Generate a unique tmux session name for this test.
    /// Pattern: dual-test-{uuid}
    pub fn session_name(&self) -> String {
        format!("dual-test-{}", self.id)
    }

    /// Create a temporary directory and register it for cleanup.
    pub fn temp_dir(&mut self) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dual-test-{}", self.id));
        std::fs::create_dir_all(&dir).expect("failed to create temp dir");
        self.temp_dirs.push(dir.clone());
        dir
    }

    /// Create a named subdirectory under a parent temp dir.
    pub fn temp_subdir(&mut self, parent: &std::path::Path, name: &str) -> PathBuf {
        let dir = parent.join(name);
        std::fs::create_dir_all(&dir).expect("failed to create temp subdir");
        // Don't register — parent dir cleanup handles it
        dir
    }

    /// Register a container name for RAII cleanup.
    pub fn register_container(&mut self, name: String) {
        self.containers.push(name);
    }

    /// Register a tmux session for RAII cleanup.
    pub fn register_tmux_session(&mut self, name: String) {
        self.tmux_sessions.push(name);
    }

    /// Create a DualConfig pointing to a test workspace root.
    /// The workspace_root is set to a temp directory.
    pub fn test_config(&mut self, workspace_root: &std::path::Path, toml_extra: &str) -> dual::config::DualConfig {
        let toml_str = format!(
            "workspace_root = \"{}\"\n{}",
            workspace_root.display(),
            toml_extra
        );
        dual::config::parse(&toml_str).expect("failed to parse test config")
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        // Cleanup containers (force-remove even if running)
        for name in &self.containers {
            let _ = Command::new("docker")
                .args(["rm", "-f", name])
                .output();
        }

        // Cleanup tmux sessions
        for session in &self.tmux_sessions {
            let _ = Command::new("tmux")
                .args(["kill-session", "-t", session])
                .output();
        }

        // Cleanup temp directories
        for dir in &self.temp_dirs {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// Defense-in-depth: remove ALL test resources matching the `dual-test-` prefix.
///
/// Run before/after test suites to clean up orphaned resources from
/// SIGKILL or other abnormal termination scenarios where Drop didn't fire.
pub fn cleanup_sweep() {
    // Remove all test containers
    let output = Command::new("docker")
        .args(["ps", "-aq", "--filter", "name=dual-test-"])
        .output();
    if let Ok(out) = output {
        let ids = String::from_utf8_lossy(&out.stdout);
        for id in ids.lines().filter(|l| !l.is_empty()) {
            let _ = Command::new("docker")
                .args(["rm", "-f", id])
                .output();
        }
    }

    // Remove all test tmux sessions
    let output = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}"])
        .output();
    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for session in stdout.lines().filter(|l| l.starts_with("dual-test-")) {
            let _ = Command::new("tmux")
                .args(["kill-session", "-t", session])
                .output();
        }
    }
}
```

#### 2. Create smoke test
**File**: `tests/harness_smoke.rs`
**Changes**: Basic test that creates and drops a TestFixture

```rust
mod harness;

#[test]
fn fixture_creates_unique_ids() {
    let f1 = harness::TestFixture::new();
    let f2 = harness::TestFixture::new();
    assert_ne!(f1.id, f2.id);
    assert_ne!(f1.container_name(), f2.container_name());
    assert_ne!(f1.session_name(), f2.session_name());
}

#[test]
fn fixture_names_use_test_prefix() {
    let f = harness::TestFixture::new();
    assert!(f.container_name().starts_with("dual-test-"));
    assert!(f.session_name().starts_with("dual-test-"));
}

#[test]
fn fixture_container_name_within_docker_limit() {
    let f = harness::TestFixture::new();
    assert!(f.container_name().len() <= 63);
}

#[test]
fn fixture_temp_dir_created_and_cleaned() {
    let dir;
    {
        let mut f = harness::TestFixture::new();
        dir = f.temp_dir();
        assert!(dir.exists());
    }
    // After Drop, temp dir should be cleaned up
    assert!(!dir.exists());
}

#[test]
fn fixture_test_config_parses() {
    let mut f = harness::TestFixture::new();
    let workspace_root = f.temp_dir();
    let config = f.test_config(
        &workspace_root,
        r#"
[[repos]]
name = "test-app"
url = "/tmp/test-repo"
branches = ["main"]
ports = [3000]
"#,
    );
    assert_eq!(config.repos.len(), 1);
    assert_eq!(config.repos[0].name, "test-app");
}

#[test]
fn cleanup_sweep_runs_without_error() {
    // Just verify it doesn't panic when no test resources exist
    harness::cleanup_sweep();
}
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` succeeds
- [x] `cargo test` passes (all existing + new tests — 65 total)
- [x] `cargo test --test harness_smoke` passes specifically (10 tests)
- [x] `cargo clippy` clean
- [x] `cargo fmt --check` clean
- [x] TestFixture creates unique names (test assertion)
- [x] TestFixture temp dir is cleaned on Drop (test assertion)
- [x] cleanup_sweep runs without panic (test assertion)

---

## Testing Strategy

### Unit Tests (in harness_smoke.rs):
- UUID uniqueness across fixtures
- Name prefix correctness (`dual-test-`)
- Docker 63-char name limit respected
- Temp dir creation and cleanup via Drop
- Config parsing with test workspace root
- Cleanup sweep runs without errors

### What is NOT tested here:
- Actual Docker container creation/cleanup (requires Docker — that's test-suite)
- Actual tmux session creation/cleanup (requires tmux — that's test-suite)
- Fixture repo creation (that's test-fixture module)

## References

- Architecture: `thoughts/ARCHITECTURE.md` — e2e-test-isolation, e2e-local-fixture-repo
- Research: `thoughts/shared/research/2026-02-13-BUILD-test-harness.md`
- Container module: `src/container.rs`
- Tmux module: `src/tmux.rs`
- Config module: `src/config.rs`
