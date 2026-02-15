---
date: 2026-02-16T14:33:08Z
researcher: jeevan
git_commit: 303c51d87d3b3aec9268789dcc38ad18489d06d9
branch: main
repository: dual
topic: "Port routing and container isolation: how Dual handles multi-app port conflicts"
tags: [research, codebase, port-routing, container, proxy, shell-interception, monorepo]
status: complete
last_updated: 2026-02-16
last_updated_by: jeevan
---

# Research: Port Routing and Container Isolation

**Date**: 2026-02-16T14:33:08Z
**Researcher**: jeevan
**Git Commit**: 303c51d87d3b3aec9268789dcc38ad18489d06d9
**Branch**: main
**Repository**: dual

## Research Question

How does Dual currently handle port routing and container isolation? When running two Next.js apps in two separate `dual launch` instances, both attempt to use the same port on localhost instead of running through the Docker container.

## Summary

Dual's port isolation architecture relies on **three layers working together**: (1) Docker containers with isolated network namespaces, (2) shell function interception that routes runtime commands (`pnpm`, `npm`, `node`, etc.) to containers via `docker exec`, and (3) a reverse proxy (`dual proxy`) that routes browser traffic from `{repo}-{branch}.localhost:{port}` to container IPs on the Docker bridge network.

The current implementation creates containers **without** any `-p` port publishing flags — containers sit on Docker's default bridge network and get private IPs (172.17.0.x). Multiple containers can each listen on port 3000 internally without conflict because they have separate network namespaces.

For this to work end-to-end, commands must be executed **inside the Dual tmux session** where shell functions intercept runtime commands and route them to containers. The reverse proxy must also be running separately via `dual proxy` for browser access.

## Detailed Findings

### 1. Container Creation — No Port Publishing

`src/container.rs:195-234` — The `build_create_args` function constructs Docker container creation arguments:

```rust
let mut args = vec![
    "create".to_string(),
    "--name".to_string(),
    name.to_string(),
    "-v".to_string(),
    format!("{}:{WORKSPACE_MOUNT}", workspace_dir.display()),
];
```

The function adds bind mounts, anonymous volumes, environment variables, working directory, the image, and `sleep infinity` to keep the container alive. **There are no `-p` port mapping flags.** Containers are created on Docker's default bridge network, which gives each container its own network namespace and a unique private IP address.

### 2. Container IP Resolution

`src/container.rs:109-127` — The `get_ip` function resolves a container's bridge network IP:

```rust
pub fn get_ip(name: &str) -> Option<String> {
    let output = Command::new("docker")
        .args(["inspect", "--format",
            "{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}", name])
        .output().ok()?;
    // Returns IP like "172.17.0.2"
}
```

This is used by the reverse proxy to route traffic to the correct container.

### 3. Shell Command Interception

`src/shell.rs:1-4` — The default commands routed to containers:

```rust
const CONTAINER_COMMANDS: &[&str] = &[
    "npm", "npx", "pnpm", "node", "python", "python3", "pip", "pip3", "curl", "make",
];
```

`src/shell.rs:72-83` — For each intercepted command, a shell function is generated:

```bash
pnpm() {
    if [ -t 1 ]; then
        command docker exec -t -w /workspace dual-repo-branch pnpm "$@"
    else
        command docker exec -w /workspace dual-repo-branch pnpm "$@"
    fi
}
```

`src/shell.rs:93-108` — The RC file is written to `~/.config/dual/rc/{container_name}.sh`.

### 4. Launch Flow — How Shell RC Gets Activated

`src/main.rs:434-450` — During `dual launch`, the shell RC is sourced into the tmux session:

```rust
// Step 4: Write shell RC file
let rc_path = shell::write_rc_file(&container_name, &hints.extra_commands)?;

// Step 5: Create tmux session if not alive
if !backend.is_alive(&session_name) {
    let source_cmd = shell::source_file_command(&rc_path);
    backend.create_session(&session_name, &workspace_dir, Some(&source_cmd))?;
}
```

`src/tmux_backend.rs:56-59` — The init command is sent via `send_keys`:

```rust
if let Some(cmd) = init_cmd {
    self.send_keys(session_name, cmd)?;
}
```

This means `source ~/.config/dual/rc/dual-repo-branch.sh` is typed and executed in the tmux session's shell. **The shell interception is only active inside this specific tmux session.**

### 5. Reverse Proxy

`src/proxy.rs:84-157` — The `dual proxy` command starts HTTP listeners on all configured ports:

```rust
let addr = SocketAddr::from(([127, 0, 0, 1], port));
let listener = TcpListener::bind(addr).await?;
```

It binds to `127.0.0.1:{port}` for each port found in workspace `.dual.toml` files. When a request arrives, it:
1. Extracts the subdomain from the Host header (`src/proxy.rs:258-270`)
2. Looks up the container IP in the routing table (`src/proxy.rs:71-76`)
3. Opens a TCP connection directly to `{container_ip}:{port}` (`src/proxy.rs:206`)
4. Forwards the request

### 6. Port Configuration

`src/config.rs:18-46` — Ports are configured in `.dual.toml`:

```rust
pub struct RepoHints {
    pub ports: Vec<u16>,  // default: empty
    // ...
}
```

`src/config.rs:99-110` — The default template shows ports as commented out:

```toml
# Ports your dev server uses (for reverse proxy routing)
# Example: ports = [3000, 3001]
# ports = []
```

### 7. Proxy Route Building

`src/proxy.rs:38-68` — `ProxyState::from_state` iterates all workspaces, checks container status, resolves IPs, and builds a routing table:

