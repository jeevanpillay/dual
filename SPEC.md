---
date_created: 2026-02-05
author: Human
status: validated
validation_progress: 27/27
claims_extracted: true
last_validated: 2026-02-13
last_validated_by: Claude
---

# Dual — Engineering Requirements (Dual)

Terminal workspace orchestrator for parallel multi-repo development with AI coding agents.

---

## CRITICAL: The Core Invariant

**Claude Code must never know it is running inside a container.**

Every architectural decision flows from this constraint. Claude runs `pnpm dev`. Claude curls `localhost:3000`. Claude runs `npx playwright test`. All of these must work exactly as they would on a bare metal machine.

- DO NOT expose Docker to Claude Code — Claude never sees containers, images, or port mappings
- DO NOT require Claude to check environment variables, remember mapped ports, or do anything non-standard
- DO NOT leak container abstractions through error messages, paths, or process names
- ONLY present an environment where `localhost:{port}` is the real service, because it is — the command executes inside the container's network namespace
- If Claude has to remember anything about the infrastructure, the system has failed

---

## CRITICAL: The Equation

**One monorepo worktree = one container = one machine.**

- DO NOT split monorepo services into separate containers
- DO NOT use docker-compose per project
- DO NOT create per-service isolation within a worktree
- ONLY create one container per worktree clone
- The developer runs `pnpm dev` and the monorepo's own tooling (turbo, nx) starts all services inside that single container on their default ports
- All services share the same `localhost` inside the container — they talk to each other normally

---

## CRITICAL: Parallel Execution

All workspaces run simultaneously. There is no concept of a single "active" workspace.

- DO NOT design around one workspace being active at a time
- DO NOT require stopping one workspace to start another
- DO NOT share state, ports, or resources between workspaces
- 5 repos × 3 worktrees each = 15 simultaneous environments, all with Claude Code sessions active, all running dev servers on default ports, with zero conflicts
- Each container has its own network namespace — 15 containers can all bind `:3000` simultaneously

---

## Architecture Overview

Dual is the orchestration layer between the terminal emulator and the runtime. It does not replace tmux. It does not replace Docker. It replaces how the developer interacts with both.

### System Layers (outermost → innermost)

| Layer | What It Is | Who Owns It |
|---|---|---|
| Terminal Emulator | Ghostty, iTerm, Kitty, Alacritty — provides the rendering surface | User's choice |
| **Dual** | Workspace switching, clone lifecycle, container lifecycle, command routing, reverse proxy | **This project** |
| Isolation Layer | Docker — one headless container per worktree, all services inside | Docker (orchestrated by Dual) |
| Reverse Proxy | Dual subprocess — maps `*.localhost:{port}` to containers | **This project** |
| Runtime Backend | tmux/zellij on the host — keeps dev tool panes alive across workspace switches | tmux (orchestrated by Dual) |
| Workspace Processes | Split: dev tools on host (nvim, claude, git) · all services in container (`pnpm dev`) | Split ownership |

### Two Audiences, Two Paths

There are exactly two consumers of a workspace's running services. They access them differently.

**Claude Code (and all CLI tools) → Container Network**

All runtime commands execute inside the container via `dual run` → `docker exec`. Inside the container, `localhost:3000` is the real service. 15 Claude sessions can all run `pnpm dev` on `:3000` simultaneously because each container has its own network namespace.

**Developer (browser) → Multi-Port Reverse Proxy**

Dual runs a lightweight reverse proxy on the host. The proxy listens on all ports that any container is using. Subdomain selects the container: `lightfast-feat-auth.localhost:3001` routes to the `dual-lightfast-feat-auth` container's `:3001`. `*.localhost` resolves to `127.0.0.1` natively in all modern browsers — no `/etc/hosts`, no DNS config, no browser extensions.

---

## Command Routing

Dual maintains a routing table that determines where each command executes.

### The Rule: File Operations → Host. Runtime Operations → Container.

