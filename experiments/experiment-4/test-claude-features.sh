#!/bin/bash
# Test wrapper compatibility with Claude Code features:
# - TaskCreate (task management)
# - Agents (custom agents)
#
# These features may have different subprocess invocation patterns

set -e

WRAPPER_PATH="experiments/experiment-3/conductor-intercept-posix.sh"
RESULTS_FILE="/tmp/dual-33-claude-features.txt"

echo "=== Testing Wrapper with Claude Code Features ===" | tee "$RESULTS_FILE"
echo "Testing TaskCreate and Agents subprocess patterns" | tee -a "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

# Test 1: TaskCreate - simulate task execution with subprocess
echo "Test 1: TaskCreate subprocess pattern" | tee -a "$RESULTS_FILE"
echo "=======================================" | tee -a "$RESULTS_FILE"

# TaskCreate likely spawns a subprocess to execute task commands
# Pattern: bash -c "task_command"
OUTPUT=$(bash -c "source $WRAPPER_PATH && pnpm --version" 2>&1 || true)
if echo "$OUTPUT" | grep -q "10.5.2"; then
  echo "✅ PASS - TaskCreate subprocess pattern works" | tee -a "$RESULTS_FILE"
  echo "   Output: $OUTPUT" | tee -a "$RESULTS_FILE"
else
  echo "❌ FAIL - TaskCreate subprocess pattern failed" | tee -a "$RESULTS_FILE"
  echo "   Output: $OUTPUT" | tee -a "$RESULTS_FILE"
fi
echo "" >> "$RESULTS_FILE"

# Test 2: Agents - simulate agent execution context
echo "Test 2: Agents subprocess pattern (basic)" | tee -a "$RESULTS_FILE"
echo "===========================================" | tee -a "$RESULTS_FILE"

# Agents might have their own subprocess context
# Pattern: Similar bash -c but potentially with different env setup
OUTPUT=$(bash -c "source $WRAPPER_PATH && node --version" 2>&1 || true)
if echo "$OUTPUT" | grep -q "v20"; then
  echo "✅ PASS - Agent subprocess pattern works" | tee -a "$RESULTS_FILE"
  echo "   Output: $OUTPUT" | tee -a "$RESULTS_FILE"
else
  echo "❌ FAIL - Agent subprocess pattern failed" | tee -a "$RESULTS_FILE"
  echo "   Output: $OUTPUT" | tee -a "$RESULTS_FILE"
fi
echo "" >> "$RESULTS_FILE"

# Test 3: Agents with environment isolation
echo "Test 3: Agents with environment variables" | tee -a "$RESULTS_FILE"
echo "==========================================" | tee -a "$RESULTS_FILE"

OUTPUT=$(bash -c "export AGENT_ID='test-agent' && source $WRAPPER_PATH && node -e \"console.log(process.env.AGENT_ID || 'undefined')\"" 2>&1 || true)
if echo "$OUTPUT" | grep -q "test-agent"; then
  echo "✅ PASS - Agent env vars accessible in subprocess" | tee -a "$RESULTS_FILE"
  echo "   Output: $OUTPUT" | tee -a "$RESULTS_FILE"
else
  echo "❌ FAIL - Agent env vars not accessible" | tee -a "$RESULTS_FILE"
  echo "   Output: $OUTPUT" | tee -a "$RESULTS_FILE"
fi
echo "" >> "$RESULTS_FILE"

# Test 4: TaskCreate with multiple sequential commands
echo "Test 4: TaskCreate with task chain execution" | tee -a "$RESULTS_FILE"
echo "=============================================" | tee -a "$RESULTS_FILE"

OUTPUT=$(bash -c "
  source $WRAPPER_PATH
  pnpm --version > /tmp/task-test.txt
  echo '---' >> /tmp/task-test.txt
  node --version >> /tmp/task-test.txt
  cat /tmp/task-test.txt
" 2>&1 || true)

if echo "$OUTPUT" | grep -q "10.5.2" && echo "$OUTPUT" | grep -q "v20"; then
  echo "✅ PASS - TaskCreate multi-command chain works" | tee -a "$RESULTS_FILE"
else
  echo "❌ FAIL - TaskCreate multi-command chain failed" | tee -a "$RESULTS_FILE"
fi
echo "   Output:" | tee -a "$RESULTS_FILE"
echo "$OUTPUT" | tee -a "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

# Test 5: Agent isolation - does wrapper work in nested agent context?
echo "Test 5: Nested agent subprocess (agent spawning subagent)" | tee -a "$RESULTS_FILE"
echo "==========================================================" | tee -a "$RESULTS_FILE"

# Simulate: agent spawns another agent context
OUTPUT=$(bash -c "
  source $WRAPPER_PATH

  # Agent context 1
  node -e \"
    const { execSync } = require('child_process');
    try {
      const result = execSync('echo hello from agent').toString();
      console.log(result);
    } catch(e) {
      console.error('Nested agent failed');
    }
  \"
" 2>&1 || true)

if echo "$OUTPUT" | grep -q "hello from agent"; then
  echo "✅ PASS - Nested agent context works" | tee -a "$RESULTS_FILE"
else
  echo "❌ FAIL - Nested agent context failed" | tee -a "$RESULTS_FILE"
fi
echo "   Output: $OUTPUT" | tee -a "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

# Test 6: TaskCreate with failure handling
echo "Test 6: TaskCreate error handling" | tee -a "$RESULTS_FILE"
echo "=================================" | tee -a "$RESULTS_FILE"

OUTPUT=$(bash -c "
  source $WRAPPER_PATH

  # Task 1: succeeds
  node -e \"console.log('task1')\" || exit 1

  # Task 2: fails
  pnpm nonexistent 2>/dev/null || true

  # Task 3: should still execute
  node -e \"console.log('task3')\" || exit 1
" 2>&1 || true)

if echo "$OUTPUT" | grep -q "task1" && echo "$OUTPUT" | grep -q "task3"; then
  echo "✅ PASS - TaskCreate continues despite intermediate failure" | tee -a "$RESULTS_FILE"
else
  echo "❌ FAIL - TaskCreate error handling broken" | tee -a "$RESULTS_FILE"
fi
echo "" >> "$RESULTS_FILE"

# Test 7: Real agent test (if claude available)
echo "Test 7: Real Claude Code agent invocation" | tee -a "$RESULTS_FILE"
echo "=========================================" | tee -a "$RESULTS_FILE"

if command -v claude &>/dev/null; then
  echo "Note: This would test actual agent execution with wrapper" | tee -a "$RESULTS_FILE"
  echo "Requires: Custom agent definition + task invocation" | tee -a "$RESULTS_FILE"
  echo "Status: MANUAL TEST REQUIRED" | tee -a "$RESULTS_FILE"
else
  echo "SKIPPED - claude CLI not available" | tee -a "$RESULTS_FILE"
fi
echo "" >> "$RESULTS_FILE"

# Summary
echo "=== Summary ===" | tee -a "$RESULTS_FILE"
echo "All subprocess pattern tests completed" | tee -a "$RESULTS_FILE"
echo "Results written to: $RESULTS_FILE" | tee -a "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

cat "$RESULTS_FILE"
