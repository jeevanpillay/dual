# Validation: docker-exec-basic

**Date**: 2026-02-05
**Claim**: "dual run <command> wraps docker exec <container-name> <command>"

## Comparison: Research vs Experiment

| Aspect | Research Prediction | Experiment Result | Match |
|--------|---------------------|-------------------|-------|
| Basic execution | Works | Works | YES |
| Exit codes | Preserved | Preserved | YES |
| Environment | Requires -e flag | Requires -e flag | YES |
| Working directory | Requires -w flag | Requires -w flag | YES |
| TTY | Requires -t flag | Requires -t flag | YES |
| Stdout/stderr | Both captured | Both captured | YES |

## Spec Claim Analysis

**Original Claim**: "dual run <command> wraps docker exec <container-name> <command>"

**Validation**:
- Docker exec CAN execute commands in running containers
- Exit codes ARE preserved
- Environment and CWD CAN be controlled with flags
- TTY allocation WORKS for interactive commands

**Verdict**: CONFIRMED

## Caveats Discovered

The spec claim is valid, but the implementation of `dual run` must:

1. **Forward environment variables** - Use `-e` for each variable that should be accessible
2. **Set working directory** - Use `-w` to preserve the expected CWD
3. **Detect TTY requirements** - Use `-it` for interactive commands, omit for scripts
4. **Handle user identity** - Consider `-u` if file permission preservation matters

These are implementation details, not limitations of docker exec itself.

## Impact on SPEC.md

No changes needed to SPEC.md. The claim is accurate. The implementation details above are refinements that should be captured in the architecture or implementation docs.

## Final Verdict

**CONFIRMED** - Docker exec provides the fundamental mechanism needed for `dual run` to wrap commands transparently.