| Command | Executes On | Why |
|---|---|---|
| `edit src/app.tsx` | Host | Bind mount — container sees the change via shared filesystem |
| `git commit -m "fix"` | Host | Uses developer's credentials and SSH keys |
| `cat package.json` | Host | Read-only file operation |
| `pnpm dev` | Container | Binds ports — must be in isolated network namespace |
| `pnpm install` | Container | Writes to `node_modules` inside container |
| `curl localhost:3000` | Container | Needs access to the container's network |
| `npx playwright test` | Container | Needs both browser runtime and network access |

### Defaults and Configuration

- Anything `npm`/`pnpm`/`node`/`python`/`curl` → container
- Anything `git`/`cat`/`ls`/`vim`/`nvim` → host
- Configurable per project via workspace config
- The mechanism: `dual run <command>` wraps `docker exec <container-name> <command>`
- Claude Code's shell is configured so that commands are intercepted and routed — Claude never sees `dual run` or `docker exec`

---

## Git Strategy

### CRITICAL: Full Clones, Not Worktrees

Every workspace is a full `git clone`. Each has its own `.git/` directory, its own `node_modules`, its own build artifacts, its own env files.

- DO NOT use `git worktree` — shared `.git` object store creates lock contention when 15 sessions are active
- DO NOT share refs across workspaces — one bad rebase can corrupt all worktrees
- DO NOT share stash, index, or configs across workspaces

**Why not worktrees:**
- Shared `.git` object store creates lock contention with 15 active sessions
- Shared refs mean one bad rebase can corrupt all worktrees
- Cannot have two worktrees on the same branch
- Stash, index, and some configs are shared — causes unexpected side effects
- Claude modifying `.git` state in one worktree can break another

**Clone strategy:**
- Remote origin → `git clone <url>` (standard network clone)
- Local path → `git clone --local <path>` (uses hardlinks — fast, low disk overhead)
- Optional `--depth N` for large repos where full history is not needed

### Filesystem Layout

```
~/dual-workspaces/
├─ lightfast-platform/          ← project (repo)
│  ├─ main/                     ← full clone, own .git, own node_modules
│  ├─ feat__auth/               ← full clone, completely independent
│  └─ fix__memory-leak/         ← full clone
└─ agent-os/                    ← project (repo)
   ├─ main/
   └─ v2-rewrite/
```

- Branch names use double-underscore as separator (`feat/auth` → `feat__auth`) because filesystem paths cannot contain `/`
- Each clone directory is completely independent — no shared state of any kind

---

## Container Lifecycle

### Zero-Config Image Generation

Dual inspects the monorepo and auto-builds a minimal container image.

- DO NOT require Dockerfiles from the developer
- DO NOT include dev tools (nvim, claude, git) in the container — those stay on the host
- ONLY install what's needed to run `pnpm dev`

| Detects | Provisions in Container |
|---|---|
| `.nvmrc` / `.node-version` | Node.js (exact version) |
| `pnpm-lock.yaml` | pnpm + turbo/nx |
| `pyproject.toml` | Python + uv/pip |
| `docker-compose.yml` | Parses service definitions → postgres, redis, etc. |
| `playwright.config.*` | Chromium + browser dependencies |
| `.env` / `.env.local` | Environment variables |
| `turbo.json` / `nx.json` | Monorepo task runner |

### Image Caching

- Image is cached per monorepo — all worktrees of the same repo share the same base image
- Rebuilt only when dependencies change (lockfile hash, node version, etc.)

### Container Naming

Pattern: `dual-{repo}-{branch}`

Examples:
- `dual-lightfast-main`
- `dual-lightfast-feat-auth`
- `dual-agent-os-feat-memory`

### Bind Mount

The worktree directory on the host is bind-mounted into the container:
- File edits on the host (via nvim or Claude) are immediately visible inside the container
- The dev server's hot reload picks up changes instantly
- `node_modules` and build artifacts live inside the container's filesystem
- The source tree is shared; the runtime state is isolated

---

## Reverse Proxy

