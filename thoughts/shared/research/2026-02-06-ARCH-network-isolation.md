---
date: 2026-02-06T12:00:00+08:00
researcher: Claude
git_commit: 120d65b05c3884378c124e18c8a48d41cdfac2ab
branch: feat/spec-v2-simplified
repository: dual
hypothesis: "Multiple Docker containers can each bind the same port (e.g., :3000) with isolated network namespaces, enabling zero-conflict parallel workspaces"
tags: [architecture, research, docker, networking, namespaces]
status: research_complete
last_updated: 2026-02-06
last_updated_by: Claude
---

# Research: Container Network Namespace Isolation

**Date**: 2026-02-06
**Researcher**: Claude
**Git Commit**: 120d65b
**Branch**: feat/spec-v2-simplified
**Repository**: dual

## Hypothesis

Multiple Docker containers can each bind the same port (e.g., :3000) with isolated network namespaces, enabling zero-conflict parallel workspaces.

## Why This Matters

This is the absolute foundation of Dual v2's architecture. The entire "one workspace = one container" model depends on containers having independent network namespaces so multiple dev servers can bind :3000 simultaneously.

## What We're Testing

- **Primary claim**: Containers have isolated network namespaces (SPEC.md L38, L59-61)
- **Success criteria**: Two+ containers each bind :3000 without conflict; each container's localhost:3000 reaches its own service
- **Scope boundary**: Network isolation only; browser access via reverse proxy is a separate claim

## Environment & Prerequisites

### Verified Present
| Tool/System | Version | Status |
|-------------|---------|--------|
| Docker | 29.2.0 | Found |
| Docker Desktop | macOS Apple Virtualization.framework | Found |
| VM Kernel | 6.12.67-linuxkit | Found |
| Storage Driver | overlayfs | Found |
| Network Driver | bridge | Found |

### Environment Details
- macOS 15.7.3 (arm64) running Docker Desktop
- Linux VM runs inside Apple Virtualization.framework
- Containers run inside VM with native Linux namespace support
- Default bridge network: 172.17.0.0/16
- Network topology: macOS → vpnkit → Linux VM → docker0 bridge → container namespaces

## Feasibility Assessment

### Technical Foundation

- **Linux network namespaces** (`CLONE_NEWNET`): Each container gets its own network stack — interfaces, routing tables, iptables rules, and socket tables are all per-namespace
- **Socket table scoping**: The kernel tracks `(namespace, proto, local_addr, local_port)` — identical bindings in different namespaces never conflict
- **Loopback isolation**: Each namespace has its own `lo` interface; `127.0.0.1` in container A ≠ `127.0.0.1` in container B

### Empirical Discoveries

- Two Alpine containers both successfully bound port 3000 internally (no conflict)
- `docker exec test-net-a wget localhost:3000` returned "container-a"
- `docker exec test-net-b wget localhost:3000` returned "container-b"
- Cross-container access works via bridge IPs (172.17.0.x)
- Host CANNOT reach unpublished container ports (container IPs not routable from macOS)
- Containers on different bridge networks are fully isolated (cannot reach each other)

### How It Works

1. Docker creates a new network namespace per container
2. Each container gets a `veth` pair: one end in container, one attached to `docker0` bridge
3. Container binds `0.0.0.0:3000` in its own namespace — kernel only checks that namespace's socket table
4. `docker exec` joins the target container's namespace via `setns(fd, CLONE_NEWNET)`, so `curl localhost:3000` reaches that container's service

### Constraints & Limitations

1. **Port mapping (`-p`) conflicts**: While containers can bind same port internally, `-p 3000:3000` on two containers would conflict on the host
2. **macOS VM hop**: Traffic goes macOS → vpnkit → VM → container (adds ~0.1-0.5ms latency)
3. **Container IPs not routable from macOS**: 172.17.x.x addresses exist only inside the Linux VM
4. **Bridge IP exhaustion**: Default /16 subnet supports ~65k containers (not a practical concern)
5. **Default bridge lacks DNS**: Container name resolution only works on custom bridge networks

## Evidence Assessment

### Supporting Evidence
- Empirical probing confirmed two containers both binding :3000 without conflict
- Each container's localhost is fully isolated
- Docker exec correctly joins the target namespace

### Contradicting Evidence
- None found for the core isolation claim

### Key Caveat for Dual
- Browser access to container services requires either port mapping (conflicts) or reverse proxy (Dual's approach)
- The reverse proxy claim (claim #11) is a hard dependency for browser access

## Assumptions Made

1. Docker Desktop for macOS will continue using Linux VM with native namespaces (high confidence)
2. Default bridge networking is sufficient for Dual's needs (custom bridge would add DNS)
3. The ~0.1-0.5ms VM hop latency is negligible for development use

## Unknowns & Open Questions

1. How does `docker exec` behave when container is under heavy load? (affects Claude Code responsiveness)
2. Maximum practical concurrent containers on macOS Docker Desktop? (memory/CPU limits)
