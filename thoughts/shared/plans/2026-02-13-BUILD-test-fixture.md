# Test Fixture Implementation Plan

## Overview

Create fixture generation helpers for E2E tests — a local git repo with package.json and a simple Node.js HTTP server that can be cloned via `git clone --local` and run inside a Docker container.

## Current State Analysis

- Test harness (`tests/harness/mod.rs`) provides TestFixture with RAII cleanup
- Clone module (`src/clone.rs`) supports local paths via `is_local_path()` and `--local` flag
- Config module (`src/config.rs`) has `parse()` for string-based config creation
- Architecture confirms: git clone --local from /tmp works with hardlinks, ~64ms average

## Desired End State

A `tests/fixtures/mod.rs` module providing:
1. `create_fixture_repo()` — creates a local git repo with package.json + server.js
2. `fixture_config_toml()` — generates dual.toml string pointing at the fixture
3. Smoke tests verifying fixture creation and clonability

## What We're NOT Doing

- NOT writing E2E tests (that's test-suite)
- NOT creating CI pipeline (that's ci-pipeline)
- NOT modifying any src/ modules

## Phase 1: Create Fixture Module

### Changes Required:

#### 1. Create fixture module
**File**: `tests/fixtures/mod.rs`
**Changes**: Fixture generation helpers

The fixture creates:
- A temp dir with `git init`
- `package.json` with a start script
- `server.js` — minimal HTTP server on :3000
- Initial git commit so `git clone --local` works

#### 2. Create smoke test
**File**: `tests/fixture_smoke.rs`
**Changes**: Tests that fixture can be created and cloned

### Success Criteria:

#### Automated Verification:
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes (all existing + new tests)
- [ ] `cargo test --test fixture_smoke` passes specifically
- [ ] `cargo clippy` clean
- [ ] `cargo fmt --check` clean
- [ ] Fixture repo creates valid git repo
- [ ] Fixture repo can be cloned via git clone --local
- [ ] Generated dual.toml parses correctly

## References

- Architecture: e2e-local-fixture-repo (CONFIRMED)
- Test harness: `tests/harness/mod.rs`
- Clone module: `src/clone.rs`
