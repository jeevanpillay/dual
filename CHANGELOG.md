# Changelog

All notable changes to Dual are documented in this file.

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
