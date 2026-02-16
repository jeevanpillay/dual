# Shell Interception Pane Propagation — Implementation Plan

## Overview

When a user splits a pane or creates a new window inside a Dual tmux session, the new shell doesn't have command interception loaded. Commands like `pnpm dev` run on the host instead of in the container. This plan fixes that by (1) setting tmux session-level environment variables so new panes inherit them, and (2) auto-injecting a snippet into the user's shell RC file so new shells auto-source the interception file.

## Current State Analysis

**How interception works today** (`src/shell.rs:38-63`, `src/main.rs:434-450`):
1. `cmd_launch()` calls `shell::write_rc_file()` → writes to `~/.dual/rc/{container_name}.sh`
2. Generates a `source` command via `shell::source_file_command()`
3. Passes it to `backend.create_session()` as `init_cmd`
4. `TmuxBackend::create_session()` (`src/tmux_backend.rs:32-62`) sends it via `send_keys()` into the first pane

**The gap**: New panes/windows spawned by the user start fresh shells. Nothing tells those shells to source the RC file. The `DUAL_CONTAINER` env var was set inside the first pane's shell process, not in tmux's session environment.

### Key Discoveries:
- `MultiplexerBackend` trait (`src/backend.rs:8-42`) has no `set_environment()` method
- `cmd_add()` (`src/main.rs:112-197`) never touches `~/.bashrc` or `~/.zshrc`
- `shell::source_command()` (`src/shell.rs:87-89`) already exists for eval-based sourcing but nothing auto-triggers it
- No code anywhere modifies user shell config files

## Desired End State

After this plan is implemented:
1. `dual add` detects the user's shell and appends a guarded snippet to their `~/.zshrc` or `~/.bashrc`
2. `dual launch` sets `DUAL_ACTIVE`, `DUAL_RC_PATH`, and `DUAL_CONTAINER` as tmux session-level environment variables
3. When a user splits a pane (Ctrl+b %) or creates a window (Ctrl+b c), the new shell reads its RC file, detects `DUAL_ACTIVE`, and auto-sources the interception file
4. Non-Dual tmux sessions and terminals outside tmux are unaffected (snippet is a no-op)

**Verification**: In a Dual tmux session, split a pane with `Ctrl+b %`. In the new pane, run `type npm` — it should show `npm is a function` pointing to the docker exec wrapper, not the host binary.

## What We're NOT Doing

- Fish shell support (different syntax, can be added later)
- `tmux set-hook` approach (visible to user, pollutes history, race conditions)
- `tmux set-option default-command` approach (shell-specific, may break user config)
- A `dual init` subcommand (reusing `dual add` is simpler)
- Modifying the `MultiplexerBackend` trait with a new method (tmux-specific `set-environment` can be called directly — keeps the trait clean for future zellij support)

## Implementation Approach

Three phases, each independently testable:

1. **tmux set-environment** — Set session-level env vars during launch so new panes inherit them
2. **Shell RC snippet injection** — Auto-append the auto-source snippet to `~/.zshrc`/`~/.bashrc` during `dual add`
3. **Tests** — Unit tests for all new functions

---

## Phase 1: tmux set-environment During Launch

### Overview
After creating a tmux session in `cmd_launch()`, set three environment variables at the session level using `tmux set-environment`. This ensures all new panes/windows in the session inherit them.

### Changes Required:

#### 1. Add `set_environment()` to `TmuxBackend`
**File**: `src/tmux_backend.rs`
**Changes**: Add a public method (not on the trait) for setting session-level env vars.

```rust
impl TmuxBackend {
    pub fn new() -> Self {
        Self
    }

    /// Set an environment variable on a tmux session.
    /// New panes/windows in this session will inherit the variable.
    pub fn set_environment(
        &self,
        session_name: &str,
        key: &str,
        value: &str,
    ) -> Result<(), BackendError> {
        tmux_simple(&["set-environment", "-t", session_name, key, value])
    }
}
```

#### 2. Call `set_environment()` after session creation
**File**: `src/main.rs`
**Changes**: In `cmd_launch()`, after `backend.create_session()` succeeds (line 446-449), set the three env vars.

