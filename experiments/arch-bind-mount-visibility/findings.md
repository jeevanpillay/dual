# Findings: bind-mount-visibility

**Date**: 2026-02-05
**Environment**: Docker 29.2.0, macOS Darwin 24.6.0, VirtioFS

## Test Results

### Test 2.1: File Creation Visibility
**Action**: `echo "test content" > /tmp/dual-bindmount-test/test-file.txt`
**Verification**: `docker exec <container> cat /workspace/test-file.txt`
**Output**: `test content from host`
**Result**: PASS

### Test 2.2: File Modification Visibility
**Action**: Modified file content on host
**Verification**: Read from container
**Output**: Updated content visible
**Result**: PASS

### Test 2.3: File Deletion Visibility
**Action**: `rm /tmp/dual-bindmount-test/test-file.txt`
**Verification**: `docker exec <container> ls /workspace/`
**Output**: Empty directory (file gone)
**Result**: PASS

### Test 2.4: Propagation Latency
**Method**: Polled file existence every 10ms from container
**Measurements** (5 runs):
- Run 1: 200ms
- Run 2: 190ms
- Run 3: 200ms
- Run 4: 190ms
- Run 5: 200ms
**Average**: 196ms
**Result**: PASS (within 500ms threshold)

### Test 2.5: File Watcher Events
**Setup**: `inotifywait -m /workspace/` in container
**Action**: Modified file on host
**Events captured**:
```
/workspace/ CREATE test-file.txt
/workspace/ ATTRIB test-file.txt
/workspace/ MODIFY test-file.txt
```
**Result**: PASS - events propagate

### Test 2.6: Bidirectional Sync
**Action**: `docker exec <container> sh -c 'echo "from container" > /workspace/container-file.txt'`
**Verification**: `cat /tmp/dual-bindmount-test/container-file.txt` on host
**Output**: `from container`
**Result**: PASS

## Summary

| Test | Result | Notes |
|------|--------|-------|
| 2.1 File creation | PASS | Visible immediately |
| 2.2 File modification | PASS | Changes propagate |
| 2.3 File deletion | PASS | Deletions propagate |
| 2.4 Latency | PASS | ~200ms (within 500ms) |
| 2.5 File watcher | PASS | inotify events work |
| 2.6 Bidirectional | PASS | Host sees container writes |

**Overall**: 6/6 tests passed

## Additional Observations

1. **Consistent latency**: 190-200ms very stable across runs
2. **Event multiplicity**: Single write generates 3 events (CREATE, ATTRIB, MODIFY)
3. **MOVE behavior**: Move appears as CREATE at destination, not atomic MOVE event
4. **No sync needed**: Changes are automatic, no manual refresh required

## Verdict

Bind mount file visibility works reliably. The 200ms latency is acceptable for development hot reload workflows.
