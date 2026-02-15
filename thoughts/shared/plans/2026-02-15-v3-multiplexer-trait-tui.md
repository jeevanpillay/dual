# v3.0.0 — Multiplexer Trait + TUI Implementation Plan

## Overview

Extract the tmux module behind a `MultiplexerBackend` trait, then build a ratatui-based TUI as the primary interface. `dual` (no args) launches the TUI. Selecting a workspace suspends the TUI, attaches the tmux session, and resumes the TUI when the user detaches.

## Current State Analysis

**Tmux integration** is 8 public functions in `src/tmux.rs` called directly from `src/main.rs` at 9 call sites. No abstraction layer exists.

**CLI-only interface** — all output via `tracing::info!()` with Unicode status symbols. No TUI dependencies.

### Key Discoveries:
- `src/tmux.rs` — 8 functions: `is_available`, `create_session`, `attach`, `detach`, `destroy`, `is_alive`, `list_sessions`, `send_keys`, `session_name`
- `src/main.rs` — 9 tmux call sites across `cmd_launch()`, `cmd_destroy()`, `print_workspace_status()`
- `src/cli.rs:11` — `command: Option<Command>` already handles `dual` (no args) via `cmd_default()`
- SPEC.md:269-294 — Backend trait contract with `create_session`, `attach`, `detach`, `destroy`, `is_alive`, `list_sessions`
- ratatui v0.30.0 — uses `ratatui::init()` / `ratatui::restore()` for terminal state management
- The TUI → tmux → TUI transition pattern: `restore()` → `tmux attach` (blocks) → tmux detach returns control → `init()` → resume TUI

### The Critical Design Decision: TUI ↔ Tmux Transition

**Model B: Dual manages terminal directly** (chosen)

```
Terminal
├── State A: Dual TUI (alternate screen, raw mode)
│   └── User selects workspace → ratatui::restore() → tmux attach
└── State B: tmux session (tmux controls terminal)
    └── User detaches (Ctrl+b d) → tmux returns → ratatui::init() → back to State A
```

