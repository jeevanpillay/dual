# GitHub Repo Setup & v2.0.0 Release Plan

## Overview

Prepare the Dual repository for a production-grade v2.0.0 release. This covers updating the README, adding a comprehensive CHANGELOG, adding an MIT LICENSE file, updating GitHub metadata, marking old TypeScript releases as pre-release, bumping the version, and tagging the release.

## Current State Analysis

- **Version**: `Cargo.toml:3` is `1.3.0`
- **README.md**: Stale — references old `dual.toml` single-file config, missing `add`/`create`/`sync`/`shell-rc` commands, outdated quick start
- **CHANGELOG**: Does not exist
- **LICENSE**: Does not exist (Cargo.toml says MIT at line 7)
- **GitHub description**: "a dotenv-like multiplexer for git worktree workflows" — completely wrong
- **GitHub topics**: None set
- **Old releases**: v0.1.0–v1.2.2 (TypeScript era) still marked as regular releases

### Key Discoveries
- Two-file config system: `.dual.toml` (per-repo hints) + `~/.dual/workspaces.toml` (global state) — `src/config.rs:18-54`, `src/state.rs:14-39`
- 10 CLI commands (+ hidden `shell-rc`) defined in `src/cli.rs:14-74`
- Release pipeline triggers on semver tags — `.github/workflows/release.yml:41-45`
- 5 commits since v1.3.0: config split, shared propagation, production readiness (thiserror/atomic/tracing), docs
- curl-based installer available via cargo-dist shell/powershell scripts
- 115 tests across unit, integration, e2e, and fixture categories

## Desired End State

After this plan is complete:
1. `README.md` accurately reflects the current architecture, CLI, and config system
2. `CHANGELOG.md` comprehensively documents v2.0.0 (including the Rust rewrite) and notes the TypeScript legacy
3. `LICENSE` file contains the MIT license
4. GitHub description and topics are updated
5. Old TypeScript releases (v0.1.0–v1.2.2) are marked as pre-release
6. `Cargo.toml` version is `2.0.0`
7. v2.0.0 tag is pushed, triggering cargo-dist to build and publish release binaries

**Verification**: `gh release view v2.0.0` shows the release with binaries for all 5 targets. `gh repo view` shows updated description and topics. Old releases show `Pre-release` badge.

## What We're NOT Doing

- No CONTRIBUTING.md, SECURITY.md, issue templates, or PR template
- No homepage URL
- No GitHub Discussions
- No GitHub Wiki changes
- No code changes — this is purely docs, metadata, and release

## Implementation Approach

Sequential phases: content first (CHANGELOG, README, LICENSE), then metadata (GitHub settings, old releases), then release (version bump, tag, push).

---

## Phase 1: CHANGELOG.md

### Overview
Create a comprehensive changelog documenting v2.0.0 (folding in v1.3.0) and noting the TypeScript legacy versions.

### Changes Required:

#### 1. Create `CHANGELOG.md`
**File**: `CHANGELOG.md` (new file, repo root)

Content structure:

```markdown
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
```

### Success Criteria:

#### Automated Verification:
- [ ] `CHANGELOG.md` exists at repo root
- [ ] File is valid markdown

#### Manual Verification:
- [ ] Content accurately reflects the v2.0.0 changes
- [ ] TypeScript legacy versions are correctly listed with dates

---

## Phase 2: README.md Rewrite

### Overview
Rewrite README.md to reflect the current architecture, two-file config system, full CLI reference, and installation methods.

### Changes Required:

#### 1. Rewrite `README.md`
**File**: `README.md`

Full replacement with updated content:

```markdown
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
dual add                         # Register this repo
dual create my-project feat/auth # Create a branch workspace
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
cargo test               # Run tests (115 tests)
cargo clippy             # Run linter
cargo fmt                # Format code
```

Targets: Linux, macOS (Intel + Apple Silicon), Windows.

## License

