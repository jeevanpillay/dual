---
date: 2026-02-13T06:00:00+08:00
researcher: Claude
git_commit: 08598f0ccb984e1dd284eb7f76d0db95987f1421
branch: feature/build-loop-pipeline
repository: dual
hypothesis: "GitHub Actions runners provide Docker and tmux for isolated E2E tests"
tags: [experiment, research, github-actions, docker, tmux, ci, e2e]
status: research_complete
last_updated: 2026-02-13
last_updated_by: Claude
---

# Research: GitHub Actions CI Environment for E2E Tests

**Date**: 2026-02-13
**Researcher**: Claude
**Git Commit**: 08598f0
**Branch**: feature/build-loop-pipeline
**Repository**: dual

## Hypothesis

GitHub Actions ubuntu-latest runners have Docker pre-installed and tmux installable via apt, and both work headless without a TTY — enabling E2E tests that exercise Docker containers and tmux sessions in CI.

## Why This Matters

E2E tests must run in CI to close the build loop. Without CI integration, regressions in container lifecycle, tmux session management, or command routing go undetected until manual testing.

## What We're Testing

- **Primary claim**: ubuntu-latest has Docker pre-installed; tmux installable via apt; both work headless
- **Success criteria**: Docker commands execute, tmux detached sessions create/destroy, no TTY required
- **Scope boundary**: CI environment availability, not test framework design

## Environment & Prerequisites

### Verified Present (Local)
| Tool/System | Version | Status |
|-------------|---------|--------|
| Docker      | 29.2.0  | ✓ Found |
| tmux        | 3.5a    | ✓ Found |

### GitHub Actions ubuntu-latest
| Tool/System | Version | Status |
|-------------|---------|--------|
| Docker      | 28.0.4+ (updating to 29.1) | ✓ Pre-installed |
| Docker Compose v2 | 2.38.2 | ✓ Pre-installed |
| tmux        | N/A     | ✓ Installable via apt |

## Feasibility Assessment

### Docker on GitHub Actions

Docker is a first-class citizen on ubuntu-latest runners. Pre-installed with no additional setup. Full `docker run`, `docker exec`, `docker build` support. Custom Docker networks work when using `docker run` directly in workflow steps.

**Networking**: No restrictions when running Docker commands as regular steps. The `--network` and `--entrypoint` restrictions only apply to the `jobs.<job_id>.container.options` field (container jobs), not to `docker run` in step commands.

### tmux on GitHub Actions

tmux is NOT pre-installed but trivially installable:
```yaml
- name: Install tmux
  run: sudo apt-get update && sudo apt-get install -y tmux
```

**Headless operation**: GitHub Actions runners do NOT provide a TTY (`$TERM=dumb`, no tty). However:
- `tmux new-session -d` (detached mode) works without a TTY
- `tmux send-keys` works for sending commands to detached sessions
- `tmux capture-pane` works for reading output from detached sessions

This is proven by existing GitHub Actions that use tmux (action-upterm, tmate debugging action).

### Empirical Discoveries

Local probe confirmed:
- `tmux new-session -d -s test-probe` created a detached session successfully
- `tmux list-sessions` confirmed the session existed
- `tmux kill-session -t test-probe` cleaned it up

### Constraints & Limitations

1. **tmux inline commands exit immediately**: `tmux new-session -d -s foo 'cmd'` terminates when cmd finishes. Use `send-keys` instead.
2. **No TTY**: Always use detached mode (`-d`), never attached. Tests requiring interactive terminal input must use `send-keys`.
3. **DockerHub rate limits**: Frequent image pulls can hit rate limits. Use ghcr.io or Docker layer caching.
4. **tmux install overhead**: ~2-3 seconds for `apt-get install -y tmux`.

## Evidence Assessment

### Supporting Evidence
- Docker pre-installed on ubuntu-latest (runner images README)
- tmux confirmed working headless in CI (action-upterm, tmate action)
- Local probe passed: detached sessions work without TTY
- No Docker networking restrictions for step-level usage

### Contradicting Evidence
- None found

## References

- [Ubuntu 24.04 Runner Image README](https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Readme.md)
- [GitHub Docs: Service Containers](https://docs.github.com/en/actions/tutorials/communicating-with-docker-service-containers)
- [action-upterm](https://github.com/lhotari/action-upterm) - tmux in GitHub Actions
- [tmux/tmux#2410](https://github.com/tmux/tmux/issues/2410) - detached session behavior

## Probing Log

```bash
# Local probe
docker --version  # Docker version 29.2.0
tmux -V           # tmux 3.5a
tmux new-session -d -s test-probe && tmux list-sessions && tmux kill-session -t test-probe  # SUCCESS
```

## Unknowns & Open Questions

None remaining. All questions answered through desk research and local probing.

## Assumptions Made

- GitHub Actions runner image versions remain consistent (Docker pre-installed, apt available)
- Dual's E2E tests will only use tmux in detached mode
