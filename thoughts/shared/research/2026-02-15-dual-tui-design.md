---
date: 2026-02-15T12:00:00+08:00
researcher: Claude
git_commit: b95e436699a1b9dfc26e39557c2057ffcaeef19c
branch: main
repository: dual
topic: "Dual TUI Design — Full Terminal Takeover with Repo/Branch Tree Navigation"
tags: [research, tui, ratatui, crossterm, terminal-interface, workspace-switching, tmux-integration]
status: complete
last_updated: 2026-02-15
last_updated_by: Claude
---

# Research: Dual TUI Design — Full Terminal Takeover with Repo/Branch Tree Navigation

**Date**: 2026-02-15
**Researcher**: Claude
**Git Commit**: b95e436699a1b9dfc26e39557c2057ffcaeef19c
**Branch**: main
**Repository**: dual

## Research Question

Design the Dual TUI as a full terminal takeover (like tmux) that shows repos at root level with collapsible branches, where clicking/selecting a branch opens that tmux session.

## Summary

### What Exists Today

Dual currently has a **CLI-only interface** — no TUI library is used. User interaction is through `clap`-based subcommands (`dual list`, `dual launch`, `dual destroy`). Output uses `tracing` macros with Unicode status symbols (`●` running, `○` stopped, `◌` lazy). The SPEC already envisions a full terminal takeover with fuzzy picker and meta-key workspace switching (Phase 4), but no implementation exists yet.

### What the SPEC Envisions

The SPEC (`SPEC.md:298-320`) describes the target UX:

1. Developer runs `dual` — **it becomes their session**
2. Fuzzy picker shows all projects/workspaces
3. Developer selects a workspace → Dual creates clone + container + tmux session → attaches
4. **Meta-key** detaches current workspace, shows overlay picker, developer selects new workspace
5. Developer **never leaves Dual's control** — never runs `tmux attach` or `docker exec` manually

### The User's Vision

The user proposes a tmux-like full terminal takeover with a **tree view**:

```
┌─ Dual ──────────────────────────────────────┐
│                                             │
│  ▼ lightfast-platform                       │
│    ● main              [attached]           │
│    ● feat/auth         [running]            │
│    ○ fix/billing       [stopped]            │
│                                             │
│  ► agent-os            (3 branches)         │
│                                             │
│  ▼ dual                                     │
│    ● main              [attached]           │
│    ◌ feat/tui          [lazy]               │
│                                             │
└─────────────────────────────────────────────┘
```

- Repos shown at root level, collapsible
- Branches nested under each repo
- Selecting a branch opens/attaches to that tmux session
- Collapsed repos show branch count summary

## Detailed Findings

### Current Codebase State

#### CLI Interface (`src/cli.rs`)
- Uses `clap` with derive macros — 74 lines
- Commands: `Add`, `Create`, `Launch`, `List`, `Destroy`, `Open`, `Urls`, `Sync`, `Proxy`, `ShellRc`
- No interactive mode, no event loop

#### Status Display (`src/main.rs:691-720`)
- `print_workspace_status()` — simple text output
- Status symbols: `●` attached, `●` running, `○` stopped, `◌` lazy
- Uses `tracing::info!()` for all output
- No color beyond what tracing provides

#### Tmux Integration (`src/tmux.rs`)
- Full session lifecycle: `create_session()`, `attach()`, `detach()`, `destroy()`
- Session naming: `dual-{repo}-{encoded_branch}` (line 122-125)
- `is_alive()` checks session existence (line 77-82)
- `list_sessions()` returns all dual-managed sessions (line 86-102)
- This module is the **bridge** between TUI and workspace sessions

#### State Data Available (`src/state.rs`)
- `WorkspaceState` at `~/.dual/workspaces.toml` — source of truth
- `WorkspaceEntry` has: `repo`, `url`, `branch`, optional `path`
- `workspace_root()`, `resolve_workspace()` — lookup methods
- Container status available from `container.rs`: Running/Stopped/Missing

