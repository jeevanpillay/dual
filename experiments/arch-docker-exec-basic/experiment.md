# Experiment: docker-exec-basic

**Claim**: Docker exec can run commands in a container
**Spec Reference**: "dual run <command> wraps docker exec <container-name> <command>"

## Hypothesis

Docker exec can execute arbitrary commands in a running container, preserving the command's behavior (output, environment access, filesystem access) as if the command ran natively.

## Test Cases

### Test 1.1: Basic Command Execution
**Goal**: Verify docker exec runs commands in container
**Command**: `docker exec <container> echo hello`
**Expected**: Outputs "hello"
**Pass Criteria**: Command executes and produces expected output

### Test 1.2: Exit Code Preservation
**Goal**: Verify exit codes pass through unchanged
**Command**: `docker exec <container> sh -c 'exit 42'; echo $?`
**Expected**: Outputs "42"
**Pass Criteria**: Exit code matches the command's exit code

### Test 1.3: Environment Variable Injection
**Goal**: Verify env vars can be passed to container
**Command**: `docker exec -e TEST_VAR=hello <container> sh -c 'echo $TEST_VAR'`
**Expected**: Outputs "hello"
**Pass Criteria**: Injected env var is accessible in container

### Test 1.4: Working Directory Control
**Goal**: Verify working directory can be set
**Command**: `docker exec -w /tmp <container> pwd`
**Expected**: Outputs "/tmp"
**Pass Criteria**: Working directory matches specified path

### Test 1.5: TTY Allocation
**Goal**: Verify TTY can be allocated for interactive commands
**Command**: `docker exec -t <container> tty`
**Expected**: Outputs a TTY path (e.g., /dev/pts/0)
**Pass Criteria**: TTY is allocated and accessible

### Test 1.6: Stdout/Stderr Separation
**Goal**: Verify both streams are captured
**Command**: `docker exec <container> sh -c 'echo out; echo err >&2'`
**Expected**: Both "out" and "err" appear in output
**Pass Criteria**: Both streams captured

## Success Criteria

- All test cases pass
- Exit codes preserved (Tests 1.2)
- Environment and CWD controllable (Tests 1.3, 1.4)
- TTY allocation works (Test 1.5)

## Failure Criteria

- Any test case fails
- Exit codes not preserved
- Cannot control environment or working directory
