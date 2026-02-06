# Research: bind-mount-visibility

**Date**: 2026-02-05
**Spec Claim**: "File edits on the host are immediately visible inside the container"
**Hypothesis**: When a directory is bind-mounted into a container, file changes made on the host are visible inside the container without delay.

## Research Method

Parallel knowledge agents:
- **knowledge-analyst**: Documented VirtioFS mechanics, caching, event propagation
- **knowledge-prober**: Empirically tested file visibility and latency

## Environment

- Docker Version: 29.2.0
- Platform: Docker Desktop on macOS (Darwin 24.6.0)
- Bind mount technology: VirtioFS (default since Docker Desktop 4.6+)

## Key Findings

### How Bind Mounts Work on macOS

1. **VirtioFS**: Paravirtualized FUSE filesystem between macOS host and Linux VM
2. **Architecture**: macOS → VirtioFS daemon → virtqueues → Linux VM → container
3. **Event propagation**: FSEvents (macOS) → VirtioFS → inotify (Linux container)

### Empirically Verified Behaviors

| Behavior | Status | Notes |
|----------|--------|-------|
| File creation visible | WORKS | New files appear in container |
| File modification visible | WORKS | Changes propagate |
| File deletion visible | WORKS | Deletions propagate |
| Bidirectional sync | WORKS | Container writes visible on host too |
| inotify events | WORKS | File watcher events propagate |

### Propagation Latency

**Measured: 190-200ms** (consistent across 5 runs)

This is specific to Docker Desktop on macOS. The latency comes from:
1. Host write completes (<1ms)
2. FSEvents notification (1-5ms)
3. VirtioFS daemon processing (varies)
4. VM boundary crossing
5. Container filesystem visibility

**Note**: knowledge-analyst research suggested 2-10ms should be possible with VirtioFS optimizations. The empirical 200ms may be due to:
- Docker Desktop version/configuration
- Test methodology (polling vs caching)
- System load

### File Watcher Behavior

| Event Type | Propagates | Notes |
|------------|------------|-------|
| CREATE | YES | File creation detected |
| MODIFY | YES | Content changes detected |
| DELETE | YES | Deletions detected |
| ATTRIB | YES | Permission/time changes detected |
| MOVE | PARTIAL | Appears as CREATE at destination |

Single file write generates 3 events: CREATE, ATTRIB, MODIFY (normal behavior).

## Implications for Dual

### What Works
- Transparent file sharing between host (nvim, git) and container (pnpm dev)
- Dev servers can detect file changes via inotify
- Hot reload will trigger on file saves

### Latency Consideration
- **200ms propagation** is acceptable for development workflows
- Most dev servers debounce anyway (typically 100-300ms)
- User perception: "instant" (below ~500ms threshold)

### SPEC.md Claim Assessment

**Claim**: "File edits on the host are **immediately** visible inside the container"

**Verdict**: CONFIRMED WITH CAVEATS

- "Immediately" is accurate in the sense that no manual sync is required
- Actual latency: ~200ms on Docker Desktop macOS
- This is fast enough for hot reload to feel instant
- Consider adding latency expectation to SPEC.md

### Recommended SPEC.md Clarification

Current: "File edits on the host are immediately visible inside the container"
Suggested: "File edits on the host are immediately visible inside the container (typically <500ms via VirtioFS)"
