---
date: 2026-02-13T00:00:00Z
researcher: Claude
git_commit: cab4d14
branch: feature/build-loop-pipeline
repository: dual
topic: "Shell module research for MVP build"
tags: [research, codebase, shell, interception, routing, build-loop]
status: complete
last_updated: 2026-02-13
last_updated_by: Claude
---

# Research: Shell Module for Dual MVP

## Summary

The shell module generates shell RC content that intercepts runtime commands and routes them to docker exec inside the workspace container. It classifies commands as host or container operations.

## Key Requirements

1. Generate shell functions that override: npm, pnpm, npx, node, python, python3, pip, curl
2. Each function wraps: docker exec [-t] -w /workspace <container> <cmd> "$@"
3. TTY detection: pass -t to docker exec when stdout is a terminal
4. Host commands pass through unchanged: git, cat, ls, vim, nvim, etc.
5. RC content is a string that gets sourced in tmux pane shells
6. Configurable per-project overrides (from config module)

## Shell Function Pattern

```bash
npm() {
    if [ -t 1 ]; then
        docker exec -t -w /workspace dual-lightfast-main npm "$@"
    else
        docker exec -w /workspace dual-lightfast-main npm "$@"
    fi
}
```

## Architecture Constraints

- shell-interception (CONFIRMED WITH CAVEATS): Absolute paths bypass functions (acceptable)
- shell-interception-transparency (CONFIRMED WITH CAVEATS): Platform leak (linux vs darwin) unfixable
- command-routing-accuracy (CONFIRMED): Classification is sound

## Default Routing Table

Container: npm, npx, pnpm, node, python, python3, pip, pip3, curl, make
Host: git, cat, ls, vim, nvim, less, head, tail, grep, find, ssh, scp
