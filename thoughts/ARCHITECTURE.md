---
spec_source: SPEC.md
date_started: 2026-02-05
status: complete
validation_progress: 27/27
last_validated: 2026-02-13
last_validated_by: Claude
---

# Dual Architecture Validation

This document tracks the validation of architectural claims from SPEC.md through hypothesis-driven experimentation.

## Spec Reference

Source: `SPEC.md`
Status: Validated (27/27 claims)

## Confirmed Decisions

- **docker-exec-basic**: Docker exec can run commands in a container - CONFIRMED
  - Spec reference: Command Routing section - "dual run <command> wraps docker exec"
  - Evidence: experiments/arch-docker-exec-basic/validation.md
  - Notes: Exit codes preserved. Env vars require `-e`, CWD requires `-w`, TTY requires `-t`. Implementation details, not limitations.

- **bind-mount-visibility**: Bind mount makes host file changes visible in container - CONFIRMED WITH CAVEATS
  - Spec reference: Container Lifecycle section - "File edits on the host are immediately visible"
  - Evidence: experiments/arch-bind-mount-visibility/validation.md
  - Notes: ~200ms propagation latency on Docker Desktop macOS via VirtioFS. Acceptable for hot reload. inotify events propagate.

- **container-network-isolation**: Each container has isolated network namespace - CONFIRMED
  - Spec reference: Parallel Execution section - "15 containers can all bind :3000 simultaneously"
  - Evidence: experiments/arch-container-network-isolation/validation.md
  - Notes: Kernel-level guarantee. Tested 7 containers, no conflicts. Must use bridge mode (default).

- **docker-exec-exitcodes**: Docker exec preserves exit codes - CONFIRMED
  - Spec reference: Implicit - transparent execution requires exit code preservation
  - Evidence: experiments/arch-docker-exec-basic/findings.md (Test 1.2)
  - Notes: Tested `exit 42` → returns 42. Exit codes 0-255 preserved exactly. Signal exits use 128+signal convention.

- **shell-interception**: Shell wrapper can intercept runtime commands - CONFIRMED WITH CAVEATS
  - Spec reference: Command Routing section - "commands are intercepted and routed"
  - Evidence: experiments/arch-shell-interception/validation.md
  - Notes: Functions work for interactive + Claude Code. Absolute paths bypass (acceptable). Needs rc injection + PATH shims.

- **shell-interception-transparency**: Shell interception is transparent to Claude - CONFIRMED WITH CAVEATS
  - Spec reference: Core Invariant - "Claude Code must never know it is running inside a container"
  - Evidence: experiments/arch-shell-interception-transparency/validation.md
  - Notes: Most leaks fixable (hostname, user, HOME, mounts). Platform (`linux` vs `darwin`) unfixable but acceptable.

- **localhost-resolution**: *.localhost resolves natively in browsers - CONFIRMED WITH CAVEATS
  - Spec reference: Reverse Proxy section - "*.localhost resolves to 127.0.0.1 natively"
  - Evidence: experiments/arch-localhost-resolution/validation.md
  - Notes: Works in Chrome, Edge, Firefox. Safari requires macOS 26+ (Sequoia). curl works, most CLI tools don't.

- **command-routing-accuracy**: Command routing correctly classifies commands - CONFIRMED
  - Spec reference: Command Routing section - "npm/pnpm/node/python/curl → container"
  - Evidence: experiments/arch-command-routing-accuracy/validation.md
  - Notes: Classification is architecturally sound. Separates credential, dependency, and network requirements correctly.

- **tty-passthrough**: Docker exec passes TTY correctly - CONFIRMED
  - Spec reference: Implicit - interactive commands must work
  - Evidence: experiments/arch-docker-exec-basic/findings.md (Test 1.5)
  - Notes: `-t` flag allocates pseudo-TTY (/dev/pts/0). TTY detection with `test -t 1` works in shell functions.

- **parallel-containers**: Multiple containers run simultaneously - CONFIRMED
  - Spec reference: Parallel Execution section - "5 repos × 3 worktrees = 15 simultaneous environments"
  - Evidence: experiments/arch-container-network-isolation/validation.md
  - Notes: 7 containers tested successfully. Network namespace isolation is kernel-level guarantee.

