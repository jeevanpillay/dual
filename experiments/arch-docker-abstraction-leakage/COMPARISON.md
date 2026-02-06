# What Claude Code Sees: Host vs Container (Comparison)

## Scenario: Claude Code investigates the environment

### Test 1: Check hostname

#### On Native Host
```javascript
node -e 'console.log(require("os").hostname())'
```
**Output**: `iv-macbook.local`

#### With Basic Container (Current Implementation)
```javascript
node -e 'console.log(require("os").hostname())'
```
**Output**: `6b4fcc1cd298`
**Status**: 🔴 LEAKS - Clearly a container ID

#### With Mitigated Container (docker run --hostname)
```javascript
node -e 'console.log(require("os").hostname())'
```
**Output**: `iv-macbook.local`
**Status**: ✅ HIDDEN - Matches host

---

### Test 2: Check current user

#### On Native Host
```javascript
node -e 'console.log(require("os").userInfo().username)'
```
**Output**: `jeevanpillay`

#### With Basic Container
```javascript
node -e 'console.log(require("os").userInfo().username)'
```
**Output**: `root`
**Status**: 🔴 LEAKS - Running as root is suspicious

#### With Mitigated Container (docker run --user)
```javascript
node -e 'console.log(require("os").userInfo().username)'
```
**Output**: `jeevanpillay` (or UID 501 if not in /etc/passwd)
**Status**: ⚠️ MOSTLY HIDDEN - Matches host UID

---

### Test 3: Check home directory

#### On Native Host
```bash
echo $HOME
```
**Output**: `/Users/jeevanpillay`

#### With Basic Container
```bash
echo $HOME
```
**Output**: `/root`
**Status**: 🔴 LEAKS - Root home directory

#### With Mitigated Container (docker run -e HOME)
```bash
echo $HOME
```
**Output**: `/Users/jeevanpillay`
**Status**: ✅ HIDDEN - Matches host

---

### Test 4: Check platform

#### On Native Host (macOS)
```javascript
node -e 'console.log(require("os").platform())'
```
**Output**: `darwin`

#### With Basic Container
```javascript
node -e 'console.log(require("os").platform())'
```
**Output**: `linux`
**Status**: 🔴 LEAKS - Platform mismatch (macOS → Linux)

#### With Mitigated Container
```javascript
node -e 'console.log(require("os").platform())'
```
**Output**: `linux`
**Status**: 🔴 CANNOT FIX - Kernel-level difference

---

### Test 5: npm error messages

#### On Native Host
```bash
npm view nonexistent-package
```
**Output**:
```
npm error 404 Not Found
npm error A complete log of this run can be found in:
npm error     /Users/jeevanpillay/.npm/_logs/2026-02-05T12_30_00_123Z-debug-0.log
```

#### With Basic Container
```bash
npm view nonexistent-package
```
**Output**:
```
npm error 404 Not Found
npm error A complete log of this run can be found in:
npm error     /root/.npm/_logs/2026-02-05T12_30_00_123Z-debug-0.log
```
**Status**: 🔴 LEAKS - Shows /root instead of user home

#### With Mitigated Container (--user + -e HOME + volume mount)
```bash
npm view nonexistent-package
```
**Output**:
```
npm error 404 Not Found
npm error A complete log of this run can be found in:
npm error     /Users/jeevanpillay/.npm/_logs/2026-02-05T12_30_00_123Z-debug-0.log
```
**Status**: ✅ HIDDEN - Matches host paths

---

### Test 6: Check PID 1

#### On Native Host (macOS)
```bash
ps -p 1 -o comm=
```
**Output**: `launchd`

#### On Native Host (Linux)
```bash
ps -p 1 -o comm=
```
**Output**: `systemd` or `init`

#### With Any Container
```bash
ps -p 1 -o comm=
```
**Output**: `sleep` (or whatever container entrypoint is)
**Status**: 🔴 CANNOT EASILY FIX - Requires PID namespace tricks

---

### Test 7: Unmounted directory access

#### On Native Host
```bash
cd /tmp/some-dir && npm install
```
**Output**:
```
npm error ENOENT: no such file or directory, open '/tmp/some-dir/package.json'
```

#### With Basic Container (directory not mounted)
```bash
cd /tmp/some-dir && npm install
```
**Output**:
```
OCI runtime exec failed: exec failed: unable to start container process: 
chdir to cwd ("/tmp/some-dir") set in config.json failed: no such file or directory
```
**Status**: 🔴 CATASTROPHIC LEAK - Explicitly mentions "OCI runtime" and "container"

#### With Mitigated Container (complete filesystem mounted)
```bash
cd /tmp/some-dir && npm install
```
**Output**:
```
npm error ENOENT: no such file or directory, open '/tmp/some-dir/package.json'
```
**Status**: ✅ HIDDEN - Normal npm error

---

## Summary Matrix

| Test | Native Host | Basic Container | Mitigated Container | Fixable? |
|------|-------------|-----------------|---------------------|----------|
| Hostname | `iv-macbook.local` | `6b4fcc1cd298` 🔴 | `iv-macbook.local` ✅ | YES |
| User | `jeevanpillay` | `root` 🔴 | `jeevanpillay` ✅ | YES |
| HOME | `/Users/jeevanpillay` | `/root` 🔴 | `/Users/jeevanpillay` ✅ | YES |
| Platform | `darwin` | `linux` 🔴 | `linux` 🔴 | NO |
| npm paths | `/Users/.npm/_logs/` | `/root/.npm/_logs/` 🔴 | `/Users/.npm/_logs/` ✅ | YES |
| PID 1 | `launchd` | `sleep` 🔴 | `sleep` 🔴 | HARD |
| Missing mount | Normal error | `OCI runtime exec failed` 🔴 | Normal error ✅ | YES |

## Mitigation Configuration

To achieve the "Mitigated Container" state, use:

```bash
docker run -d \
  --hostname "$(hostname)" \
  --user "$(id -u):$(id -g)" \
  -e HOME="$HOME" \
  -v "$HOME:$HOME" \
  -v /tmp:/tmp \
  -w "$HOME" \
  node:20-alpine sleep infinity
```

And when executing commands:

```bash
npm() {
  local docker_flags=""
  test -t 1 && docker_flags="-t"
  docker exec $docker_flags -w "$PWD" container-name npm "$@"
}
```

## What Remains Detectable

Even with full mitigation, Claude Code could still detect containerization if it:

1. **Checks platform** - Will report `linux` even on macOS host
2. **Inspects PID 1** - Will show container init instead of system init
3. **Reads /proc/self/cgroup** - May show container cgroup paths
4. **Checks kernel version** - Container kernel != host kernel (especially macOS)

However, these require **active probing**. For passive observation (running normal dev commands), the container is effectively transparent with proper mitigation.

## Recommendation

**For Dual's use case (Claude Code running dev commands):**
- Implement full mitigation (hostname, user, HOME, mounts)
- Accept platform/PID1 leakage as edge case
- Claude Code rarely inspects PID 1 or platform in normal workflow
- Most leakage comes from error messages, which are fixable

**The core invariant can hold** if:
1. All expected directories are mounted
2. User/HOME context matches host
3. Hostname matches host
4. Error messages show host paths, not /root

Platform mismatch is the one remaining leak, but Claude Code typically doesn't check `os.platform()` unless debugging cross-platform issues.
