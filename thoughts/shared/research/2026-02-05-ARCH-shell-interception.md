# Research: shell-interception

**Date**: 2026-02-05
**Spec Claim**: "Claude Code's shell is configured so that commands are intercepted and routed"
**Hypothesis**: Shell functions or PATH manipulation can intercept commands and route them to docker exec transparently.

## Research Method

Parallel knowledge agents:
- **knowledge-analyst**: Documented interception methods, prior art, limitations
- **knowledge-prober**: Empirically tested function interception with docker exec

## Environment

- Shell: zsh (macOS default)
- Docker: 29.2.0
- Platform: macOS Darwin 24.6.0

## Key Findings

### Interception Methods Compared

| Method | Intercepts `npm` | Intercepts `/usr/bin/npm` | Works in scripts | Transparent to `which` |
|--------|------------------|---------------------------|------------------|------------------------|
| Shell functions | YES | NO | NO (unless exported) | YES |
| Aliases | YES | NO | NO (unless enabled) | YES |
| PATH shims | YES | NO | YES | NO |

### Shell Function Interception - Validated

```bash
npm() {
  local docker_flags=""
  test -t 1 && docker_flags="-t"
  docker exec $docker_flags container-name npm "$@"
}
```

**Empirically verified behaviors**:
- Arguments pass through correctly with `"$@"`
- Exit codes preserved (tested `return 42` → returned 42)
- TTY can be conditionally allocated (`test -t 1`)
- Works with docker exec end-to-end

### Bypass Methods (Cannot Intercept)

| Pattern | Bypasses Functions | Bypasses PATH Shims |
|---------|-------------------|---------------------|
| `/usr/bin/npm` | YES | YES |
| `command npm` | YES | NO |
| `\npm` | YES (bash), NO (zsh) | NO |

### Prior Art

- **nvm**: Uses shell functions for `node`, `npm`, `npx`
- **pyenv/rbenv**: Uses PATH shims for `python`, `ruby`
- **direnv**: Uses PROMPT_COMMAND to modify env per-directory
- **devcontainers**: Entire shell runs in container (not selective)

### Recommended Approach: Functions + PATH Shims

1. **Interactive shells**: Define functions in `.bashrc`/`.zshrc`
   - Transparent to `which npm` (shows real binary)
   - Fast (no file lookup)

2. **Scripts**: Create shim scripts in PATH
   - Works without sourcing rc files
   - Catches `command npm` bypass

## Fundamental Limitations

**Cannot intercept absolute paths** - If a process calls `/usr/bin/npm` directly, no shell mechanism can intercept. The kernel's `exec()` goes directly to the inode.

**This is acceptable for Dual because**:
- Claude Code uses command names, not absolute paths
- Most dev tooling uses PATH resolution
- Scripts using absolute paths are edge cases

## Implementation Requirements

For transparent routing to docker exec:

1. **Forward environment variables**: `-e KEY=VALUE` for each
2. **Set working directory**: `-w "$PWD"` (sync with host CWD)
3. **Conditional TTY**: `test -t 1 && flags="-t"`
4. **Argument preservation**: `"$@"` with quotes
5. **Exit code passthrough**: Automatic (function returns last command's code)

## Verdict

**CONFIRMED WITH CAVEATS**

The spec claim that commands can be "intercepted and routed" is valid for the common case. Shell functions provide transparent interception for:
- User-typed commands
- Claude Code bash tool commands
- Most scripts

**Caveats**:
- Absolute paths cannot be intercepted (acceptable limitation)
- Functions must be loaded in each shell session (rc file injection)
- PATH shims needed for full script compatibility
