# Changelog

All notable changes to Dual are documented in this file.

## [3.0.0] - 2026-02-16

v3.0 architecture rewrite — devcontainer.json as primary config, interactive init wizard, and shell interception pane propagation.

### Added

- **devcontainer.json support** — New `src/devcontainer.rs` parser reads `image`, `build.dockerfile`, `forwardPorts`, `containerEnv`, `postCreateCommand`, and `mounts` from `.devcontainer/devcontainer.json` (or root `devcontainer.json`). Repos with existing devcontainer configs work with zero Dual-specific setup
- **Dockerfile build support** — `build.dockerfile` in devcontainer.json triggers `docker build` instead of `docker pull`, with support for build args, target stages, and custom context paths
- **Interactive init wizard** — `dual init` launches a 5-step interactive wizard (image selection, ports, setup command, review/confirm) powered by `dialoguer`. Skip prompts with `dual init --yes`
- **Shell interception pane propagation** — New tmux panes and windows auto-load command interception. `tmux set-environment` sets `DUAL_ACTIVE`, `DUAL_RC_PATH`, `DUAL_CONTAINER` at the session level; a shell RC snippet in `~/.zshrc`/`~/.bashrc` auto-sources the interception file
- **Shell hook auto-install** — Dual automatically installs a guarded snippet in the user's shell RC file on first run (idempotent, no-op outside Dual tmux sessions)
- **`.dual/settings.json`** — New per-repo config file for Dual-specific orchestration (`extra_commands`, `anonymous_volumes`, `shared`, `devcontainer` path override)

### Changed

- **`dual add` renamed to `dual init`** — The command for registering a workspace is now `dual init` (breaking)
- **Config file format** — Per-repo config moved from `.dual.toml` (TOML) to `.dual/settings.json` (JSON) with a new schema separating container config (devcontainer.json) from orchestration config (settings.json)
- **Two-file config architecture** — Container settings (`image`, `ports`, `env`, `setup`) now live in `devcontainer.json`; Dual-specific settings (`extra_commands`, `anonymous_volumes`, `shared`) live in `.dual/settings.json`
- **Config load order** — `load_hints()` merges `.dual/settings.json` + `devcontainer.json` into `RepoHints`. devcontainer.json is the primary source for container config
- **Proxy error messages** — Now reference `devcontainer.json` for port configuration instead of `.dual.toml`

### Removed

- **`.dual.toml`** — Replaced by `.dual/settings.json` + `devcontainer.json`
- **`dual add` subcommand** — Replaced by `dual init`
- **Container fields in Dual config** — `image`, `ports`, `setup`, `[env]` removed from Dual config (now in devcontainer.json)

---

## [2.3.1] - 2026-02-16

Fix macOS shell RC source path quoting.

### Fixed

- **Shell RC source path** — Quoted the RC file path in `source_file_command()` to handle paths with spaces
- **RC file location** — Moved RC files to `~/.dual/rc/` directory

---

## [2.3.0] - 2026-02-16

TUI workspace browser and multiplexer abstraction layer.

### Added

- **TUI workspace browser** — `dual` (no args) launches a ratatui-based terminal UI with repo/branch tree view, status colors (green=running, yellow=stopped, gray=lazy), and keyboard navigation (j/k, Enter, q)
- **MultiplexerBackend trait** — Abstract interface over terminal multiplexers (`is_available`, `create_session`, `attach`, `detach`, `destroy`, `is_alive`, `list_sessions`, `send_keys`, `is_inside`), with `TmuxBackend` as the concrete implementation
- **TUI suspend/resume loop** — Selecting a workspace suspends the TUI, launches/attaches tmux, and resumes the TUI on detach (Ctrl+b d)
- **Panic hook** — Terminal state (alternate screen, raw mode) restored on crash
- **Inside-tmux TUI handling** — When launched from inside tmux, uses `switch-client` and exits cleanly instead of looping

### Changed

- **`dual` (no args)** — Now launches TUI instead of printing usage. `dual list` remains as the non-interactive fallback
- **Tmux module** — `src/tmux.rs` replaced by `src/backend.rs` (trait) + `src/tmux_backend.rs` (implementation)
- **`session_name()` moved to config** — Session naming is now in `src/config.rs` alongside `container_name()`

---

## [2.2.0] - 2026-02-15

Bug fixes, feature completions, and context-aware CLI improvements.

### Added

