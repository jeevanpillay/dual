---
date_run: 2026-02-06T12:52:31Z
experiment_design: inline (concurrent-containers)
status: complete
tests_run: 3 of 3
duration: ~10s
---

# Experiment Findings: Concurrent Containers

## Test Results

### Test 13.1: 5 Containers Running Simultaneously
**All 5 started**: Yes, all in "Up" status
**Each binding :3000**: Yes, no conflicts
**Memory per container**: ~720KB (Alpine + nc)
**Total Docker Desktop memory**: 7.653GiB available
**Spot checks**: Containers 1, 3, 5 all responded with correct identifiers

### Test 13.2: Memory Usage
| Container | Memory |
|-----------|--------|
| concurrent-1 | 720KB |
| concurrent-2 | 716KB |
| concurrent-3 | 728KB |
| concurrent-4 | 724KB |
| concurrent-5 | 732KB |

Note: Real workspaces with Node.js dev servers would use ~100-300MB each. With 7.6GB available, that's 25-75 workspaces theoretically (limited by CPU more than RAM).

### Test 13.3: Isolation Under Concurrency
All containers respond independently with their unique identifiers even when running simultaneously.

## Summary

5+ containers run simultaneously without resource issues. Memory overhead per lightweight container is <1MB. Real workspaces with dev servers would use more (~100-300MB) but Docker Desktop has ample headroom for typical development (5-10 workspaces).
