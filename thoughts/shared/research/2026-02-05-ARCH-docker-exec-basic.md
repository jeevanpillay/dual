# Research: docker-exec-basic

**Date**: 2026-02-05
**Spec Claim**: "dual run <command> wraps docker exec <container-name> <command>"
**Hypothesis**: Docker exec can execute arbitrary commands in a running container, preserving behavior as if the command ran natively.

## Research Method

Parallel knowledge agents:
- **knowledge-analyst**: Documented docker exec mechanics, API behavior, constraints
- **knowledge-prober**: Empirically tested docker exec on this system

## Environment

- Docker Version: 29.2.0 (build 0b9d198)
- Platform: Docker Desktop on macOS (Darwin 24.6.0)
- Container: alpine:latest (BusyBox v1.37.0)

## Key Findings

### What Docker Exec Does

1. **Mechanism**: Creates a new process inside an existing container's namespaces (PID, network, mount, IPC, UTS)
2. **Requirement**: Container must be in "running" state
3. **macOS Layer**: Commands route through Docker Desktop VM (10-50ms startup overhead)

### Empirically Verified Behaviors

| Behavior | Status | Notes |
|----------|--------|-------|
| Exit code preservation | WORKS | `exit 42` returns 42 to host |
| Stdout streaming | WORKS | Real-time, not buffered |
| Stderr capture | WORKS | Both streams captured |
| Command chaining | WORKS | `&&`, `\|\|`, `;`, pipes all work |
| Shell syntax | WORKS | Variable expansion, redirects work |

### What Requires Explicit Handling

| Aspect | Default Behavior | Required for Transparency |
|--------|------------------|--------------------------|
| Environment | NOT passed through | `-e VAR=value` for each |
| Working directory | `/` (root) | `-w /path` to set CWD |
| TTY allocation | None | `-t` for interactive, omit for scripts |
| User identity | Container's default (often root) | `-u uid:gid` if needed |

### Important Flags

- `-i` (interactive): Keep stdin open
- `-t` (tty): Allocate pseudo-TTY
- `-e KEY=VALUE`: Set environment variable
- `-w /path`: Set working directory
- `-u user:group`: Set user identity

### Edge Cases Discovered

1. **Stdout/stderr interleaving**: When both streams are used, order may vary
2. **Host env vars**: Do NOT automatically pass through (critical for wrapper design)
3. **TTY detection**: Commands checking `isatty()` will fail without `-t`

## Implications for Dual

1. **Shell wrapper must forward environment variables explicitly** - Cannot rely on automatic passthrough
2. **Shell wrapper must track and set working directory** - Default is `/`, not CWD
3. **Shell wrapper must detect interactive vs non-interactive** - Use `-it` for interactive, neither for scripts
4. **Exit codes work perfectly** - No special handling needed

## Verdict

**CONFIRMED** - Docker exec provides the fundamental mechanism needed for transparent command routing. The spec claim that `dual run <command>` can wrap `docker exec <container-name> <command>` is valid.

## Caveats

- Environment and CWD require explicit handling in the wrapper
- TTY detection must be implemented
- 10-50ms startup latency per command on macOS (acceptable for most use cases)
