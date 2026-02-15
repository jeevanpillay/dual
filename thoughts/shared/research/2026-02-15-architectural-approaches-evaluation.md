---
date: 2026-02-15T04:34:36Z
researcher: Claude
git_commit: c2c8192a469bd88c8a5beea67c9cdf5f2cbfb6ea
branch: main
repository: dual
topic: "Evaluation of architectural approaches for dual - centralized config vs per-repo tunnel vs standalone conductor vs hybrid"
tags: [research, architecture, design-evaluation, developer-experience, config, dual-toml, workspace-management]
status: complete
last_updated: 2026-02-15
last_updated_by: Claude
---

# Research: Architectural Approaches for Dual

**Date**: 2026-02-15T04:34:36Z
**Researcher**: Claude
**Git Commit**: c2c8192a469bd88c8a5beea67c9cdf5f2cbfb6ea
**Branch**: main
**Repository**: dual

## Research Question

What are the different architectural approaches for dual's design, and what does the developer experience look like for each? Evaluate: centralized config-driven (current), per-repo tunnel/plugin, standalone conductor with state, hybrid (per-repo declaration + centralized state), git-native, and shell-native activation patterns. Determine which architecture best serves dual's core mission.

## Summary

Six distinct architectural approaches exist for dual. Each makes different tradeoffs around where config lives, who manages state, and how the developer interacts with the tool. The current approach (centralized config-driven) is the simplest but creates friction for dynamic workflows. The per-repo tunnel pattern is familiar but fragmenting. The standalone conductor is powerful but heavyweight. The hybrid approach (per-repo runtime hints + centralized workspace state) captures the best tradeoffs — it mirrors the Terraform config/state split that has become an industry standard for managing declarative infrastructure.

---

## The Approaches

### Approach 1: Centralized Config-Driven (Current Implementation)

**Mental Model**: "One file declares everything. Edit the file, then launch."

**How it works today** (from `src/config.rs:7-32`, `src/main.rs:52-135`):
- Single `dual.toml` at project root or `~/.config/dual/dual.toml`
- Config declares repos, URLs, branches, and ports
- All workspace identity derived from `(repo.name, branch)` tuple
- No runtime state file — config IS the registry
- To add a workspace: edit `dual.toml`, then `dual launch`

**Config example** (current `dual.toml`):
```toml
workspace_root = "~/dual-workspaces"

[[repos]]
name = "lightfast"
url = "git@github.com:org/lightfast.git"
branches = ["main", "feat/auth", "fix/memory-leak"]
ports = [3000, 3001]

[[repos]]
name = "agent-os"
url = "/local/path/to/agent-os"
branches = ["main", "v2-rewrite"]
```

**Developer Experience**:

| Journey | Experience |
|---------|-----------|
| First-time setup | Create `dual.toml` manually, list repos/branches/ports |
| Add a new repo | Edit `dual.toml`, add `[[repos]]` block |
| New branch workspace | Edit `dual.toml`, add branch to array, then launch |
| Launch workspace | `dual launch lightfast-feat__auth` |
| Switch workspace | Detach tmux, launch different workspace |
| Onboard teammate | Share `dual.toml` (or they write their own) |
| Move between machines | Copy `dual.toml`, re-launch |
| Multi-repo | All repos in same file — natural |

**Strengths**:
- Simplest mental model: one file, everything explicit
- No hidden state — what you see is what you get
- Easy to version control (check `dual.toml` into a dotfiles repo)
- Deterministic: same config = same workspaces everywhere