Why Model B:
- No nested tmux (Model A's problem)
- No terminal emulator reimplementation (Model C's problem)
- Uses ratatui's built-in `init()`/`restore()` for clean state transitions
- `tmux attach-session` is a blocking call — when user detaches, it returns, and we resume

## Desired End State

After v3.0.0:
1. `dual` → launches TUI with tree view of repos/branches
2. Arrow keys/j/k to navigate, Enter to select
3. Selecting a workspace: suspends TUI → launches/attaches tmux session
4. Detaching from tmux (Ctrl+b d) → TUI resumes automatically
5. `q` in TUI → exits Dual
6. `dual list` → non-interactive fallback (existing behavior)
7. All `main.rs` code uses `MultiplexerBackend` trait instead of `tmux::*` directly

## What We're NOT Doing

- **Not implementing ZellijBackend** — trait extraction makes it possible, but only TmuxBackend ships in v3.0.0
- **Not implementing BasicBackend** — deferred to v3.1
- **Not adding session layout config** — pane/window configuration deferred to v3.1
- **Not building a full plugin system** — hooks are v3.2+
- **Not adding fuzzy search in TUI** — simple navigation first, fuzzy filter in v3.1
- **Not adding meta-key workspace switching** — requires tmux hooks or keybinding config, deferred

---

## Phase 1: Multiplexer Trait Extraction

### Overview
Extract `src/tmux.rs` functions behind a `MultiplexerBackend` trait. Replace all direct `tmux::*` calls in `main.rs` with trait method calls. The `TmuxBackend` is the only implementation. Zero behavior change — pure refactor.

### Changes Required:

#### 1. Create `src/backend.rs` — Trait Definition
**File**: `src/backend.rs` (new)

```rust
use std::path::Path;

/// Abstraction over terminal multiplexers (tmux, zellij, etc.)
pub trait MultiplexerBackend {
    /// Check if the multiplexer is installed and available.
    fn is_available(&self) -> bool;

    /// Create a new detached session with the given name and working directory.
    /// Optionally run an initial command after creation.
    fn create_session(
        &self,
        session_name: &str,
        cwd: &Path,
        init_cmd: Option<&str>,
    ) -> Result<(), BackendError>;

    /// Attach the current terminal to an existing session.
    /// If already inside the multiplexer, use switch-client instead.
    fn attach(&self, session_name: &str) -> Result<(), BackendError>;

    /// Detach the current terminal from a session.
    fn detach(&self, session_name: &str) -> Result<(), BackendError>;

    /// Destroy a session and all its windows/panes.
    fn destroy(&self, session_name: &str) -> Result<(), BackendError>;

    /// Check if a session exists and has running processes.
    fn is_alive(&self, session_name: &str) -> bool;

    /// List all sessions managed by Dual (filtered by prefix).
    fn list_sessions(&self) -> Vec<String>;

    /// Send keystrokes to a session's active pane.
    fn send_keys(&self, session_name: &str, keys: &str) -> Result<(), BackendError>;

    /// Generate the session name for a given repo/branch.
    fn session_name(&self, repo: &str, branch: &str) -> String;

    /// Check if we're currently inside this multiplexer.
    fn is_inside(&self) -> bool;
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("{multiplexer} is not installed")]
    NotFound { multiplexer: String },

    #[error("{operation} failed for session '{session}': {stderr}")]
    Failed {
        operation: String,
        session: String,
        stderr: String,
    },
}
```

#### 2. Create `src/tmux_backend.rs` — TmuxBackend Implementation
**File**: `src/tmux_backend.rs` (new)

Move all logic from `src/tmux.rs` into a `TmuxBackend` struct implementing `MultiplexerBackend`:

```rust
pub struct TmuxBackend;

impl TmuxBackend {
    pub fn new() -> Self {
        Self
    }
}

impl MultiplexerBackend for TmuxBackend {
    fn is_available(&self) -> bool {
        // existing tmux::is_available() logic
    }

    fn attach(&self, session_name: &str) -> Result<(), BackendError> {
        // Use switch-client if is_inside(), otherwise attach-session
        if self.is_inside() {
            // tmux switch-client -t {session_name}
        } else {
            // tmux attach-session -t {session_name}
        }
    }

    fn is_inside(&self) -> bool {
        std::env::var("TMUX").is_ok()
    }

    // ... rest of trait methods wrapping existing tmux.rs functions
}
```

**Key change in `attach()`**: The v2.2.0 `$TMUX` detection logic moves here as `is_inside()` + conditional `switch-client` vs `attach-session`.

#### 3. Update `src/main.rs` — Use Trait Instead of Direct Calls
**File**: `src/main.rs`

Replace all 9 tmux call sites:

| Current | New |
|---------|-----|
| `tmux::session_name(&repo, &branch)` | `backend.session_name(&repo, &branch)` |
| `tmux::is_alive(&session_name)` | `backend.is_alive(&session_name)` |
| `tmux::create_session(...)` | `backend.create_session(...)` |
| `tmux::attach(&session_name)` | `backend.attach(&session_name)` |
| `tmux::destroy(&session_name)` | `backend.destroy(&session_name)` |

The backend is created once in `main()` and passed to command handlers:

```rust
fn main() {
    let backend = TmuxBackend::new();
    // ... dispatch commands with &backend
}
```

#### 4. Update `src/lib.rs` — Export New Modules
**File**: `src/lib.rs`

```rust
pub mod backend;
pub mod tmux_backend;
// Keep pub mod tmux for backward compatibility of tests, mark deprecated
```

#### 5. Update Tests
- Move tmux-specific unit tests from `src/tmux.rs` to `src/tmux_backend.rs`
- Add trait-level tests in `src/backend.rs` (test that TmuxBackend implements the trait)
- Update `tests/e2e.rs` tmux tests to use `TmuxBackend` struct
- Keep existing test patterns (build + execute separation)

#### 6. Remove `src/tmux.rs`
After all references are migrated, delete `src/tmux.rs`.

### Success Criteria:

#### Automated Verification:
- [ ] `cargo test` — all 99+ tests pass (no behavior change)
- [ ] `cargo clippy` — no warnings
- [ ] `cargo fmt --check` — properly formatted
- [ ] E2E tmux tests pass through `TmuxBackend`
- [ ] No direct `tmux::` imports remain in `main.rs`

#### Manual Verification:
- [ ] `dual launch` creates tmux session and attaches (same as before)
- [ ] `dual destroy` kills tmux session (same as before)
- [ ] `dual list` shows correct tmux status (same as before)
- [ ] Running from inside tmux uses `switch-client`

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to Phase 2.

---

## Phase 2: TUI Scaffold

### Overview
Add ratatui + crossterm dependencies. Create the TUI module with a basic tree view of repos/branches. `dual` (no args) launches the TUI. `dual list` remains as the non-interactive fallback.

### Changes Required:

#### 1. Add Dependencies
**File**: `Cargo.toml`

```toml
[dependencies]
ratatui = "0.30"
crossterm = "0.28"
```

No tree widget crate needed initially — we'll render the tree manually with ratatui's `List` widget. This avoids an extra dependency and gives us full control.

#### 2. Create `src/tui/mod.rs` — TUI Module
**File**: `src/tui/mod.rs` (new)

```rust
mod app;
mod ui;
mod event;

pub use app::run;
```

#### 3. Create `src/tui/app.rs` — App State and Main Loop
**File**: `src/tui/app.rs` (new)

```rust
use crate::backend::MultiplexerBackend;
use crate::state::WorkspaceState;

pub struct App {
    /// All workspace entries grouped by repo
    repos: Vec<RepoGroup>,
    /// Currently selected index in the flattened list
    selected: usize,
    /// Whether the app should exit
    should_quit: bool,
    /// Action to take after TUI exits (launch a workspace)
    pending_action: Option<PendingAction>,
}

pub struct RepoGroup {
    pub name: String,
    pub workspaces: Vec<WorkspaceItem>,
    pub expanded: bool,
}

pub struct WorkspaceItem {
    pub branch: String,
    pub workspace_id: String,
    pub status: WorkspaceStatus,
}

pub enum WorkspaceStatus {
    Attached,    // tmux session attached
    Running,     // tmux session exists, not attached
    Stopped,     // container stopped
    Lazy,        // not cloned yet
}

pub enum PendingAction {
    Launch(String), // workspace_id
}

/// Main TUI entry point.
/// Returns Ok(Some(workspace_id)) if user selected a workspace to launch.
/// Returns Ok(None) if user quit.
pub fn run(
    state: &WorkspaceState,
    backend: &dyn MultiplexerBackend,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();
    let mut app = App::new(state, backend);

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if let Some(action) = event::handle_events(&mut app)? {
            match action {
                event::Action::Quit => break,
                event::Action::Select(workspace_id) => {
                    // Restore terminal before launching tmux
                    ratatui::restore();
                    return Ok(Some(workspace_id));
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    ratatui::restore();
    Ok(None)
}
```

#### 4. Create `src/tui/ui.rs` — Rendering
**File**: `src/tui/ui.rs` (new)

```rust
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use super::app::{App, WorkspaceStatus};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Title block
    let block = Block::default()
        .title(" dual ")
        .borders(Borders::ALL);

    // Build list items from repo groups
    let items: Vec<ListItem> = app.flatten_items()
        .into_iter()
        .map(|item| {
            let style = match item.status {
                Some(WorkspaceStatus::Attached) => Style::default().fg(Color::Green),
                Some(WorkspaceStatus::Running) => Style::default().fg(Color::Blue),
                Some(WorkspaceStatus::Stopped) => Style::default().fg(Color::Yellow),
                Some(WorkspaceStatus::Lazy) => Style::default().fg(Color::DarkGray),
                None => Style::default().add_modifier(Modifier::BOLD), // repo header
            };
            ListItem::new(Line::from(item.display)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default();
    state.select(Some(app.selected));

    frame.render_stateful_widget(list, area, &mut state);
}
```

#### 5. Create `src/tui/event.rs` — Event Handling
**File**: `src/tui/event.rs` (new)

```rust
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use super::app::App;

pub enum Action {
    Quit,
    Select(String), // workspace_id
}

pub fn handle_events(app: &mut App) -> Result<Option<Action>, Box<dyn std::error::Error>> {
    if event::poll(std::time::Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            return Ok(handle_key(app, key));
        }
    }
    Ok(None)
}

fn handle_key(app: &mut App, key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_up();
            None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_down();
            None
        }
        KeyCode::Enter => {
            if let Some(workspace_id) = app.selected_workspace_id() {
                Some(Action::Select(workspace_id))
            } else {
                // Selected a repo header — toggle expand/collapse
                app.toggle_expand();
                None
            }
        }
        _ => None,
    }
}
```

#### 6. Update `src/main.rs` — Wire TUI as Default
**File**: `src/main.rs`

Change `cmd_default()` to launch TUI:

```rust
fn cmd_default(backend: &dyn MultiplexerBackend) -> i32 {
    let state = match state::load() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to load state: {e}");
            return 1;
        }
    };

    if state.all_workspaces().is_empty() {
        info!("No workspaces configured. Run `dual add` in a repo to get started.");
        return 0;
    }

    match tui::run(&state, backend) {
        Ok(Some(workspace_id)) => {
            // User selected a workspace — launch it
            cmd_launch_by_id(&workspace_id, backend)
        }
        Ok(None) => 0, // User quit
        Err(e) => {
            error!("TUI error: {e}");
            1
        }
    }
}
```

#### 7. Update `src/lib.rs` — Export TUI Module
```rust
pub mod tui;
```

### Success Criteria:

#### Automated Verification:
- [ ] `cargo build` — compiles with new dependencies
- [ ] `cargo test` — all existing tests pass
- [ ] `cargo clippy` — no warnings
- [ ] TUI module unit tests pass (app state, navigation, event handling)

#### Manual Verification:
- [ ] `dual` shows a TUI with repos and branches listed
- [ ] Arrow keys / j/k navigate the list
- [ ] Enter on a repo header toggles expand/collapse
- [ ] Enter on a branch shows workspace status indicators
- [ ] `q` exits cleanly (terminal restored properly)
- [ ] `dual list` still works as non-interactive output

**Implementation Note**: After completing this phase and all verification passes, pause here for manual confirmation before proceeding to Phase 3.

---

## Phase 3: TUI ↔ Tmux Transition

### Overview
Wire the TUI selection to the full launch pipeline. When user selects a workspace: suspend TUI → launch workspace (clone/container/session if needed) → attach tmux session → when user detaches, resume TUI.

### Changes Required:

#### 1. Implement the Suspend → Launch → Resume Loop
**File**: `src/main.rs`

The key insight: `tmux attach-session` is a **blocking call**. It takes over the terminal and only returns when the user detaches (`Ctrl+b d`). This makes the suspend/resume pattern straightforward:

```rust
fn cmd_default(backend: &dyn MultiplexerBackend) -> i32 {
    let mut state = match state::load() { ... };

    loop {
        // Reload state each iteration (workspaces may have changed)
        state = match state::load() {
            Ok(s) => s,
            Err(e) => { error!("{e}"); return 1; }
        };

        match tui::run(&state, backend) {
            Ok(Some(workspace_id)) => {
                // TUI already called ratatui::restore() before returning
                // Terminal is now in normal mode — tmux can take over

                // Run the full launch pipeline
                let exit_code = cmd_launch_by_id(&workspace_id, backend);

                if exit_code != 0 {
                    // Launch failed — show error, re-enter TUI
                    eprintln!("Launch failed (exit code {exit_code}). Press Enter to continue...");
                    let _ = std::io::stdin().read_line(&mut String::new());
                }

                // When tmux detach returns (or launch failed), loop back to TUI
                // ratatui::init() is called inside tui::run() at the top
                continue;
            }
            Ok(None) => return 0, // User quit
            Err(e) => {
                error!("TUI error: {e}");
                return 1;
            }
        }
    }
}
```

#### 2. Update `tui::run()` — Handle Re-entry
**File**: `src/tui/app.rs`

The TUI needs to handle being called multiple times in a loop. Each call:
1. `ratatui::init()` — enter alternate screen
2. Build fresh app state from workspace state
3. Run event loop
4. On selection: `ratatui::restore()` — leave alternate screen
5. Return selected workspace

```rust
pub fn run(
    state: &WorkspaceState,
    backend: &dyn MultiplexerBackend,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();

    // Build fresh state each time (may have changed since last TUI session)
    let mut app = App::new(state, backend);

    let result = loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if let Some(action) = event::handle_events(&mut app)? {
            match action {
                event::Action::Quit => break Ok(None),
                event::Action::Select(workspace_id) => {
                    break Ok(Some(workspace_id));
                }
            }
        }
    };

    ratatui::restore();
    result
}
```

#### 3. Ensure Proper Terminal Restoration on Panic
**File**: `src/main.rs`

Add a panic hook to restore terminal state:

```rust
fn main() {
    // Install panic hook BEFORE entering TUI
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Best-effort terminal restoration
        let _ = crossterm::execute!(
            std::io::stderr(),
            crossterm::terminal::LeaveAlternateScreen
        );
        let _ = crossterm::terminal::disable_raw_mode();
        original_hook(panic_info);
    }));

    // ... rest of main
}
```

#### 4. Handle "Already Inside Tmux" Case in TUI
**File**: `src/tui/app.rs`

If the user launches `dual` from inside a tmux session, the TUI still works. When they select a workspace, `backend.attach()` uses `switch-client` (from Phase 1). But after switching, the TUI process is still running in the original tmux session's pane.

Two options:
- **Option A**: Exit TUI after switch-client (user returns to TUI by running `dual` again)
- **Option B**: Keep TUI running, user switches back to it via tmux

We choose **Option A** for simplicity:

```rust
event::Action::Select(workspace_id) => {
    if backend.is_inside() {
        // Inside tmux — switch-client won't block, so exit TUI
        ratatui::restore();
        cmd_launch_by_id(&workspace_id, backend);
        return Ok(None); // Don't loop back — tmux switch is instant
    } else {
        // Outside tmux — attach blocks until detach
        break Ok(Some(workspace_id));
    }
}
```

### Success Criteria:

#### Automated Verification:
- [ ] `cargo test` — all tests pass
- [ ] `cargo clippy` — no warnings
- [ ] Panic hook test — TUI restores terminal on panic

#### Manual Verification:
- [ ] `dual` → TUI → select workspace → tmux session appears
- [ ] Detach from tmux (Ctrl+b d) → TUI reappears automatically
- [ ] Select another workspace → switches to it
- [ ] `q` in TUI → exits cleanly
- [ ] Run `dual` from inside tmux → TUI shows → select workspace → tmux switches client
- [ ] If launch fails (e.g., container not available), error message shown, TUI resumes
- [ ] Ctrl+C during TUI → terminal restored properly
- [ ] Panic during TUI → terminal restored properly

**Implementation Note**: After completing this phase and all verification passes, this constitutes the v3.0.0 release.

---

## Phase 4: v3.1 Scope (Not Implemented Here)

For reference, the next iteration after v3.0.0:

1. **Fuzzy search in TUI** — type to filter repos/branches
2. **Session layout configuration** — `[layout]` section in `.dual.toml`
3. **BasicBackend** — no-multiplexer fallback
4. **ZellijBackend** — zellij implementation of `MultiplexerBackend`
5. **Status refresh** — TUI periodically refreshes container/tmux status
6. **Meta-key workspace switching** — tmux hook that launches Dual picker

---

## Linear Issues to Create

### v3.0.0 Milestone:

| Title | Priority | Labels | Phase |
|-------|----------|--------|-------|
| Extract MultiplexerBackend trait from tmux module | Urgent | Core | 1 |
| Implement TmuxBackend with is_inside() detection | Urgent | Core, Tmux | 1 |
| Replace all tmux:: calls in main.rs with backend trait | High | Core | 1 |
| Add ratatui + crossterm dependencies | High | Core, DX | 2 |
| Create TUI module with tree view of repos/branches | High | Core, DX | 2 |
| Wire `dual` (no args) to launch TUI | High | Core, DX | 2 |
| Implement TUI suspend/resume loop for tmux transitions | Urgent | Core, Tmux, DX | 3 |
| Add panic hook for terminal state restoration | High | Core | 3 |
| Handle inside-tmux TUI launch (switch-client path) | High | Tmux, DX | 3 |

---

## Testing Strategy

### Unit Tests:
- `backend.rs`: Trait definition compiles, error types work
- `tmux_backend.rs`: All existing tmux tests migrated, `is_inside()` tested with env var mock
- `tui/app.rs`: App state construction from WorkspaceState, navigation (up/down/expand/collapse), selected_workspace_id()
- `tui/event.rs`: Key event → action mapping
- `tui/ui.rs`: Render doesn't panic for empty state, single repo, multiple repos

### Integration Tests:
- Full TUI → launch → detach → resume cycle (requires tmux, marked `#[ignore]`)
- TUI exit restores terminal (check raw mode disabled, alternate screen left)

### Manual Testing Steps:
1. `dual` with no workspaces → helpful message, no TUI crash
2. `dual` with workspaces → TUI with tree view
3. Navigate with j/k and arrow keys
4. Enter on repo header → expand/collapse
5. Enter on branch → full launch pipeline → tmux session
6. Ctrl+b d → TUI returns
7. Select another branch → switches
8. q → exits, terminal clean
9. Run from inside tmux → switch-client behavior
10. Kill TUI with Ctrl+C → terminal clean

## Performance Considerations

- **TUI startup**: ratatui::init() is ~1ms. No performance concern.
- **State loading**: `state::load()` reads `~/.dual/workspaces.toml` (small file). Fine on each TUI loop iteration.
- **Status checks**: `backend.is_alive()` shells out to `tmux has-session` for each workspace. With 15 workspaces, this is 15 subprocess calls (~150ms total). Acceptable for TUI entry, but v3.1 should cache and refresh on a timer.
- **Container status**: `container::status()` shells out to `docker inspect`. Same concern — cache in v3.1.

## Migration Notes

### v2.2.0 → v3.0.0
- **`dual` (no args) behavior change**: Was `cmd_default()` showing list output. Now launches TUI.
- **`dual list` unchanged**: Non-interactive fallback, same output as before.
- **New dependencies**: `ratatui` (~1.5MB), `crossterm` (~500KB). Adds to binary size.
- **`src/tmux.rs` removed**: Replaced by `src/backend.rs` + `src/tmux_backend.rs`. All logic preserved.
- **No config changes**: `.dual.toml` format unchanged. `workspaces.toml` unchanged.

## File Structure After v3.0.0

```
src/
├── lib.rs              # Module exports
├── main.rs             # Entry point, command dispatch (uses backend trait)
├── cli.rs              # Clap CLI definitions
├── backend.rs          # MultiplexerBackend trait + BackendError (NEW)
├── tmux_backend.rs     # TmuxBackend implementation (NEW, replaces tmux.rs)
├── config.rs           # Per-repo hints, naming conventions
├── state.rs            # Global workspace state
├── clone.rs            # Git clone management
├── container.rs        # Docker container lifecycle
├── shell.rs            # Shell RC generation
├── proxy.rs            # Reverse proxy
├── shared.rs           # Config propagation
└── tui/                # TUI module (NEW)
    ├── mod.rs           # Public run() entry point
    ├── app.rs           # App state, RepoGroup, WorkspaceItem
    ├── ui.rs            # Rendering with ratatui
    └── event.rs         # Keyboard event handling
```

## References

- v2 roadmap: `thoughts/shared/plans/2026-02-15-v2-to-v3-roadmap.md`
- Architecture research: `thoughts/shared/research/2026-02-15-v3-architecture-rethink.md`
- SPEC.md:269-294 — Runtime Backend Contract
- SPEC.md:298-320 — User Experience (first launch, workspace switching)
- SPEC.md:396-401 — Phase 4 (TUI, zellij, BasicBackend)
- ratatui docs: https://ratatui.rs / https://docs.rs/ratatui
