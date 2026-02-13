---
spec_source: SPEC.md
arch_source: thoughts/ARCHITECTURE.md
date_started: 2026-02-13
status: in_progress
build_progress: 9/10
---

# Dual MVP Build

This document tracks the implementation of Dual's MVP modules, informed by validated architectural decisions from ARCHITECTURE.md.

## Architecture Reference

Source: `thoughts/ARCHITECTURE.md`
Status: Complete (24/24 validated)

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

## Failed Modules

[None]

## Unbuilt Modules

- **e2e-pipeline**: Isolated E2E test pipeline (local + CI)
  - Depends on: all MVP modules (exercises full lifecycle)
  - Architecture claims: e2e-ci-environment (#25), e2e-test-isolation (#26), e2e-local-fixture-repo (#27)
  - Deliverables:
    1. `tests/e2e.rs` — Rust integration tests exercising full workspace lifecycle
    2. Test harness with RAII cleanup guards (Docker containers, tmux sessions, temp dirs)
    3. Local git fixture repos (no network dependency)
    4. `.github/workflows/test.yml` — CI workflow with Docker + tmux
  - Blocks: nothing (closes the testing loop)

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