```rust
for entry in state.all_workspaces() {
    let container_name = config::container_name(&entry.repo, &entry.branch);
    if container::status(&container_name) != ContainerStatus::Running { continue; }
    let ip = container::get_ip(&container_name)?;
    let workspace_id = config::workspace_id(&entry.repo, &entry.branch);
    for &port in &hints.ports {
        routes.entry(port).or_default().insert(workspace_id.clone(), ip.clone());
    }
}
```

Multiple workspaces can share the same port number because the routing table maps `(port, subdomain)` pairs to different container IPs.

## Code References

- `src/container.rs:195-234` — `build_create_args()` — Container creation with no `-p` flags
- `src/container.rs:109-127` — `get_ip()` — Bridge network IP resolution
- `src/shell.rs:1-4` — `CONTAINER_COMMANDS` — Default intercepted commands
- `src/shell.rs:38-64` — `generate_rc()` — Shell RC generation
- `src/shell.rs:72-83` — `generate_function()` — Individual command wrapper
- `src/shell.rs:93-108` — `write_rc_file()` — RC file persistence
- `src/main.rs:267-460` — `cmd_launch()` — Full workspace launch flow
- `src/main.rs:434-441` — Shell RC write during launch
- `src/main.rs:443-450` — tmux session creation with init command
- `src/tmux_backend.rs:32-62` — `create_session()` — Session creation and init_cmd via send_keys
- `src/proxy.rs:38-68` — `ProxyState::from_state()` — Route table construction
- `src/proxy.rs:84-157` — `start()` — Proxy server startup with per-port listeners
- `src/proxy.rs:160-252` — `handle_request()` — Request routing to containers
- `src/proxy.rs:258-270` — `extract_subdomain()` — Host header parsing
- `src/config.rs:18-46` — `RepoHints` struct with ports field
- `src/config.rs:77-89` — `load_hints()` — Config loading from `.dual.toml`

## Architecture Documentation

### Port Isolation Design

The port isolation relies on Docker's network namespace isolation:

1. **No port publishing**: Containers are created without `-p` flags, so container ports are NOT exposed on the host
2. **Bridge network**: Each container gets a unique IP on Docker's bridge network (172.17.0.x)
3. **Direct IP routing**: The reverse proxy connects directly to container IPs — `TcpStream::connect(format!("{container_ip}:{port}"))`
4. **Subdomain routing**: Browser access uses `{workspace_id}.localhost:{port}` which the proxy resolves to the correct container

### Command Routing Design

Commands are split between host and container:

| Runs on Host | Runs in Container (via docker exec) |
|---|---|
| git, cat, ls, vim, nvim, ssh | npm, npx, pnpm, node, python, python3, pip, pip3, curl, make |

Shell functions are injected into the tmux session shell. They wrap each intercepted command with `docker exec -w /workspace {container_name} {command} "$@"`.

### End-to-End Flow

1. User runs `dual launch repo-branch`
2. Dual resolves/clones workspace, creates/starts container, writes shell RC
3. Dual creates a tmux session and sends `source ~/.config/dual/rc/dual-repo-branch.sh` via send_keys
4. User types `pnpm dev` in the tmux session
5. The shell function intercepts it and runs `docker exec -w /workspace dual-repo-branch pnpm dev`
6. Next.js starts inside the container, binding to port 3000 inside the container's network namespace
7. The `dual proxy` command (run separately) listens on host port 3000
8. Browser requests to `repo-branch.localhost:3000` are routed through the proxy to `172.17.0.x:3000`

### Where The User's Scenario Fits

In a monorepo with 5 apps:
- Dual creates **one workspace per branch** — the entire monorepo is cloned once
- **One container** serves all 5 apps for that branch
- Inside the container, each app can bind its own port (3000, 3001, etc.) without conflict with other containers
- The `.dual.toml` `ports` array should list ALL ports used by apps in the monorepo (e.g., `ports = [3000, 3001, 3002, 3003, 3004]`)

For two branches of the same monorepo:
- Two separate containers are created, each with their own network namespace
- Both can internally listen on ports 3000-3004 without conflict
- The proxy differentiates them by subdomain: `repo-main.localhost:3000` vs `repo-feature.localhost:3000`

## Historical Context (from thoughts/)

- `thoughts/shared/research/2026-02-05-ARCH-container-network-isolation.md` — Research confirming Docker bridge network provides network namespace isolation
- `thoughts/shared/research/2026-02-05-ARCH-localhost-resolution.md` — Research on RFC 6761 `*.localhost` DNS resolution
- `thoughts/shared/research/2026-02-05-ARCH-shell-interception.md` — Research on shell function interception methods
- `thoughts/shared/research/2026-02-05-ARCH-shell-interception-transparency.md` — Research on what leaks vs stays transparent
- `thoughts/shared/research/2026-02-13-BUILD-proxy.md` — Proxy implementation research: port discovery, container IP resolution
- `thoughts/shared/research/2026-02-13-BUILD-container.md` — Container module implementation research
- `thoughts/shared/research/2026-02-13-BUILD-shell.md` — Shell module implementation research

## Related Research

- `thoughts/shared/research/2026-02-06-ARCH-network-isolation.md`
- `thoughts/shared/research/2026-02-15-v3-architecture-rethink.md`
- `thoughts/shared/research/2026-02-15-architecture-rethink-bugs-tui-plugins.md`

## Open Questions

1. Was the `dual proxy` command running when the port conflict occurred?
2. Were the `pnpm dev` commands executed inside the Dual tmux sessions (where shell interception is active) or from a separate terminal?
3. How are the 5 apps in the monorepo started — via a single command (e.g., turbo dev) or individually?
4. Does the monorepo's `.dual.toml` have all 5 ports listed in the `ports` array?
