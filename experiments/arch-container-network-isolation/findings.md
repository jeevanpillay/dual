# Findings: container-network-isolation

**Date**: 2026-02-05
**Environment**: Docker 29.2.0, macOS Darwin 24.6.0, bridge network

## Test Results

### Test 3.1: Multiple Containers Bind Same Port
**Setup**: 7 containers, each running HTTP server on :3000
**Result**: All 7 containers started successfully
**Port bindings**: All bound :3000 with zero conflicts
**Result**: PASS

### Test 3.2: Namespace ID Uniqueness
**Method**: `ls -la /proc/self/ns/net` in each container
**Observation**: Each container reported distinct namespace IDs
**Result**: PASS

### Test 3.3: localhost Isolation
**Setup**: Container A serves "A", Container B serves "B" on :3000
**Test**: `curl localhost:3000` from within each
**Results**:
- Container A: Received "A"
- Container B: Received "B"
**Result**: PASS - localhost is isolated

### Test 3.4: Cross-Container Access
**Setup**: Containers on default bridge network
**IPs**: 172.17.0.2, 172.17.0.3, etc.
**Test**: curl from one container to another's IP:3000
**Result**: PASS - cross-container communication works

### Test 3.5: Independent Accessibility
**Setup**: 7 containers with unique content
**Access method**: Host port mapping (8001-8007 → 3000)
**Results**: Each container returned its unique content
**Result**: PASS

## Summary

| Test | Result | Notes |
|------|--------|-------|
| 3.1 Same port binding | PASS | 7 containers, zero conflicts |
| 3.2 Namespace uniqueness | PASS | Distinct /proc/self/ns/net |
| 3.3 localhost isolation | PASS | Each sees only own service |
| 3.4 Cross-container access | PASS | Via container IPs |
| 3.5 Independent accessibility | PASS | All reachable |

**Overall**: 5/5 tests passed

## Additional Observations

1. **Scaling**: Easily scaled to 7 containers; no indication of practical limits
2. **Performance**: No noticeable overhead from namespace isolation
3. **IP allocation**: Bridge network assigned IPs automatically (172.17.0.x)
4. **Other ports**: Each container also has other ports available (3001, etc.)

## Verdict

Container network namespace isolation works exactly as documented. Multiple containers can bind the same port simultaneously with complete isolation.
