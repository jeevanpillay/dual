# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What Is Dual

Dual is a terminal workspace orchestrator for parallel multi-repo development with AI coding agents. It manages isolated development environments — one full git clone per workspace — so a developer can run multiple repos × multiple branches simultaneously, all with Claude Code sessions active, all running dev servers on default ports, with zero conflicts.

**The Core Invariant**: Claude Code must never need to know that commands are routed to containers. It runs `pnpm dev`, curls `localhost:3000`, runs tests — all transparently intercepted and executed in the right place.

## Development Commands

```bash
cargo build              # Build debug binary
cargo build --release    # Build release binary
cargo run                # Run the binary
cargo test               # Run tests
cargo clippy             # Run linter
cargo fmt                # Format code
```

## Architecture Overview

**One workspace = one full clone = one container**

- Dev tools (nvim, claude, git, shell) → run on **host** with your config/credentials
- Runtime processes (pnpm dev, node, curl localhost) → run in **container** via transparent interception
- Browser access → `{repo}-{branch}.localhost:{port}` routes through reverse proxy to container
- tmux/zellij → runs on host, manages terminal sessions across workspace switches

## Project Structure

```
src/           # Rust source code
experiments/   # Isolated experiment directories
thoughts/      # Research and design documents
.claude/       # Claude Code commands and agents
```

## Technical Guidelines

### Rust
- Edition 2024
- Standard Rust idioms
- Use `cargo clippy` before committing
- Target platforms: Linux, macOS (Intel + Apple Silicon), Windows

### Command Routing Rule
File operations → Host. Runtime operations → Container.

| Host | Container |
|------|-----------|
| git, cat, ls, vim | npm, pnpm, node, python |
| File reads/writes | Port-binding processes |
| SSH, credentials | curl localhost, tests |

## Experiment Framework

The `.claude/` directory contains a hypothesis-driven experiment framework:

- `/research_experiment` — Research feasibility and unknowns
- `/create_experiment` — Design methodology and success criteria
- `/run_experiment` — Execute and capture measurements
- `/validate_experiment` — Compare results vs research predictions

Research outputs go to `thoughts/shared/research/`. Experiment artifacts go to `experiments/{slug}/`.
