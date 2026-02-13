# Claude Code Inside Container — Experiment Findings

**Date**: 2026-02-06
**Claim**: "Put Claude inside the container. Then there's nothing to hide." (SPEC.md L28-30)
**Verdict**: VALIDATED with caveats

---

## Setup

### Docker Image (`dual-node20`)
- Base: `node:20-bookworm-slim`
- Added: `git curl bash openssh-client ca-certificates`
- Installed: `@anthropic-ai/claude-code@latest` (v2.1.34)
- Final size: 653MB
- Build time: ~45s

### Dockerfile
```dockerfile
FROM node:20-bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    git curl bash openssh-client ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN npm install -g @anthropic-ai/claude-code@latest
RUN claude --version
WORKDIR /workspace
```

### Container Launch
```bash
docker run -d --name claude-test \
  -v ~/project:/workspace \
  -v /workspace/node_modules \
  -v ~/.claude:/root/.claude \
  -v ~/.gitconfig:/root/.gitconfig:ro \
  -v "$SSH_AUTH_SOCK:/ssh-agent" \
  -e SSH_AUTH_SOCK=/ssh-agent \
  dual-node20 sleep 600
```

---

## Test Results

### Test 1: Claude Code Binary Runs
```
$ docker exec claude-test claude --version
2.1.34 (Claude Code)
```
**Result**: PASS

### Test 2: Claude Code Reads Files
Prompt: "Read /workspace/index.js and tell me what it does"
Response: "This file simply prints 'hello from workspace' to the console."
**Result**: PASS

### Test 3: Claude Code Creates Files
Prompt: "Create a new file at /workspace/greeting.js that exports a greet(name) function"
Result: File created. Verified on host:
```javascript
function greet(name) {
  return `Hello, ${name}!`;
}
module.exports = { greet };
```
File created inside container is **immediately visible on host** via bind mount.
**Result**: PASS

### Test 4: Claude Code Executes Commands
Prompt: "Run node to test the greeting module"
Result: Claude ran `node -e "..."` and got `Hello, Dual!`
**Result**: PASS

### Test 5: Claude Code Starts Server + Curls localhost
Prompt: "Create an HTTP server on port 3000, start it, curl localhost:3000"
Result: Claude created server.js, started it in background, server responds:
```
$ docker exec claude-test curl -s localhost:3000
Dual works!
```
Claude treated localhost:3000 as native — no Docker awareness.
**Result**: PASS — This is THE core validation.

---

## Critical Finding: Authentication

### Problem
Claude Code on macOS stores OAuth tokens in the **macOS Keychain** (service name: `Claude Code-credentials`). Inside a Linux container, there is no macOS Keychain.

### Solution
Claude Code has a **plaintext credential fallback** at `~/.claude/.credentials.json`. The credential loading path:
1. Try system keychain (macOS Keychain / Linux Secret Service)
2. Fall back to `~/.claude/.credentials.json` (plaintext file, mode 600)

### What Dual Must Do
On `dual init` or `dual new`, Dual must:
1. Extract OAuth credentials from macOS Keychain: `security find-generic-password -s "Claude Code-credentials" -w`
2. Write them to `~/.claude/.credentials.json` (or a container-specific path)
3. Mount with appropriate permissions

### SPEC Impact
The credential table in SPEC.md needs updating:

| Credential | Mount | Mode | Notes |
|---|---|---|---|
| `~/.claude/` | Claude Code auth + settings | **read-write** | Claude Code writes debug logs, todos, session data |
| SSH agent socket | Git SSH authentication | forwarded | |
| `~/.gitconfig` | Git user config | read-only | |
| `~/.npmrc` | npm registry tokens (if exists) | read-only | |
| `~/.claude/.credentials.json` | OAuth token (extracted from Keychain) | mode 600 | **NEW: Dual must create this** |

Key change: `.claude/` must be mounted **read-write**, not read-only. Claude Code writes to:
- `.claude/debug/` (debug logs)
- `.claude/todos/` (task tracking)
- `.claude/settings.json` (settings updates)
- `.claude/session-env/` (session state)
- `.claude/history.jsonl` (conversation history)

---

## Architecture Validation Summary

| SPEC Claim | Result |
|---|---|
| Claude Code runs inside container | CONFIRMED |
| No command routing needed | CONFIRMED — all commands run in one place |
| No shell interception needed | CONFIRMED — nothing to intercept |
| localhost:3000 just works | CONFIRMED — server bound, curl returned data |
| Claude has no awareness of Docker | CONFIRMED — treated environment as native Linux |
| File edits visible on host | CONFIRMED — bind mount bidirectional |
| Standard base image works | CONFIRMED — 3-line Dockerfile + npm install |

---

## Caveats

1. **`.claude/` must be read-write**: SPEC says read-only, but Claude Code needs write access for debug logs, todos, settings, session state. This is a SPEC correction.

2. **OAuth credentials need extraction**: Dual must extract credentials from macOS Keychain and write `~/.claude/.credentials.json`. This is a one-time setup step during `dual init`.

3. **Image size**: 653MB for `dual-node20`. Acceptable but could be optimized (Alpine would be smaller but may have compatibility issues with Claude Code's native deps).

4. **Container entrypoint**: Using `sleep 600` as entrypoint for testing. Production should use a proper entrypoint that keeps the container alive for `docker exec`.
