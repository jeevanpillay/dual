---
spec_source: SPEC.md
arch_source: thoughts/ARCHITECTURE.md
date_started: 2026-02-13
status: complete
build_progress: 6/6
date_completed: 2026-02-13
---

# Dual MVP Build

This document tracks the implementation of Dual's MVP modules, informed by validated architectural decisions from ARCHITECTURE.md.

## Architecture Reference

Source: `thoughts/ARCHITECTURE.md`
Status: Complete (24/24 validated)

## Built Modules

- **cli**: Entry point, arg parsing (`dual`, `dual list`, `dual destroy`, `dual open`, `dual urls`) - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-cli.md
  - Evidence: cargo build/test/clippy/fmt all pass, 8 unit tests, all subcommands produce correct output
  - Notes: Stub handlers only — real implementations come from downstream modules. Uses clap v4 derive macros.

- **config**: Workspace config parsing from `dual.toml` - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-config.md
  - Evidence: cargo build/test/clippy/fmt all pass, 11 unit tests, full TOML parsing, validation, path generation
  - Notes: Supports repo definitions with branches, workspace_root config, branch encoding (feat/auth → feat__auth), container naming, config discovery (cwd → ~/.config/dual/). Dead code warnings expected until downstream modules consume config.

- **clone**: Full git clone management (`git clone`, `git clone --local`) - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-clone.md
  - Evidence: cargo build/test/clippy/fmt all pass, 5 unit tests, local/remote detection, clone command construction
  - Notes: Detects local vs remote URLs. Local paths use --local flag for hardlink clones. Creates parent dirs, checks for existing clones, supports removal. Uses config module for path generation.

- **container**: Docker container lifecycle (create, start, stop, destroy, exec) - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-container.md
  - Evidence: cargo build/test/clippy/fmt all pass, 5 unit tests, command construction for create/exec verified
  - Notes: Bind mount workspace to /workspace, anonymous volume for node_modules isolation, docker exec with TTY/CWD support, container status detection, list all dual-managed containers. Default image node:20 for MVP.

- **shell**: Shell RC generation + command routing (classify + intercept) - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-shell.md
  - Evidence: cargo build/test/clippy/fmt all pass, 8 unit tests, classification and RC generation verified
  - Notes: Generates bash/zsh-compatible shell functions for npm/npx/pnpm/node/python/curl/make. TTY detection in generated functions. Classifies commands as host vs container. Exports DUAL_CONTAINER env var.

- **tmux**: tmux session management (create, attach, detach, destroy, list) - BUILT
  - Plan: thoughts/shared/plans/2026-02-13-BUILD-tmux.md
  - Evidence: cargo build/test/clippy/fmt all pass, 5 unit tests, session naming matches container naming
  - Notes: Creates detached sessions with CWD, sources shell RC for command interception, checks tmux availability, lists dual-managed sessions. Session names match container names (dual-{repo}-{branch}).

## Failed Modules

[Modules that failed implementation — needs rework]

## Unbuilt Modules

Modules extracted from ARCHITECTURE.md and SPEC.md, organized by dependency order:

### Layer 1 - Foundations (no upstream dependencies)

1. ~~**cli**: Entry point, arg parsing~~ → BUILT (see Built Modules)

2. ~~**config**: Workspace config format and discovery~~ → BUILT (see Built Modules)

### Layer 2 - Core Mechanisms (depend on foundations)

3. ~~**clone**: Full clone management~~ → BUILT (see Built Modules)

4. ~~**container**: Docker lifecycle~~ → BUILT (see Built Modules)

### Layer 3 - Integration (compose mechanisms)

5. ~~**shell**: RC injection + command routing~~ → BUILT (see Built Modules)

6. ~~**tmux**: Session management~~ → BUILT (see Built Modules)

## Iteration Log

- 1: "cli" → BUILT (plans/2026-02-13-BUILD-cli.md)
- 2: "config" → BUILT (plans/2026-02-13-BUILD-config.md)
- 3: "clone" → BUILT (plans/2026-02-13-BUILD-clone.md)
- 4: "container" → BUILT (plans/2026-02-13-BUILD-container.md)
- 5: "shell" → BUILT (plans/2026-02-13-BUILD-shell.md)
- 6: "tmux" → BUILT (plans/2026-02-13-BUILD-tmux.md)
