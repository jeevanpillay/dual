# Validation: shell-interception-transparency

**Date**: 2026-02-05
**Claim**: "Claude Code must never know it is running inside a container"

## Findings Summary

### Leakage Vectors Tested

| Vector | Leaks? | Fixable? | Risk Level |
|--------|--------|----------|------------|
| Hostname | YES | YES | CRITICAL → Fixed |
| Platform (os.platform) | YES | NO | CRITICAL but acceptable |
| User/UID | YES | YES | HIGH → Fixed |
| HOME path | YES | YES | HIGH → Fixed |
| Error messages | YES | YES | HIGH → Fixed |
| PID 1 cmdline | YES | HARD | CRITICAL but niche |
| Missing mount error | YES | YES | CATASTROPHIC → Fixed |
| `which` output | NO | N/A | OK |
| Exit codes | NO | N/A | OK |
| Argument handling | NO | N/A | OK |

### Mitigation Required

```bash
docker run -d \
  --hostname "$(hostname)" \
  --user "$(id -u):$(id -g)" \
  -e HOME="$HOME" \
  -v "$HOME:$HOME" \
  -v /tmp:/tmp \
  -w "$PWD" \
  container-image
```

### Unfixable but Acceptable

**Platform mismatch** (`linux` vs `darwin` on macOS):
- Cannot be fixed - containers run Linux kernel
- Low impact - rarely checked in typical dev workflows
- Claude Code doesn't routinely check `process.platform`

## Verdict

**CONFIRMED WITH CAVEATS**

The spec claim "Claude Code must never know it is running inside a container" is:

- **VALID for practical purposes** - typical dev commands work transparently
- **REQUIRES mitigation** - container must be configured properly
- **HAS unfixable gaps** - platform detection will reveal Linux

### Implementation Requirements for SPEC.md

The container lifecycle section should ensure:
1. Container hostname matches host
2. Container user matches host user
3. HOME env var matches host
4. All working directories are mounted
5. Error paths resolve to mounted locations

### Final Assessment

The core invariant can be maintained. Claude Code running `npm install`, `pnpm dev`, `node test.js` will not know it's in a container. Only deliberate platform probing would reveal the abstraction.

**Acceptable limitation**: Platform always reports `linux` on macOS. This is a fundamental property of containers and does not affect normal development workflows.
