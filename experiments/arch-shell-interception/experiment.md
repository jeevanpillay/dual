# Experiment: shell-interception

**Claim**: Shell wrapper can intercept runtime commands
**Spec Reference**: "Claude Code's shell is configured so that commands are intercepted and routed"

## Hypothesis

A shell wrapper using function overrides can intercept commands like `node`, `pnpm`, `npm` and route them to `docker exec` while preserving exit codes, arguments, and TTY behavior.

## Test Cases

### Test 5.1: Basic Interception
**Goal**: Verify function intercepts command before PATH lookup
**Setup**: Define `npm() { echo "[INTERCEPTED]"; }`
**Test**: Run `npm install`
**Expected**: Outputs "[INTERCEPTED]"
**Pass Criteria**: Function executes instead of binary

### Test 5.2: Argument Passthrough
**Goal**: Verify arguments pass through correctly
**Setup**: `npm() { echo "args: $@"; }`
**Test**: `npm install lodash --save`
**Expected**: `args: install lodash --save`
**Pass Criteria**: All arguments preserved

### Test 5.3: Exit Code Preservation
**Goal**: Verify exit codes return correctly
**Setup**: `npm() { return 42; }`
**Test**: `npm; echo $?`
**Expected**: `42`
**Pass Criteria**: Exit code matches function return

### Test 5.4: Special Characters
**Goal**: Verify special characters in arguments preserved
**Test**: `npm install "package name" 'with spaces'`
**Expected**: Spaces and quotes preserved
**Pass Criteria**: Arguments intact

### Test 5.5: TTY Detection
**Goal**: Verify TTY can be detected for conditional -it flags
**Setup**: `npm() { test -t 1 && echo "TTY" || echo "NO TTY"; }`
**Test**: Run interactively vs in pipe
**Expected**: "TTY" vs "NO TTY" appropriately
**Pass Criteria**: Correct TTY detection

### Test 5.6: Docker Exec Integration
**Goal**: Verify function can route to docker exec
**Setup**: Container running, function routes to it
**Test**: `npm --version`
**Expected**: Returns version from container's npm
**Pass Criteria**: Command executes in container

### Test 5.7: Bypass Methods
**Goal**: Document what bypasses interception
**Test**:
- `command npm`
- `/usr/bin/npm`
- `\npm`
**Expected**: Bypasses function, runs real binary
**Pass Criteria**: Understand bypass vectors

## Success Criteria

- Function interception works for common commands
- Arguments and exit codes preserved
- TTY can be detected
- Docker exec routing works
- Bypass methods documented

## Failure Criteria

- Function cannot intercept commands
- Arguments corrupted
- Exit codes lost
- Cannot route to docker exec
