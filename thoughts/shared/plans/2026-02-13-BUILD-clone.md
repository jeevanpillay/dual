# Clone Module Implementation Plan

## Overview

Implement full git clone management for workspaces. Creates independent clones per repo/branch combination, detects local vs remote sources, and manages the workspace directory layout.

## Current State Analysis

- config module provides workspace_dir(), RepoConfig, encode_branch()
- No clone management code exists
- git is available on all target platforms

## Desired End State

A `clone` module that:
- Creates full git clones for workspace repo/branch combos
- Detects local path vs remote URL for clone strategy
- Checks if a workspace clone already exists
- Lists all existing workspace clones for a repo
- Removes workspace clones
- Passes cargo build, test, clippy, fmt

## What We're NOT Doing

- git worktrees (explicitly rejected by SPEC)
- Shallow clone support (defer, optional optimization)
- Clone progress reporting / UI
- Automatic clone on workspace access (that's the tmux/orchestration module)

## Phase 1: Clone Module

### Changes Required:

#### 1. Create src/clone.rs
- `clone_workspace()`: Create a full git clone
- `workspace_exists()`: Check if clone dir exists
- `list_workspaces()`: Find all existing clones for a repo
- `remove_workspace()`: Delete a workspace clone
- `is_local_path()`: Detect local vs remote URL
- Error types for clone failures

### Success Criteria:

#### Automated Verification:
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean (dead_code warnings acceptable)
- [ ] `cargo fmt --check` passes
- [ ] Unit tests for path detection (local vs remote)
- [ ] Unit tests for clone command construction
