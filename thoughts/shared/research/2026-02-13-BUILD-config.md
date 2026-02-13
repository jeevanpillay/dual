---
date: 2026-02-13T00:00:00Z
researcher: Claude
git_commit: cab4d14
branch: feature/build-loop-pipeline
repository: dual
topic: "Config module research for MVP build"
tags: [research, codebase, config, toml, build-loop]
status: complete
last_updated: 2026-02-13
last_updated_by: Claude
---

# Research: Config Module for Dual MVP

## Research Question

What configuration structure does Dual need and how should it be discovered?

## Summary

The config module parses `dual.toml` files containing repo definitions with branch lists, optional image overrides, and optional command routing overrides. Discovery searches current dir then `~/.config/dual/`. The workspace_root defaults to `~/dual-workspaces/`.

## Key Findings

### Explicit from SPEC.md
- Filesystem layout: `~/dual-workspaces/{repo}/{branch}/`
- Container naming: `dual-{repo}-{branch}`
- Branch encoding: `feat/auth` → `feat__auth`
- Command routing defaults: npm/pnpm/node/python/curl → container, git/cat/ls/vim/nvim → host
- Command routing is configurable per project
- Image generation is auto-detected but overridable
- LAZY workspace state = config-only, no clone on disk

### Dependencies needed
- `serde` + `serde_derive` for serialization
- `toml` for TOML parsing
- `dirs` for home directory resolution

### Config structure (inferred from SPEC)
```toml
workspace_root = "~/dual-workspaces"  # optional, has default

[[repos]]
name = "lightfast-platform"
url = "git@github.com:org/lightfast.git"
branches = ["main", "feat/auth"]
```
