# Docker Abstraction Leakage Experiment

**Date**: 2026-02-05
**Investigator**: Claude Code (knowledge-prober agent)
**Objective**: Determine if shell function interception leaks Docker abstractions to the caller

---

## Executive Summary

**Question**: Can Claude Code detect that commands are being routed through Docker containers?

**Answer**: **YES** - Multiple leakage vectors exist that make containerization detectable.

**Severity**: Without mitigation, containerization is **easily detectable**. With proper mitigation, most leaks can be hidden, but complete transparency is impossible due to kernel-level differences.

---

## Files in This Experiment

1. **README.md** (this file) - Overview and navigation
2. **LEAKAGE_FINDINGS.md** - Comprehensive probe results with all test outputs
3. **LEAKAGE_SUMMARY.md** - Quick reference table of all leakage vectors
4. **COMPARISON.md** - Side-by-side comparison of native vs container vs mitigated container
5. **TEST_COMMANDS.md** - Reproducible test commands with exact outputs

---

## Key Findings

### Critical Leaks (Immediate Detection)

| Vector | How to Detect | Fixable? |
|--------|---------------|----------|
| **Hostname** | `hostname` returns container ID (e.g., `6b4fcc1cd298`) | ✅ YES - use `--hostname` |
| **Platform** | `os.platform()` returns `linux` when host is macOS | ❌ NO - kernel-level |
| **PID 1** | `cat /proc/1/cmdline` shows `sleep infinity` | ⚠️ HARD - needs PID namespace |
| **Missing Mount** | Error shows "OCI runtime exec failed" | ✅ YES - mount all paths |

### High Priority Leaks

| Vector | How to Detect | Fixable? |
|--------|---------------|----------|
| **User Context** | `whoami` returns `root` | ✅ YES - use `--user` |
| **HOME Directory** | `echo $HOME` returns `/root` | ✅ YES - set HOME env |
| **Error Paths** | npm errors show `/root/.npm/_logs/` | ✅ YES - user + HOME fix |

### What Does NOT Leak

- Command execution mechanism (no "docker exec" visible)
- Command locations (`which npm` shows host path)
- Exit codes (properly preserved)
- PATH variable (looks normal)

---

## Test Methodology

### Phase 1: Error Message Inspection
Run commands that fail in various ways and inspect error output for Docker terminology.

**Result**: Error messages don't mention "docker" directly, but reveal `/root/` paths.

### Phase 2: Process Inspection
Check what `ps`, `which`, `type` reveal about the execution environment.

**Result**: PID 1 being `sleep infinity` is a dead giveaway.

### Phase 3: Environment Inspection
Check hostname, environment variables, user context.

**Result**: Multiple leaks - container ID hostname, root user, /root HOME.

### Phase 4: Path Inspection
Check if file paths in errors reveal container filesystem.

**Result**: npm errors consistently show `/root/.npm/_logs/` paths.

### Phase 5: Node.js Probes
Use Node.js to programmatically inspect environment.

**Result**: `os.hostname()`, `os.platform()`, `os.userInfo()` all reveal container.

### Phase 6: Missing Mount Test
Test what happens when directory is not mounted.

**Result**: CATASTROPHIC - error explicitly says "OCI runtime exec failed".

---

## Mitigation Strategy

### Required Configuration

To minimize leakage, containers must be configured with:

```bash
docker run -d \
  --name workspace-container \
  --hostname "$(hostname)" \
  --user "$(id -u):$(id -g)" \
  -e HOME="$HOME" \
  -v "$HOME:$HOME" \
  -v /tmp:/tmp \
  -w "$HOME" \
  node:20-alpine sleep infinity
```

And shell functions must use:

```bash
npm() {
  local docker_flags=""
  test -t 1 && docker_flags="-t"
  docker exec $docker_flags -w "$PWD" workspace-container npm "$@"
}
```

### What This Fixes

- ✅ Hostname matches host
- ✅ User matches host
- ✅ HOME matches host
- ✅ Error paths show host paths
- ✅ No "OCI runtime" errors

### What Remains Unfixed

- ❌ Platform still reports `linux` on macOS
- ❌ PID 1 still shows container init
- ⚠️ Cgroup paths may reveal container

---

## Impact on Dual's Core Invariant

**Dual's Claim**: "Claude Code must never need to know that commands are routed to containers."

### Is This Achievable?

**For normal workflow**: YES, with proper mitigation
- Running `npm install`, `node test.js`, etc. → No detection
- Error messages show host paths → No detection
- Commands work transparently → No detection

**For active probing**: NO, some leaks remain unfixable
- Checking `os.platform()` → Reveals Linux kernel
- Checking PID 1 → Reveals container init
- Checking hostname pattern → Fixable with `--hostname`

### Recommendation

Implement full mitigation (hostname, user, HOME, complete mounts). Accept that active probing can still detect containerization through platform/PID1, but this is not part of Claude Code's normal workflow.

**The core invariant holds for practical purposes**, meaning Claude Code running typical dev commands will not detect containerization.

---

## Testing Instructions

To validate mitigation effectiveness:

1. Start a properly configured container (see mitigation section)
2. Define shell interception functions
3. Run the detection script:

```bash
node detect_container.js
```

4. Check that no indicators are detected (except platform on macOS)

---

## Related Experiments

- **arch-shell-interception** - Validates that shell functions can intercept commands
- **arch-docker-exec-basic** - Validates docker exec preserves exit codes, TTY, etc.
- **arch-bind-mount-visibility** - Validates filesystem mounting works correctly

---

## References

- Test environment: macOS Darwin 24.6.0, Docker 29.2.0
- Container image: node:20-alpine
- Shell: bash with function interception
- Test date: 2026-02-05

---

## Conclusion

**Docker abstraction leakage is significant but manageable.**

Most leaks are configuration issues (hostname, user, HOME) that can be fixed. The fundamental kernel-level differences (Linux vs macOS platform) cannot be hidden but rarely affect normal dev workflows.

**For Dual**: Implement the full mitigation strategy. The core invariant will hold for Claude Code's typical usage patterns, though active environmental probing could still detect containerization through platform checks.

**Next Steps**:
1. Implement hostname matching
2. Implement user/HOME mapping
3. Ensure complete filesystem mounts
4. Test with actual Claude Code workflows
5. Document any remaining edge cases
