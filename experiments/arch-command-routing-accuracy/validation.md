# Validation: command-routing-accuracy

**Date**: 2026-02-05
**Claim**: "npm/pnpm/node/python/curl → container" and "git/cat/ls/vim/nvim → host"

## Analysis Summary

### Classification Correctness

| Command | Routed To | Correct? | Reason |
|---------|-----------|----------|--------|
| npm/pnpm | Container | YES | Needs node_modules in container volume |
| node | Container | YES | Needs node_modules, may bind ports |
| python | Container | YES | Needs venv, may bind ports |
| curl | Container | YES | Tests localhost in container network |
| git | Host | YES | Needs SSH keys, GPG, .gitconfig |
| cat | Host | YES | Reads source files via bind mount |
| ls | Host | YES | Lists source tree |
| vim/nvim | Host | YES | Uses host config, edits shared files |

### The Rule is Sound

**"File Operations → Host. Runtime Operations → Container."**

This correctly separates:
- Credential-requiring operations → Host (git)
- Dependency-requiring operations → Container (npm, node, python)
- Network-namespace-requiring operations → Container (curl localhost)
- Host-config-requiring operations → Host (vim, nvim)

### Edge Cases Identified

1. **Mixed operations**: Scripts needing both `git` and `npm` cannot run atomically
   - e.g., `npm version patch && git commit`
   - Must split into sequential steps

2. **Container filesystem invisible from host**:
   - `ls node_modules` on host shows empty (node_modules in container volume)
   - Solution: `dual run ls node_modules`

3. **Implicit command routing**:
   - `npx playwright test` → follows node pattern → container
   - `psql -h localhost` → needs container network → should be container
   - `tsc`, `webpack` → need node_modules → container

### Verdict

**CONFIRMED**

The command routing classification in SPEC.md is architecturally sound. It correctly separates:
- Commands needing host credentials (git)
- Commands needing container dependencies (npm, node, python)
- Commands needing container network (curl localhost)
- Commands using host config (vim, nvim)

### Constraint Noted

The split-execution model means no single-context execution for mixed operations. This is an inherent tradeoff, not a misclassification.