MIT
```

### Success Criteria:

#### Automated Verification:
- [ ] `README.md` is valid markdown
- [ ] All CLI commands from `src/cli.rs` are documented

#### Manual Verification:
- [ ] Installation URLs are correct (will be verified after release)
- [ ] Config examples match the actual config structures in `src/config.rs` and `src/state.rs`
- [ ] Architecture diagram is accurate

---

## Phase 3: LICENSE File

### Overview
Add MIT license file to match `Cargo.toml:7`.

### Changes Required:

#### 1. Create `LICENSE`
**File**: `LICENSE` (new file, repo root)

Standard MIT license text with:
- Year: 2025 (project started Oct 2025)
- Copyright holder: Jeevan Pillay

### Success Criteria:

#### Automated Verification:
- [ ] `LICENSE` file exists at repo root
- [ ] Contains "MIT License" text

---

## Phase 4: GitHub Metadata & Old Releases

### Overview
Update GitHub repo description and topics. Mark old TypeScript releases as pre-release.

### Changes Required:

#### 1. Update repo description and topics
```bash
gh repo edit jeevanpillay/dual \
  --description "Terminal workspace orchestrator for parallel multi-repo development with AI coding agents" \
  --add-topic rust,cli,terminal,docker,tmux,developer-tools,workspace,ai-coding
```

#### 2. Mark old releases as pre-release
Mark all TypeScript-era releases (v0.1.0 through v1.2.2) as pre-release:
```bash
gh release edit v0.1.0 --prerelease
gh release edit v0.2.0 --prerelease
gh release edit v0.2.1 --prerelease
gh release edit v0.2.2 --prerelease
gh release edit v0.3.0 --prerelease
gh release edit v1.0.0 --prerelease
gh release edit v1.1.0 --prerelease
gh release edit v1.2.0 --prerelease
gh release edit v1.2.1 --prerelease
gh release edit v1.2.2 --prerelease
```

### Success Criteria:

#### Automated Verification:
- [ ] `gh repo view jeevanpillay/dual --json description` shows new description
- [ ] `gh release list` shows old releases as "Pre-release"

---

## Phase 5: Version Bump, Tag & Release

### Overview
Bump version to 2.0.0 in Cargo.toml, commit all changes, tag, and push to trigger the cargo-dist release pipeline.

### Changes Required:

#### 1. Bump version
**File**: `Cargo.toml:3`
```toml
version = "2.0.0"
```

#### 2. Commit all changes
```bash
git add CHANGELOG.md README.md LICENSE Cargo.toml
git commit -m "chore: v2.0.0 release — changelog, readme, license, version bump"
```

#### 3. Tag and push
```bash
git tag v2.0.0
git push origin main --tags
```

This triggers `.github/workflows/release.yml` which builds binaries for all 5 targets and creates the GitHub Release.

#### 4. Update release notes
After cargo-dist creates the release, update the release body with the v2.0.0 changelog content:
```bash
gh release edit v2.0.0 --notes-file CHANGELOG_RELEASE.md
```
(Where `CHANGELOG_RELEASE.md` is a temp file with just the v2.0.0 section extracted from CHANGELOG.md, appended after the auto-generated install instructions.)

### Success Criteria:

#### Automated Verification:
- [ ] `cargo build` succeeds with version 2.0.0
- [ ] `cargo test` passes
- [ ] `cargo clippy` passes
- [ ] Git tag `v2.0.0` exists
- [ ] `gh release view v2.0.0` shows release with binaries

#### Manual Verification:
- [ ] Release page on GitHub looks correct with install instructions and binaries
- [ ] curl installer URL works after release is published
- [ ] Old releases show "Pre-release" badge on GitHub

---

## References

- Research: `thoughts/shared/research/2026-02-15-github-repo-setup-v2-release.md`
- CLI definition: `src/cli.rs:14-74`
- Config hints: `src/config.rs:18-54`
- Workspace state: `src/state.rs:14-39`
- Release workflow: `.github/workflows/release.yml:41-45`
- Cargo.toml: `Cargo.toml:1-30`
