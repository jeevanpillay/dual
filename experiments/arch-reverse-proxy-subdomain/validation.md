# Validation: reverse-proxy-subdomain

**Date**: 2026-02-05
**Claim**: "{repo}-{branch}.localhost:{port} → container's :{port}"

## Analysis Summary

### Routing Mechanism

1. Browser resolves `*.localhost` to `127.0.0.1` (no DNS needed)
2. Proxy listens on host port (e.g., :3000)
3. Proxy reads HTTP Host header: `lightfast-feat-auth.localhost`
4. Proxy matches subdomain pattern → looks up container
5. Proxy queries container IP: `docker inspect` → `172.17.0.2`
6. Proxy forwards to `http://172.17.0.2:3000`

### Implementation Options

| Approach | Dynamic Updates | Docker Integration | Complexity |
|----------|-----------------|-------------------|------------|
| Traefik | YES (native) | YES (Docker API) | Medium |
| Caddy | Reload | Manual | Low |
| Nginx | Reload | Manual | Low |
| Custom (~150 LOC) | YES (full control) | Programmatic | Medium |

### Verdict

**CONFIRMED**

The spec claim that a reverse proxy can route `{repo}-{branch}.localhost:{port}` to the appropriate container is valid:

- Host header routing is standard proxy functionality
- Container IPs discoverable via `docker inspect`
- Multi-port listening achievable (one server per port)
- WebSocket/SSE support available in all approaches
- No port publishing needed (direct container IP access)

### Constraints Discovered

1. **Ports must be known** before proxy starts listening (except custom implementation)
2. **Container IPs change** on restart → must re-query
3. **Services must bind to 0.0.0.0** (not 127.0.0.1) inside container

### Recommendation

Custom Go or Node.js proxy for Dual's use case:
- Full dynamic port management
- Direct Docker API integration
- Container lifecycle awareness
- ~150 lines of code (per SPEC.md suggestion)
