# Dual

Terminal workspace orchestrator for parallel multi-repo development with AI coding agents.

Dual manages isolated development environments -- one full git clone per workspace, one Docker container per clone -- so you can run multiple repos on multiple branches simultaneously, all with Claude Code sessions active, all running dev servers on default ports, with zero conflicts.

## Prerequisites

- [Rust](https://rustup.rs/) (edition 2024)
- [Docker](https://docs.docker.com/get-docker/)
- [tmux](https://github.com/tmux/tmux)

## Quick Start

Install:

```bash
cargo install --path .
```

Create a `dual.toml` in your project directory or `~/.config/dual/dual.toml`:

```toml
workspace_root = "~/dual-workspaces"

[[repos]]
name = "lightfast"
url = "git@github.com:org/lightfast.git"
branches = ["main", "feat/auth", "fix/memory-leak"]
ports = [3000, 3001]

[[repos]]
name = "agent-os"
url = "/local/path/to/agent-os"
branches = ["main", "v2-rewrite"]
ports = [8080]
```

Launch a workspace:

```bash
dual launch lightfast-main
```

## Config Format

`dual.toml` fields:

| Field            | Description                                      |
|------------------|--------------------------------------------------|
| `workspace_root` | Root directory for clones (default: `~/dual-workspaces`) |
| `repos[].name`   | Short name for the repo                          |
| `repos[].url`    | Git URL or local path to clone from              |
| `repos[].branches` | Branches to create workspaces for              |
| `repos[].ports`  | Ports that services bind to (for reverse proxy)  |

Branch names with `/` are encoded as `__` in workspace identifiers (e.g. `feat/auth` becomes `feat__auth`).

## CLI Commands

| Command                    | Description                                      |
|----------------------------|--------------------------------------------------|
| `dual`                     | List workspaces with status and launch hint       |
| `dual launch <workspace>`  | Clone repo, start container, open tmux session    |
| `dual list`                | List all workspaces and their status              |
| `dual destroy <workspace>` | Stop container, kill tmux session, remove clone   |
| `dual open [workspace]`    | Open workspace services in the browser            |
| `dual urls [workspace]`    | Print workspace URLs                              |
| `dual proxy`               | Start the reverse proxy for browser access        |

## How It Works

When you run `dual launch`, Dual clones the repo into `{workspace_root}/{repo}/{branch}/`, creates a Docker container with the clone bind-mounted, generates a shell RC file that transparently routes runtime commands (`pnpm`, `node`, `curl localhost`) into the container via `docker exec`, and opens a tmux session in the workspace directory. Your editor, git, and credentials stay on the host. The container handles all runtime processes. Claude Code never knows it is running inside a container.

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

File operations run on the host. Runtime operations run in the container.

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

See repository for license details.
