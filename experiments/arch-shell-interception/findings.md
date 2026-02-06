# Findings: shell-interception

**Date**: 2026-02-05
**Environment**: zsh, Docker 29.2.0, macOS Darwin 24.6.0

## Test Results

### Test 5.1: Basic Interception
**Setup**: `npm() { echo "[INTERCEPTED] npm $@"; }`
**Test**: `npm install`
**Output**: `[INTERCEPTED] npm install`
**Result**: PASS - function executes before PATH lookup

### Test 5.2: Argument Passthrough
**Setup**: `npm() { echo "args: $@"; }`
**Test**: `npm install lodash --save`
**Output**: `args: install lodash --save`
**Result**: PASS - all arguments preserved

### Test 5.3: Exit Code Preservation
**Setup**: `npm() { return 42; }`
**Test**: `npm; echo $?`
**Output**: `42`
**Result**: PASS - exit code matches return

### Test 5.4: Special Characters
**Tests**:
- `npm install "package with spaces"` → preserved
- `npm run 'quoted arg'` → preserved
- `npm install \$literal` → preserved
**Result**: PASS - special characters handled correctly with `"$@"`

### Test 5.5: TTY Detection
**Setup**: `npm() { test -t 1 && echo "TTY" || echo "NO TTY"; }`
**Interactive**: `npm` → "TTY"
**In pipe**: `npm | cat` → "NO TTY"
**Result**: PASS - correct TTY detection

### Test 5.6: Docker Exec Integration
**Setup**:
```bash
npm() {
  local flags=""
  test -t 1 && flags="-t"
  docker exec $flags container npm "$@"
}
```
**Test**: `npm --version`
**Output**: Version from container's npm
**Result**: PASS - routing works

### Test 5.7: Bypass Methods
| Pattern | Result |
|---------|--------|
| `command npm` | Bypasses function (calls real binary) |
| `/usr/bin/npm` | Bypasses function (absolute path) |
| `\npm` | Does NOT bypass in zsh (differs from bash) |

**Result**: PASS - bypass methods documented

## Summary

| Test | Result | Notes |
|------|--------|-------|
| 5.1 Basic interception | PASS | Functions intercept before PATH |
| 5.2 Argument passthrough | PASS | `"$@"` preserves all |
| 5.3 Exit codes | PASS | Correctly propagated |
| 5.4 Special characters | PASS | Handled with proper quoting |
| 5.5 TTY detection | PASS | `test -t 1` works |
| 5.6 Docker exec | PASS | End-to-end routing works |
| 5.7 Bypass methods | PASS | Documented |

**Overall**: 7/7 tests passed

## Additional Observations

1. **Function template**:
   ```bash
   npm() {
     local flags=""
     test -t 1 && flags="-t"
     docker exec $flags container-name npm "$@"
   }
   ```

2. **zsh vs bash**: Backslash prefix (`\npm`) bypasses in bash but NOT in zsh

3. **Needs rc injection**: Functions must be defined in `.bashrc`/`.zshrc`

4. **PATH shims for scripts**: Scripts that don't source rc need PATH shims

## Verdict

Shell function interception is viable for transparent command routing. The mechanism works for the common case (user-typed and Claude Code commands). Absolute paths are a known bypass but acceptable for the target use case.
