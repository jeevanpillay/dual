# Rust Production Readiness Implementation Plan

## Overview

Address three production readiness gaps identified in the [audit](../research/2026-02-15-rust-production-readiness-audit.md): error handling boilerplate, state file safety, and structured logging. These are independent improvements that can land as separate PRs.

## Current State Analysis

**Error handling**: 6 hand-rolled error enums across modules, each with manual `Display` and `Error` trait impls. ~150 lines of boilerplate. The pattern is correct but verbose. 5 `unwrap()` calls in `proxy.rs` on `Response::builder()` that are technically safe but look alarming in code review.

**State persistence**: `state.rs:169` uses `std::fs::write()` which truncates-then-writes. If the process is killed mid-write, the state file is corrupted. No backup, no file locking.

**Logging**: Only `println!`/`eprintln!` throughout. No debug/trace output for troubleshooting.

### Key Discoveries:
- `src/config.rs:124-149` — `HintsError` with 4 variants, 28 lines of manual Display
- `src/state.rs:226-261` — `StateError` with 7 variants, 36 lines of manual Display
- `src/container.rs:200-223` — `ContainerError` with 2 variants, 24 lines
- `src/clone.rs:101-130` — `CloneError` with 3 variants, 30 lines
- `src/tmux.rs:150-173` — `TmuxError` with 2 variants, 24 lines
- `src/shared.rs:153-170` — `SharedError` with 2 variants, 18 lines
- `src/proxy.rs:183,206,219,245,254` — 5 `unwrap()` calls on `Response::builder()`
- `src/state.rs:160-171` — `save()` uses non-atomic `std::fs::write()`

## Desired End State

- All error enums use `thiserror` derive macros, eliminating manual `Display`/`Error` impls
- Proxy response building uses a helper function with a documented `expect()` instead of scattered `unwrap()` calls
- State file writes are atomic (write-to-temp-then-rename) with a `.bak` backup
- Advisory file locking prevents concurrent state corruption
- `tracing` provides structured logging with `DUAL_LOG` env var control
- All existing tests continue to pass
- `cargo clippy -- -D warnings` passes

### How to verify:
```bash
cargo test               # All 108 tests pass
cargo clippy -- -D warnings  # No warnings
cargo fmt --check        # No formatting issues
```

## What We're NOT Doing

- **Not changing the exit-code pattern in main.rs** — returning `i32` from `cmd_*` functions is appropriate for a CLI tool
- **Not adopting `anyhow`** — the typed error enums are more appropriate for a library crate
- **Not adding metrics/telemetry** — this is a local dev tool, not a server
- **Not adding a daemon or hot-reload** — the proxy's immutable-state-at-startup pattern is fine
- **Not changing the shell-out-over-SDK pattern** — this is a deliberate design choice

## Implementation Approach

Three independent phases, each landing as its own PR. Phase 1 is mechanical refactoring with no behavior change. Phase 2 adds safety to the most critical data path. Phase 3 adds observability.

---

## Phase 1: Error Handling Cleanup

### Overview
Replace hand-rolled error enums with `thiserror` derive macros. Extract a helper for proxy response building. Net reduction of ~100 lines of boilerplate with identical behavior.

### Changes Required:

