# Docker Spike Experiments

This directory contains the Docker spike experiments for validating the Conductor architecture.

## Experiments

### Experiment 1: File watch latency measurement
**Status:** Pending
**Location:** `experiment-1/`

Measures VirtioFS bind mount latency on macOS. Success criteria: p95 latency <500ms.

### Experiment 2: Reverse proxy + WebSocket + *.localhost end-to-end
**Status:** Pending
**Location:** `experiment-2/`

Validates the entire developer-facing network path through the reverse proxy.

### Experiment 3: Transparent command interception
**Status:** ✅ COMPLETE
**Location:** `experiment-3/`

Validates that commands can be transparently routed through `docker exec` without Claude Code detecting it.

**Key deliverables:**
- `conductor-intercept.sh` — Working wrapper script (ready for deployment)
- `EXPERIMENT_3_FINDINGS.md` — Full findings report
- `test-cases.sh` — Automated test harness

**Result:** ✅ PASS — All 7 tests pass. Architecture is viable.

### Experiment 4: Multi-service monorepo (conditional)
**Status:** Pending
**Location:** `experiment-4/`

Validates that multiple services can run inside a single container with inter-service communication over localhost.

## Running Experiments

Each experiment is self-contained. See the individual `EXPERIMENT_N_FINDINGS.md` files for setup and execution instructions.

### Quick Start for Experiment 3

```bash
# 1. Start the container
docker run -d \
  -v ~/dev/polychromos:/workspace \
  -w /workspace \
  --name spike-intercept \
  node:20-slim \
  bash -c "corepack enable pnpm && tail -f /dev/null"

# 2. Source the wrapper
source experiments/experiment-3/conductor-intercept.sh

# 3. Run tests
bash experiments/experiment-3/test-cases.sh
```

## Architecture Context

These experiments validate the **Conductor** architecture, which enables developers to work inside isolated Docker containers that simulate bare-metal machines, while Claude Code on the host remains unaware it's executing remotely.

Core architectural invariant: **Claude Code must never know it is running inside a container.**

Each experiment tests a critical assumption:
- Exp 1: File watching through bind mounts performs acceptably
- Exp 2: Network path through proxy is transparent
- Exp 3: Command interception is transparent ✅
- Exp 4: Multi-service monorepo model works