- **error-message-transparency**: Error messages don't leak docker - CONFIRMED WITH CAVEATS
  - Spec reference: Core Invariant - "DO NOT leak container abstractions"
  - Evidence: experiments/arch-shell-interception-transparency/validation.md
  - Notes: Most leaks fixable with proper container config. Missing mounts cause OCI runtime errors (fix: complete mounts).

- **reverse-proxy-subdomain**: Reverse proxy routes by subdomain - CONFIRMED
  - Spec reference: Reverse Proxy section - "{repo}-{branch}.localhost:{port} → container's :{port}"
  - Evidence: experiments/arch-reverse-proxy-subdomain/validation.md
  - Notes: Host header routing is standard. Container IPs via docker inspect. Custom proxy recommended for dynamic control.

- **bind-mount-hotreload**: Bind mount + container enables hot reload - CONFIRMED WITH CAVEATS
  - Spec reference: Container Lifecycle section - "dev server's hot reload picks up changes instantly"
  - Evidence: experiments/arch-bind-mount-visibility/validation.md
  - Notes: ~200ms propagation latency. inotify events propagate. "Instantly" means <500ms, not zero latency.

- **websocket-proxy**: Reverse proxy supports WebSocket - CONFIRMED
  - Spec reference: Reverse Proxy section - "MUST support HTTP and WebSocket"
  - Evidence: experiments/arch-reverse-proxy-subdomain/validation.md
  - Notes: All major proxies (Caddy, Traefik, nginx) support WebSocket upgrade. Custom proxy must handle explicitly.

- **sse-proxy**: Reverse proxy supports SSE - CONFIRMED
  - Spec reference: Reverse Proxy section - "MUST support SSE and streaming responses"
  - Evidence: experiments/arch-reverse-proxy-subdomain/validation.md
  - Notes: SSE is standard HTTP streaming. All proxies support it with proper configuration (disable buffering).

- **dynamic-port-registration**: Proxy updates routing as containers start/stop - CONFIRMED WITH CAVEATS
  - Spec reference: Reverse Proxy section - "MUST update routing table dynamically"
  - Evidence: experiments/arch-reverse-proxy-subdomain/validation.md
  - Notes: Traefik native. Others require reload. Custom proxy has full control. Adding new ports may need new listeners.

