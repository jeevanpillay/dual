---
date: 2026-02-13T00:00:00Z
researcher: Claude
git_commit: cab4d14
branch: feature/build-loop-pipeline
repository: dual
topic: "Clone module research for MVP build"
tags: [research, codebase, clone, git, build-loop]
status: complete
last_updated: 2026-02-13
last_updated_by: Claude
---

# Research: Clone Module for Dual MVP

## Research Question

What does the clone module need to do and how should it be implemented?

## Summary

The clone module manages full git clones — one per workspace. It uses the config module's RepoConfig to know what to clone and where. Key operations: create clone, check if exists, list existing clones, remove clone.

## Key Requirements (from SPEC.md)

1. Full clones (NOT git worktrees) — each workspace has independent .git
2. Remote URLs → standard `git clone <url>`
3. Local paths → `git clone --local <path>` (hardlinks for speed)
4. Branch checkout: `git clone -b <branch>` or clone + checkout
5. Filesystem layout: `~/dual-workspaces/{repo}/{branch}/`
6. Each clone is completely independent — no shared state

## Implementation Approach

- Use `std::process::Command` to shell out to `git`
- Detect local vs remote by checking if URL is a filesystem path
- Clone into config.workspace_dir(repo, branch)
- Check for existing clones before creating
- Provide clone status (exists, missing) for list command

## Architecture Constraints

- full-clone-no-contention (CONFIRMED WITH CAVEATS): Full clones avoid lock contention. Lock contention risk overstated for worktrees but branch constraint is valid.
- Each clone dir has its own .git/, node_modules, build artifacts, env files
