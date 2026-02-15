---
date: 2026-02-15T07:10:36Z
researcher: Claude
git_commit: b95e436699a1b9dfc26e39557c2057ffcaeef19c
branch: main
repository: dual
topic: "GitHub repo setup for production-grade release and v2 planning"
tags: [research, github, release, readme, v2, production]
status: complete
last_updated: 2026-02-15
last_updated_by: Claude
---

# Research: GitHub Repo Setup for Production-Grade Release & v2 Planning

**Date**: 2026-02-15T07:10:36Z
**Researcher**: Claude
**Git Commit**: b95e436
**Branch**: main
**Repository**: dual

## Research Question

The codebase has undergone a complete architecture rewrite from a TypeScript npm package (v0.1.0–v1.2.2, Oct 2025) to a Rust binary (v1.3.0+, Feb 2026). We need to:
1. Update README.md, GitHub description, topics, and repo metadata to reflect the new architecture
2. Set up the repo for production-grade open-source presence
3. Plan and publish a new release, potentially moving to v2

## Summary

Dual has been completely rewritten. The old codebase (TypeScript, npm packages, git worktrees, dotenv multiplexer) was archived to an `archive` branch. The new codebase is Rust (Edition 2024), uses full git clones (not worktrees), Docker containers for isolation, transparent command routing via shell RC, and a reverse proxy for browser access. The GitHub repo metadata is stale — description still reads "a dotenv-like multiplexer for git worktree workflows" and there are no repository topics set.

## Detailed Findings

### 1. Current GitHub Repo State (What's Stale)

| Field | Current Value | Problem |
|-------|--------------|---------|
| **Description** | "a dotenv-like multiplexer for git worktree workflows" | Completely wrong — Dual is no longer a dotenv multiplexer or worktree-based |
| **Topics** | None set | Missing entirely |
| **Homepage URL** | Empty | No homepage configured |
| **Wiki** | Enabled | Not used — should be disabled |
| **Projects** | Enabled | Not used for this repo |
| **Visibility** | Public | Correct |
| **Default branch** | main | Correct |
| **Issues** | Enabled | Correct |

### 2. Current README.md (What's Outdated)

The README (`README.md:1-113`) was last meaningfully updated in the TypeScript era with minor patches. Key issues:

- **Config format section** (lines 45-57): Still shows the old `dual.toml` format with `workspace_root`, `repos[].name`, `repos[].url`, `repos[].branches`, `repos[].ports`. The new architecture uses two files: `~/.dual/workspaces.toml` (global state) + `.dual.toml` (per-repo hints)
- **CLI Commands table** (lines 59-69): Missing `dual add`, `dual create`, `dual sync`, `dual shell-rc` commands. Some existing commands have changed behavior
- **Quick Start section** (lines 13-43): Still shows `cargo install --path .` but doesn't mention curl-based installer. Config example is outdated
- **Prerequisites** (lines 7-11): Correct but incomplete (doesn't mention what's optional)
- **Architecture diagram** (lines 76-93): Reasonable but could be more detailed
- **Development section** (lines 94-103): Correct
- **License section** (line 112): Says "See repository for license details" — no LICENSE file exists, but Cargo.toml says MIT

### 3. Current Architecture (What README Should Reflect)

The new architecture (from SPEC.md, ARCHITECTURE.md, BUILD.md, and source analysis):

**Core Concept**: Terminal workspace orchestrator for parallel multi-repo development with AI coding agents