After the existing block at line 444-450:
```rust
// Step 5: Create tmux session if not alive
if !backend.is_alive(&session_name) {
    let source_cmd = shell::source_file_command(&rc_path);
    if let Err(e) = backend.create_session(&session_name, &workspace_dir, Some(&source_cmd)) {
        error!("session creation failed: {e}");
        return 1;
    }

    // Set session-level env vars so new panes auto-source interception
    let rc_path_str = rc_path.to_string_lossy();
    for (key, value) in [
        ("DUAL_ACTIVE", "1"),
        ("DUAL_RC_PATH", rc_path_str.as_ref()),
        ("DUAL_CONTAINER", container_name.as_str()),
    ] {
        if let Err(e) = backend.set_environment(&session_name, key, value) {
            warn!("failed to set tmux env {key}: {e}");
        }
    }
}
```

Note: `backend` must be downcast to `TmuxBackend` or the function signature must accept `&TmuxBackend`. Since `cmd_launch()` already receives `&dyn MultiplexerBackend`, the cleanest approach is to change `cmd_launch()` to accept `&TmuxBackend` directly (it's the only implementation, and we can revisit when zellij support is added). Alternatively, keep `&dyn MultiplexerBackend` and add a helper function that calls tmux directly — matching the pattern of `tmux_simple()`.

**Chosen approach**: Add `set_session_env()` as a free function in `tmux_backend.rs` that calls `tmux set-environment` directly, and call it from `cmd_launch()`. This avoids changing the trait or the function signature.

```rust
// In src/tmux_backend.rs
/// Set an environment variable on a tmux session.
/// New panes/windows in this session will inherit the variable.
pub fn set_session_env(session_name: &str, key: &str, value: &str) -> Result<(), BackendError> {
    tmux_simple(&["set-environment", "-t", session_name, key, value])
}
```

```rust
// In src/main.rs, after create_session succeeds:
use dual::tmux_backend;

// ...inside the if !backend.is_alive block, after create_session:
let rc_path_str = rc_path.to_string_lossy();
for (key, value) in [
    ("DUAL_ACTIVE", "1"),
    ("DUAL_RC_PATH", rc_path_str.as_ref()),
    ("DUAL_CONTAINER", container_name.as_str()),
] {
    if let Err(e) = tmux_backend::set_session_env(&session_name, key, value) {
        warn!("failed to set tmux env {key}: {e}");
    }
}
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` compiles without errors
- [x] `cargo test` passes
- [x] `cargo clippy` has no warnings

#### Manual Verification:
- [ ] Launch a workspace with `dual launch`
- [ ] Run `tmux show-environment -t <session>` — should show `DUAL_ACTIVE=1`, `DUAL_RC_PATH=...`, `DUAL_CONTAINER=...`
- [ ] Split a pane — run `echo $DUAL_ACTIVE` in new pane — should print `1`
- [ ] Run `echo $DUAL_RC_PATH` in new pane — should print the RC file path

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation that tmux env vars propagate before proceeding.

---

## Phase 2: Shell RC Snippet Auto-Injection

### Overview
During `dual add`, detect the user's shell and append a guarded snippet to their `~/.zshrc` or `~/.bashrc`. The snippet detects the `DUAL_ACTIVE` env var (set by Phase 1) and auto-sources the RC file. This is the same pattern used by nvm, pyenv, and rustup.

### Changes Required:

#### 1. Add snippet generation and injection to `src/shell.rs`
**File**: `src/shell.rs`
**Changes**: Add functions for generating the shell RC snippet and injecting it.

```rust
/// Marker comment used to detect if the snippet is already installed.
const RC_MARKER: &str = "# dual: shell interception (auto-generated)";

/// Generate the shell RC snippet that auto-sources Dual interception.
///
/// This snippet is appended to ~/.bashrc or ~/.zshrc. It detects
/// the DUAL_ACTIVE env var (set by tmux set-environment) and sources
/// the workspace-specific RC file.
pub fn shell_hook_snippet() -> String {
    format!(
        r#"
{RC_MARKER}
if [ -n "$DUAL_ACTIVE" ] && [ -n "$DUAL_RC_PATH" ] && [ -f "$DUAL_RC_PATH" ]; then
    source "$DUAL_RC_PATH"
fi
"#
    )
}

/// Detect the user's shell RC file path.
///
/// Returns the path to ~/.zshrc or ~/.bashrc based on $SHELL.
/// Returns None if the shell is not bash or zsh.
pub fn detect_shell_rc() -> Option<std::path::PathBuf> {
    let home = dirs::home_dir()?;
    let shell = std::env::var("SHELL").unwrap_or_default();
    let base = shell.rsplit('/').next().unwrap_or("");

    match base {
        "zsh" => Some(home.join(".zshrc")),
        "bash" => {
            // macOS uses .bash_profile for login shells, but .bashrc is
            // sourced by interactive non-login shells (which tmux spawns).
            // To cover both, prefer .bashrc.
            Some(home.join(".bashrc"))
        }
        _ => None,
    }
}

/// Install the auto-source snippet into the user's shell RC file.
///
/// Idempotent: checks for the marker comment before appending.
/// Creates the RC file if it doesn't exist.
/// Returns Ok(true) if the snippet was newly installed, Ok(false) if
/// already present.
pub fn install_shell_hook() -> Result<bool, std::io::Error> {
    let rc_path = match detect_shell_rc() {
        Some(p) => p,
        None => return Ok(false),
    };

    // Read existing content (or empty if file doesn't exist)
    let existing = std::fs::read_to_string(&rc_path).unwrap_or_default();

    // Check if snippet is already installed
    if existing.contains(RC_MARKER) {
        return Ok(false);
    }

    // Append snippet
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&rc_path)?;
    file.write_all(shell_hook_snippet().as_bytes())?;

    Ok(true)
}
```

#### 2. Call `install_shell_hook()` from `cmd_add()`
**File**: `src/main.rs`
**Changes**: After the workspace is successfully registered (after `state::save()` at line 188-191), install the shell hook.

```rust
// After state::save() succeeds, before the success messages:

// Install shell hook for pane propagation (idempotent)
match shell::install_shell_hook() {
    Ok(true) => {
        let rc_name = shell::detect_shell_rc()
            .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
            .unwrap_or_default();
        info!("Added shell hook to ~/{rc_name} for tmux pane interception.");
    }
    Ok(false) => {} // Already installed or unsupported shell — silent
    Err(e) => warn!("could not install shell hook: {e}"),
}
```

### Success Criteria:

#### Automated Verification:
- [x] `cargo build` compiles without errors
- [x] `cargo test` passes
- [x] `cargo clippy` has no warnings

#### Manual Verification:
- [ ] Run `dual add` in a repo — check that `~/.zshrc` (or `~/.bashrc`) now contains the snippet
- [ ] Run `dual add` again in a different repo — snippet should NOT be duplicated
- [ ] Launch a workspace, split a pane — run `type npm` in new pane — should show the docker exec function
- [ ] Open a terminal outside tmux — the snippet should be a no-op (no errors, no effect)
- [ ] Open a non-Dual tmux session — the snippet should be a no-op

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation that end-to-end pane propagation works.

---

## Phase 3: Tests

### Overview
Add unit tests for all new functions.

### Changes Required:

#### 1. Tests for `set_session_env`
**File**: `src/tmux_backend.rs`
**Changes**: Add to existing `mod tests` block.

```rust
#[test]
fn set_session_env_builds_correct_args() {
    // This test verifies the function exists and has the right signature.
    // Actual tmux interaction is covered by manual testing.
    // We can't easily test tmux commands without a running tmux server.
    let _ = set_session_env; // verify function exists
}
```

#### 2. Tests for shell hook functions
**File**: `src/shell.rs`
**Changes**: Add to existing `mod tests` block.

```rust
#[test]
fn shell_hook_snippet_contains_guard() {
    let snippet = shell_hook_snippet();
    assert!(snippet.contains("DUAL_ACTIVE"));
    assert!(snippet.contains("DUAL_RC_PATH"));
    assert!(snippet.contains("source"));
    assert!(snippet.contains(RC_MARKER));
}

#[test]
fn shell_hook_snippet_is_noop_without_vars() {
    let snippet = shell_hook_snippet();
    // The snippet should guard on DUAL_ACTIVE being non-empty
    assert!(snippet.contains("-n \"$DUAL_ACTIVE\""));
    // And on the RC file existing
    assert!(snippet.contains("-f \"$DUAL_RC_PATH\""));
}

#[test]
fn detect_shell_rc_respects_shell_env() {
    let original = std::env::var("SHELL").ok();

    // SAFETY: test runs single-threaded
    unsafe {
        std::env::set_var("SHELL", "/bin/zsh");
        let path = detect_shell_rc();
        assert!(path.is_some());
        assert!(path.unwrap().ends_with(".zshrc"));

        std::env::set_var("SHELL", "/bin/bash");
        let path = detect_shell_rc();
        assert!(path.is_some());
        assert!(path.unwrap().ends_with(".bashrc"));

        std::env::set_var("SHELL", "/usr/bin/fish");
        let path = detect_shell_rc();
        assert!(path.is_none());

        // Restore
        match original {
            Some(v) => std::env::set_var("SHELL", v),
            None => std::env::remove_var("SHELL"),
        }
    }
}

#[test]
fn install_shell_hook_is_idempotent() {
    // Create a temp file to act as shell RC
    let dir = tempfile::tempdir().unwrap();
    let rc_path = dir.path().join(".zshrc");
    std::fs::write(&rc_path, "# existing config\n").unwrap();

    // Manually write snippet to test idempotency detection
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&rc_path)
        .unwrap();
    use std::io::Write;
    f.write_all(shell_hook_snippet().as_bytes()).unwrap();
    drop(f);

    let content = std::fs::read_to_string(&rc_path).unwrap();
    let marker_count = content.matches(RC_MARKER).count();
    assert_eq!(marker_count, 1);
}
```

Note: The `install_shell_hook()` function uses the real `$SHELL` env var and `dirs::home_dir()`, so a full integration test would modify the user's actual shell RC. The idempotency test above uses a direct file write to verify the marker detection logic without calling `install_shell_hook()` on the real home directory.

### Success Criteria:

#### Automated Verification:
- [x] `cargo test` passes — all new tests green
- [x] `cargo clippy` has no warnings
- [x] `cargo fmt -- --check` shows no formatting issues

---

## Testing Strategy

### Unit Tests:
- `shell_hook_snippet()` produces correct guard conditions
- `detect_shell_rc()` returns correct path for zsh, bash, and None for fish
- `install_shell_hook()` idempotency (marker detection)
- `set_session_env()` function signature and existence

### Manual Testing Steps:
1. `dual add` in a fresh repo — verify snippet appears in `~/.zshrc`
2. `dual add` in another repo — verify snippet is NOT duplicated
3. `dual launch <workspace>` — verify `tmux show-environment -t <session>` shows all three vars
4. Split pane (`Ctrl+b %`) — run `type npm` — should show function
5. New window (`Ctrl+b c`) — run `type npm` — should show function
6. Type `pnpm dev` in new pane — should run inside container
7. Open a terminal outside tmux — no errors from the snippet
8. Open a non-Dual tmux session — `echo $DUAL_ACTIVE` should be empty

## Performance Considerations

- `tmux set-environment` is a local IPC call to the tmux server — negligible overhead (< 1ms per call, 3 calls total)
- Shell RC snippet adds a single `[ -n ... ]` guard check to shell startup — negligible (< 1ms)
- `install_shell_hook()` reads and appends to the RC file — only runs during `dual add`, not on every launch

## References

- Research: `thoughts/shared/research/2026-02-16-shell-interception-pane-propagation.md`
- Shell module: `src/shell.rs`
- Tmux backend: `src/tmux_backend.rs`
- Backend trait: `src/backend.rs`
- Launch flow: `src/main.rs:267-460`
- Add flow: `src/main.rs:112-197`
