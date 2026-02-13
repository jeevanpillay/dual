---
spec_source: SPEC.md
arch_source: thoughts/ARCHITECTURE.md
date_started: 2026-02-13
status: complete
date_completed: 2026-02-13
build_progress: 13/13
---

# Dual MVP Build

This document tracks the implementation of Dual's MVP modules, informed by validated architectural decisions from ARCHITECTURE.md.

## Architecture Reference

Source: `thoughts/ARCHITECTURE.md`
Status: Complete (27/27 validated)

## Built Modules

- **cli**: Entry point, arg parsing (`dual`, `dual launch`, `dual list`, `dual destroy`, `dual open`, `dual urls`, `dual proxy`, `dual shell-rc`) - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-cli.md
  - Evidence: cargo build/test/clippy/fmt all pass, all subcommands produce correct output
  - Notes: Uses clap v4 derive macros. Hidden shell-rc subcommand for internal use.

- **config**: Workspace config parsing from `dual.toml` - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-config.md
  - Evidence: cargo build/test/clippy/fmt all pass, 14 unit tests
  - Notes: Supports repo definitions with branches and ports, workspace_root config, branch encoding, container naming, config discovery, workspace resolution, all_workspaces iterator.

- **clone**: Full git clone management (`git clone`, `git clone --local`) - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-clone.md
  - Evidence: cargo build/test/clippy/fmt all pass, 5 unit tests
  - Notes: Detects local vs remote URLs. Local paths use --local flag for hardlink clones.

- **container**: Docker container lifecycle (create, start, stop, destroy, exec) - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-container.md
  - Evidence: cargo build/test/clippy/fmt all pass, 5 unit tests
  - Notes: Bind mount workspace to /workspace, anonymous volume for node_modules isolation, sleep infinity keep-alive, container IP resolution for proxy.

- **shell**: Shell RC generation + command routing (classify + intercept) - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-shell.md
  - Evidence: cargo build/test/clippy/fmt all pass, 10 unit tests
  - Notes: Generates bash/zsh-compatible shell functions. TTY detection. RC file persistence to ~/.config/dual/rc/.

- **tmux**: tmux session management (create, attach, detach, destroy, list) - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-tmux.md
  - Evidence: cargo build/test/clippy/fmt all pass, 5 unit tests
  - Notes: Session names match container names (dual-{repo}-{branch}).

- **wire-cli**: CLI integration — wired all stub handlers to real module calls - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-wire-cli.md
  - Research: thoughts/shared/research/2026-02-13-BUILD-wire-cli.md
  - Evidence: All commands produce correct output, 48 tests at time of build
  - Notes: Full workspace orchestration: clone→container→shell→tmux→attach.

- **end-to-end**: Full flow verification with real Docker + tmux - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-end-to-end.md
  - Research: thoughts/shared/research/2026-02-13-BUILD-end-to-end.md
  - Evidence: 7/7 test phases pass with Docker 29.2.0 + tmux 3.5a
  - Notes: Full lifecycle verified: lazy → launch → running → exec → destroy → lazy.

- **proxy**: Reverse proxy for browser access + URL management - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-proxy.md
  - Research: thoughts/shared/research/2026-02-13-BUILD-proxy.md
  - Evidence: cargo build/test/clippy/fmt all pass, 55 total tests (7 proxy tests)
  - Notes: HTTP reverse proxy using hyper+tokio. Routes by Host header subdomain. Ports configured in dual.toml. Container IP via docker inspect. dual urls shows all URLs. dual open opens in browser. dual proxy starts proxy server. WebSocket upgrade support via http1 with_upgrades.

- **test-harness**: RAII test fixture framework (UUID naming, RAII Drop cleanup, prefix sweep) - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-test-harness.md
  - Research: thoughts/shared/research/2026-02-13-BUILD-test-harness.md
  - Evidence: cargo build/test/clippy/fmt all pass, 65 total tests (10 harness smoke tests)
  - Notes: Restructured crate to lib+binary for integration test support. TestFixture with RAII Drop for containers, tmux sessions, temp dirs. UUID-based naming (dual-test-{uuid}). cleanup_sweep() for SIGKILL defense-in-depth. uuid v1 dev-dependency.

- **test-fixture**: Minimal monorepo fixture (git init, package.json, HTTP server) - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-test-fixture.md
  - Evidence: cargo build/test/clippy/fmt all pass, 72 total tests (7 fixture smoke tests)
  - Notes: create_fixture_repo() creates local git repo with package.json + server.js. fixture_config_toml() generates dual.toml pointing at fixture. Works with clone module's --local flag. Sub-100ms clone time confirmed.

- **test-suite**: E2E integration tests with real Docker + tmux - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-test-suite.md
  - Evidence: cargo build/test/clippy/fmt all pass, 9 e2e tests (3 clone + 4 Docker + 2 tmux)
  - Notes: Tests cover: clone lifecycle, container lifecycle, exit code preservation, bind mount visibility, network isolation (2 containers on same port), tmux session lifecycle, tmux send-keys. Docker/tmux tests use #[ignore] and run with --ignored flag.

- **ci-pipeline**: GitHub Actions workflow for CI/CD - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-ci-pipeline.md
  - Evidence: Workflow YAML valid, all local checks pass
  - Notes: Three jobs: check (fmt + clippy + build), unit-tests (cargo test), e2e-tests (Docker + tmux + --include-ignored). Pre/post cleanup sweeps. Cargo caching. tmux installed via apt.

## Failed Modules

[None]

## Unbuilt Modules

### Layer 4 - E2E Test Infrastructure (close the loop)

Architecture basis: e2e-ci-environment (#25), e2e-test-isolation (#26), e2e-local-fixture-repo (#27)

[All modules built]

## Iteration Log

- 1: "cli" → BUILT (plans/2026-02-13-BUILD-cli.md)
- 2: "config" → BUILT (plans/2026-02-13-BUILD-config.md)
- 3: "clone" → BUILT (plans/2026-02-13-BUILD-clone.md)
- 4: "container" → BUILT (plans/2026-02-13-BUILD-container.md)
- 5: "shell" → BUILT (plans/2026-02-13-BUILD-shell.md)
- 6: "tmux" → BUILT (plans/2026-02-13-BUILD-tmux.md)
- 7: "wire-cli" → BUILT (plans/2026-02-13-BUILD-wire-cli.md)
- 8: "end-to-end" → BUILT (plans/2026-02-13-BUILD-end-to-end.md)
- 9: "proxy" → BUILT (plans/2026-02-13-BUILD-proxy.md)
- 10: "test-harness" → BUILT (plans/2026-02-13-BUILD-test-harness.md)
- 11: "test-fixture" → BUILT (plans/2026-02-13-BUILD-test-fixture.md)
- 12: "test-suite" → BUILT (plans/2026-02-13-BUILD-test-suite.md)
- 13: "ci-pipeline" → BUILT (plans/2026-02-13-BUILD-ci-pipeline.md)