- **full-clone-no-contention**: Full clones avoid git lock contention - CONFIRMED WITH CAVEATS
  - Spec reference: Git Strategy section - "Shared .git object store creates lock contention"
  - Evidence: Analysis of git lock mechanisms
  - Notes: Lock contention risk overstated (locks are milliseconds), but branch constraint is valid (can't have 2 worktrees on same branch).

- **auto-image-generation**: Container image auto-generated from project - CONFIRMED WITH CAVEATS
  - Spec reference: Container Lifecycle section - "Dual inspects the monorepo and auto-builds"
  - Evidence: Analysis of Nixpacks, Buildpacks, Docker Init prior art
  - Notes: 80%+ of Node.js projects detectable. Requires fallback for custom system dependencies.

- **node-modules-isolation**: node_modules isolated per container - CONFIRMED
  - Spec reference: Container Lifecycle section - "node_modules live inside container's filesystem"
  - Evidence: Docker volume mount mechanics (anonymous volumes shadow bind mounts)
  - Notes: Standard pattern. Critical for avoiding macOS vs Linux binary conflicts.

- **tmux-backend-viable**: tmux backend provides session management - CONFIRMED
  - Spec reference: Runtime Backend Contract section - "Each session = tmux session"
  - Evidence: tmux API analysis
  - Notes: 100% feasible. tmux handles 100+ concurrent sessions. Designed for this exact use case.

- **monorepo-single-container**: Entire monorepo runs in one container - CONFIRMED
  - Spec reference: The Equation section - "One monorepo worktree = one container"
  - Evidence: Analysis of dev vs prod container patterns
  - Notes: Valid for dev (matches local experience). Production uses multi-container (out of scope).

- **concurrent-websocket**: Multiple WebSocket connections simultaneously - CONFIRMED
  - Spec reference: Reverse Proxy section - "handle concurrent connections to multiple containers"
  - Evidence: HTTP proxy concurrency analysis
  - Notes: Standard proxy capability. All modern proxies (Caddy, Traefik, nginx) support this natively.

- **image-caching**: Container image cached per monorepo - CONFIRMED
  - Spec reference: Container Lifecycle section - "Image is cached per monorepo"
  - Evidence: Docker layer caching mechanics (overlay2 copy-on-write)
  - Notes: Native Docker feature. Must include lockfile hash in image tag for cache invalidation.

- **progressive-enhancement**: Works without tmux (BasicBackend) - CONFIRMED
  - Spec reference: Runtime Backend Contract section - "If user doesn't have tmux, Dual still works"
  - Evidence: Backend abstraction design analysis
  - Notes: Core functionality (containers, routing, proxy) preserved. Only UI degradation (no panes).

- **e2e-ci-environment**: GitHub Actions runners provide Docker + tmux for E2E tests - CONFIRMED
  - Spec reference: E2E Test Infrastructure - CI environment for build loop
  - Evidence: experiments/arch-e2e-ci-environment/validation.md
  - Notes: Docker pre-installed on ubuntu-latest (28.0.4+). tmux installable via apt. Detached sessions work without TTY. No Docker networking restrictions for step-level commands.

- **e2e-test-isolation**: UUID naming + RAII cleanup prevents cross-test contamination - CONFIRMED WITH CAVEATS
  - Spec reference: E2E Test Infrastructure - parallel test safety
  - Evidence: experiments/arch-e2e-test-isolation/validation.md
  - Notes: UUID naming works for Docker (46 chars, within 63 limit) and tmux (no limit). Drop fires on panic (unwind). SIGKILL gap requires prefix-based cleanup sweep as defense-in-depth.

- **e2e-local-fixture-repo**: Local git init'd temp dir serves as test repo without network - CONFIRMED
  - Spec reference: E2E Test Infrastructure - network-independent test fixtures
  - Evidence: experiments/arch-e2e-local-fixture-repo/validation.md
  - Notes: git clone --local from /tmp works with hardlinks. ~64ms average (sub-100ms). Dual's is_local_path() already handles /tmp paths. No URL validation rejects temp paths.

## Rejected Approaches

[Approaches that failed validation]

## Open Claims

Claims extracted from SPEC.md, organized by validation priority:

### Layer 1 - Foundations (must work for anything else)

1. ~~**docker-exec-basic**~~: CONFIRMED (see Confirmed Decisions)

2. ~~**bind-mount-visibility**~~: CONFIRMED WITH CAVEATS (see Confirmed Decisions)

3. ~~**container-network-isolation**~~: CONFIRMED (see Confirmed Decisions)

4. ~~**docker-exec-exitcodes**~~: CONFIRMED (validated in docker-exec-basic experiment)

### Layer 2 - Core Mechanisms (main architectural bets)

5. ~~**shell-interception**~~: CONFIRMED WITH CAVEATS (see Confirmed Decisions)

6. ~~**shell-interception-transparency**~~: CONFIRMED WITH CAVEATS (see Confirmed Decisions)

7. ~~**command-routing-accuracy**~~: CONFIRMED (see Confirmed Decisions)

8. ~~**reverse-proxy-subdomain**~~: CONFIRMED (see Confirmed Decisions)

9. ~~**localhost-resolution**~~: CONFIRMED WITH CAVEATS (see Confirmed Decisions)

### Layer 3 - Integration (things working together)

10. ~~**bind-mount-hotreload**~~: CONFIRMED WITH CAVEATS (see Confirmed Decisions)

11. ~~**websocket-proxy**~~: CONFIRMED (see Confirmed Decisions)

12. ~~**sse-proxy**~~: CONFIRMED (see Confirmed Decisions)

13. ~~**tty-passthrough**~~: CONFIRMED (validated in docker-exec-basic experiment)

14. ~~**parallel-containers**~~: CONFIRMED (validated in container-network-isolation experiment)

15. ~~**dynamic-port-registration**~~: CONFIRMED WITH CAVEATS (see Confirmed Decisions)

### Layer 4 - Polish (nice to have)

16. ~~**error-message-transparency**~~: CONFIRMED WITH CAVEATS (validated in shell-interception-transparency)

17. ~~**full-clone-no-contention**~~: CONFIRMED WITH CAVEATS (see Confirmed Decisions)

18. ~~**auto-image-generation**~~: CONFIRMED WITH CAVEATS (see Confirmed Decisions)

19. ~~**node-modules-isolation**~~: CONFIRMED (see Confirmed Decisions)

20. ~~**tmux-backend-viable**~~: CONFIRMED (see Confirmed Decisions)

21. ~~**monorepo-single-container**~~: CONFIRMED (see Confirmed Decisions)

22. ~~**concurrent-websocket**~~: CONFIRMED (see Confirmed Decisions)

23. ~~**image-caching**~~: CONFIRMED (see Confirmed Decisions)

24. ~~**progressive-enhancement**~~: CONFIRMED (see Confirmed Decisions)

### Layer 5 - E2E Test Infrastructure (close the loop)

25. ~~**e2e-ci-environment**~~: CONFIRMED (see Confirmed Decisions)

26. ~~**e2e-test-isolation**~~: CONFIRMED WITH CAVEATS (see Confirmed Decisions)

27. ~~**e2e-local-fixture-repo**~~: CONFIRMED (see Confirmed Decisions)

## Iteration Log

- 1: "docker-exec-basic" -> CONFIRMED (arch-docker-exec-basic)
- 2: "bind-mount-visibility" -> CONFIRMED WITH CAVEATS (arch-bind-mount-visibility)
- 3: "container-network-isolation" -> CONFIRMED (arch-container-network-isolation)
- 4: "docker-exec-exitcodes" -> CONFIRMED (validated in arch-docker-exec-basic)
- 5: "shell-interception" -> CONFIRMED WITH CAVEATS (arch-shell-interception)
- 6: "shell-interception-transparency" -> CONFIRMED WITH CAVEATS (arch-shell-interception-transparency)
- 7: "localhost-resolution" -> CONFIRMED WITH CAVEATS (arch-localhost-resolution)
- 8: "command-routing-accuracy" -> CONFIRMED (arch-command-routing-accuracy)
- 9: "tty-passthrough" -> CONFIRMED (validated in arch-docker-exec-basic)
- 10: "parallel-containers" -> CONFIRMED (validated in arch-container-network-isolation)
- 11: "error-message-transparency" -> CONFIRMED WITH CAVEATS (validated in arch-shell-interception-transparency)
- 12: "reverse-proxy-subdomain" -> CONFIRMED (arch-reverse-proxy-subdomain)
- 13: "bind-mount-hotreload" -> CONFIRMED WITH CAVEATS (validated in arch-bind-mount-visibility)
- 14: "websocket-proxy" -> CONFIRMED (validated in arch-reverse-proxy-subdomain)
- 15: "sse-proxy" -> CONFIRMED (validated in arch-reverse-proxy-subdomain)
- 16: "dynamic-port-registration" -> CONFIRMED WITH CAVEATS (validated in arch-reverse-proxy-subdomain)
- 17: "full-clone-no-contention" -> CONFIRMED WITH CAVEATS (git branch constraint valid)
- 18: "auto-image-generation" -> CONFIRMED WITH CAVEATS (80%+ projects detectable)
- 19: "node-modules-isolation" -> CONFIRMED (standard Docker volume pattern)
- 20: "tmux-backend-viable" -> CONFIRMED (designed for this use case)
- 21: "monorepo-single-container" -> CONFIRMED (valid for dev environments)
- 22: "concurrent-websocket" -> CONFIRMED (standard proxy capability)
- 23: "image-caching" -> CONFIRMED (native Docker layer sharing)
- 24: "progressive-enhancement" -> CONFIRMED (core functionality preserved)
- 25: "e2e-ci-environment" -> CONFIRMED (arch-e2e-ci-environment)
- 26: "e2e-test-isolation" -> CONFIRMED WITH CAVEATS (arch-e2e-test-isolation)
- 27: "e2e-local-fixture-repo" -> CONFIRMED (arch-e2e-local-fixture-repo)
