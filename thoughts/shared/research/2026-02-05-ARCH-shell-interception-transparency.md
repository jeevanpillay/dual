# Research: shell-interception-transparency

**Date**: 2026-02-05
**Spec Claim**: "Claude Code must never know it is running inside a container"
**Hypothesis**: Shell interception can be configured to not leak Docker abstractions.

## Research Method

- **knowledge-prober**: Empirically tested what information leaks through shell function interception

## Environment

- Container: node:20-alpine
- Docker: 29.2.0
- Host: macOS Darwin 24.6.0

## Key Findings

### What LEAKS (Containerization Visible)

| Vector | Severity | Example Output | Fixable? |
|--------|----------|----------------|----------|
| Hostname | CRITICAL | `6b4fcc1cd298` (container ID) | YES (`--hostname`) |
| Platform | CRITICAL | `linux` (vs macOS) | NO (kernel) |
| User/UID | HIGH | `root` (vs jeevanpillay) | YES (`--user`) |
| HOME path | HIGH | `/root` (vs /Users/...) | YES (`-e HOME`) |
| Error paths | HIGH | `/root/.npm/_logs/...` | YES (user+HOME) |
| PID 1 | CRITICAL | `sleep infinity` | HARD |
| Missing mount | CATASTROPHIC | `OCI runtime exec failed` | YES (mounts) |

### What Does NOT Leak

- Command execution mechanism (no "docker exec" visible)
- Command locations (`which npm` shows host path)
- Exit codes (properly preserved)
- PATH variable (looks normal)
- Argument passing (transparent)

### Mitigation Strategy

Most leaks can be fixed with proper container configuration:

```bash
docker run -d \
  --hostname "$(hostname)" \        # Fixes hostname leak
  --user "$(id -u):$(id -g)" \      # Fixes user context
  -e HOME="$HOME" \                  # Fixes HOME path
  -v "$HOME:$HOME" \                 # Fixes error paths
  -v /tmp:/tmp \                     # Prevents missing mount
  container-image sleep infinity
```

### Unfixable Leaks

**Platform mismatch** is fundamental:
- Container always runs Linux kernel
- On macOS host, `process.platform` returns `linux`, not `darwin`
- Cannot be fixed - containers ARE Linux

**Impact**: Low for typical dev workflows. Platform checks are rare during `npm install`, `node test.js`, etc.

## Implications for Dual

### Core Invariant Assessment

**Spec Claim**: "Claude Code must never need to know that commands are routed to containers"

**For typical Claude Code workflow**:
- `npm install` → Works transparently
- `pnpm dev` → Works transparently
- `node test.js` → Works transparently
- Error messages → Can show host paths with mitigation

**For active environmental probing**:
- Platform check reveals Linux
- PID 1 inspection reveals container
- Hostname can reveal container ID (fixable)

### Recommendation

Implement full mitigation:
1. **Hostname**: Match host hostname
2. **User**: Match host user
3. **HOME**: Match host HOME
4. **Mounts**: Ensure all needed directories mounted

The core invariant holds for **practical purposes**. Claude Code running typical development commands will not detect containerization.

## Verdict

**CONFIRMED WITH CAVEATS**

- **Most leaks are fixable** with proper container configuration
- **Platform mismatch is unfixable** but doesn't affect typical workflows
- **Missing mounts cause catastrophic leaks** - must ensure complete mounts
- **The SPEC claim holds** for normal dev operations