#### Dependencies (`Cargo.toml`)
- No TUI library present
- Has `tokio` (async runtime), `hyper` (HTTP), `clap`, `serde`, `tracing`
- Adding `ratatui` + `crossterm` would be new dependencies

### Design Deferred in Build Plans

From `thoughts/shared/plans/2026-02-13-BUILD-cli.md:26`:
> "NOT doing interactive TUI or fuzzy picker"

From `thoughts/shared/plans/2026-02-13-BUILD-tmux.md:32`:
> "Workspace fuzzy picker (that requires TUI, deferred)"

From `thoughts/shared/plans/2026-02-13-BUILD-wire-cli.md`:
> "fuzzy picker for `dual` (no args) — that's polish"

The TUI was explicitly deferred from MVP to Phase 4 ("Polish" in `SPEC.md:397`).

### Rust TUI Framework Landscape (2025-2026)

#### Ratatui + Crossterm (Recommended Stack)

**Ratatui** is the dominant Rust TUI framework (successor to archived tui-rs):
- Immediate-mode rendering with widget-based architecture
- Full terminal takeover via alternate screen buffer
- Mouse event handling (clicks, scrolling, drag)
- Keyboard event routing with modifier keys
- Stateful widgets for selection and scroll state
- Double buffering for efficient rendering
- Active community, comprehensive docs at https://ratatui.rs

**Crossterm** is the recommended backend:
- Cross-platform (Windows, Linux, macOS)
- Most actively maintained terminal backend
- Comprehensive API: mouse, keyboard, terminal manipulation
- Default in all Ratatui examples

#### Key Ecosystem Crates

| Crate | Purpose | Relevance |
|-------|---------|-----------|
| `tui-tree-widget` | Hierarchical tree view with collapse/expand | Core — repo/branch tree |
| `tui-input` | Text input widgets | Fuzzy search/filter |
| `tui-textarea` | Multi-line text editing | Low — not needed initially |
| `tui-popup` | Modal/popup support | Confirmation dialogs |

#### Reference Implementations

- **gitui** — Git TUI built with ratatui. Tree views, tabs, popups, async operations. Best reference for a git-related TUI.
- **bottom** — System monitor TUI. Real-time data, graphs, mouse support.
- **oxker** — Docker container manager TUI. Tree views, hierarchical navigation. Directly relevant.

### Design Space: TUI Architecture

#### Model 1: Full Takeover (tmux-like)

```
┌─ Dual ─────────────────────────────────────────────────┐
│ ┌─ Workspaces ──────────┐ ┌─ Preview ────────────────┐ │
│ │ ▼ lightfast-platform  │ │ lightfast-platform/main  │ │
│ │   ● main     attached │ │                          │ │
│ │   ● feat/auth running │ │ Container: running       │ │
│ │   ○ fix/bill  stopped │ │ Ports: 3000, 3001, 4001 │ │
│ │                       │ │ URLs:                    │ │
│ │ ► agent-os   (3)      │ │  lightfast-main:3000     │ │
│ │                       │ │  lightfast-main:3001     │ │
│ │ ▼ dual                │ │  lightfast-main:4001     │ │
│ │   ● main     attached │ │                          │ │
│ │   ◌ feat/tui    lazy  │ │ tmux: 3 panes            │ │
│ │                       │ │ Claude: active            │ │
│ └───────────────────────┘ └──────────────────────────┘ │
│ [q]uit  [a]dd  [c]reate  [d]estroy  [/]filter         │
└────────────────────────────────────────────────────────┘
```

**Behavior**:
- `dual` with no args → enters TUI, takes over terminal
- Tree view on left: repos → branches (collapsible)
- Preview pane on right: workspace details, URLs, status
- `Enter` on a branch → exits TUI, attaches to tmux session
- Meta-key from within tmux → returns to TUI (like tmux prefix)
- All other `dual` subcommands still work as CLI

