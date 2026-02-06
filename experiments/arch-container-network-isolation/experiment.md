# Experiment: container-network-isolation

**Claim**: Each container has isolated network namespace allowing same-port binding
**Spec Reference**: "15 containers can all bind :3000 simultaneously"

## Hypothesis

Each Docker container has its own isolated network namespace, allowing multiple containers to each bind the same port (e.g., :3000) without conflict.

## Test Cases

### Test 3.1: Multiple Containers Bind Same Port
**Goal**: Verify multiple containers can bind :3000 simultaneously
**Setup**: Start 5+ containers, each running HTTP server on :3000
**Expected**: All containers start successfully, all bind :3000
**Pass Criteria**: No EADDRINUSE errors, all containers running

### Test 3.2: Namespace ID Uniqueness
**Goal**: Verify each container has a unique network namespace
**Setup**: Check /proc/self/ns/net in each container
**Expected**: Different namespace IDs
**Pass Criteria**: All namespace IDs are distinct

### Test 3.3: localhost Isolation
**Goal**: Verify localhost in each container is isolated
**Setup**: Container A serves "A", Container B serves "B" on :3000
**Action**: curl localhost:3000 from within each container
**Expected**: Each sees only its own service
**Pass Criteria**: Container A sees "A", Container B sees "B"

### Test 3.4: Cross-Container Access
**Goal**: Verify containers can reach each other via IP
**Setup**: Containers on bridge network
**Action**: curl from Container A to Container B's IP:3000
**Expected**: Connection succeeds
**Pass Criteria**: Response received from other container

### Test 3.5: Independent Accessibility
**Goal**: Verify each container's service is independently reachable
**Setup**: Multiple containers with unique content
**Action**: Access each via container IP from host
**Expected**: Each returns its unique content
**Pass Criteria**: All containers accessible, correct content

## Success Criteria

- All containers bind :3000 without conflict
- Namespace IDs are unique
- localhost is isolated per container
- Services independently accessible

## Failure Criteria

- Any container fails to bind :3000
- Port conflict errors (EADDRINUSE)
- localhost leaks between containers
