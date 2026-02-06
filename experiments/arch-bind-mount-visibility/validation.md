# Validation: bind-mount-visibility

**Date**: 2026-02-05
**Claim**: "File edits on the host are immediately visible inside the container"

## Comparison: Research vs Experiment

| Aspect | Research Prediction | Experiment Result | Match |
|--------|---------------------|-------------------|-------|
| Visibility mechanism | VirtioFS (FUSE) | Confirmed | YES |
| File creation | Works | Works | YES |
| File modification | Works | Works | YES |
| File deletion | Works | Works | YES |
| Bidirectional | Works | Works | YES |
| inotify events | Propagate | Propagate | YES |
| Latency | 2-200ms range | ~200ms | YES* |

*Research suggested optimized VirtioFS could achieve 2-10ms; empirical testing showed ~200ms on this system.

## Spec Claim Analysis

**Original Claim**: "File edits on the host are **immediately** visible inside the container"

**Validation**:
- Files ARE visible in container without manual sync
- Propagation latency is ~200ms on Docker Desktop macOS
- inotify events DO propagate for file watchers
- Bidirectional sync WORKS

**Verdict**: CONFIRMED WITH CAVEATS

## Caveats Discovered

1. **"Immediately" has nuance**: ~200ms latency on Docker Desktop macOS
   - Still acceptable for hot reload (feels instant to humans)
   - Most dev servers debounce anyway

2. **Event multiplicity**: Single write generates 3 inotify events
   - Dev servers should debounce or dedupe

3. **MOVE semantics**: Moves appear as CREATE, not atomic MOVE
   - May cause duplicate events in some watchers

## Impact on SPEC.md

The claim is functionally accurate. Optional clarification:

Current:
> "File edits on the host are immediately visible inside the container"

Could add:
> "File edits on the host are immediately visible inside the container (via VirtioFS, typically <500ms propagation)"

**Recommendation**: No change required. The spec claim is accurate for developer expectations. The 200ms latency is an implementation detail that doesn't affect the architectural validity.

## Final Verdict

**CONFIRMED WITH CAVEATS**

- The mechanism works as specified
- Latency (~200ms) is acceptable for development workflows
- No architectural changes needed
- Implementation should account for event debouncing
