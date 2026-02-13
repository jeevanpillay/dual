---
date_created: 2026-02-06T12:00:00+08:00
research_doc: thoughts/shared/research/2026-02-06-ARCH-network-isolation.md
hypothesis: "Multiple Docker containers can each bind the same port with isolated network namespaces"
status: design_complete
---

# Experiment: Container Network Namespace Isolation

## Research Reference

- **Document**: thoughts/shared/research/2026-02-06-ARCH-network-isolation.md
- **Hypothesis**: Multiple containers can bind :3000 simultaneously without conflict
- **Date researched**: 2026-02-06

## Experiment Objective

Empirically prove that Docker containers have fully isolated network namespaces, enabling Dual's "one workspace = one container" model where every workspace runs dev servers on the same ports.

## Desired End State

### What Success Looks Like
- 3+ containers each bind :3000 simultaneously
- Each container's localhost:3000 returns its unique response
- No cross-contamination between container network stacks
- `docker exec` correctly routes to each container's namespace

### What Failure Looks Like
- Port binding errors when multiple containers use :3000
- Localhost in one container returns another container's response
- Docker exec reaches wrong namespace

## What We're NOT Testing
- Reverse proxy routing (separate claim #11)
- Browser access to container services (depends on proxy)
- Port mapping with `-p` flag (Dual doesn't use this)
- Performance under load (separate claim #13)

## Prerequisites

| Tool/System | Version | Status |
|-------------|---------|--------|
| Docker | 29.2.0 | Verified |
| Docker Desktop | macOS arm64 | Verified |
| Alpine image | latest | Available via pull |

## Test Cases

### Test Group 1: Core Functionality

#### Test 1.1: Three Containers Bind Same Port

**Tests**: Multiple containers can each bind :3000 without conflict
**Research said**: "Socket table scoping means identical bindings in different namespaces never conflict"

**Procedure**:
```bash
# Create three containers each binding :3000
docker run -d --name arch-net-1 alpine sh -c 'while true; do echo -e "HTTP/1.1 200 OK\r\nContent-Length: 12\r\n\r\nworkspace-01" | nc -l -p 3000; done'
docker run -d --name arch-net-2 alpine sh -c 'while true; do echo -e "HTTP/1.1 200 OK\r\nContent-Length: 12\r\n\r\nworkspace-02" | nc -l -p 3000; done'
docker run -d --name arch-net-3 alpine sh -c 'while true; do echo -e "HTTP/1.1 200 OK\r\nContent-Length: 12\r\n\r\nworkspace-03" | nc -l -p 3000; done'
# Verify all running
docker ps --filter name=arch-net --format "{{.Names}} {{.Status}}"
```

**Expected result**: All three containers start and stay running

**Measurements**:
- [ ] All three containers in "Up" status
- [ ] No error output during creation

#### Success Criteria:

**Automated Verification:**
- [ ] All three `docker run` commands exit with code 0
- [ ] `docker ps` shows all three containers as running

---

#### Test 1.2: Localhost Isolation Per Container

**Tests**: Each container's localhost:3000 returns its own unique response
**Research said**: "Each namespace has its own lo interface; 127.0.0.1 in container A ≠ 127.0.0.1 in container B"

**Procedure**:
```bash
# Query each container's localhost:3000
docker exec arch-net-1 wget -qO- http://localhost:3000
docker exec arch-net-2 wget -qO- http://localhost:3000
docker exec arch-net-3 wget -qO- http://localhost:3000
```

**Expected result**: Each returns its unique identifier (workspace-01, workspace-02, workspace-03)

**Measurements**:
- [ ] Response from container 1
- [ ] Response from container 2
- [ ] Response from container 3

#### Success Criteria:

**Automated Verification:**
- [ ] Container 1 returns "workspace-01"
- [ ] Container 2 returns "workspace-02"
- [ ] Container 3 returns "workspace-03"
- [ ] No cross-contamination

---

#### Test 1.3: Exit Code Preservation Through Docker Exec

**Tests**: docker exec preserves the exit code of the executed command
**Research said**: "docker exec joins container namespace via setns()"

**Procedure**:
```bash
# Success case
docker exec arch-net-1 sh -c 'exit 0'; echo "exit: $?"
# Failure case
docker exec arch-net-1 sh -c 'exit 42'; echo "exit: $?"
# Command not found
docker exec arch-net-1 sh -c 'nonexistent_command 2>/dev/null'; echo "exit: $?"
```

**Expected result**: Exit codes 0, 42, and 127 respectively

**Measurements**:
- [ ] Exit code for success case
- [ ] Exit code for failure case
- [ ] Exit code for command-not-found

#### Success Criteria:

**Automated Verification:**
- [ ] First command: exit code 0
- [ ] Second command: exit code 42
- [ ] Third command: exit code 127

---

### Test Group 2: Unknown Validation

#### Test 2.1: Host Cannot Reach Unpublished Container Ports

**Tests**: macOS host cannot directly access container services without port mapping
**Research asked**: "How does host access container services?"

**Procedure**:
```bash
# Get container IP
CONTAINER_IP=$(docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' arch-net-1)
echo "Container IP: $CONTAINER_IP"
# Try to reach from host (should fail — IPs are in VM, not routable from macOS)
curl -s --connect-timeout 2 http://$CONTAINER_IP:3000 2>&1 || echo "Connection failed (expected)"
```

**Expected result**: Connection fails — container IPs exist only inside Docker Desktop's Linux VM

**Measurements**:
- [ ] Container IP address
- [ ] Connection attempt result

**Resolves unknown if**: Connection fails with timeout/refused
**Still unknown if**: Connection succeeds (would change architecture assumptions)

---

### Test Group 3: Assumption Validation

#### Test 3.1: Containers Get Unique Bridge IPs

**Tests**: Each container gets a unique IP on the bridge network
**Research assumed**: "Docker assigns unique bridge IPs to each container"

**Procedure**:
```bash
docker inspect -f '{{.Name}} {{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' arch-net-1 arch-net-2 arch-net-3
```

**Expected result**: Three distinct IP addresses in 172.17.0.0/16 range

**Measurements**:
- [ ] IP addresses for all three containers

**Assumption valid if**: All IPs are unique and in bridge subnet
**Assumption invalid if**: IPs are identical or outside expected range

---

### Test Group 4: Edge Cases

#### Test 4.1: Container Restart Preserves Isolation

**Tests**: Restarting a container maintains network isolation
**Research identified**: "Each container start creates fresh namespace; state does not persist"

**Procedure**:
```bash
# Restart container 2
docker restart arch-net-2
sleep 2
# Verify it still works independently
docker exec arch-net-2 wget -qO- http://localhost:3000
# Verify container 1 was unaffected
docker exec arch-net-1 wget -qO- http://localhost:3000
```

**Expected result**: Both containers respond correctly after restart

**Measurements**:
- [ ] Container 2 response after restart
- [ ] Container 1 response (unaffected)

**Handles correctly if**: Both return their unique identifiers
**Breaks if**: Either returns wrong response or fails

---

#### Test 4.2: Simultaneous Port Binding Stress

**Tests**: Multiple containers binding same port under rapid creation
**Research identified**: "Namespace creation is atomic per container"

**Procedure**:
```bash
# Rapidly create 5 more containers
for i in 4 5 6 7 8; do
  docker run -d --name arch-net-$i alpine sh -c "while true; do echo -e 'HTTP/1.1 200 OK\r\nContent-Length: 12\r\n\r\nworkspace-0$i' | nc -l -p 3000; done"
done
sleep 2
# Verify all 8 are running
docker ps --filter name=arch-net --format "{{.Names}}" | sort
# Spot check a few
docker exec arch-net-5 wget -qO- http://localhost:3000
docker exec arch-net-8 wget -qO- http://localhost:3000
```

**Expected result**: All 8 containers running, each responding correctly

**Measurements**:
- [ ] Number of running containers
- [ ] Spot check responses

**Handles correctly if**: All 8 start and respond independently
**Breaks if**: Any fail to start or return wrong response

---

## Teardown

```bash
docker rm -f arch-net-1 arch-net-2 arch-net-3 arch-net-4 arch-net-5 arch-net-6 arch-net-7 arch-net-8 2>/dev/null
```

## Execution Order

1. Test Group 1 (core isolation)
2. Test Group 2 (host access limitation)
3. Test Group 3 (bridge IP uniqueness)
4. Test Group 4 (edge cases — restart, stress)
5. Teardown

## Expected Artifacts

- `findings.md` — Raw results from Run phase
- `measurements/` — Captured outputs

## Notes for Run Phase

- Run all tests even if earlier ones fail
- Capture exact output, don't interpret
- Note any deviations from procedure
- Record timestamps for each test
