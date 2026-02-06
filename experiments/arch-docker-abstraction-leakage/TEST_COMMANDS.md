# Reproducible Test Commands

All commands used to discover Docker abstraction leakage, with exact outputs.

## Setup

```bash
# Start test container with bind mount
docker run -d --name dual-leakage-test-mounted --rm \
  -v /Users/jeevanpillay:/Users/jeevanpillay \
  -w /Users/jeevanpillay \
  node:20-alpine sleep infinity

# Define interception functions
npm() {
  docker exec -w "$PWD" dual-leakage-test-mounted npm "$@"
}

node() {
  docker exec -w "$PWD" dual-leakage-test-mounted node "$@"
}

export -f npm node
```

## Error Message Inspection

### Test: Invalid npm flag
```bash
npm --invalid-flag-xyz 2>&1
```
**Output**:
```
npm <command>
...
Specify configs in the ini-formatted file:
    /root/.npmrc
...
```
**Leak**: `/root/.npmrc`

### Test: Nonexistent package
```bash
npm view nonexistent-package-xyz-123-456 2>&1
```
**Output**:
```
npm error 404 Not Found
npm error A complete log of this run can be found in: /root/.npm/_logs/2026-02-05T12_26_58_072Z-debug-0.log
```
**Leak**: `/root/.npm/_logs/`

### Test: Node syntax error
```bash
node -e 'this is invalid syntax' 2>&1
```
**Output**:
```
[eval]:1
this is invalid syntax
     ^^

SyntaxError: Unexpected identifier 'is'
    at makeContextifyScript (node:internal/vm:185:14)
    ...
```
**Leak**: None (standard Node error)

## Process Inspection

### Test: Process list
```bash
docker exec dual-leakage-test-mounted ps aux
```
**Output**:
```
PID   USER     TIME  COMMAND
    1 root      0:00 sleep infinity
  116 root      0:00 ps aux
```
**Leak**: PID 1 is `sleep infinity`

### Test: Command location
```bash
docker exec dual-leakage-test-mounted which npm
```
**Output**:
```
/usr/local/bin/npm
```
**Leak**: None

### Test: Command type
```bash
docker exec dual-leakage-test-mounted sh -c 'type npm'
```
**Output**:
```
npm is /usr/local/bin/npm
```
**Leak**: None

## Environment Inspection

### Test: Docker-specific env vars
```bash
docker exec dual-leakage-test-mounted env | grep -i docker
```
**Output**: (empty)
**Leak**: None

### Test: Hostname
```bash
docker exec dual-leakage-test-mounted hostname
```
**Output**:
```
6b4fcc1cd298
```
**Leak**: Container ID format

### Test: PID 1 command line
```bash
docker exec dual-leakage-test-mounted cat /proc/1/cmdline
```
**Output**:
```
sleep infinity
```
**Leak**: Container init process

### Test: Cgroup info
```bash
docker exec dual-leakage-test-mounted cat /proc/self/cgroup
```
**Output**:
```
0::/
```
**Leak**: Minor (cgroups v2 format)

### Test: Environment variables
```bash
docker exec dual-leakage-test-mounted env | sort
```
**Output**:
```
HOME=/root
HOSTNAME=6b4fcc1cd298
NODE_VERSION=20.20.0
PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
YARN_VERSION=1.22.22
```
**Leaks**: 
- `HOME=/root`
- `HOSTNAME=6b4fcc1cd298`

## Path Inspection

### Test: PATH contents
```bash
docker exec dual-leakage-test-mounted sh -c 'echo $PATH | tr : "\n"'
```
**Output**:
```
/usr/local/sbin
/usr/local/bin
/usr/sbin
/usr/bin
/sbin
/bin
```
**Leak**: None

### Test: Default working directory
```bash
docker exec dual-leakage-test-mounted pwd
```
**Output**:
```
/
```
**Leak**: Minor (defaults to root)

### Test: Error with file path
```bash
docker exec dual-leakage-test-mounted sh -c 'echo "bad syntax" > /tmp/bad.js'
docker exec dual-leakage-test-mounted node /tmp/bad.js 2>&1
```
**Output**:
```
/tmp/bad.js:1
bad syntax
    ^^^

SyntaxError: Unexpected identifier 'syntax'
    ...
```
**Leak**: Shows container path `/tmp/bad.js`

## Node.js Environment Probes

