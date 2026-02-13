# CI Pipeline Implementation Plan

## Overview

Create a GitHub Actions workflow that runs unit tests and E2E integration tests with Docker and tmux on ubuntu-latest.

## Current State Analysis

- No `.github/workflows/` directory exists
- Unit tests: `cargo test` (75 tests, no external deps)
- E2E tests: `cargo test --test e2e -- --include-ignored` (9 tests, require Docker + tmux)
- Architecture confirms: Docker pre-installed on ubuntu-latest, tmux via apt

## Desired End State

`.github/workflows/test.yml` that:
1. Runs unit tests (`cargo test`)
2. Runs E2E tests with Docker + tmux (`cargo test --test e2e -- --ignored`)
3. Runs cleanup sweep after E2E tests (defense-in-depth)
4. Runs clippy and fmt checks

## What We're NOT Doing

- NOT setting up release builds or dist
- NOT configuring Docker image caching (premature optimization)
- NOT modifying src/ modules

## Phase 1: Create GitHub Actions Workflow

### Changes Required:

#### 1. Create workflow file
**File**: `.github/workflows/test.yml`

### Success Criteria:

#### Automated Verification:
- [ ] Workflow file exists and is valid YAML
- [ ] `cargo build` still succeeds
- [ ] `cargo test` still passes
- [ ] `cargo clippy` clean
- [ ] `cargo fmt --check` clean

## References

- Architecture: e2e-ci-environment (CONFIRMED — Docker pre-installed, tmux via apt)
- Architecture: e2e-test-isolation (CONFIRMED WITH CAVEATS — prefix cleanup sweep)
