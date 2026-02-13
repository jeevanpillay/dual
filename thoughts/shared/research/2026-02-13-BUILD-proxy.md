---
date: 2026-02-13T00:00:00+00:00
researcher: Claude
git_commit: 8864b9f
branch: feature/build-loop-pipeline
repository: dual
topic: "Reverse proxy for browser access to workspace services"
tags: [research, build, proxy, reverse-proxy]
status: complete
last_updated: 2026-02-13
last_updated_by: Claude
---

# Research: Proxy Module

## Research Question

How to implement the reverse proxy for routing `{repo}-{branch}.localhost:{port}` to container services?

## Summary

The proxy needs to listen on host ports, read HTTP Host headers, and forward requests to container IPs on the Docker bridge network. For MVP, ports are configured explicitly in dual.toml. Container IPs are resolved via `docker inspect`. Implementation uses hyper + tokio for async HTTP proxying with WebSocket support.

## Architecture Decisions Referenced

- **reverse-proxy-subdomain**: CONFIRMED — Host header routing is standard. Custom proxy recommended for dynamic control.
- **websocket-proxy**: CONFIRMED — All proxies support WebSocket upgrade.
- **sse-proxy**: CONFIRMED — SSE is standard HTTP streaming.
- **dynamic-port-registration**: CONFIRMED WITH CAVEATS — Adding new ports needs new listeners. For MVP, static config.

## Design Decisions

### Port Discovery: Explicit Configuration
Containers run `sleep infinity` with no exposed ports. Services run inside via docker exec. Container ports aren't visible to Docker — they're just processes inside the container's network namespace.

Solution: Configure ports in dual.toml:
```toml
[[repos]]
name = "lightfast"
url = "..."
branches = ["main", "feat/auth"]
ports = [3000, 3001, 4001]
```

### Container IP Resolution
Docker bridge network assigns IPs (172.17.0.x). Resolve via:
```
docker inspect --format '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' dual-lightfast-main
```

### Proxy Architecture
- One listener per unique port across all containers
- Each request: read Host header → parse subdomain → look up container IP → forward
- Support HTTP/1.1, WebSocket upgrade, SSE (streaming responses)

### CLI Integration
- `dual proxy` — start proxy in foreground (run in separate terminal/tmux pane)
- `dual urls` — show URLs for running workspaces
- `dual open` — open URLs in browser

### Dependencies Required
- `tokio` (async runtime)
- `hyper` + `hyper-util` (HTTP server/client)
- `http-body-util` (body handling)

## Implementation Approach

1. Add optional ports field to RepoConfig
2. Add container IP resolution to container module
3. Implement proxy server with hyper
4. Add proxy and urls subcommands
5. Wire dual open to use `open` command
