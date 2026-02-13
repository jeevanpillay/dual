# Proxy Module Implementation Plan

## Overview

Implement a reverse proxy for browser access to workspace container services. Routes `{repo}-{branch}.localhost:{port}` → container's `:{port}` using HTTP Host header.

## Current State

- All workspace lifecycle works (launch, list, destroy)
- Containers run on Docker bridge network with IPs (172.17.0.x)
- No port exposure or mapping from containers to host
- `dual open` and `dual urls` are stubs

## Desired End State

- `dual proxy` starts reverse proxy listening on configured ports
- `dual urls` shows URLs for running workspace services
- `dual open` opens workspace URLs in default browser
- Proxy routes HTTP + WebSocket + SSE by subdomain

## What We're NOT Doing

- Auto-discovery of container ports (needs explicit config)
- HTTPS/TLS support
- Proxy auto-start on `dual launch` (manual start for MVP)
- Proxy management (daemonization, PID file, restart)

## Phase 1: Config — Add Ports

**File**: `src/config.rs`
**Changes**: Add optional `ports` field to `RepoConfig`

### Success Criteria:
- [x] `cargo build` succeeds
- [x] Config parses with and without ports field
- [x] Tests updated

## Phase 2: Container IP Resolution

**File**: `src/container.rs`
**Changes**: Add `get_ip(name: &str) -> Option<String>` that runs `docker inspect` to get container IP.

### Success Criteria:
- [x] `cargo build` succeeds
- [x] `cargo test` passes

## Phase 3: Implement Proxy Server

**File**: `src/proxy.rs` (NEW)
**Dependencies**: Added tokio, hyper, hyper-util, http-body-util to Cargo.toml

### Success Criteria:
- [x] `cargo build` succeeds
- [x] `cargo test` passes (7 proxy tests)
- [x] `cargo clippy` clean

## Phase 4: Wire CLI Commands

**File**: `src/main.rs`, `src/cli.rs`
**Changes**: Added `Proxy` subcommand, wired `dual urls` and `dual open`.

### Success Criteria:
- [x] `cargo build` succeeds
- [x] `cargo test` passes — 55 total tests
- [x] `cargo clippy` — clean (dead_code warnings only)
- [x] `cargo fmt --check` — clean
- [x] `dual --help` shows proxy command
- [x] `dual urls` shows configured URLs with status indicators
- [x] `dual urls <workspace>` filters by workspace
