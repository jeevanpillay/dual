---
date: 2026-02-16T08:11:46+08:00
researcher: jeevan
git_commit: ebfe191
branch: main
repository: dual
topic: "Shell interception pane propagation: how command routing breaks on new tmux panes/windows"
tags: [research, codebase, shell-interception, tmux, pane-propagation, command-routing]
status: complete
last_updated: 2026-02-16
last_updated_by: jeevan
---

# Research: Shell Interception Pane Propagation

**Date**: 2026-02-16T08:11:46+08:00
**Researcher**: jeevan
**Git Commit**: ebfe191
**Branch**: main
**Repository**: dual

## Research Question

In a Dual tmux session, only the auto-created first pane has shell interception active. When users split panes or create new windows, the new shells don't have the command routing functions loaded. How does the current mechanism work and what are the options for propagating interception to all panes?

## Summary

The current shell interception mechanism writes an RC file to `~/.dual/rc/{container_name}.sh` and sources it via `tmux send-keys` into the first pane only. New panes/windows created by the user (Ctrl+b %, Ctrl+b ", Ctrl+b c) start fresh shells without the interception functions. The fix is a two-part approach: (1) `tmux set-environment` to set session-level env vars that new panes inherit, and (2) a snippet in the user's shell RC (`~/.bashrc`/`~/.zshrc`) that detects these vars and auto-sources the interception file.

## Detailed Findings

### 1. Current Mechanism — How Interception Gets Loaded Today

**RC file generation** (`src/shell.rs:38-63`):

`generate_rc()` produces a shell script containing:
- `export DUAL_CONTAINER="{container_name}"` — sets an env var identifying the container
- One shell function per intercepted command (npm, pnpm, node, etc.)
- Each function wraps `docker exec -w /workspace {container_name} {command} "$@"`

**RC file persistence** (`src/shell.rs:93-108`):

`write_rc_file()` writes the generated content to `~/.dual/rc/{container_name}.sh`.

**Initial sourcing** (`src/main.rs:434-450`):

During `cmd_launch()`, after writing the RC file:
```rust
let rc_path = shell::write_rc_file(&container_name, &hints.extra_commands)?;
// ...
let source_cmd = shell::source_file_command(&rc_path);
backend.create_session(&session_name, &workspace_dir, Some(&source_cmd))?;
```

**Session creation with init command** (`src/tmux_backend.rs:32-62`):

`create_session()` runs `tmux new-session -d -s {name} -c {cwd}`, then sends the source command via `send_keys()`:
```rust
if let Some(cmd) = init_cmd {
    self.send_keys(session_name, cmd)?;
}
```

This executes `source "~/.dual/rc/dual-repo-branch.sh"` as keystrokes typed into the first pane.

### 2. The Gap — Why New Panes Don't Have Interception

When a user creates a new pane or window in the tmux session:
- tmux spawns a fresh shell process (the user's `$SHELL`)
- The shell reads its standard RC files (`~/.bashrc`, `~/.zshrc`)
- **Nothing** in those RC files knows about Dual's interception
- The `DUAL_CONTAINER` env var was set in the first pane's shell process, not in tmux's session environment
- Result: `pnpm dev` in the new pane runs on the **host**, not in the container

### 3. What Already Exists — `source_command()` and `shell-rc` Subcommand

`src/shell.rs:87-89` has a function for eval-based sourcing:
```rust
pub fn source_command(container_name: &str) -> String {
    format!("eval \"$(dual shell-rc {container_name})\"")
}
```

`src/main.rs:676-679` implements the `shell-rc` subcommand:
```rust
fn cmd_shell_rc(container_name: &str) -> i32 {
    print!("{}", shell::generate_rc(container_name, &[]));
    0
}
```

These provide a mechanism for users to manually load interception, but nothing auto-triggers them in new panes.

### 4. tmux Mechanisms Available for Propagation

#### 4a. `tmux set-environment` (session-level env vars)

Sets environment variables at the tmux session level. All new panes/windows in that session inherit these variables automatically.

```bash
tmux set-environment -t dual-repo-branch DUAL_ACTIVE 1
tmux set-environment -t dual-repo-branch DUAL_RC_PATH ~/.dual/rc/dual-repo-branch.sh
tmux set-environment -t dual-repo-branch DUAL_CONTAINER dual-repo-branch
```

- Session-scoped (`-t session-name`) — doesn't affect other tmux sessions
- Inherited by all child processes (new panes, new windows)
- Survives pane creation, window creation, shell restarts within panes

#### 4b. `tmux set-hook` (event hooks)

tmux supports hooks for pane/window creation events:

```bash
tmux set-hook -t dual-repo-branch after-split-window \
  "send-keys -t '#{pane_id}' 'source ~/.dual/rc/dual-repo-branch.sh' Enter"

tmux set-hook -t dual-repo-branch after-new-window \
  "send-keys -t '#{window_id}.1' 'source ~/.dual/rc/dual-repo-branch.sh' Enter"
```

- Visible to user (source command appears in terminal)
- Pollutes shell history
- Race condition if user types before source completes
- Session-scoped (`-t`) so doesn't affect other sessions

#### 4c. `tmux set-option default-command`

Sets the command to run instead of the default shell for new panes:

```bash
tmux set-option -t dual-repo-branch default-command \
  "bash --init-file ~/.dual/rc/dual-repo-branch.sh -i"
```

- Shell-specific (bash `--init-file` vs zsh `ZDOTDIR` vs fish `--init-command`)
- May interfere with user's shell configuration
- Session-scoped possible

### 5. Recommended Approach: `set-environment` + Shell RC Snippet

**Part 1 — Dual sets session-level env vars** (code change in `tmux_backend.rs`):

After `create_session()`, set environment variables on the session:
```
tmux set-environment -t {session_name} DUAL_ACTIVE 1
tmux set-environment -t {session_name} DUAL_RC_PATH {rc_path}
tmux set-environment -t {session_name} DUAL_CONTAINER {container_name}
```

**Part 2 — User adds snippet to shell RC** (one-time setup):

For `~/.bashrc`:
```bash
# Dual workspace command interception
if [ -n "$DUAL_ACTIVE" ] && [ -n "$DUAL_RC_PATH" ] && [ -f "$DUAL_RC_PATH" ]; then
    source "$DUAL_RC_PATH"
fi
```

For `~/.zshrc`:
```zsh
# Dual workspace command interception
if [[ -n "$DUAL_ACTIVE" && -n "$DUAL_RC_PATH" && -f "$DUAL_RC_PATH" ]]; then
    source "$DUAL_RC_PATH"
fi
```

**Part 3 — `dual add` auto-injects the snippet** (optional automation):

Dual could detect the user's shell and append the snippet to their RC file during `dual add`, similar to how tools like nvm, pyenv, and direnv do it.

### 6. Why This Approach Works

| Scenario | Behavior |
|----------|----------|
| First pane (initial launch) | Shell starts → reads `~/.bashrc` → detects `DUAL_ACTIVE` → sources RC → interception active |
| User splits pane (Ctrl+b %) | New shell → reads `~/.bashrc` → detects `DUAL_ACTIVE` → sources RC → interception active |
| User creates window (Ctrl+b c) | New shell → reads `~/.bashrc` → sources RC → interception active |
| User types `bash` or `zsh` | New subshell → reads RC → sources RC → interception active |
| Non-Dual tmux session | `DUAL_ACTIVE` not set → snippet is no-op → zero impact |
| Terminal outside tmux | `DUAL_ACTIVE` not set → snippet is no-op → zero impact |

### 7. What Changes in the Codebase

**`src/tmux_backend.rs`** — Add `set-environment` calls after session creation:
- `DUAL_ACTIVE=1`
- `DUAL_RC_PATH={rc_path}`
- `DUAL_CONTAINER={container_name}`

**`src/shell.rs`** — Add function to generate the shell RC snippet for user setup.

**`src/main.rs`** — During `dual add` (or a new `dual init` command), detect user's shell and offer to append the snippet to their RC file.

**The existing `send_keys` mechanism stays** — it handles the first pane on initial launch (before the user has added the snippet). Once the snippet is in place, it's redundant but harmless.

## Code References

- `src/shell.rs:38-63` — `generate_rc()` generates interception functions
- `src/shell.rs:46-47` — `export DUAL_CONTAINER` already set in RC content
- `src/shell.rs:87-89` — `source_command()` for eval-based sourcing
- `src/shell.rs:93-108` — `write_rc_file()` writes to `~/.dual/rc/`
- `src/shell.rs:111-113` — `source_file_command()` generates source command
- `src/main.rs:434-450` — Launch pipeline: write RC then create session with source command
- `src/main.rs:676-679` — `cmd_shell_rc()` prints RC content for eval
- `src/tmux_backend.rs:32-62` — `create_session()` sends init_cmd via send_keys
- `src/tmux_backend.rs:125-127` — `send_keys()` implementation
- `src/backend.rs:1-42` — `MultiplexerBackend` trait definition

## Architecture Documentation

### Current Interception Chain
```
dual launch
  → shell::write_rc_file() → ~/.dual/rc/dual-repo-branch.sh
  → backend.create_session(name, cwd, source_cmd)
    → tmux new-session -d -s {name} -c {cwd}
    → tmux send-keys -t {name} "source ~/.dual/rc/..." Enter
  → ONLY first pane has interception
```

### Proposed Interception Chain
```
dual launch
  → shell::write_rc_file() → ~/.dual/rc/dual-repo-branch.sh
  → backend.create_session(name, cwd, source_cmd)
    → tmux new-session -d -s {name} -c {cwd}
    → tmux set-environment -t {name} DUAL_ACTIVE 1
    → tmux set-environment -t {name} DUAL_RC_PATH {rc_path}
    → tmux set-environment -t {name} DUAL_CONTAINER {container_name}
    → tmux send-keys -t {name} "source ~/.dual/rc/..." Enter  (still needed for first pane)
  → ALL panes have interception (via user's shell RC snippet)
```

## Historical Context (from thoughts/)

- `thoughts/shared/research/2026-02-05-ARCH-shell-interception.md` — Original validation that shell functions intercept commands. Notes: "Functions must be loaded in each shell session (rc file injection)" — this is the exact gap now being addressed.
- `thoughts/shared/research/2026-02-05-ARCH-shell-interception-transparency.md` — Transparency analysis of what leaks vs stays transparent.
- `thoughts/shared/research/2026-02-13-BUILD-shell.md` — Shell module implementation research.
- `thoughts/shared/research/2026-02-16-port-routing-container-isolation.md` — Documents that shell interception is only active inside the Dual tmux session.

## Related Research

- `thoughts/shared/research/2026-02-15-v3-architecture-rethink.md` — Documents the full tmux integration model and edge cases.
- `thoughts/shared/plans/2026-02-15-v3-multiplexer-trait-tui.md` — v3 plan that discusses tmux hooks.

## Open Questions

1. Should `dual add` auto-inject the shell RC snippet, or should it just print instructions for the user to copy-paste?
2. Should tmux hooks (`after-split-window`, `after-new-window`) be used as a fallback for users who haven't added the snippet yet?
3. Fish shell uses different syntax — should Dual generate a fish-compatible snippet too?