**Pros**:
- Single interface for all workspace management
- Visual overview of entire development environment
- Natural for mouse users (click to open)
- Keyboard-driven for terminal users (j/k/Enter)

**Cons**:
- More complex to implement
- Must handle terminal state carefully (alternate screen)
- Need to manage TUI ↔ tmux transitions cleanly

#### Model 2: Overlay/Popup (tmux choose-tree style)

```
╔══ Dual Workspace Picker ══════════════════╗
║ Filter: _                                 ║
║                                           ║
║ ▼ lightfast-platform                      ║
║   ● main              attached            ║
║   ● feat/auth         running             ║
║   ○ fix/billing       stopped             ║
║                                           ║
║ ► agent-os            (3 branches)        ║
║                                           ║
║ ▼ dual                                    ║
║   ● main              attached            ║
║   ◌ feat/tui          lazy                ║
║                                           ║
║ Enter: attach  d: destroy  Esc: cancel    ║
╚═══════════════════════════════════════════╝
```

**Behavior**:
- Triggered by meta-key from within a workspace
- Overlays on current terminal content
- Quick selection → dismisses overlay, attaches to chosen workspace
- Closer to `tmux choose-tree` or `fzf` UX
- Minimal — just a picker, not a dashboard

**Pros**:
- Simpler to implement
- Faster for switching (no full redraw)
- Familiar pattern (fzf, tmux choose-tree)
- Lower cognitive overhead

**Cons**:
- Less information visible
- No persistent overview
- Can't manage workspaces without being in one

#### Model 3: Hybrid — TUI Home + Overlay Switcher

```
                    ┌───────────────────┐
                    │   dual (no args)  │
                    │   Full TUI Home   │
                    └────────┬──────────┘
                             │
                    Enter on branch
                             │
                             ▼
                    ┌───────────────────┐
                    │   tmux session    │
                    │   (workspace)     │◄─── Esc returns to TUI Home
                    └────────┬──────────┘
                             │
                      Meta-key
                             │
                             ▼
                    ┌───────────────────┐
                    │  Overlay picker   │
                    │  (quick switch)   │──── Enter → attach to new workspace
                    └───────────────────┘
```

**Behavior**:
- `dual` → Full TUI home screen with tree view + details
- Select branch → transitions to tmux session
- Within tmux, meta-key → lightweight overlay picker for quick switching
- `Esc` from tmux → returns to TUI home for full management
- Two modes: **home** (full management) and **switcher** (quick navigation)

**This is the model the SPEC describes** (SPEC.md:298-320).

### Design Space: Tree View Interactions

#### Collapse/Expand

```
Before:                          After clicking ▼:
▼ lightfast-platform             ► lightfast-platform  (3)
  ● main         attached
  ● feat/auth    running
  ○ fix/billing  stopped
```

- `▼` = expanded, `►` = collapsed
- Collapsed shows branch count in parentheses
- Toggle with Enter, Space, or mouse click on triangle
- Remember collapse state across TUI sessions

#### Status Indicators

| Symbol | State | Meaning |
|--------|-------|---------|
| `●` green | attached | Container running, tmux session alive, currently viewed |
| `●` blue | running | Container running, tmux session alive, not currently viewed |
| `○` yellow | stopped | Container exists but stopped |
| `◌` dim | lazy | Not yet cloned, will be provisioned on first access |

#### Keyboard Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move cursor down |
| `k` / `↑` | Move cursor up |
| `Enter` | Open workspace (attach to tmux) or toggle collapse |
| `Space` | Toggle repo collapse/expand |
| `l` / `→` | Expand repo |
| `h` / `←` | Collapse repo |
| `/` | Enter filter/search mode |
| `a` | Add repo (from current directory) |
| `c` | Create branch workspace |
| `d` | Destroy selected workspace (with confirmation) |
| `s` | Sync shared config |
| `p` | Start/stop proxy |
| `u` | Show URLs for selected workspace |
| `q` | Quit Dual |
| `?` | Help overlay |

