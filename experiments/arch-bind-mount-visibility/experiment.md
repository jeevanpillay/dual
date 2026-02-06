# Experiment: bind-mount-visibility

**Claim**: Bind mount makes host file changes visible in container
**Spec Reference**: "File edits on the host are immediately visible inside the container"

## Hypothesis

When a directory is bind-mounted into a Docker container, file changes made on the host filesystem are visible inside the container without any explicit sync operation.

## Test Cases

### Test 2.1: File Creation Visibility
**Goal**: Verify files created on host appear in container
**Setup**: Container running with bind mount
**Action**: Create file on host
**Expected**: File visible in container
**Pass Criteria**: `ls` in container shows the new file

### Test 2.2: File Modification Visibility
**Goal**: Verify file changes propagate to container
**Setup**: File exists in bind-mounted directory
**Action**: Modify file content on host
**Expected**: New content readable in container
**Pass Criteria**: `cat` in container shows updated content

### Test 2.3: File Deletion Visibility
**Goal**: Verify file deletions propagate to container
**Setup**: File exists in bind-mounted directory
**Action**: Delete file on host
**Expected**: File no longer exists in container
**Pass Criteria**: `ls` in container does not show the file

### Test 2.4: Propagation Latency
**Goal**: Measure time for changes to become visible
**Setup**: Container polling for file existence
**Action**: Create file on host, measure time until visible
**Expected**: <500ms propagation
**Pass Criteria**: File visible within acceptable latency

### Test 2.5: File Watcher Events
**Goal**: Verify inotify receives events from host changes
**Setup**: inotifywait running in container
**Action**: Modify file on host
**Expected**: inotify event received
**Pass Criteria**: Event captured in container

### Test 2.6: Bidirectional Sync
**Goal**: Verify container writes are visible on host
**Setup**: Container running with bind mount
**Action**: Create/modify file from inside container
**Expected**: Changes visible on host
**Pass Criteria**: Host can read container's changes

## Success Criteria

- All visibility tests pass (2.1-2.3, 2.6)
- Latency <500ms (Test 2.4)
- File watcher events work (Test 2.5)

## Failure Criteria

- Files not visible across mount boundary
- Propagation latency >1000ms
- File watcher events don't propagate
