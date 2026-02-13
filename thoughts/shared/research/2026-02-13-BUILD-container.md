---
date: 2026-02-13T00:00:00Z
researcher: Claude
git_commit: cab4d14
branch: feature/build-loop-pipeline
repository: dual
topic: "Container module research for MVP build"
tags: [research, codebase, container, docker, build-loop]
status: complete
last_updated: 2026-02-13
last_updated_by: Claude
---

# Research: Container Module for Dual MVP

## Summary

The container module manages Docker container lifecycle for workspaces. Each workspace gets one container with bind-mounted source code, isolated node_modules, and its own network namespace.

## Key Requirements

1. Container naming: `dual-{repo}-{encoded_branch}`
2. Create container: docker create with bind mount, node_modules volume, network isolation
3. Start/stop/destroy: docker start/stop/rm
4. Exec: docker exec with exit code preservation, CWD, env, TTY support
5. Status check: is container running?
6. List containers: find all dual-managed containers

## Docker Commands

- `docker create --name dual-{repo}-{branch} -v {workspace}:/workspace -v /workspace/node_modules -w /workspace {image}`
- `docker start dual-{repo}-{branch}`
- `docker stop dual-{repo}-{branch}`
- `docker rm dual-{repo}-{branch}`
- `docker exec [-t] [-w dir] [-e KEY=VAL] dual-{repo}-{branch} <cmd>`
- `docker ps -a --filter name=dual- --format '{{.Names}}\t{{.Status}}'`

## Architecture Constraints

- docker-exec-basic: Exit codes preserved. -e for env, -w for CWD, -t for TTY.
- bind-mount-visibility: ~200ms latency on macOS (VirtioFS). Acceptable.
- container-network-isolation: Bridge mode (default). Kernel-level guarantee.
- node-modules-isolation: Anonymous volume at /workspace/node_modules shadows bind mount.
- monorepo-single-container: One container per workspace. Services share localhost inside.

## MVP Scope

For MVP, use a pre-existing base image (e.g. node:20). Auto-image generation is deferred.
