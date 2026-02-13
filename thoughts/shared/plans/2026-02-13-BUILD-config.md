# Config Module Implementation Plan

## Overview

Implement workspace config parsing from `dual.toml` files. Provides repo definitions, branch lists, filesystem path generation, and container naming. Foundation module that all downstream modules depend on.

## Current State Analysis

- No config module exists
- CLI module is built (src/cli.rs, src/main.rs)
- No serde/toml dependencies

## Desired End State

A `config` module that:
- Defines `DualConfig` and `RepoConfig` structs with serde deserialization
- Discovers `dual.toml` in current dir, then `~/.config/dual/dual.toml`
- Parses and validates config with clear error messages
- Provides helper methods: workspace_dir(), container_name(), encode_branch()
- Passes cargo build, test, clippy, fmt

## What We're NOT Doing

- Image auto-detection logic (that's the container module)
- Command routing implementation (that's the shell module)
- Actually creating directories or clones
- Runtime config reloading

## Phase 1: Config Module

### Changes Required:

#### 1. Add dependencies to Cargo.toml
- `serde` with derive feature
- `toml` for parsing
- `dirs` for home directory

#### 2. Create src/config.rs
- `DualConfig` struct: workspace_root, repos vec
- `RepoConfig` struct: name, url, branches
- `load()` function: discover and parse dual.toml
- `encode_branch()`: feat/auth → feat__auth
- `workspace_dir()`: ~/dual-workspaces/{repo}/{branch}
- `container_name()`: dual-{repo}-{branch}
- Validation with clear error messages

#### 3. Wire into main.rs
- Add `mod config;` declaration

### Success Criteria:

#### Automated Verification:
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes (all existing + new config tests)
- [ ] `cargo clippy` clean
- [ ] `cargo fmt --check` passes
- [ ] Config parsing works with valid TOML
- [ ] Invalid config produces clear errors
- [ ] Branch encoding works correctly
- [ ] Path generation produces correct filesystem paths
- [ ] Container naming follows pattern

## References

- Research: thoughts/shared/research/2026-02-13-BUILD-config.md
- SPEC.md: Workspace config, filesystem layout, container naming sections
