---
date: 2026-02-13T00:00:00Z
researcher: Claude
git_commit: cab4d14
branch: feature/build-loop-pipeline
repository: dual
topic: "Tmux module research for MVP build"
tags: [research, codebase, tmux, session, build-loop]
status: complete
last_updated: 2026-02-13
last_updated_by: Claude
---

# Research: Tmux Module for Dual MVP

## Summary

The tmux module implements the runtime backend contract from SPEC.md using tmux as the session manager. Each workspace gets a tmux session with configurable panes.

## Key Requirements

1. Create tmux sessions named after workspaces (dual-{repo}-{branch})
2. Attach/detach sessions (native tmux operations)
3. Destroy sessions (kill-session)
4. Check if session is alive (has-session)
5. List all dual-managed sessions
6. Set working directory to workspace clone dir
7. Source shell RC in pane for command interception

## tmux Commands

- `tmux new-session -d -s {name} -c {cwd}` — create detached session
- `tmux attach-session -t {name}` — attach to session
- `tmux detach-client -s {name}` — detach from session
- `tmux kill-session -t {name}` — destroy session
- `tmux has-session -t {name}` — check if exists (exit code 0 = exists)
- `tmux list-sessions -F '#{session_name}'` — list all sessions
- `tmux send-keys -t {name} '{cmd}' Enter` — send command to pane

## Session Initialization

When creating a new session:
1. Create detached session with CWD = workspace clone dir
2. Send shell RC sourcing command to the pane
3. Optionally send initial commands (e.g. pnpm dev)

## Architecture Constraints

- tmux-backend-viable (CONFIRMED): Designed for this use case, 100+ concurrent sessions
- progressive-enhancement (CONFIRMED): If tmux not available, still works (BasicBackend)

## MVP Scope

Implement TmuxBackend only. BasicBackend and ZellijBackend are future work.
tmux availability check for progressive enhancement.
