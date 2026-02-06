# Docker Abstraction Leakage - Empirical Findings

**Date**: 2026-02-05
**Environment**: Docker 29.2.0, macOS Darwin 24.6.0, node:20-alpine container
**Test Setup**: Shell function interception routing commands to `docker exec`

## Executive Summary

**VERDICT: SIGNIFICANT LEAKAGE DETECTED**

When shell function interception routes commands through `docker exec`, multiple aspects of the container environment leak through to the caller. A sophisticated observer (like Claude Code running debugging commands) can detect containerization through:

1. **Hostname** (container ID format)
2. **File paths** (container-specific paths in error messages)
3. **Process info** (PID 1 reveals container init)
4. **User context** (running as root)
5. **Platform mismatch** (Linux reported when host is macOS)

---

## Probe 1: Error Message Inspection

### Test Setup
```bash
npm() {
  docker exec dual-leakage-test-mounted npm "$@"
}
```

### Test 1.1: Invalid Flag Error
**Command**: `npm --invalid-flag-xyz`
**Output**:
```
npm <command>
...
Specify configs in the ini-formatted file:
    /root/.npmrc
or on the command line via: npm <command> --key=value
...
npm@10.8.2 /usr/local/lib/node_modules/npm
```

**Leaks**:
- `/root/.npmrc` → Container path, not host user's home
- `/usr/local/lib/node_modules/npm` → Container npm location

### Test 1.2: Nonexistent Package Error
**Command**: `npm view nonexistent-package-xyz-123-456`
**Output**:
```
npm error code E404
npm error 404 Not Found - GET https://registry.npmjs.org/nonexistent-package-xyz-123-456 - Not found
npm error A complete log of this run can be found in: /root/.npm/_logs/2026-02-05T12_26_58_072Z-debug-0.log
```

**Leaks**:
- `/root/.npm/_logs/` → Container root user's home directory

### Test 1.3: Node Syntax Error
**Command**: `node -e 'this is invalid syntax'`
**Output**:
```
[eval]:1
this is invalid syntax
     ^^

SyntaxError: Unexpected identifier 'is'
    at makeContextifyScript (node:internal/vm:185:14)
    ...

Node.js v20.20.0
```

**Leaks**: 
- Stack traces show internal Node.js paths but these are standard
- No obvious container-specific information in stack traces

### Discovery: Error Path Leakage
**Finding**: Error messages consistently reveal `/root/` paths, exposing that the process runs as container root user, not the host user.

---

## Probe 2: Process Inspection

### Test 2.1: Process List
**Command**: `docker exec dual-leakage-test ps aux`
**Output**:
```
PID   USER     TIME  COMMAND
    1 root      0:00 sleep infinity
  116 root      0:00 ps aux
```

**CRITICAL LEAK**: PID 1 is `sleep infinity`, not a system init process
- On real macOS: PID 1 would be `launchd`
- On real Linux: PID 1 would be `systemd` or `init`
- This is a dead giveaway of containerization

### Test 2.2: Command Location
**Command**: `docker exec dual-leakage-test which npm`
**Output**: `/usr/local/bin/npm`

**No leak**: Standard binary path, nothing container-specific

### Test 2.3: Command Type
**Command**: `docker exec dual-leakage-test sh -c 'type npm'`
**Output**: `npm is /usr/local/bin/npm`

**No leak**: Standard output

### Discovery: PID 1 Process
**Finding**: The PID 1 process is the most reliable indicator of containerization. Any inspection of `/proc/1/cmdline` or `ps` output immediately reveals the container environment.

---

## Probe 3: Environment Inspection

### Test 3.1: Docker-Specific Variables
**Command**: `docker exec dual-leakage-test env | grep -i docker`
**Output**: (empty)

**No leak**: No environment variables with "docker" in the name

### Test 3.2: Hostname
**Command**: `docker exec dual-leakage-test hostname`
**Output**: `b246fe556857`

