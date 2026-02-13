---
spec_source: SPEC.md
arch_source: thoughts/ARCHITECTURE.md
date_started: 2026-02-13
status: in_progress
build_progress: 0/6
---

# Dual MVP Build

This document tracks the implementation of Dual's MVP modules, informed by validated architectural decisions from ARCHITECTURE.md.

## Architecture Reference

Source: `thoughts/ARCHITECTURE.md`
Status: Complete (24/24 validated)

## Built Modules

[Modules implemented and verified]

## Failed Modules

[Modules that failed implementation — needs rework]

## Unbuilt Modules

Modules extracted from ARCHITECTURE.md and SPEC.md, organized by dependency order:

### Layer 1 - Foundations (no upstream dependencies)

1. **cli**: Entry point, arg parsing (`dual`, `dual list`, `dual destroy`)
   - Depends on: nothing
   - Blocks: everything (entry point)

2. **config**: Workspace config format and discovery (`dual.toml`)
   - Depends on: nothing
   - Blocks: clone, container, shell, tmux

### Layer 2 - Core Mechanisms (depend on foundations)

3. **clone**: Full clone management (`git clone --local`, filesystem layout)
   - Depends on: config (workspace definitions)
   - Blocks: container (needs clone dir for bind mount)
   - Architecture: full-clone-no-contention (CONFIRMED WITH CAVEATS)

4. **container**: Docker lifecycle (create, start, stop, destroy)
   - Depends on: config (image config), clone (bind mount source)
   - Blocks: shell (needs container target for docker exec)
   - Architecture: docker-exec-basic (CONFIRMED), bind-mount-visibility (CONFIRMED WITH CAVEATS), container-network-isolation (CONFIRMED), node-modules-isolation (CONFIRMED)

### Layer 3 - Integration (compose mechanisms)

5. **shell**: RC injection + command routing (shell functions → docker exec)
   - Depends on: container (needs running container to route to)
   - Blocks: tmux (shell interceptors must be active in tmux panes)
   - Architecture: shell-interception (CONFIRMED WITH CAVEATS), shell-interception-transparency (CONFIRMED WITH CAVEATS), command-routing-accuracy (CONFIRMED)

6. **tmux**: Session create/attach/detach/destroy + workspace switching
   - Depends on: shell (panes need interceptors), clone (CWD), container (lifecycle)
   - Blocks: nothing (terminal module)
   - Architecture: tmux-backend-viable (CONFIRMED), progressive-enhancement (CONFIRMED)

## Iteration Log

[Record of each build iteration]