**Two-File Config System** (PR #112):
- `~/.dual/workspaces.toml` — Global workspace state (repo, url, branch, path for each workspace)
- `.dual.toml` — Per-repo hints (image, ports, setup, env, shared files)

**CLI Commands** (from `src/cli.rs`):
| Command | Description |
|---------|-------------|
| `dual` | Show workspace list with status |
| `dual add` | Register current git repo as workspace |
| `dual create <repo> <branch>` | Create branch workspace for existing repo |
| `dual launch <workspace>` | Clone → container → shell → tmux → attach |
| `dual list` | List all workspaces with status |
| `dual destroy <workspace>` | Tear down workspace (tmux → container → clone) |
| `dual open [workspace]` | Open services in browser |
| `dual urls [workspace]` | Display workspace URLs |
| `dual sync [workspace]` | Sync shared config files across branches |
| `dual proxy` | Start reverse proxy server |

**Modules** (11 source files, 3,578 lines of Rust):
- `cli.rs` — Clap-based CLI parsing
- `config.rs` — Per-repo hints (.dual.toml), naming conventions
- `state.rs` — Global workspace state with atomic writes + file locking
- `clone.rs` — Git clone management (full clones, --local for local repos)
- `container.rs` — Docker lifecycle (create, start, stop, destroy, exec)
- `shell.rs` — Shell RC generation for transparent command routing
- `tmux.rs` — tmux session management
- `proxy.rs` — HTTP reverse proxy (hyper+tokio) with subdomain routing
- `shared.rs` — Config propagation across workspaces (symlinks on Unix, copies on Windows)

**Test Suite**: 115 tests (79 lib + 16 main + 3 e2e + 7 fixture + 10 harness)

**CI/CD**:
- `.github/workflows/test.yml` — Check/Lint, Unit Tests, E2E Tests (Docker + tmux)
- `.github/workflows/release.yml` — cargo-dist release pipeline (5 targets)

**Release Targets** (from `dist-workspace.toml`):
- aarch64-apple-darwin (Apple Silicon macOS)
- x86_64-apple-darwin (Intel macOS)
- aarch64-unknown-linux-gnu (ARM64 Linux)
- x86_64-unknown-linux-gnu (x64 Linux)
- x86_64-pc-windows-msvc (x64 Windows)

### 4. Release History

| Version | Date | Era | Notes |
|---------|------|-----|-------|
| v0.1.0–v0.3.0 | Oct 15, 2025 | TypeScript | npm packages, worktree lifecycle |
| v1.0.0–v1.2.2 | Oct 15-16, 2025 | TypeScript | Dotenv compatibility, env loading |
| v1.3.0 | Feb 13, 2026 | **Rust** | First Rust release, cargo-dist |
| **v2.0.0** | Pending | **Rust** | Should mark the full architecture break |

The v1.3.0 release was the first Rust release but doesn't reflect the magnitude of the change. A v2.0.0 would correctly signal the complete rewrite.

### 5. What's Missing for Production-Grade Setup

**Files missing from repo:**
- [ ] `LICENSE` — Cargo.toml says MIT but no LICENSE file exists
- [ ] `CONTRIBUTING.md` — No contribution guidelines
- [ ] `SECURITY.md` — No security policy
- [ ] `CHANGELOG.md` — No changelog (release notes only on GitHub)
- [ ] `.github/ISSUE_TEMPLATE/` — No issue templates
- [ ] `.github/PULL_REQUEST_TEMPLATE.md` — No PR template
- [ ] `.github/FUNDING.yml` — No funding/sponsorship info (optional)

**GitHub settings to update:**
- [ ] Description: Update to match Cargo.toml ("Terminal workspace orchestrator for parallel multi-repo development with AI coding agents")
- [ ] Topics: Add relevant topics (rust, cli, terminal, docker, tmux, developer-tools, workspace, monorepo, ai-coding, claude-code)
- [ ] Homepage URL: Set if applicable
- [ ] Wiki: Disable (not used)
- [ ] Discussions: Consider enabling

**README sections to add/update:**
- [ ] Installation section (curl-based installer + cargo install)
- [ ] Quick Start with new workflow (`dual add` → `dual create` → `dual launch`)
- [ ] Config format (.dual.toml + workspaces.toml)
- [ ] Full CLI reference
- [ ] Architecture diagram update
- [ ] Badge row (CI status, version, license)

### 6. v2.0.0 Release Plan

**Rationale for v2**: Complete rewrite from TypeScript to Rust. Different language, different architecture (worktrees → full clones), different config format, different CLI commands, different installation method. This is a breaking change in every dimension.

**Steps to release v2.0.0:**
1. Update `Cargo.toml` version from `1.3.0` to `2.0.0`
2. Update README.md with new content
3. Update GitHub repo metadata (description, topics)
4. Add LICENSE file (MIT)
5. Add any other missing production files
6. Create git tag `v2.0.0` and push
7. cargo-dist release pipeline auto-creates GitHub Release with binaries
8. Update release notes with v2 changelog

**Tag push triggers release**: The `release.yml` workflow triggers on `push.tags` matching `**[0-9]+.[0-9]+.[0-9]+*`, so pushing tag `v2.0.0` will automatically build and publish release binaries.

## Code References

- `Cargo.toml:4` — Current version: 1.3.0
- `Cargo.toml:5` — Description field
- `Cargo.toml:6` — Repository URL: https://github.com/jeevanpillay/dual
- `Cargo.toml:7` — License: MIT
- `dist-workspace.toml:1-14` — cargo-dist config with 5 targets
- `.github/workflows/release.yml:1-297` — Release pipeline
- `.github/workflows/test.yml:1-106` — CI pipeline
- `src/cli.rs:1-74` — All CLI commands
- `src/main.rs:28-40` — Command dispatch
- `README.md:1-113` — Current (stale) README

## Architecture Documentation

The architecture has been extensively documented through:
- `SPEC.md` — Full engineering requirements (27 validated claims)
- `thoughts/ARCHITECTURE.md` — Architecture validation tracker (27/27 confirmed)
- `thoughts/BUILD.md` — MVP build tracker (14/14 modules built)
- `thoughts/shared/plans/` — 15 build plans
- `thoughts/shared/research/` — 23 research documents

## Historical Context (from thoughts/)

- `thoughts/ARCHITECTURE.md` — Complete architecture validation with 27 hypothesis-driven experiments
- `thoughts/BUILD.md` — MVP implementation log covering all 14 modules
- `thoughts/shared/plans/2026-02-15-rust-production-readiness.md` — Latest production readiness plan (thiserror, atomic state, tracing)
- `thoughts/shared/plans/2026-02-15-shared-config-propagation.md` — Shared config design
- `thoughts/shared/research/2026-02-15-config-propagation-across-workspaces.md` — Config propagation research
- `thoughts/shared/research/2026-02-15-rust-production-readiness-audit.md` — Production readiness audit

## Open Questions

1. Should we write a comprehensive CHANGELOG.md covering the v1.x → v2.0 transition, or just put it in the v2.0.0 release notes?
2. Should `dual.toml` be gitignored? (It's currently untracked and in `.gitignore` status)
3. Should the old TypeScript releases (v0.x, v1.0-v1.2) be marked as pre-release or left as-is?
4. Is there a homepage URL to set on the repo?
5. Should we enable GitHub Discussions for community Q&A?
