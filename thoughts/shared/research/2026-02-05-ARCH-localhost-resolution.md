# Research: localhost-resolution

**Date**: 2026-02-05
**Spec Claim**: "*.localhost resolves to 127.0.0.1 natively in all modern browsers"
**Hypothesis**: Developers can access `{repo}-{branch}.localhost:{port}` without any configuration.

## Research Method

Parallel knowledge agents:
- **knowledge-analyst**: Documented RFC 6761, browser implementations, OS support
- **knowledge-prober**: Empirically tested resolution on macOS

## Key Findings

### RFC 6761 Standard

- Published February 2013
- IANA registered as special-use domain
- Specifies `localhost` and `*.localhost` should resolve to 127.0.0.1 / ::1
- Implementation is voluntary, not mandatory

### Browser Support Matrix

| Browser | *.localhost Support | Implementation |
|---------|---------------------|----------------|
| Chrome | YES | Built-in DNS bypass |
| Edge | YES | Built-in DNS bypass (Chromium) |
| Firefox | YES (84+) | Built-in DNS bypass |
| Safari | DEPENDS | macOS 26+ only |

### macOS CLI Tool Support (Empirically Verified)

| Tool | Resolves *.localhost? | Method |
|------|----------------------|--------|
| curl 8.7.1 | **YES** | Built-in RFC 6761 |
| dig/host | NO | External DNS → NXDOMAIN |
| ping | NO | getaddrinfo fails |
| Node.js | NO | getaddrinfo fails |
| Python | NO | getaddrinfo fails |
| netcat | NO | getaddrinfo fails |

### Critical Insight

**DNS queries don't resolve *.localhost** - the RFC specifies that resolvers should return loopback addresses WITHOUT querying DNS. Implementation is at the application level, not system DNS.

### Browser vs CLI Behavior

- **Browsers** (Chrome, Firefox, Edge): Implement RFC 6761 internally
- **curl 8.7.1+**: Implements RFC 6761 internally
- **Most other tools**: Use system `getaddrinfo()` which doesn't implement RFC 6761 on macOS

### macOS-Specific Limitation

- macOS <26 (pre-Sequoia): System resolver doesn't support *.localhost
- macOS 26+ (Sequoia): Fixed at OS framework level
- Safari depends on OS resolver, so broken on older macOS

## Implications for Dual

### What Works for the Spec Use Case

1. **Browser access** (primary use case): Chrome, Firefox, Edge work
2. **curl testing**: Works (built-in support)
3. **Reverse proxy**: Matches on HTTP Host header, not DNS

### What Doesn't Work

1. **Safari on macOS <26**: Significant limitation
2. **Node.js HTTP clients**: Can't resolve *.localhost URLs
3. **Container-to-container via *.localhost**: Won't work (use container IPs)

### Architecture Insight

The reverse proxy doesn't need DNS resolution to work:
1. Proxy listens on `0.0.0.0:3000`
2. Request arrives with `Host: lightfast-feat-auth.localhost`
3. Proxy reads Host header, routes to appropriate container
4. No DNS lookup needed - pure HTTP header matching

## Verdict

**CONFIRMED WITH CAVEATS**

The spec claim that "*.localhost resolves natively in all modern browsers" is:

- **TRUE** for Chrome, Edge, Firefox
- **FALSE** for Safari on macOS <26
- **TRUE** for curl
- **FALSE** for most other CLI tools

**Acceptable for Dual** because:
- Browser access is the primary use case
- Most developers use Chrome/Firefox
- curl works for CLI testing
- Reverse proxy uses Host header matching, not DNS

### Recommended SPEC.md Clarification

Current:
> "*.localhost resolves to 127.0.0.1 natively in all modern browsers"

Suggested:
> "*.localhost resolves to 127.0.0.1 natively in Chrome, Edge, and Firefox. Safari requires macOS 26+ (Sequoia)."
