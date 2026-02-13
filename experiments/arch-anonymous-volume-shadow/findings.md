---
date_run: 2026-02-06T12:42:22Z
experiment_design: inline (anonymous-volume-shadow)
status: complete
tests_run: 5 of 5
duration: ~10s
---

# Experiment Findings: Anonymous Volume Shadows node_modules

## Test Results

### Test 1.1: Anonymous Volume Shadows Bind Mount Path
**Procedure**: Created host node_modules with marker file, mounted project with `-v /workspace/node_modules`
**Result**: Container's node_modules is empty (anonymous volume), host marker NOT visible
**Host node_modules**: Still has `.host-marker` (unchanged)
**Verdict**: PASS — anonymous volume completely shadows the bind-mounted node_modules

### Test 1.2: npm install Writes to Container Volume, Not Host
**Procedure**: Ran `npm install lodash` inside container
**Result**: Container has `lodash` in node_modules; host node_modules only has `.host-marker`
**Verdict**: PASS — npm installs go to container-local volume

### Test 2.1: Volume Persists Across Exec Sessions
**Procedure**: Read lodash/package.json from container
**Result**: lodash 4.17.23 visible and readable
**Verdict**: PASS

### Test 2.2: Container Restart Preserves Volume
**Procedure**: `docker restart vol-test`, then check node_modules
**Result**: lodash still present after restart
**Verdict**: PASS — anonymous volumes survive restarts

### Test 3.1: Host Package.json Changes Visible Despite Volume Shadow
**Procedure**: Modified package.json on host, read from container
**Result**: Container sees updated package.json (version 2.0.0)
**Verdict**: PASS — only node_modules is shadowed; other bind-mounted files work normally

## Summary

| Aspect | Result |
|--------|--------|
| Volume shadows host node_modules | Yes |
| npm install stays in container | Yes |
| Host node_modules unchanged | Yes |
| Other bind-mounted files visible | Yes |
| Volume survives restart | Yes |

The `-v /workspace/node_modules` syntax creates an anonymous Docker volume that shadows the node_modules path within the bind mount. This elegantly separates platform-specific compiled dependencies (in container) from source code (shared via bind mount).