### CRITICAL: Multi-Port Subdomain Routing

The proxy does not run on a single port. It binds to every port that any container is using.

- DO NOT use a single gateway port with path-based routing
- DO NOT require the developer to remember port mappings
- DO NOT require any host-side configuration (no `/etc/hosts`, no DNS, no browser extensions)
- ONLY use subdomain + port to select the target: `{repo}-{branch}.localhost:{port}` → container's `:{port}`

### URL Pattern

```
{repo}-{branch}.localhost:{port}
```

### How It Works

1. Proxy is a Dual subprocess — starts automatically when Dual starts, zero config
2. When a container starts and binds ports (`:3000`, `:3001`, `:4001`), Dual registers those ports with the proxy
3. Proxy starts listening on any newly registered ports (if not already listening)
4. Request arrives at `lightfast-feat-auth.localhost:3001`:
   - Proxy reads the `Host` header → `lightfast-feat-auth.localhost`
   - Proxy reads the port → `:3001`
   - Proxy looks up the container → `dual-lightfast-feat-auth`
   - Proxy forwards the request to that container's `:3001`
5. When containers stop, their port registrations are removed. If no other container uses that port, the proxy stops listening on it.

### Requirements

- MUST support HTTP and WebSocket (for hot reload / HMR)
- MUST support SSE (Server-Sent Events) and streaming responses
- MUST update routing table dynamically as containers start and stop
- MUST handle concurrent connections to multiple containers simultaneously
- Implementation: Caddy, Traefik, or a custom ~150 line Go/Node HTTP proxy

### Example: Full URL Table

For 2 monorepos × 3 worktrees each (6 containers, 18+ services):

```
lightfast-main.localhost:3000           → www (marketing site)
lightfast-main.localhost:3001           → app (dashboard)
lightfast-main.localhost:4001           → api (backend)
lightfast-main.localhost:3002           → docs

lightfast-feat-auth.localhost:3000      → www
lightfast-feat-auth.localhost:3001      → app
lightfast-feat-auth.localhost:4001      → api
lightfast-feat-auth.localhost:3002      → docs

lightfast-fix-billing.localhost:3000    → www
lightfast-fix-billing.localhost:3001    → app
lightfast-fix-billing.localhost:4001    → api
lightfast-fix-billing.localhost:3002    → docs

agent-os-main.localhost:8080            → runtime (engine)
agent-os-main.localhost:3000            → studio (visual builder)
agent-os-main.localhost:9000            → gateway (API)

agent-os-feat-memory.localhost:8080     → runtime
agent-os-feat-memory.localhost:3000     → studio
agent-os-feat-memory.localhost:9000     → gateway

agent-os-perf-hotpath.localhost:8080    → runtime
agent-os-perf-hotpath.localhost:3000    → studio
agent-os-perf-hotpath.localhost:9000    → gateway
```

All on default ports inside their containers. All accessible simultaneously from the developer's browser.

---

## Runtime Backend Contract

The runtime backend is an abstraction over the terminal multiplexer. Dual ships with a tmux backend as the default, but the interface is designed to be swappable.

### Interface

| Method | Signature | Description |
|---|---|---|
| `create_session` | `(workspace_id, processes[])` | Create a new runtime session with the given processes. Returns a session handle. |
| `attach` | `(session_handle)` | Connect the current terminal to this session. User sees the processes. |
| `detach` | `(session_handle)` | Disconnect terminal from session. Processes keep running in background. |
| `destroy` | `(session_handle)` | Kill all processes and tear down the session. |
| `is_alive` | `(session_handle) → bool` | Check if the session still has running processes. |
| `list_sessions` | `() → session_handle[]` | Return all managed sessions. |

### Implementations

| Backend | Status | Description |
|---|---|---|
| `TmuxBackend` | Default | Each session = tmux session. Processes = tmux panes. Attach/detach is native tmux. |
| `ZellijBackend` | Future | Same model, different multiplexer. |
| `BasicBackend` | Fallback | Background processes only. No multiplexing or panes. |

