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

Register your repo, create a branch workspace, and launch it:

```bash
cd ~/code/my-project
dual add                          # Register this repo
dual create my-project feat/auth  # Create a branch workspace
dual launch my-project-feat__auth # Launch it
```

This clones the repo, starts a Docker container, generates transparent command routing, and opens a tmux session. Your editor, git, and credentials stay on the host. Runtime commands (`pnpm dev`, `node`, `curl localhost`) are transparently routed to the container.

## CLI Commands

| Command | Description |
|---------|-------------|
| `dual` | Show workspace list with status |
| `dual add [--name NAME]` | Register current git repo as a workspace |
| `dual create <repo> <branch>` | Create a new branch workspace for an existing repo |
| `dual launch <workspace>` | Clone, start container, open tmux session |
| `dual list` | List all workspaces with status |
| `dual destroy <workspace>` | Tear down workspace (container, tmux, clone) |
| `dual open [workspace]` | Open workspace services in browser |
| `dual urls [workspace]` | Display workspace URLs |
| `dual sync [workspace]` | Sync shared config files across branch workspaces |
| `dual proxy` | Start reverse proxy for browser access |

## Configuration

Dual uses two config files:

### `.dual.toml` (per-repo hints)

Lives in your project root. Committed to git. Controls runtime behavior.

```toml
image = "node:20"
ports = [3000, 3001]
setup = "pnpm install"

[env]
NODE_ENV = "development"

[shared]
files = [".vercel", ".env.local"]
```

| Field | Description | Default |
|-------|-------------|---------|
| `image` | Docker image for the container | `node:20` |
| `ports` | Ports that services bind to (for reverse proxy) | `[]` |
| `setup` | Command to run on container start | None |
| `env` | Environment variables passed to the container | `{}` |
| `shared.files` | Files/directories to share across branch workspaces | `[]` |

### `~/.dual/workspaces.toml` (global state)

Managed by Dual. Tracks all registered workspaces.

```toml
workspace_root = "~/dual-workspaces"

[[workspaces]]
repo = "my-project"
url = "git@github.com:org/my-project.git"
branch = "main"

[[workspaces]]
repo = "my-project"
url = "git@github.com:org/my-project.git"
branch = "feat/auth"
```

## How It Works

When you run `dual launch`, Dual:

1. Clones the repo into `{workspace_root}/{repo}/{branch}/`
2. Creates a Docker container with the clone bind-mounted
3. Generates a shell RC file that transparently routes runtime commands into the container via `docker exec`
4. Opens a tmux session in the workspace directory

Your editor, git, and credentials stay on the host. The container handles all runtime processes. Claude Code never knows it's running inside a container.

## Architecture

```
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
cargo test               # Run tests
cargo clippy             # Run linter
cargo fmt                # Format code
```

Targets: Linux, macOS (Intel + Apple Silicon), Windows.

## License

MIT
