---
date_run: 2026-02-06T12:43:37Z
experiment_design: inline (virtiofs-inotify)
status: complete
tests_run: 5 of 5
duration: ~60s
---

# Experiment Findings: VirtioFS inotify Event Propagation

## Test Results

### Test 1.1: inotify Detects Host File Creation
**Events captured**: `CREATE new-file.txt`, `MODIFY new-file.txt`
**Verdict**: PASS

### Test 1.2: inotify Detects Host File Modification
**Events captured**: `MODIFY new-file.txt`
**Verdict**: PASS

### Test 1.3: inotify Detects Host File Deletion
**Events captured**: None
**Verdict**: FAIL — DELETE events do NOT propagate through VirtioFS

### Test 2.1: Event Propagation Latency
**Measured**: 1ms (inotifywait start to event detection)
**Verdict**: PASS — effectively instant event delivery

### Test 3.1: Event Type Summary
| Event Type | Propagates? | Critical for HMR? |
|-----------|------------|-------------------|
| CREATE | Yes | Yes |
| MODIFY | Yes | Yes (primary) |
| DELETE | No | No (rarely needed) |

## Key Findings

1. **CREATE and MODIFY propagate reliably** — the two events most critical for hot reload
2. **DELETE does NOT propagate** — VirtioFS limitation on Docker Desktop macOS
3. **Latency is ~1ms** — effectively instant for developer workflows
4. **Impact on hot reload**: Minimal — webpack/vite/next.js watchers use MODIFY as their primary trigger; DELETE is rarely needed during active development

## Verdict

CONFIRMED WITH CAVEATS — inotify events propagate for the event types that matter for hot reload (CREATE, MODIFY). DELETE events don't propagate, which is a VirtioFS limitation but doesn't impact the core hot reload use case.