### Progressive Enhancement

If a user doesn't have tmux, Dual still works. They get clone management, workspace switching, container lifecycle, and the reverse proxy — just without pane layouts. The `BasicBackend` is the floor. Multiplexer backends are progressive enhancements.

---

## User Experience

### First Launch

1. Developer opens terminal emulator
2. Runs `dual` — it becomes their session
3. Dual shows fuzzy picker across all projects/workspaces
4. Developer selects a workspace
5. Dual: creates clone if needed → starts container → creates tmux session → attaches
6. Developer lands in their workspace with nvim, claude, and shell running

### Workspace Switching (The Meta-Key Moment)

1. Developer presses the meta key
2. Current workspace is "frozen" (detached from runtime — processes keep running)
3. Dual overlay appears with fuzzy picker
4. Shows: project name, branch, session status (● running / ○ stopped)
5. Developer types to filter, presses enter to select
6. Dual attaches to the new workspace's runtime session
7. If no session exists, Dual creates one (clone + container + launch processes)
8. Loops back — developer can switch again at any time

The developer never leaves Dual's control. They never run `tmux attach`. They never run `docker exec`. They never manage sessions manually.

### CLI Commands

```
dual                          → launch (shows fuzzy picker)
dual open                     → opens all services for current workspace in browser
dual open lightfast-main      → opens all lightfast-main services in browser
dual urls                     → lists all running workspace URLs
dual urls lightfast-feat-auth → URLs for that specific workspace
```

#### `dual urls` Output

```
lightfast-feat-auth  ● running
  www      lightfast-feat-auth.localhost:3000
  app      lightfast-feat-auth.localhost:3001
  api      lightfast-feat-auth.localhost:4001
  docs     lightfast-feat-auth.localhost:3002

agent-os-feat-memory  ● running
  studio   agent-os-feat-memory.localhost:3000
  runtime  agent-os-feat-memory.localhost:8080
  gateway  agent-os-feat-memory.localhost:9000
```

---

## Workspace States

| State | Description |
|---|---|
| **ATTACHED** | Runtime session is connected to your terminal. You see and interact with processes. |
| **BACKGROUND** | Processes still running (Claude is thinking, dev server is serving) but not displayed. |
| **STOPPED** | No runtime session. Clone exists on disk. Session created on first access. |
| **LAZY** | Config-only. No clone on disk. Clone + session created on first access. |

---

## What Dual Does NOT Do

- Does NOT replace tmux/zellij — it orchestrates them
- Does NOT replace Docker — it orchestrates it
- Does NOT require Dockerfiles — it auto-generates container images
- Does NOT expose Docker to Claude Code — Claude never knows containers exist
- Does NOT require port remapping awareness from any tool
- Does NOT require `/etc/hosts` changes or DNS configuration
- Does NOT run dev tools (nvim, claude, git) inside containers — those stay on the host
- Does NOT manage a single "active" workspace — all workspaces run simultaneously

---

## Implementation Phases

### Phase 1: Core Loop
- Workspace config format and discovery
- Full clone management (`git clone --local`)
- tmux backend (create, attach, detach, destroy)
- Meta-key workspace switcher with fuzzy picker
- Filesystem layout (`~/dual-workspaces/{repo}/{branch}/`)

### Phase 2: Isolation
- Container image auto-generation from project files
- Container lifecycle management (create, start, stop, destroy)
- Bind mount configuration
- Command routing (`dual run` → `docker exec`)
- Shell integration for transparent command interception

### Phase 3: Network Access
- Multi-port reverse proxy
- Dynamic port registration as containers start/stop
- Subdomain routing (`*.localhost`)
- WebSocket and SSE support
- `dual open` and `dual urls` commands

### Phase 4: Polish
- TUI with workspace sidebar showing live status
- `dual urls` in TUI sidebar
- Container image caching and rebuild detection
- Zellij backend
- `BasicBackend` for users without a multiplexer
