#!/bin/bash
# Test Claude Code integration with conductor-intercept.sh wrapper
#
# Claude Code invokes commands via subprocess (bash -c "..."), not interactive shell.
# This test simulates that pattern to validate the wrapper works end-to-end.
#
# Prerequisites:
#   1. Docker container "spike-intercept" must be running
#   2. polychromos cloned at ~/dev/polychromos
#   3. This script sources conductor-intercept.sh in a subprocess context

set -e

RESULTS_FILE="/tmp/claude-code-integration-test.txt"

echo "=== Claude Code Integration Test ===" | tee "$RESULTS_FILE"
echo "Testing wrapper with subprocess invocation (how Claude Code calls commands)" | tee -a "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

# Test 1: Subprocess command invocation (how Claude Code calls commands)
echo "Test 1: Subprocess invocation of pnpm command" | tee -a "$RESULTS_FILE"

# Simulate Claude Code invoking a command in a subprocess
bash -c "source experiments/experiment-3/conductor-intercept.sh 2>/dev/null && pnpm --version" 2>/dev/null | grep -q "10.5.2"
if [ $? -eq 0 ]; then
  echo "✅ PASS - pnpm routed through wrapper in subprocess" | tee -a "$RESULTS_FILE"
else
  echo "❌ FAIL - pnpm did not route in subprocess" | tee -a "$RESULTS_FILE"
fi

echo "" >> "$RESULTS_FILE"

# Test 2: Multiple commands in sequence (mimics Claude Code workflow)
echo "Test 2: Sequential commands in subprocess context" | tee -a "$RESULTS_FILE"

bash -c "
  source experiments/experiment-3/conductor-intercept.sh 2>/dev/null

  # Command 1: pnpm install
  pnpm install > /dev/null 2>&1

  # Command 2: node execution
  node -e \"console.log('test')\" > /tmp/test-output.txt 2>&1

  # Command 3: Check exit code
  node -e \"process.exit(0)\"
" 2>/dev/null

if [ $? -eq 0 ]; then
  echo "✅ PASS - Sequential commands executed in subprocess" | tee -a "$RESULTS_FILE"
else
  echo "❌ FAIL - Sequential commands failed in subprocess" | tee -a "$RESULTS_FILE"
fi

echo "" >> "$RESULTS_FILE"

# Test 3: Command with exit code checking (critical for Claude Code)
echo "Test 3: Exit code propagation in subprocess" | tee -a "$RESULTS_FILE"

bash -c "
  source experiments/experiment-3/conductor-intercept.sh 2>/dev/null
  node -e \"process.exit(42)\"
" 2>/dev/null
EXIT_CODE=$?

if [ $EXIT_CODE -eq 42 ]; then
  echo "✅ PASS - Exit code 42 propagated correctly" | tee -a "$RESULTS_FILE"
else
  echo "❌ FAIL - Exit code not propagated (got $EXIT_CODE, expected 42)" | tee -a "$RESULTS_FILE"
fi

echo "" >> "$RESULTS_FILE"

# Test 4: Environment variable passing through subprocess
echo "Test 4: Environment variable propagation in subprocess" | tee -a "$RESULTS_FILE"

bash -c "
  export NODE_ENV=production
  source experiments/experiment-3/conductor-intercept.sh 2>/dev/null
  node -e \"console.log(process.env.NODE_ENV)\"
" 2>/dev/null | grep -q "production"

if [ $? -eq 0 ]; then
  echo "✅ PASS - NODE_ENV propagated through subprocess" | tee -a "$RESULTS_FILE"
else
  echo "❌ FAIL - NODE_ENV not propagated in subprocess" | tee -a "$RESULTS_FILE"
fi

echo "" >> "$RESULTS_FILE"

# Test 5: Piped commands in subprocess (complex I/O)
echo "Test 5: Piped commands in subprocess context" | tee -a "$RESULTS_FILE"

bash -c "
  source experiments/experiment-3/conductor-intercept.sh 2>/dev/null
  echo 'hello' | node -e \"let d=''; process.stdin.on('data',c=>d+=c); process.stdin.on('end',()=>console.log(d.toUpperCase()))\"
" 2>/dev/null | grep -q "HELLO"

if [ $? -eq 0 ]; then
  echo "✅ PASS - Piped commands work in subprocess" | tee -a "$RESULTS_FILE"
else
  echo "❌ FAIL - Piped commands failed in subprocess" | tee -a "$RESULTS_FILE"
fi

echo "" >> "$RESULTS_FILE"

# Test 6: Shell detection test
echo "Test 6: Shell selection test (bash vs sh)" | tee -a "$RESULTS_FILE"

# Test with bash (should work)
bash -c "source experiments/experiment-3/conductor-intercept.sh 2>/dev/null && pnpm --version" 2>/dev/null | grep -q "10.5.2"
BASH_RESULT=$?

# Test with sh (may fail if sh is dash/ash/etc)
sh -c "source experiments/experiment-3/conductor-intercept.sh 2>/dev/null && pnpm --version" 2>/dev/null | grep -q "10.5.2"
SH_RESULT=$?

if [ $BASH_RESULT -eq 0 ]; then
  echo "✅ bash: pnpm routed correctly" | tee -a "$RESULTS_FILE"
else
  echo "❌ bash: pnpm routing failed" | tee -a "$RESULTS_FILE"
fi

if [ $SH_RESULT -eq 0 ]; then
  echo "✅ sh: pnpm routed correctly (function overrides work in sh)" | tee -a "$RESULTS_FILE"
else
  echo "⚠️  sh: pnpm routing failed (sh doesn't support bash function overrides - expected)" | tee -a "$RESULTS_FILE"
fi

echo "" >> "$RESULTS_FILE"

echo "=== Integration Test Complete ===" >> "$RESULTS_FILE"
echo "See results above. If Test 6 shows sh failure, consider creating dash/sh variant of wrapper." >> "$RESULTS_FILE"

cat "$RESULTS_FILE"