#### Mouse Support

- Click on repo name → toggle collapse
- Click on branch → select it
- Double-click on branch → open workspace
- Scroll wheel → scroll tree view
- Click on action buttons in status bar

### Design Space: TUI ↔ tmux Transition

This is the critical design challenge. When the user selects a workspace, the TUI must:

1. **Leave alternate screen** (restore normal terminal)
2. **Attach to tmux session** (tmux takes over terminal)
3. When user triggers meta-key or detaches:
4. **Re-enter alternate screen** (TUI takes over again)
5. **Refresh state** (container status may have changed)

#### Option A: Dual Runs Inside tmux

```
Outer tmux session (dual-control)
├── Window 0: Dual TUI (always exists)
├── Window 1: lightfast-main session
├── Window 2: lightfast-feat-auth session
└── Window 3: agent-os-main session
```

- Dual creates an outer tmux session
- TUI runs in window 0
- Each workspace gets its own window
- Switching = tmux window switch (instant)
- Meta-key = `select-window -t :0` (back to TUI)

**Pros**: tmux handles all terminal management, transitions are instant
**Cons**: Nested tmux (if user already uses tmux), relies on tmux for everything

#### Option B: Dual Manages Terminal Directly

```
Terminal emulator
├── State A: Dual TUI (alternate screen)
│   └── User selects workspace → leave alternate screen
├── State B: tmux attach (tmux controls terminal)
│   └── User presses meta-key → tmux detach → enter alternate screen
└── Back to State A
```

- Dual owns the terminal, tmux is a child
- TUI uses alternate screen, yields to tmux when attaching
- On detach, TUI reclaims terminal

**Pros**: Clean separation, no nested tmux, works with zellij too
**Cons**: Terminal state transitions more complex, potential flicker

#### Option C: Dual Replaces tmux

```
Terminal emulator
└── Dual TUI (full control)
    ├── Tree view (workspace list)
    └── Embedded terminal pane (workspace session)
        └── Running: shell, claude, dev server
```

- Dual is the multiplexer
- Uses a PTY library (like `portable-pty` or `rustix`) to spawn pseudo-terminals
- Renders terminal output within ratatui
- No tmux dependency at all

**Pros**: Complete control, no dependency on tmux/zellij, unified UX
**Cons**: Massive implementation effort, reimplementing terminal emulation, very hard to get right

### Design Space: Data Flow

```
~/.dual/workspaces.toml ──► WorkspaceState
                               │
                               ▼
                          ┌─────────┐
                          │ TUI App │
                          │         │
   Container status ─────►│  Model  │◄──── tmux session status
   (docker inspect)       │         │      (tmux has-session)
                          │  View   │
                          │         │
                          │  Event  │◄──── keyboard/mouse input
                          │  Loop   │
                          └─────────┘
                               │
                               ▼
                    ┌──────────────────────┐
                    │  Actions             │
                    │  - tmux attach       │
                    │  - container start   │
                    │  - clone workspace   │
                    │  - destroy workspace │
                    └──────────────────────┘
```

### Design Space: Async Considerations

The TUI event loop needs to:

1. **Poll keyboard/mouse** events (crossterm event stream)
2. **Refresh container status** periodically (docker inspect is ~50ms)
3. **Refresh tmux status** periodically (tmux has-session is ~5ms)
4. **Handle terminal resize** (SIGWINCH)

Options:
- **Tokio select!** — Already have tokio for the proxy. TUI event loop can be async.
- **Polling** — Check container/tmux status every N seconds, not on every frame.
- **File watcher** — Watch `workspaces.toml` for changes from other `dual` CLI invocations.

### Cargo Dependencies Needed

