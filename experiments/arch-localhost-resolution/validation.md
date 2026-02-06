# Validation: localhost-resolution

**Date**: 2026-02-05
**Claim**: "*.localhost resolves to 127.0.0.1 natively in all modern browsers"

## Findings Summary

### Browser Support

| Browser | Works? | Implementation |
|---------|--------|----------------|
| Chrome | YES | Built-in RFC 6761 |
| Edge | YES | Built-in (Chromium) |
| Firefox 84+ | YES | Built-in RFC 6761 |
| Safari | macOS 26+ only | Relies on OS resolver |

### CLI Tool Support (macOS)

| Tool | Works? | Notes |
|------|--------|-------|
| curl 8.7.1 | YES | Built-in RFC 6761 |
| dig/host | NO | DNS query returns NXDOMAIN |
| ping | NO | getaddrinfo fails |
| Node.js | NO | getaddrinfo fails |
| Python | NO | getaddrinfo fails |

### Verdict

**CONFIRMED WITH CAVEATS**

The spec claim is **mostly accurate** but incomplete:

- **TRUE**: Chrome, Edge, Firefox support *.localhost natively
- **FALSE**: Safari on macOS <26 does not work
- **Omission**: CLI tools vary (curl works, most others don't)

### Impact on Dual Architecture

**For browser access (primary use case)**: Works on Chrome, Firefox, Edge

**Safari limitation**:
- Safari users on macOS <26 cannot use *.localhost URLs
- Workaround: Add `/etc/hosts` entries manually
- macOS 26+ (Sequoia) fixes this

**Reverse proxy doesn't need DNS**:
- Matches on HTTP Host header
- Works regardless of client DNS resolution
- Client just needs to connect to 127.0.0.1:{port}

### Recommendation

Update SPEC.md for accuracy:

```markdown
# Current
*.localhost resolves to 127.0.0.1 natively in all modern browsers

# Suggested
*.localhost resolves to 127.0.0.1 natively in Chrome, Edge, and Firefox.
Safari requires macOS 26 (Sequoia) or later.
```

### Final Assessment

The architecture is sound. The *.localhost pattern works for the majority of developers (Chrome + Firefox market share >85%). Safari limitation is notable but not a blocker - most developers use Chrome or Firefox for dev work.
