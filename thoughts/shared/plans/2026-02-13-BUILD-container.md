# Container Module Implementation Plan

## Overview

Implement Docker container lifecycle management. Creates, starts, stops, destroys containers per workspace. Provides docker exec wrapper for command execution inside containers.

## Current State

- config module provides container_name(), workspace_dir()
- clone module manages workspace clones on disk
- No Docker interaction code exists

## Desired End State

A `container` module that:
- Creates Docker containers with bind mounts and node_modules isolation
- Starts, stops, destroys containers
- Executes commands inside containers via docker exec
- Checks container status (running, stopped, missing)
- Lists all dual-managed containers
- Passes cargo build, test, clippy, fmt

## What We're NOT Doing

- Auto-image generation (use configurable base image)
- Port discovery/registration (that's the proxy module, deferred)
- Container networking beyond default bridge mode
- Health checks or restart policies

## Phase 1: Container Module

### Success Criteria:
- [ ] cargo build succeeds
- [ ] cargo test passes
- [ ] cargo clippy clean (dead_code acceptable)
- [ ] cargo fmt --check passes
- [ ] Command construction tests for create, start, stop, rm, exec
- [ ] Container status detection logic