### Test: Hostname via Node
```bash
node -e 'console.log("Hostname:", require("os").hostname())' 2>&1
```
**Output**:
```
Hostname: 6b4fcc1cd298
```
**Leak**: Container ID

### Test: Platform and architecture
```bash
node -e 'const os = require("os"); console.log("Platform:", os.platform(), "Arch:", os.arch())' 2>&1
```
**Output**:
```
Platform: linux Arch: arm64
```
**Leak**: Reports `linux` when host is macOS

### Test: Home directory via Node
```bash
node -e 'console.log("HOME:", process.env.HOME)' 2>&1
```
**Output**:
```
HOME: /root
```
**Leak**: Container root home

### Test: Current user via Node
```bash
node -e 'console.log("User:", require("os").userInfo().username, "UID:", require("os").userInfo().uid)' 2>&1
```
**Output**:
```
User: root UID: 0
```
**Leak**: Root user (UID 0)

### Test: PID 1 via Node
```bash
node -e 'console.log("PID 1:", require("fs").readFileSync("/proc/1/cmdline", "utf8"))' 2>&1
```
**Output**:
```
PID 1: sleep infinity
```
**Leak**: Container init

### Test: Cgroup via Node
```bash
node -e 'console.log(require("fs").readFileSync("/proc/self/cgroup", "utf8"))' 2>&1
```
**Output**:
```
0::/
```
**Leak**: Minor

## Critical Test: Missing Directory Mount

### Test: Command in unmounted directory
```bash
# Without bind mount for /tmp/test-dir
docker exec -w "/tmp/test-dir" dual-leakage-test-mounted npm --version 2>&1
```
**Output**:
```
OCI runtime exec failed: exec failed: unable to start container process: 
chdir to cwd ("/tmp/test-dir") set in config.json failed: no such file or directory
```
**Leak**: CATASTROPHIC - explicitly mentions OCI runtime and container

## Comprehensive Detection Script

Save as `detect_container.js`:

```javascript
const os = require('os');
const fs = require('fs');

console.log('=== Environment Detection ===');
console.log('Platform:', os.platform());
console.log('Hostname:', os.hostname());
console.log('User:', os.userInfo().username);
console.log('UID:', os.userInfo().uid);
console.log('HOME:', process.env.HOME);
console.log('CWD:', process.cwd());

try {
  const pid1 = fs.readFileSync('/proc/1/cmdline', 'utf8').trim();
  console.log('PID 1:', pid1);
} catch (e) {
  console.log('PID 1: (cannot read)');
}

try {
  const cgroup = fs.readFileSync('/proc/self/cgroup', 'utf8').trim();
  console.log('Cgroup:', cgroup);
} catch (e) {
  console.log('Cgroup: (cannot read)');
}

console.log('\n=== Container Indicators ===');
const indicators = [];

// Check hostname pattern (12-char hex)
if (/^[a-f0-9]{12}$/.test(os.hostname())) {
  indicators.push('✗ Hostname matches container ID pattern');
}

// Check if running as root
if (os.userInfo().uid === 0) {
  indicators.push('✗ Running as root (UID 0)');
}

// Check if HOME is /root
if (process.env.HOME === '/root') {
  indicators.push('✗ HOME is /root');
}

// Check platform on macOS host
// (This check only makes sense if we know the host is macOS)
// if (os.platform() === 'linux' && /* host is macOS */) {
//   indicators.push('✗ Platform mismatch (Linux vs macOS)');
// }

// Check PID 1
try {
  const pid1 = fs.readFileSync('/proc/1/cmdline', 'utf8').trim();
  if (pid1 !== 'launchd' && pid1 !== 'systemd' && pid1 !== '/sbin/init') {
    indicators.push(`✗ PID 1 is "${pid1}" (not system init)`);
  }
} catch (e) {
  // Cannot read, skip
}

if (indicators.length === 0) {
  console.log('✓ No obvious container indicators detected');
} else {
  console.log('Container detected via:');
  indicators.forEach(i => console.log(i));
}
```

Run with:
```bash
node detect_container.js
```

## Cleanup

```bash
docker stop dual-leakage-test-mounted
```

---

## Notes

- All tests performed on macOS Darwin 24.6.0 with Docker 29.2.0
- Container: `node:20-alpine`
- Shell: bash with function interception
- Test date: 2026-02-05