**Tensions**:
- Branch lists go stale — finished branches stay in config
- Adding a branch requires editing config before launching (two-step)
- Config mixes concerns: project runtime hints (image, ports) with ephemeral workspace state (which branches are active)
- No `dual create <branch>` command exists (`src/config.rs:63-73` — you cannot launch what isn't in config)
- Doesn't scale to "explore a branch quickly, throw it away"

**Comparable tools**: tmuxinator (centralized YAML per session), Vagrantfile (per-project but monolithic)

---

### Approach 2: Per-Repo Tunnel / Plugin

**Mental Model**: "Install dual into each repo. The repo declares what it needs."

**How it would work**:
- Each managed repo contains a `.dual.toml` or `.dual/config.toml`
- Dual discovers repos by scanning for this marker file
- The repo's config declares runtime needs: image, ports, env vars, commands
- Branches are managed dynamically, not declared in config
- Dual acts as a "runtime layer" that activates per-repo

**Config example** (inside each repo):
```toml
# lightfast/.dual.toml
image = "node:20"
ports = [3000, 3001]
setup = "pnpm install"

[commands]
dev = "pnpm dev"
test = "pnpm test"
```

**Developer Experience**:

| Journey | Experience |
|---------|-----------|
| First-time setup | `cd lightfast && dual init` → creates `.dual.toml` |
| Add a new repo | `cd new-repo && dual init` → creates `.dual.toml` |
| New branch workspace | `dual branch feat/auth` → clones + launches from current repo |
| Launch workspace | `dual up` (from within repo) or `dual up lightfast` |
| Switch workspace | `dual switch lightfast-feat__auth` |
| Onboard teammate | `.dual.toml` checked into repo, teammate runs `dual up` |
| Move between machines | Config travels with repo, just clone and `dual up` |
| Multi-repo | Each repo self-contained — need registry to see all repos |

**Strengths**:
- Familiar pattern (devcontainer.json, docker-compose.yml, .envrc, Dockerfile)
- Config travels with the repo — onboarding is `clone + dual up`
- Separation of concerns: each repo declares only its own needs
- No centralized file to maintain
- Runtime config checked into repo = team-wide consistency

**Tensions**:
- No single place to see "all my workspaces across repos"
- Discovery problem: how does dual know about all your repos?
- Cross-repo coordination hard (e.g., "launch lightfast + agent-os together")
- Config in repo means repo owners control it — what if you want custom ports?
- Still need central state for "which branches am I working on?"
- You'd need to clone a repo before dual can read its config — chicken-and-egg

**Comparable tools**: VS Code Dev Containers (`.devcontainer/devcontainer.json`), Docker Compose (`docker-compose.yml`), direnv (`.envrc`)

---

### Approach 3: Standalone Conductor

**Mental Model**: "Dual is the centralized manager. It knows about everything, manages everything."

**How it would work**:
- Dual maintains its own state directory (`~/.dual/`)
- `dual add <url>` registers a repo
- `dual branch <repo> <branch>` creates a workspace
- State file tracks repos, branches, container status, session state
- Dual is always running (daemon) or reconstructs state on demand
- No config file in any repo

**Config/State example** (`~/.dual/state.toml`):
```toml
[state]
workspace_root = "~/dual-workspaces"

[[repos]]
name = "lightfast"
url = "git@github.com:org/lightfast.git"
registered_at = 2026-02-15T04:00:00Z

[[repos.workspaces]]
branch = "main"
status = "running"
container_id = "abc123"
created_at = 2026-02-15T04:01:00Z

[[repos.workspaces]]
branch = "feat/auth"
status = "stopped"
container_id = "def456"
created_at = 2026-02-15T04:05:00Z
```

**Developer Experience**:

| Journey | Experience |
|---------|-----------|
| First-time setup | `dual init` → creates `~/.dual/` |
| Add a new repo | `dual add git@github.com:org/lightfast.git` |
| New branch workspace | `dual create lightfast feat/auth` |
| Launch workspace | `dual launch lightfast-feat__auth` |
| Switch workspace | `dual switch lightfast-main` |
| Onboard teammate | They run `dual add` for the same repos |
| Move between machines | Export/import state, or re-add repos |
| Multi-repo | Natural — dual sees everything |

**Strengths**:
- Most powerful: dynamic workspace creation without config editing
- Single command to add repos and branches
- Dual has full visibility across all repos and workspaces
- State can track richer information (timestamps, usage patterns, health)
- Natural place for `dual status` — shows everything with live state
- Supports `dual create <branch>` naturally

**Tensions**:
- State file can drift from reality (containers killed outside dual, branches deleted)
- Reconciliation needed: state says "running" but container is gone
- Not portable: state is machine-specific
- Teammates can't share config by checking it into a repo
- More complex to implement: state management, conflict resolution
- Losing `~/.dual/state.toml` means losing your workspace registry
- No team-wide consistency: each developer configures independently

**Comparable tools**: Terraform (state file + commands), Docker Desktop (centralized container management), Kubernetes (desired state + reconciliation)

---

### Approach 4: Hybrid — Per-Repo Declaration + Centralized State

**Mental Model**: "Repos declare what they need. Dual tracks what you're working on."

**How it would work**:
- Repos contain a `.dual.toml` with **runtime hints** (image, ports, env, setup)
- Centralized `~/.dual/workspaces.toml` tracks **active workspaces** (which branches you're developing)
- `dual create <repo> <branch>` adds to centralized state
- On launch, dual reads repo's `.dual.toml` for runtime config
- Chicken-and-egg solved: `dual add <url>` clones repo, reads its `.dual.toml`

**Config examples**:

Per-repo (checked into repo):
```toml
# lightfast/.dual.toml — RUNTIME HINTS
image = "node:20"
ports = [3000, 3001]
setup = "pnpm install"
env = { NODE_ENV = "development" }
```

Centralized (user's machine):
```toml
# ~/.dual/workspaces.toml — ACTIVE WORKSPACE STATE
workspace_root = "~/dual-workspaces"

[[workspaces]]
repo = "lightfast"
url = "git@github.com:org/lightfast.git"
branch = "main"

[[workspaces]]
repo = "lightfast"
url = "git@github.com:org/lightfast.git"
branch = "feat/auth"

[[workspaces]]
repo = "agent-os"
url = "/local/path/to/agent-os"
branch = "main"
```

**Developer Experience**:

| Journey | Experience |
|---------|-----------|
| First-time setup | `dual add git@github.com:org/lightfast.git` → clones, reads `.dual.toml` |
| Add a new repo | `dual add <url>` → registers repo, creates `main` workspace |
| New branch workspace | `dual create lightfast feat/auth` → adds to state, ready to launch |
| Launch workspace | `dual launch lightfast-feat__auth` |
| Switch workspace | `dual switch lightfast-main` |
| Onboard teammate | `.dual.toml` in repo ensures same image/ports. They `dual add` the repo |
| Move between machines | `dual add` same repos. Runtime config comes from repo |
| Multi-repo | Centralized state shows all workspaces. Per-repo config handles specifics |

**Strengths**:
- Clean separation: runtime hints (team-shared) vs workspace state (personal)
- Repos declare needs, developer controls what to work on
- `dual create <branch>` works naturally — modifies centralized state only
- Team consistency: everyone gets same image/ports from repo's `.dual.toml`
- Developer freedom: custom branches without touching repo config
- Familiar patterns: like Terraform (config in repo, state centralized)

**Tensions**:
- Two config locations to understand
- What happens when repo's `.dual.toml` changes? (need reconciliation)
- Centralized state still machine-specific
- Need to handle: repo added but `.dual.toml` doesn't exist yet
- Slightly more complex mental model than "one file"

**Comparable tools**: Terraform (HCL in repo + state file), Nix flakes (flake.nix in repo + profiles centralized)

---

### Approach 5: Git-Native

**Mental Model**: "Dual extends git. Workspaces are a git concept."

**How it would work**:
- `dual` behaves like a git extension: `git dual branch feat/auth`
- Uses git config for settings (image, ports)
- Workspace creation is a git-aware operation
- Could extend git worktrees with container isolation
- Or wrap full clones behind git-like UX

**Config example** (in `.git/config` or `.gitattributes`):
```ini
[dual]
    image = node:20
    ports = 3000,3001
    setup = pnpm install
```

**Developer Experience**:

| Journey | Experience |
|---------|-----------|
| First-time setup | `git dual init` in repo |
| Add a new repo | Just clone it — dual config in git config |
| New branch workspace | `git dual branch feat/auth` |
| Launch workspace | `git dual launch feat/auth` |
| Switch workspace | `git dual checkout main` |
| Onboard teammate | Config in git config, shared via repo |
| Move between machines | Clone repo, config travels with it |
| Multi-repo | Awkward — git is per-repo, not cross-repo |

**Strengths**:
- Familiar git-like workflow
- Branch management is git's core competency
- Natural for single-repo workflows
- Config can travel with repo via `.gitconfig` or attributes

**Tensions**:
- Git is per-repo — cross-repo management is unnatural
- Git extensions are awkward to install and maintain
- Git config is not expressive (no arrays, no nested structures)
- Conflates git concepts with container orchestration
- Doesn't map well to dual's multi-repo identity
- Git worktrees have constraints (can't checkout same branch twice)

**Comparable tools**: git worktrees, git-repo (Android's multi-repo tool)

---

### Approach 6: Shell-Native / Activation Pattern

**Mental Model**: "cd into a workspace directory and everything activates."

**How it would work**:
- Workspaces are directories with a `.dual/` marker
- Shell hook (like direnv) detects when you `cd` into a workspace
- Automatically sources RC file, sets up routing
- Container starts on first command that needs it
- Deactivates when you `cd` out

**Config example** (per workspace directory):
```
~/dual-workspaces/lightfast/main/.dual/config.toml
~/dual-workspaces/lightfast/feat__auth/.dual/config.toml
```

**Developer Experience**:

| Journey | Experience |
|---------|-----------|
| First-time setup | `dual init <url>` → creates workspace dir with `.dual/` |
| Add a new repo | `dual clone <url>` |
| New branch workspace | `dual branch lightfast feat/auth` |
| Launch workspace | `cd ~/dual-workspaces/lightfast/main/` → auto-activates |
| Switch workspace | `cd` to different workspace → auto-switches |
| Onboard teammate | Share workspace recipe / `dual.toml` |
| Move between machines | Re-create workspaces from registry |
| Multi-repo | Need to `cd` between directories — no unified view |

**Strengths**:
- Zero-friction switching: just `cd`
- Familiar to direnv/nix users
- Lazy activation: resources only created when needed
- Natural shell integration
- Works without tmux (each terminal tab = one workspace)

**Tensions**:
- Requires shell hook installation (modifies `.zshrc` / `.bashrc`)
- `cd` semantics change: side effects on directory change
- No overview of all workspaces without a separate command
- Container management on `cd` can be slow
- Doesn't work in non-interactive contexts (scripts, CI)
- Conflicts with other shell hooks (direnv, nvm, etc.)

**Comparable tools**: direnv (`.envrc`), nvm (`.nvmrc`), pyenv (`.python-version`)

---

## Comparative Analysis

### Decision Matrix

| Factor | 1. Centralized Config | 2. Per-Repo Tunnel | 3. Standalone Conductor | 4. Hybrid | 5. Git-Native | 6. Shell-Native |
|--------|:---:|:---:|:---:|:---:|:---:|:---:|
| Setup friction | Medium | Low | Low | Low | Low | Medium |
| Add branch | High (edit file) | Low | Low | Low | Low | Low |
| Multi-repo UX | Great | Poor | Great | Great | Poor | Poor |
| Team sharing | Good | Great | Poor | Great | Good | Poor |
| Portability | Good | Great | Poor | Good | Good | Poor |
| Dynamic workflows | Poor | Medium | Great | Great | Medium | Great |
| Mental model simplicity | Great | Good | Good | Medium | Good | Medium |
| State management | None | None | Complex | Medium | None | Medium |
| Implementation effort | Done | Moderate | High | Moderate | High | High |

### Key Tradeoffs

**Config location vs discovery**: Centralized config (1, 3) makes cross-repo operations natural. Per-repo config (2, 5) makes onboarding natural. Hybrid (4) achieves both at the cost of complexity.

**Static declaration vs dynamic creation**: Current approach (1) requires pre-declaring branches. Approaches 3, 4, and 6 allow dynamic creation. This is the core tension — pre-declaration provides clarity but creates friction.

**Shared config vs personal state**: Runtime hints (image, ports) should be shared across a team. Active branches are personal. Only approach 4 cleanly separates these concerns.

**Single-repo vs multi-repo identity**: Approaches 2, 5, and 6 are naturally per-repo. Approaches 1, 3, and 4 are naturally cross-repo. Dual's core mission is multi-repo orchestration, which favors cross-repo approaches.

---

## User Story Walkthroughs

### Story: "I want to quickly explore a bug fix branch, then throw it away"

| Approach | Steps | Friction |
|----------|-------|----------|
| 1. Centralized | Edit dual.toml → add branch → save → `dual launch` → work → edit dual.toml → remove branch → `dual destroy` | **High** — 4 config edits |
| 2. Per-Repo | `cd lightfast && dual branch fix/bug` → work → `dual destroy fix/bug` | **Low** — but no cross-repo view |
| 3. Conductor | `dual create lightfast fix/bug` → `dual launch` → work → `dual destroy` | **Low** |
| 4. Hybrid | `dual create lightfast fix/bug` → `dual launch` → work → `dual destroy` | **Low** |
| 5. Git-Native | `git dual branch fix/bug` → work → `git dual destroy fix/bug` | **Low** — single repo only |
| 6. Shell-Native | `dual branch lightfast fix/bug && cd ~/dual-workspaces/lightfast/fix__bug` → work → `dual destroy` | **Low** — requires cd |

### Story: "Onboard a new team member to work on the same repos"

| Approach | Steps | Friction |
|----------|-------|----------|
| 1. Centralized | Share `dual.toml` → they run `dual launch` per workspace | **Medium** — dual.toml might have your personal branches |
| 2. Per-Repo | Clone repos → `.dual.toml` already in repo → `dual up` | **Low** — config travels with repo |
| 3. Conductor | They run `dual add` per repo → `dual create` per branch | **Medium** — manual setup per machine |
| 4. Hybrid | Clone repos → `.dual.toml` gives runtime config → `dual add <url>` → `dual create <branch>` | **Low** — runtime config shared, workspace state personal |
| 5. Git-Native | Clone repos → config in .git → `git dual branch` | **Medium** — per-repo only |
| 6. Shell-Native | Install shell hook → clone repos → `cd` into workspaces | **High** — shell hook setup |

### Story: "I work on 3 repos simultaneously, each with 2-3 branches"

| Approach | Steps | Friction |
|----------|-------|----------|
| 1. Centralized | One `dual.toml` with all repos and branches → `dual list` shows everything | **Low** for daily use — **high** for config maintenance |
| 2. Per-Repo | No unified view. Must manage each repo separately | **High** — fragmented experience |
| 3. Conductor | `dual status` shows all repos/branches/health | **Low** — best visibility |
| 4. Hybrid | `dual status` shows all workspaces. Per-repo config handles runtime | **Low** — good visibility + shared config |
| 5. Git-Native | Per-repo commands, no cross-repo view | **High** — no orchestration |
| 6. Shell-Native | Tab per workspace, no unified view | **High** — manual coordination |

---

## Analysis: Which Architecture Fits Dual's Mission?

### Dual's Core Mission (from CLAUDE.md)

> "A terminal workspace orchestrator for parallel multi-repo development with AI coding agents."

Key phrases:
- **Orchestrator** — implies centralized coordination, not per-repo isolation
- **Parallel multi-repo** — cross-repo management is essential, not optional
- **AI coding agents** — transparency is critical (Claude Code must not know about containers)
- **Development** — branches are ephemeral, workflows are dynamic

### Architecture Alignment

**Approaches 2 (Per-Repo), 5 (Git-Native), 6 (Shell-Native)** are fundamentally per-repo patterns. They work well for single-repo tools but fight against dual's multi-repo orchestration identity. A developer working on 3 repos × 3 branches = 9 workspaces needs a unified view, not 3 separate configs.

**Approach 1 (Current)** has the right cross-repo model but conflates runtime config (image, ports) with workspace state (active branches). This means:
- Personal branch lists leak into shared config
- Adding a branch is a config edit, not a command
- Stale branches accumulate in config

**Approach 3 (Conductor)** has the right dynamic model but loses team-shareability. Every developer configures from scratch. There's no "this repo needs node:20 and port 3000" encoded anywhere reusable.

**Approach 4 (Hybrid)** separates the two concerns cleanly:
- **What a repo needs** (image, ports, setup) → per-repo `.dual.toml`, checked in, team-shared
- **What you're working on** (active branches) → centralized `~/.dual/`, personal, dynamic

This matches the Terraform insight: infrastructure definition belongs in the repo (HCL files), but infrastructure state belongs on the operator's machine (state file).

### The Conductor vs Hybrid Question

The remaining architectural question is whether approach 3 or 4 is better. The key difference:

| Factor | Conductor (3) | Hybrid (4) |
|--------|:---:|:---:|
| "This repo needs node:20" | Stored in `~/.dual/` — personal | In repo `.dual.toml` — team-shared |
| "I'm working on feat/auth" | Stored in `~/.dual/` — personal | In `~/.dual/` — personal |
| Teammate onboarding | Manual: tell them image/ports | Automatic: `.dual.toml` in repo |
| Repo runtime changes | Each developer updates independently | One PR updates for everyone |

The hybrid approach wins because runtime config (image, ports, setup commands) is a **team-level concern** — when the project switches from node:18 to node:20, everyone should get that automatically. The conductor approach makes this a per-developer problem.

---

## Code References

- `src/config.rs:7-32` — Current DualConfig and RepoConfig structs
- `src/config.rs:48-99` — Workspace identity system (paths, names, encoding)
- `src/config.rs:103-118` — Config loading and discovery
- `src/config.rs:63-73` — Workspace resolution (requires pre-declaration)
- `src/main.rs:52-135` — Launch flow (5-step orchestration)
- `src/main.rs:158-218` — Destroy flow (reverse teardown)
- `src/cli.rs:1-52` — CLI command definitions (no create/add commands)
- `src/container.rs:6` — Hardcoded `node:20` image
- `src/container.rs:159-179` — Docker create args
- `src/shell.rs:2-4` — Hardcoded container command list
- `src/shell.rs:37-55` — RC file generation
- `src/proxy.rs:26-54` — Routing table construction from config

## Architecture Documentation

### Current Architecture: Config-Driven Stateless (Approach 1)

The current codebase implements approach 1 with 27/27 validated architectural claims (see `thoughts/ARCHITECTURE.md`). The core mechanisms are proven:
- Shell interception for transparent command routing — confirmed
- Container network isolation for port independence — confirmed
- Bind mount for file synchronization — confirmed
- Reverse proxy for browser access — confirmed
- tmux for session management — confirmed

These mechanisms are orthogonal to config architecture. All six approaches can use the same isolation/routing/proxy stack. The architectural choice is about **how config and state are managed**, not about how workspaces work.

### Comparable Tool Patterns

| Tool | Config Location | State Location | Model |
|------|----------------|----------------|-------|
| Dev Containers | `.devcontainer/` in repo | VS Code internal | Per-repo declaration |
| Docker Compose | `docker-compose.yml` in project | Docker daemon | Per-project declaration |
| Terraform | `.tf` files in repo | `terraform.tfstate` (local/remote) | Hybrid: config in repo + state separate |
| tmuxinator | `~/.config/tmuxinator/` | tmux server | Centralized config |
| direnv | `.envrc` per directory | Shell environment | Per-directory activation |
| Nix flakes | `flake.nix` in repo | Nix store | Per-repo with centralized store |
| Vagrant | `Vagrantfile` in project | `.vagrant/` in project | Per-project declaration + local state |
| git worktrees | `.git` + worktree links | Git internal | Git-native |

## Historical Context (from thoughts/)

- `thoughts/ARCHITECTURE.md` — 27/27 validated claims, all confirmed. Isolation mechanisms are proven and approach-independent.
- `thoughts/BUILD.md` — MVP modules built and tested. Current implementation follows approach 1.
- `thoughts/shared/research/2026-02-15-config-workspace-state-architecture.md` — Detailed analysis of current config system, identified the tension between project config and workspace state.
- `thoughts/shared/research/2026-02-13-BUILD-config.md` — Original config module research, established TOML parsing and discovery.
- `thoughts/shared/research/2026-02-05-ARCH-shell-interception.md` — Shell interception research, confirmed approach.
- `thoughts/shared/research/2026-02-05-ARCH-container-network-isolation.md` — Network isolation research, confirmed approach.

## Related Research

- `thoughts/shared/research/2026-02-15-config-workspace-state-architecture.md` — Companion document: current config system in detail
- `thoughts/shared/research/2026-02-13-BUILD-config.md` — Config module implementation research
- `thoughts/shared/research/2026-02-13-BUILD-cli.md` — CLI module research (command structure)

## Open Questions

1. **If hybrid (approach 4), what exactly goes in per-repo `.dual.toml` vs centralized state?** Need to enumerate every field and decide which side it belongs to.
2. **Should centralized state be auto-reconciled?** (Terraform-style `dual plan` to preview changes)
3. **How should `dual add <url>` handle repos without `.dual.toml`?** (Fallback defaults vs interactive prompts)
4. **Should the current `dual.toml` remain supported as a migration path?** (Single-file mode as syntactic sugar over hybrid)
5. **What does `dual status` look like in the hybrid model?** (Merge per-repo runtime info + centralized workspace state + live container/tmux state)
6. **How does workspace state sync across machines?** (Git repo for dotfiles? Cloud state like Terraform Cloud?)