- **Context-aware `dual create`** — Repo auto-detected from cwd when `--repo` is omitted. New syntax: `dual create <branch> [--repo NAME]`
- **Context-aware `dual launch` / `dual destroy`** — Workspace arg now optional, auto-detected from cwd
- **Tmux nested session detection** — Detects `$TMUX` env var and uses `switch-client` instead of `attach-session` to avoid nesting
- **Grouped `dual list` output** — Workspaces displayed grouped by repo with detailed container/tmux status
- **`clone_from_local()`** — New fast clone strategy using `git clone --local` from main workspace + `git checkout -b` for new branches
- **Commented `.dual.toml` template** — `dual add` creates a `.dual.toml` with helpful inline documentation
- **Environment variable support** — `[env]` section in `.dual.toml` passed as `-e KEY=VALUE` to `docker create`
- **Setup command support** — `setup = "pnpm install"` in `.dual.toml` runs via `docker exec` after first container creation
- **Configurable container commands** — `extra_commands` field in `.dual.toml` merges with default command routing list
- **Configurable anonymous volumes** — `anonymous_volumes` field in `.dual.toml` replaces hardcoded `node_modules` volume

### Changed

- **`dual create` CLI** — Args changed from `<repo> <branch>` to `<branch> [--repo NAME]` (breaking)
- **`dual launch` CLI** — Workspace arg now optional (backward compatible)
- **`dual destroy` CLI** — Workspace arg now optional (backward compatible)
- **`container::create()` signature** — Now accepts `env` and `anonymous_volumes` parameters
- **`shell::generate_rc()` / `write_rc_file()`** — Now accept `extra_commands` parameter

---

## [2.0.0] - 2026-02-15

Complete rewrite from TypeScript to Rust. Dual is now a compiled binary with Docker-based workspace isolation, transparent command routing, and a reverse proxy for browser access.

### Added

- **Rust rewrite** — Full rewrite in Rust (Edition 2024) replacing the TypeScript/npm implementation
- **Docker container isolation** — Each workspace runs in its own Docker container with bind-mounted source
- **Transparent command routing** — Shell RC intercepts runtime commands (`pnpm`, `node`, `curl localhost`) and routes them to containers via `docker exec`
- **Reverse proxy** — HTTP reverse proxy (hyper + tokio) with `{repo}-{branch}.localhost:{port}` subdomain routing for browser access
- **tmux session management** — Automatic tmux sessions per workspace with attach/detach lifecycle
- **Two-file config system** — Split architecture: `.dual.toml` (per-repo hints, committed to git) + `~/.dual/workspaces.toml` (global state, managed by Dual)
- **`dual add` command** — Register an existing git repo as a Dual workspace
- **`dual create` command** — Create a new branch workspace for an existing repo
- **`dual sync` command** — Propagate shared config files (`.vercel`, `.env.local`, etc.) across branch workspaces using symlinks (Unix) or copies (Windows)
- **`dual shell-rc` command** — Generate shell RC for transparent command routing (internal use)
- **Shared config propagation** — Declare shared files in `.dual.toml` `[shared]` section, synced across all branches of a repo via `~/.dual/shared/{repo}/`
- **Atomic state writes** — File locking (`fs2`) + temp file + atomic rename for crash-safe state persistence
- **Structured logging** — `tracing` + `tracing-subscriber` with `DUAL_LOG` environment variable filter
- **Typed errors** — `thiserror`-based error types replacing string errors
- **curl-based installer** — Shell and PowerShell install scripts via cargo-dist
- **Cross-platform builds** — Pre-built binaries for macOS (Apple Silicon + Intel), Linux (ARM64 + x64), and Windows (x64)
- **CI/CD pipeline** — GitHub Actions with check/lint, unit tests, E2E tests (Docker + tmux), and automated releases

### Changed

- **Architecture** — Worktree-based workflows replaced with full git clone per workspace
- **Config format** — Single `dual.toml` replaced with two-file system (`.dual.toml` + `workspaces.toml`)
- **Installation** — npm package replaced with compiled binary (`cargo install` or curl installer)
- **Language** — TypeScript → Rust (Edition 2024)

### Removed

- npm package distribution
- Git worktree-based workspace management
- dotenv multiplexer functionality
- Environment variable file loading

---

## Legacy Releases (TypeScript)

The following releases are from the original TypeScript implementation and are no longer supported. They have been marked as pre-release on GitHub.

- **1.2.2** (2025-10-16) — Final TypeScript release
- **1.2.1** (2025-10-16) — Patch release
- **1.2.0** (2025-10-16) — Dotenv compatibility improvements
- **1.1.0** (2025-10-16) — Environment variable loading
- **1.0.0** (2025-10-15) — First stable TypeScript release
- **0.3.0** (2025-10-15) — Worktree lifecycle management with hooks
- **0.2.2** (2025-10-15) — Patch release
- **0.2.1** (2025-10-15) — Patch release
- **0.2.0** (2025-10-15) — Feature release
- **0.1.0** (2025-10-14) — Initial release
