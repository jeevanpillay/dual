# Dual

Terminal workspace orchestrator for parallel multi-repo development with AI coding agents.

Dual manages isolated development environments — one full git clone per workspace, one Docker container per clone — so you can run multiple repos on multiple branches simultaneously, all with Claude Code sessions active, all running dev servers on default ports, with zero conflicts.

## Installation

**curl (macOS/Linux):**

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/jeevanpillay/dual/releases/latest/download/dual-installer.sh | sh
```

**PowerShell (Windows):**

```powershell
powershell -ExecutionPolicy ByPass -c "irm https://github.com/jeevanpillay/dual/releases/latest/download/dual-installer.ps1 | iex"
```

**From source:**

```bash
cargo install --path .
```

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- [tmux](https://github.com/tmux/tmux)

## Quick Start

```bash
# 1. Register your repo
cd ~/code/my-project
dual add

# 2. Create a branch workspace
dual create feat/auth

# 3. Launch the TUI and select a workspace
dual
```

`dual` opens an interactive workspace browser. Select a workspace to launch it — Dual clones the repo, starts a Docker container, sets up transparent command routing, and drops you into a tmux session. Detach from tmux (`Ctrl+b d`) and you're back in the browser.

## The TUI

Running `dual` with no arguments opens the workspace browser:

```
 dual  workspace browser
┌──────────────────────────────────────┐
│▼ my-project                          │
│  main                     ● running  │
│  feat/auth                ○ stopped  │
│  feat/billing             ◌ lazy     │
│▼ agent-os                            │
│  main                     ● running  │
└──────────────────────────────────────┘
 j/k navigate  enter launch  q quit
```

- **j/k** or arrow keys to navigate
- **Enter** on a workspace to launch it (clone + container + tmux)
- **Enter** on a repo header to expand/collapse
- **q** or Esc to quit

When you select a workspace, the TUI suspends, tmux takes over. Detach from tmux (`Ctrl+b d`) and the TUI resumes automatically with fresh status.

### Tmux keybinding

For quick access from any tmux session, add to `~/.tmux.conf`:

```bash
# Prefix + Space to open Dual picker in a popup
bind-key Space display-popup -E -w 60% -h 60% "dual"

# Or without prefix — Alt+Space (Meta+Space)
# bind-key -n M-Space display-popup -E -w 60% -h 60% "dual"
```

`Prefix + Space` opens the Dual picker in a popup overlay. Select a workspace and the popup disappears as tmux switches to it.

## CLI Commands

| Command | Description |
|---------|-------------|
| `dual` | Open TUI workspace browser |
| `dual add [--name NAME]` | Register current git repo as a workspace |
| `dual create <branch> [--repo NAME]` | Create a new branch workspace |
| `dual launch [workspace]` | Launch a workspace (auto-detects from cwd) |
| `dual list` | List all workspaces with status (non-interactive) |
| `dual destroy [workspace]` | Tear down workspace (container, tmux, clone) |
| `dual open [workspace]` | Open workspace services in browser |
| `dual urls [workspace]` | Display workspace URLs |
| `dual sync [workspace]` | Sync shared config files across branch workspaces |
| `dual proxy` | Start reverse proxy for browser access |

## Configuration

Dual uses two config files:

### `.dual.toml` (per-repo hints)

Lives in your project root. Committed to git. Controls runtime behavior.

```toml
# Docker image for the container runtime
image = "node:20"

# Ports your dev server uses (for reverse proxy routing)
ports = [3000, 3001]

# Shell command to run after container creation (e.g., dependency install)
setup = "pnpm install"

# Commands to route to the container (in addition to defaults)
# Default: npm, npx, pnpm, node, python, python3, pip, pip3, curl, make
extra_commands = ["cargo", "go"]

# Directories to isolate with anonymous Docker volumes
anonymous_volumes = ["node_modules", ".next"]

# Environment variables passed to the container
[env]
NODE_ENV = "development"

# Files to share across all workspaces of this repo
[shared]
files = [".vercel", ".env.local"]
```

| Field | Description | Default |
|-------|-------------|---------|
| `image` | Docker image for the container | `node:20` |
| `ports` | Ports that services bind to (for reverse proxy) | `[]` |
| `setup` | Command to run after first container creation | None |
| `env` | Environment variables passed to the container | `{}` |
| `shared.files` | Files/directories to share across branch workspaces | `[]` |
| `extra_commands` | Additional commands to route to the container | `[]` |
| `anonymous_volumes` | Container volumes (e.g., `node_modules`) | `["node_modules"]` |

### `~/.dual/workspaces.toml` (global state)

Managed by Dual. Tracks all registered workspaces.

```toml
workspace_root = "~/dual-workspaces"

[[workspaces]]
repo = "my-project"
url = "git@github.com:org/my-project.git"
branch = "main"
path = "/Users/you/code/my-project"

[[workspaces]]
repo = "my-project"
url = "git@github.com:org/my-project.git"
branch = "feat/auth"
```

## How It Works

When you select a workspace (via `dual` or `dual launch`):

1. **Clone** — Clones the repo into `{workspace_root}/{repo}/{branch}/` (uses `git clone --local` from main workspace for speed)
2. **Shared files** — Copies shared config files (`.env.local`, `.vercel`, etc.) from `~/.dual/shared/{repo}/`
3. **Container** — Creates and starts a Docker container with the clone bind-mounted
4. **Setup** — Runs `setup` command on first launch (e.g., `pnpm install`)
5. **Shell RC** — Generates transparent command routing that intercepts runtime commands and routes them to the container via `docker exec`
6. **Tmux** — Creates a tmux session in the workspace directory and attaches

Your editor, git, and credentials stay on the host. The container handles all runtime processes. Claude Code never knows it's running inside a container.

## Architecture

```
Terminal
├── State A: Dual TUI (ratatui)
│   └── Select workspace → suspend TUI → launch pipeline → tmux attach
└── State B: tmux session
    └── Detach (Ctrl+b d) → resume TUI

Host                          Container
+--------------------------+  +--------------------------+
| nvim, git, claude, ssh   |  | pnpm, node, python       |
| file reads/writes        |  | curl localhost, tests    |
| credentials, SSH keys    |  | port-binding processes   |
+--------------------------+  +--------------------------+
        |    bind mount    |
        +------------------+

Browser --> {repo}-{branch}.localhost:{port}
        --> reverse proxy
        --> container
```

| Host | Container |
|------|-----------|
| git, cat, ls, vim | npm, pnpm, node, python |
| File reads/writes | Port-binding processes |
| SSH, credentials | curl localhost, tests |

## Development

```bash
cargo build              # Build debug binary
cargo build --release    # Build release binary
cargo test               # Run tests (~150 tests)
cargo clippy             # Run linter
cargo fmt                # Format code
```

Targets: Linux, macOS (Intel + Apple Silicon), Windows.

## License

MIT
