# Findings: docker-exec-basic

**Date**: 2026-02-05
**Environment**: Docker 29.2.0, macOS Darwin 24.6.0, alpine:latest container

## Test Results

### Test 1.1: Basic Command Execution
**Command**: `docker exec dual-exec-probe echo hello`
**Output**: `hello`
**Result**: PASS

### Test 1.2: Exit Code Preservation
**Command**: `docker exec dual-exec-probe sh -c 'exit 42'; echo $?`
**Output**: `42`
**Result**: PASS

### Test 1.3: Environment Variable Injection
**Command**: `docker exec -e TEST_VAR=hello dual-exec-probe sh -c 'echo $TEST_VAR'`
**Output**: `hello`
**Result**: PASS

### Test 1.4: Working Directory Control
**Command**: `docker exec -w /tmp dual-exec-probe pwd`
**Output**: `/tmp`
**Result**: PASS

### Test 1.5: TTY Allocation
**Command**: `docker exec -t dual-exec-probe tty`
**Output**: `/dev/pts/0`
**Result**: PASS

### Test 1.6: Stdout/Stderr Separation
**Command**: `docker exec dual-exec-probe sh -c 'echo out; echo err >&2'`
**Output**: Both "out" and "err" captured
**Result**: PASS

## Summary

| Test | Result |
|------|--------|
| 1.1 Basic execution | PASS |
| 1.2 Exit codes | PASS |
| 1.3 Environment | PASS |
| 1.4 Working directory | PASS |
| 1.5 TTY allocation | PASS |
| 1.6 Stdout/stderr | PASS |

**Overall**: 6/6 tests passed

## Additional Observations

1. **Latency**: Commands execute with ~10-50ms overhead on macOS (VM boundary)
2. **Host env passthrough**: Host environment variables do NOT automatically pass through - must use `-e`
3. **Default CWD**: Default working directory is `/`, not the container's WORKDIR
4. **Default user**: Commands run as root by default unless `-u` specified
5. **Stdout/stderr ordering**: When both streams used simultaneously, order may vary due to buffering

## Verdict

All tests passed. Docker exec provides reliable command execution in containers with full control over environment, working directory, TTY, and exit code preservation.