**CRITICAL LEAK**: Hostname is the container ID (first 12 chars)
- Real macOS hostname: `iv-macbook.local`
- Container hostname: `b246fe556857` (hex string)
- Easily detectable pattern

### Test 3.3: PID 1 Command Line
**Command**: `docker exec dual-leakage-test cat /proc/1/cmdline`
**Output**: `sleep infinity`

**CRITICAL LEAK**: Confirms containerization immediately

### Test 3.4: Cgroup Info
**Command**: `docker exec dual-leakage-test cat /proc/self/cgroup`
**Output**: `0::/`

**Minor leak**: Modern Docker (cgroups v2) shows `0::/` which is less obvious than older versions that showed `/docker/...` paths

### Test 3.5: Environment Variables
**Command**: `docker exec dual-leakage-test env`
**Output**:
```
HOME=/root
HOSTNAME=b246fe556857
NODE_VERSION=20.20.0
PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
YARN_VERSION=1.22.22
```

**Leaks**:
- `HOME=/root` → Running as root
- `HOSTNAME=b246fe556857` → Container ID

### Discovery: Hostname Pattern
**Finding**: The hostname in container format (12-char hexadecimal) is immediately recognizable as a container ID.

---

## Probe 4: Path Inspection

### Test 4.1: PATH Variable
**Command**: `docker exec dual-leakage-test sh -c 'echo $PATH'`
**Output**:
```
/usr/local/sbin
/usr/local/bin
/usr/sbin
/usr/bin
/sbin
/bin
```

**No leak**: Standard Unix paths, nothing container-specific

### Test 4.2: Default Working Directory
**Command**: `docker exec dual-leakage-test pwd` (no `-w` flag)
**Output**: `/`

**Minor leak**: Working directory defaults to root `/` instead of user's home

### Test 4.3: Error Paths
**Command**: `node /tmp/bad.js` (file with syntax error)
**Output**:
```
/tmp/bad.js:1
this is bad syntax
...
```

**Leak**: File path in error reveals container filesystem, not host path

### Test 4.4: npm Error Log Paths
**Command**: `npm view fake-pkg`
**Output**:
```
npm error A complete log of this run can be found in: /root/.npm/_logs/2026-02-05T12_27_36_600Z-debug-0.log
```

**Leak**: Persistent `/root/` paths in error messages

---

## Probe 5: Comprehensive Leakage via Node.js

Testing what Claude Code could discover by running Node.js inspection scripts:

### Test 5.1: Hostname Check
**Command**: `node -e 'console.log("Hostname:", require("os").hostname())'`
**Output**: `Hostname: 6b4fcc1cd298`

**CRITICAL LEAK**: Container ID visible

### Test 5.2: Platform and Architecture
**Command**: `node -e 'const os = require("os"); console.log("Platform:", os.platform(), "Arch:", os.arch())'`
**Output**: `Platform: linux Arch: arm64`

**CRITICAL LEAK**: Reports `linux` when host is macOS
- If Claude Code knows it's on macOS (from env context), this mismatch is suspicious
- Cross-platform discrepancy is a red flag

### Test 5.3: Home Directory
**Command**: `node -e 'console.log("HOME:", process.env.HOME)'`
**Output**: `HOME: /root`

**LEAK**: Running as root user

### Test 5.4: Current User
**Command**: `node -e 'console.log("User:", require("os").userInfo().username, "UID:", require("os").userInfo().uid)'`
**Output**: `User: root UID: 0`

**LEAK**: Explicitly identifies as root user (UID 0)

### Test 5.5: PID 1 Inspection
**Command**: `node -e 'console.log("PID 1 cmdline:", require("fs").readFileSync("/proc/1/cmdline", "utf8"))'`
**Output**: `PID 1 cmdline: sleep infinity`

**CRITICAL LEAK**: Direct evidence of containerization

### Test 5.6: Cgroup Check
**Command**: `node -e 'console.log(require("fs").readFileSync("/proc/self/cgroup", "utf8"))'`
**Output**: `0::/`

**Minor leak**: Cgroup info less revealing in modern Docker

