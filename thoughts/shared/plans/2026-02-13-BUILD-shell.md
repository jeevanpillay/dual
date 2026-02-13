# Shell Module Implementation Plan

## Overview

Implement shell RC generation and command routing. Generates shell function definitions that intercept runtime commands and route them via docker exec to the workspace container.

## Current State

- container module provides exec() and container naming
- config module defines default routing (expandable with per-project overrides)
- No shell integration code exists

## Desired End State

A `shell` module that:
- Classifies commands as host or container operations
- Generates shell RC content (sourceable bash/zsh functions)
- Each intercepted command becomes a shell function wrapping docker exec
- TTY detection built into generated functions
- Passes cargo build, test, clippy, fmt

## What We're NOT Doing

- PATH shims (shell functions are sufficient for MVP)
- Zsh-specific completions
- Per-project config overrides (structure exists, implementation deferred)
- Actually sourcing the RC (that's tmux module's job)

## Phase 1: Shell Module

### Success Criteria:
- [ ] cargo build succeeds
- [ ] cargo test passes
- [ ] cargo clippy clean (dead_code acceptable)
- [ ] cargo fmt --check passes
- [ ] Command classification tests
- [ ] RC generation produces valid shell syntax
