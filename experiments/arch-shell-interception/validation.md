# Validation: shell-interception

**Date**: 2026-02-05
**Claim**: "Claude Code's shell is configured so that commands are intercepted and routed"

## Comparison: Research vs Experiment

| Aspect | Research Prediction | Experiment Result | Match |
|--------|---------------------|-------------------|-------|
| Function interception | Works for command names | Confirmed | YES |
| Argument passthrough | `"$@"` preserves all | Confirmed | YES |
| Exit code preservation | Automatic | Confirmed | YES |
| TTY detection | `test -t 1` works | Confirmed | YES |
| Docker exec routing | Possible | Confirmed | YES |
| Absolute path bypass | Cannot intercept | Confirmed | YES |
| `command` bypass | Bypasses function | Confirmed | YES |

## Spec Claim Analysis

**Original Claim**: "Claude Code's shell is configured so that commands are intercepted and routed — Claude never sees `dual run` or `docker exec`"

**Validation**:
- Commands CAN be intercepted via shell functions
- Arguments and exit codes ARE preserved
- Docker exec routing WORKS end-to-end
- Claude Code WILL NOT see `docker exec` (function is transparent)

**Verdict**: CONFIRMED WITH CAVEATS

## Caveats Discovered

1. **Absolute paths bypass**: `/usr/bin/npm` cannot be intercepted
   - Acceptable: Claude Code uses command names, not paths
   - Risk: Low - most tooling uses PATH resolution

2. **Function must be loaded**: Requires rc file injection
   - Dual must add functions to user's `.bashrc`/`.zshrc`
   - Or: Use PATH shims for broader coverage

3. **Scripts may bypass**: Scripts that don't source rc miss functions
   - Mitigation: PATH shims provide fallback

## Implementation Recommendations

1. **Combined approach**: Functions (interactive) + PATH shims (scripts)
2. **Function template**:
   ```bash
   npm() {
     local flags=""
     test -t 1 && flags="-t"
     docker exec $flags $DUAL_CONTAINER npm "$@"
   }
   ```
3. **Inject into rc files**: Add functions to `.bashrc`/`.zshrc` on workspace activation
4. **Container detection**: Use `$DUAL_CONTAINER` env var to know which container

## Impact on SPEC.md

No changes needed. The spec claim is accurate:
- Commands ARE intercepted transparently
- Claude never sees `docker exec`
- The mechanism works for the intended use case

The absolute path limitation is fundamental and acceptable.

## Final Verdict

**CONFIRMED WITH CAVEATS**

Shell interception via functions is viable and provides transparent command routing. The spec's core claim holds: Claude Code can run `npm install` and it executes in the container without Claude knowing.

Implementation details (rc injection, PATH shims) are refinements, not limitations.