```toml
# TUI core
ratatui = "0.29"          # TUI framework
crossterm = "0.28"        # Terminal backend

# TUI ecosystem
tui-tree-widget = "0.22"  # Tree view for repos/branches

# Optional
tui-input = "0.11"        # Text input for search/filter
```

## Code References

- `src/cli.rs:1-74` — Current CLI definitions (would gain a `Tui` subcommand or become default)
- `src/main.rs:46-67` — `cmd_default()` currently shows list, would become TUI entry point
- `src/main.rs:691-720` — `print_workspace_status()` status display logic to port to TUI
- `src/tmux.rs:49-64` — `attach()` — the key function called when user selects a workspace
- `src/tmux.rs:77-82` — `is_alive()` — needed for live status in tree view
- `src/tmux.rs:86-102` — `list_sessions()` — enumerate active sessions
- `src/state.rs:14-39` — `WorkspaceState`/`WorkspaceEntry` — data model for tree items
- `src/container.rs:7-12` — `ContainerStatus` enum — Running/Stopped/Missing for status icons
- `src/container.rs:77-93` — `status()` — check container state for display
- `src/proxy.rs:273-297` — `workspace_urls()` — URL data for preview pane
- `SPEC.md:298-320` — Target UX description (fuzzy picker, meta-key switching)
- `SPEC.md:349-357` — Workspace states (ATTACHED, BACKGROUND, STOPPED, LAZY)
- `SPEC.md:397` — Phase 4: "TUI with workspace sidebar showing live status"

## Architecture Documentation

### Existing Patterns That Support TUI

1. **Workspace state is file-based** (`~/.dual/workspaces.toml`) — TUI can read it on startup
2. **Container/tmux status are queryable** — `container::status()` and `tmux::is_alive()` exist
3. **Session naming is deterministic** — `dual-{repo}-{encoded_branch}` for both containers and tmux
4. **All operations are functions** — `cmd_launch()`, `cmd_destroy()` etc. can be called from TUI actions
5. **Proxy runs independently** — TUI doesn't need to embed the proxy

### New Patterns the TUI Would Introduce

1. **Event loop** — Currently all commands are run-to-completion; TUI needs a persistent event loop
2. **Alternate screen** — Terminal state management (enter/leave)
3. **Async status polling** — Container and tmux status refresh on a timer
4. **Terminal transitions** — Handing control to tmux and getting it back
5. **Mouse input** — Currently keyboard-only

## Historical Context (from thoughts/)

- `thoughts/shared/plans/2026-02-13-BUILD-cli.md:26` — "NOT doing interactive TUI or fuzzy picker" (explicitly deferred from MVP)
- `thoughts/shared/plans/2026-02-13-BUILD-tmux.md:32` — "Workspace fuzzy picker (that requires TUI, deferred)"
- `thoughts/shared/plans/2026-02-13-BUILD-wire-cli.md` — "fuzzy picker for `dual` (no args) — that's polish"
- `thoughts/ARCHITECTURE.md` — Layer 4 "Polish" mentions interactive features as nice-to-have
- `thoughts/BUILD.md` — MVP build status tracker, shows current CLI phase

## Open Questions

1. **Option A vs B vs C**: Does Dual run inside tmux (Option A), manage terminal directly and yield to tmux (Option B), or replace tmux entirely (Option C)?
2. **Meta-key binding**: How does the user get back to the TUI from within a tmux session? tmux hook? Custom key binding? Dual wrapper?
3. **Proxy integration**: Should the TUI embed the proxy (show live request logs) or keep it separate?
4. **Fuzzy search vs tree**: The SPEC describes a fuzzy picker, the user describes a tree. Are these complementary (tree + filter) or alternative modes?
5. **zellij support**: If we go Option A (Dual inside tmux), how does zellij backend work?
6. **Startup behavior**: Should `dual` always launch TUI, or only when no args? Should `dual list` still work as CLI output?
7. **Multiple terminals**: If a user has two terminal windows, can both show the TUI? Or is one TUI instance the controller?
