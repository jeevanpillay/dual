# Docker Abstraction Leakage - Quick Reference

**Test Date**: 2026-02-05

## Leakage Vectors Summary

| Vector | Severity | Detection Method | Example Output | Mitigation |
|--------|----------|------------------|----------------|------------|
| **Hostname** | CRITICAL | `hostname` or `os.hostname()` | `6b4fcc1cd298` (container ID) | Set `--hostname` to host name |
| **Platform** | CRITICAL | `os.platform()` | `linux` (when host is macOS) | Cannot fix (kernel-level) |
| **PID 1** | CRITICAL | `cat /proc/1/cmdline` | `sleep infinity` | Cannot fix without PID namespace tricks |
| **User** | HIGH | `whoami` or `os.userInfo()` | `root` (UID 0) | Use `--user` with matching UID |
| **HOME** | HIGH | `echo $HOME` | `/root` | Set HOME env var to host user home |
| **Error Paths** | HIGH | npm/node errors | `/root/.npm/_logs/...` | Mount host home, set HOME correctly |
| **Missing Mount** | CATASTROPHIC | Any command in unmounted dir | `OCI runtime exec failed` | Ensure all paths mounted |
| **Working Dir** | MEDIUM | `pwd` (without `-w`) | `/` | Always use `-w "$PWD"` |
| **Cgroups** | MEDIUM | `cat /proc/self/cgroup` | `0::/` or `/docker/...` | Cannot fix |
| **Config Paths** | LOW | npm help output | `/root/.npmrc` | Set HOME correctly |

## Key Discovery: What Leaks vs What Doesn't

### LEAKS (Detectable)
- ✗ Hostname (container ID format)
- ✗ Platform mismatch (Linux vs macOS)
- ✗ User context (root instead of host user)
- ✗ Home directory (/root instead of /Users/username)
- ✗ PID 1 process (container init)
- ✗ Error messages with container paths

### DOES NOT LEAK (Hidden)
- ✓ Command execution mechanism (no "docker exec" in output)
- ✓ PATH variable (looks normal)
- ✓ Command locations (which/type show normal paths on host)
- ✓ Exit codes (properly preserved)
- ✓ stdout/stderr (properly separated)

## Detection Scenarios

### Scenario 1: Claude Code runs version check
```bash
node --version  # Works fine, no leak
```
**Result**: No detection

### Scenario 2: Claude Code runs debugging probe
```javascript
node -e 'console.log(require("os").hostname())'
// Output: 6b4fcc1cd298
```
**Result**: Container detected via hostname

### Scenario 3: Claude Code checks platform consistency
```javascript
node -e 'console.log(require("os").platform())'
// Output: linux
// But Claude knows host is macOS (from env context)
```
**Result**: Container detected via platform mismatch

### Scenario 4: npm install fails with error
```
npm error A complete log of this run can be found in: /root/.npm/_logs/...
```
**Result**: Container suspected via /root path

### Scenario 5: Claude tries to cd to unmounted directory
```bash
cd /tmp/some-dir
npm install
# Output: OCI runtime exec failed: exec failed: unable to start container process
```
**Result**: Container explicitly revealed in error message

## Recommended Mitigations (Priority Order)

1. **MUST FIX**:
   - Set `--hostname` to match host
   - Use `--user $(id -u):$(id -g)` to match host user
   - Set `-e HOME=$HOME` to match host home
   - Ensure complete filesystem mounts (no "OCI runtime" errors)

2. **SHOULD FIX**:
   - Always use `-w "$PWD"` for working directory
   - Mount user's home directory

3. **CANNOT FIX**:
   - Platform reporting (will always be Linux kernel)
   - PID 1 process (without complex PID namespace manipulation)
   - Cgroup paths (visible in /proc/self/cgroup)

## Test Commands for Validation

After implementing mitigations, run these tests:

```bash
# Should match host hostname
node -e 'console.log(require("os").hostname())'

# Should show host user, not root
node -e 'console.log(require("os").userInfo().username)'

# Should show host home, not /root
node -e 'console.log(process.env.HOME)'

# Should not mention "OCI runtime" or "container"
cd /tmp/test-dir && npm install nonexistent-pkg 2>&1 | grep -i "OCI\|container\|docker"

# Should show host UID, not 0
node -e 'console.log(require("os").userInfo().uid)'
```

## Conclusion

**Current State**: Shell function interception leaks Docker abstractions through multiple vectors.

**Feasibility**: Most leaks are fixable with proper container configuration (hostname, user mapping, HOME env, complete mounts).

**Limitation**: Platform mismatch (Linux kernel in container vs macOS host) is unfixable but may be acceptable if other leaks are closed.

**Verdict**: Transparency is achievable for common cases, but perfect abstraction is impossible due to kernel-level differences.
