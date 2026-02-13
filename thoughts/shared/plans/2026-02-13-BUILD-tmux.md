# Tmux Module Implementation Plan

## Overview

Implement the tmux runtime backend for session management. Creates, attaches, detaches, and destroys tmux sessions per workspace. Sources shell RC for command interception.

## Current State

- shell module generates RC content for command interception
- config module provides workspace paths and naming
- clone module manages workspace directories
- container module manages Docker lifecycle
- No tmux integration exists

## Desired End State

A `tmux` module that:
- Creates tmux sessions named after workspaces
- Attaches/detaches sessions
- Destroys sessions
- Checks session liveness
- Lists all dual-managed sessions
- Sources shell RC in new sessions
- Checks tmux availability
- Passes cargo build, test, clippy, fmt

## What We're NOT Doing

- Pane layout management (single pane per session for MVP)
- ZellijBackend
- BasicBackend fallback
- Workspace fuzzy picker (that requires TUI, deferred)
- Meta-key binding for workspace switching

## Phase 1: Tmux Module

### Success Criteria:
- [ ] cargo build succeeds
- [ ] cargo test passes
- [ ] cargo clippy clean (dead_code acceptable)
- [ ] cargo fmt --check passes
- [ ] Session command construction tests
- [ ] tmux availability detection