---

## Probe 6: Missing Directory Error (CRITICAL)

### Test: Working Directory Not Mounted
**Setup**: Shell function uses `-w "$PWD"` but directory not mounted in container
**Command**: `npm --version` (from `/tmp/test-dir`)
**Output**:
```
OCI runtime exec failed: exec failed: unable to start container process: 
chdir to cwd ("/tmp/test-dir") set in config.json failed: no such file or directory
```

**CATASTROPHIC LEAK**:
- "OCI runtime exec failed" → Explicitly mentions container runtime
- "container process" → Uses the word "container"
- "config.json" → Docker/OCI internal terminology

**Discovery**: If directories are not properly mounted, error messages directly expose Docker internals.

---

## Summary of Leaks by Severity

### Critical (Immediate Detection)
1. **Hostname**: Container ID format (12-char hex) vs real hostname
2. **Platform Mismatch**: Reports `linux` when host is macOS
3. **PID 1 Process**: `sleep infinity` instead of system init
4. **Missing Mount Errors**: "OCI runtime exec failed" explicitly mentions containers

### High (Easily Detectable)
5. **User Context**: Running as `root` (UID 0) instead of host user
6. **HOME Path**: `/root` instead of `/Users/username`
7. **Error Log Paths**: Persistent `/root/.npm/_logs/` in npm errors

### Medium (Detectable with Investigation)
8. **Working Directory**: Defaults to `/` without `-w` flag
9. **Cgroup Info**: Shows `0::/` (less obvious in cgroups v2)

### Low (Subtle Indicators)
10. **Config Paths**: `/root/.npmrc` in help messages
11. **npm Install Location**: `/usr/local/lib/node_modules/npm`

---

## Implications for Dual

### Can Claude Code Detect Containerization?

**YES**, through multiple vectors:

1. **Proactive Detection**: If Claude Code runs debugging commands like:
   ```javascript
   node -e 'console.log(require("os").hostname())'
   node -e 'console.log(require("os").platform())'
   node -e 'require("fs").readFileSync("/proc/1/cmdline", "utf8")'
   ```
   Containerization is immediately obvious.

2. **Reactive Detection**: Error messages mentioning `/root/` paths or missing mount errors reveal the container environment.

3. **Platform Consistency**: If Claude Code is told it's on macOS but Node.js reports `linux`, this is a clear inconsistency.

### Mitigation Strategies Required

To achieve true transparency, Dual would need:

1. **Hostname Spoofing**: Set container hostname to match host hostname
2. **User Mapping**: Run container processes as matching UID/GID of host user
3. **HOME Path Mapping**: Set HOME to match host user's home directory
4. **Complete Bind Mounts**: Ensure all directories are mounted to prevent "OCI runtime" errors
5. **Platform Reporting**: (Impossible to fix - container will always report Linux kernel)

### What Cannot Be Hidden

Some leaks are fundamental to containerization:
- **Platform**: Container kernel will always be Linux (even on macOS/Windows)
- **PID 1**: Will always be container init unless using PID namespace magic
- **Cgroups**: /proc/self/cgroup will show container cgroup paths (depending on version)

---

## Conclusion

The current shell function interception approach **leaks Docker abstractions significantly**. While error messages don't explicitly mention "docker" or "container" in most cases, the environment reveals containerization through:

- Container ID hostname pattern
- Platform mismatch (Linux vs macOS)
- Root user context
- Container-specific file paths
- PID 1 process inspection

A sophisticated AI agent like Claude Code could detect containerization through standard debugging practices. The leakage is **not subtle** to an observer who knows what to look for.

For Dual's core invariant to hold ("Claude Code must never need to know that commands are routed to containers"), additional abstraction layers beyond simple shell function interception are required.

---

## Test Artifacts

All tests performed with:
- Container: `node:20-alpine`
- Docker: 29.2.0
- Host: macOS Darwin 24.6.0
- Shell: bash with function interception

Test scripts available in `/tmp/test_*.sh`