#### 1. Add `thiserror` dependency
**File**: `Cargo.toml`
**Changes**: Add `thiserror` to dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
dirs = "6"
http-body-util = "0.1"
hyper = { version = "1", features = ["http1", "server", "client"] }
hyper-util = { version = "0.1", features = ["tokio", "http1"] }
serde = { version = "1", features = ["derive"] }
thiserror = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "net"] }
toml = "0.8"
```

#### 2. Convert `HintsError`
**File**: `src/config.rs`
**Changes**: Replace lines 124-149 with thiserror derive

```rust
#[derive(Debug, thiserror::Error)]
pub enum HintsError {
    #[error("Failed to read {}: {1}", .0.display())]
    ReadError(PathBuf, #[source] std::io::Error),

    #[error("Failed to write {}: {1}", .0.display())]
    WriteError(PathBuf, #[source] std::io::Error),

    #[error("Failed to parse {}: {1}", .0.display())]
    ParseError(PathBuf, #[source] toml::de::Error),

    #[error("Failed to serialize hints: {0}")]
    SerializeError(#[source] toml::ser::Error),
}
```

Removes: manual `Display` impl (lines 132-147) and `Error` impl (line 149).

#### 3. Convert `StateError`
**File**: `src/state.rs`
**Changes**: Replace lines 226-261 with thiserror derive

```rust
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("Could not determine home directory")]
    NoHomeDir,

    #[error("Failed to read {}: {1}", .0.display())]
    ReadError(PathBuf, #[source] std::io::Error),

    #[error("Failed to write {}: {1}", .0.display())]
    WriteError(PathBuf, #[source] std::io::Error),

    #[error("Failed to parse {}: {1}", .0.display())]
    ParseError(PathBuf, #[source] toml::de::Error),

    #[error("Failed to serialize state: {0}")]
    SerializeError(#[source] toml::ser::Error),

    #[error("Invalid state: {0}")]
    Validation(String),

    #[error("Workspace {0}/{1} already exists")]
    DuplicateWorkspace(String, String),
}
```

#### 4. Convert `ContainerError`
**File**: `src/container.rs`
**Changes**: Replace lines 200-223 with thiserror derive

```rust
#[derive(Debug, thiserror::Error)]
pub enum ContainerError {
    #[error("docker not found: {0}")]
    DockerNotFound(String),

    #[error("docker {operation} failed for {name}: {stderr}")]
    Failed {
        operation: String,
        name: String,
        stderr: String,
    },
}
```

#### 5. Convert `CloneError`
**File**: `src/clone.rs`
**Changes**: Replace lines 101-130 with thiserror derive

```rust
#[derive(Debug, thiserror::Error)]
pub enum CloneError {
    #[error("git not found: {0}")]
    GitNotFound(String),

    #[error("git clone failed for {repo}/{branch}: {stderr}")]
    GitFailed {
        repo: String,
        branch: String,
        stderr: String,
    },

    #[error("filesystem error at {}: {1}", .0.display())]
    Filesystem(PathBuf, #[source] std::io::Error),
}
```

#### 6. Convert `TmuxError`
**File**: `src/tmux.rs`
**Changes**: Replace lines 150-173 with thiserror derive

```rust
#[derive(Debug, thiserror::Error)]
pub enum TmuxError {
    #[error("tmux not found: {0}")]
    NotFound(String),

    #[error("tmux {operation} failed for {session}: {stderr}")]
    Failed {
        operation: String,
        session: String,
        stderr: String,
    },
}
```

#### 7. Convert `SharedError`
**File**: `src/shared.rs`
**Changes**: Replace lines 153-170 with thiserror derive

```rust
#[derive(Debug, thiserror::Error)]
pub enum SharedError {
    #[error("Could not determine home directory")]
    NoHomeDir,

    #[error("Filesystem error at {}: {1}", .0.display())]
    Filesystem(PathBuf, #[source] std::io::Error),
}
```

#### 8. Extract proxy response helper
**File**: `src/proxy.rs`
**Changes**: Add a helper function and replace 5 `unwrap()` sites

Add after the imports (after line 15):

```rust
/// Build a 502 Bad Gateway response with a text body.
fn bad_gateway(body: impl Into<String>) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::BAD_GATEWAY)
        .body(Full::new(Bytes::from(body.into())))
        .expect("valid status code always produces valid response")
}
```

Replace each `Response::builder()...unwrap()` call:
- Line 180-183: `return Ok(bad_gateway(body));`
- Line 203-206: `return Ok(bad_gateway(body));`
- Line 216-219: `return Ok(bad_gateway(body));`
- Line 242-245: `Ok(bad_gateway(body))`
- Line 251-254: `Ok(bad_gateway(body))`

### Success Criteria:

#### Automated Verification:
- [x] All tests pass: `cargo test`
- [x] No clippy warnings: `cargo clippy -- -D warnings`
- [x] Formatting clean: `cargo fmt --check`
- [x] Error messages unchanged (verify with `cargo test` — existing tests cover error Display output)

#### Manual Verification:
- [ ] `dual add` with invalid path shows same error message as before
- [ ] `dual launch nonexistent` shows same error message as before

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to the next phase.

---

## Phase 2: State File Safety

### Overview
Make state file writes atomic to prevent corruption, keep a backup for recovery, and add advisory file locking to prevent concurrent access conflicts.

### Changes Required:

#### 1. Add `fs2` dependency
**File**: `Cargo.toml`
**Changes**: Add `fs2` for cross-platform advisory file locking

```toml
fs2 = "0.4"
```

#### 2. Implement atomic save with backup and locking
**File**: `src/state.rs`
**Changes**: Rewrite `save()` and `save_to()` functions (lines 160-183)

```rust
use fs2::FileExt;
use std::fs::{self, File};
use std::io::Write;

/// Save state to default location. Uses atomic write with backup.
pub fn save(state: &WorkspaceState) -> Result<(), StateError> {
    let path = state_path().ok_or(StateError::NoHomeDir)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| StateError::WriteError(parent.to_path_buf(), e))?;
    }

    atomic_save(state, &path)
}

/// Save state to a specific path. Uses atomic write with backup.
pub fn save_to(state: &WorkspaceState, path: &Path) -> Result<(), StateError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| StateError::WriteError(parent.to_path_buf(), e))?;
    }

    atomic_save(state, path)
}

/// Atomic save: serialize → write to temp file → backup existing → rename.
///
/// Uses advisory file locking to prevent concurrent writes.
/// The lock is held on the target file (or a lockfile) for the
/// duration of the write-backup-rename sequence.
fn atomic_save(state: &WorkspaceState, path: &Path) -> Result<(), StateError> {
    let contents = toml::to_string_pretty(state).map_err(StateError::SerializeError)?;

    let parent = path.parent().unwrap_or(Path::new("."));
    let lock_path = path.with_extension("lock");

    // Acquire advisory lock
    let lock_file = File::create(&lock_path)
        .map_err(|e| StateError::WriteError(lock_path.clone(), e))?;
    lock_file
        .lock_exclusive()
        .map_err(|e| StateError::WriteError(lock_path.clone(), e))?;

    // Write to temp file in the same directory (same filesystem for rename)
    let tmp_path = path.with_extension("tmp");
    let mut tmp_file = File::create(&tmp_path)
        .map_err(|e| StateError::WriteError(tmp_path.clone(), e))?;
    tmp_file
        .write_all(contents.as_bytes())
        .map_err(|e| StateError::WriteError(tmp_path.clone(), e))?;
    tmp_file
        .sync_all()
        .map_err(|e| StateError::WriteError(tmp_path.clone(), e))?;

    // Backup existing file (best-effort — don't fail if original doesn't exist)
    let bak_path = path.with_extension("toml.bak");
    if path.exists() {
        let _ = fs::copy(path, &bak_path);
    }

    // Atomic rename (POSIX guarantees this is atomic on same filesystem)
    fs::rename(&tmp_path, path)
        .map_err(|e| StateError::WriteError(path.to_path_buf(), e))?;

    // Release lock (dropped with file, but explicit for clarity)
    let _ = lock_file.unlock();
    let _ = fs::remove_file(&lock_path);

    Ok(())
}
```

#### 3. Add `StateError` variant for lock failure (optional)
**File**: `src/state.rs`
**Changes**: The `WriteError` variant already covers lock file I/O errors. No new variant needed — lock failures are write errors on the lock path.

#### 4. Update `save_to()` in tests
**File**: `tests/` and `src/state.rs` tests
**Changes**: No changes needed — `save_to()` signature is unchanged. Tests that use temp directories will now also create `.tmp` and `.lock` files, which are cleaned up.

### Success Criteria:

#### Automated Verification:
- [x] All tests pass: `cargo test`
- [x] No clippy warnings: `cargo clippy -- -D warnings`
- [x] Formatting clean: `cargo fmt --check`

#### Manual Verification:
- [ ] Run `dual add` in a real workspace — verify `~/.dual/workspaces.toml` is written correctly
- [ ] Verify `~/.dual/workspaces.toml.bak` exists after a second save
- [ ] Kill `dual add` mid-execution — verify state file is not corrupted
- [ ] No stale `.lock` or `.tmp` files left after normal operation

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to the next phase.

---

## Phase 3: Structured Logging

### Overview
Replace `println!`/`eprintln!` with `tracing` macros. User-facing output uses `info!`/`warn!`/`error!` events. Debug-level output added at key decision points. Controlled via `DUAL_LOG` env var (default: `info`).

### Changes Required:

#### 1. Add `tracing` dependencies
**File**: `Cargo.toml`
**Changes**: Add tracing and tracing-subscriber

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

#### 2. Initialize tracing in main
**File**: `src/main.rs`
**Changes**: Add subscriber initialization at the top of `main()`

```rust
use tracing::{debug, error, info, warn};

fn main() {
    // Initialize tracing with DUAL_LOG env var (default: info)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("DUAL_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .without_time()
        .with_target(false)
        .init();

    let cli = Cli::parse();
    // ... rest unchanged
```

The `.without_time()` and `.with_target(false)` keep the output clean for a CLI tool — output looks like:
```
INFO  launching workspace myapp/main
WARN  shared init failed: ...
ERROR container start failed: ...
```

#### 3. Replace `println!` with `info!` in main.rs
**File**: `src/main.rs`
**Changes**: Replace user-facing status messages. Examples:

- `println!("Added workspace...")` → `info!("Added workspace...")`
- `println!("Launching...")` → `info!("Launching...")`
- `println!("Destroyed...")` → `info!("Destroyed...")`

#### 4. Replace `eprintln!("error: ...")` with `error!()` in main.rs
**File**: `src/main.rs`
**Changes**: Replace error messages. Examples:

- `eprintln!("error: {e}")` → `error!("{e}")`
- `eprintln!("error: unknown workspace '{}'")` → `error!("unknown workspace '{workspace}'")`

#### 5. Replace `eprintln!("warning: ...")` with `warn!()` in main.rs
**File**: `src/main.rs`
**Changes**: Replace warning messages. Examples:

- `eprintln!("warning: shared init failed: {e}")` → `warn!("shared init failed: {e}")`
- `eprintln!("warning: container stop failed: {e}")` → `warn!("container stop failed: {e}")`

#### 6. Replace `eprintln!` in proxy.rs with `warn!`/`error!`
**File**: `src/proxy.rs`
**Changes**: Replace the 3 eprintln calls:

- `eprintln!("accept error on port {}: {e}")` → `warn!(port, "accept error: {e}")`
- `eprintln!("connection error: {e}")` → (leave filtered silently as current behavior)
- `eprintln!("backend connection error: {e}")` → `warn!("backend connection error: {e}")`

#### 7. Add `debug!` at key decision points
**File**: `src/main.rs` and module files
**Changes**: Add debug-level traces at key points (only visible with `DUAL_LOG=debug`):

```rust
// In cmd_launch, after resolving workspace:
debug!(repo = %entry.repo, branch = %entry.branch, "resolved workspace");

// In container::create, before docker command:
debug!(name, image, "creating container");

// In proxy handle_request, on each request:
debug!(host, port, "routing request");

// In state::load, after successful load:
debug!(count = state.workspaces.len(), "loaded state");
```

### Success Criteria:

#### Automated Verification:
- [x] All tests pass: `cargo test`
- [x] No clippy warnings: `cargo clippy -- -D warnings`
- [x] Formatting clean: `cargo fmt --check`
- [x] Binary size increase is reasonable (tracing adds ~200KB)

#### Manual Verification:
- [ ] `dual list` shows workspace list without log prefixes cluttering output (info level should look clean)
- [ ] `DUAL_LOG=debug dual launch myapp` shows debug-level decision points
- [ ] `DUAL_LOG=warn dual list` suppresses info-level status messages
- [ ] Error messages still display correctly with the tracing format
- [ ] Proxy shows connection info at debug level

**Implementation Note**: The tracing format needs careful tuning to keep CLI output clean. The `without_time()` and `with_target(false)` options help, but the `INFO`/`WARN`/`ERROR` prefixes will be new. If the user prefers completely clean output at info level, consider using a custom formatter that omits the level prefix for `info!` events and only shows prefixes for `warn!`/`error!`.

---

## Testing Strategy

### Unit Tests:
- Error Display output tested by existing tests — thiserror must produce identical strings
- State save/load roundtrip tests cover the atomic write path
- No new unit tests needed for Phase 1 (behavior-preserving refactor)

### Integration Tests:
- E2E tests (`tests/e2e.rs`) exercise the full add/launch/destroy flow including state persistence
- These tests will implicitly validate the atomic save and tracing initialization

### Manual Testing Steps:
1. Run `dual add` in a git repo — verify workspace is added and state file exists
2. Run `dual launch <workspace>` — verify container starts and tmux session opens
3. Run `dual destroy <workspace>` — verify clean teardown
4. Check `~/.dual/workspaces.toml.bak` exists after multiple operations
5. Run with `DUAL_LOG=debug` to see debug output
6. Run with `DUAL_LOG=error` to suppress all info/warn output

## Performance Considerations

- `thiserror` is a proc-macro crate — adds compile time, zero runtime cost
- `fs2` advisory locking adds one syscall per state save (negligible)
- `tracing` adds ~200KB to binary size; runtime overhead is negligible for a CLI tool
- Atomic save adds one extra file write + rename vs. direct write (negligible)

## Dependencies Added

| Crate | Version | Purpose | Maintenance |
|-------|---------|---------|-------------|
| `thiserror` | 2 | Error derive macros | dtolnay, widely used |
| `fs2` | 0.4 | Cross-platform file locking | Stable, mature |
| `tracing` | 0.1 | Structured logging framework | tokio-rs, ecosystem standard |
| `tracing-subscriber` | 0.3 | Log output formatting | tokio-rs, ecosystem standard |

Total new dependencies: 4 crates (all ecosystem staples).

## References

- Audit: `thoughts/shared/research/2026-02-15-rust-production-readiness-audit.md`
- Architecture: `thoughts/ARCHITECTURE.md`
- State module: `src/state.rs`
- Proxy module: `src/proxy.rs`
- Error enums: `src/config.rs:124`, `src/state.rs:226`, `src/container.rs:200`, `src/clone.rs:101`, `src/tmux.rs:150`, `src/shared.rs:153`
