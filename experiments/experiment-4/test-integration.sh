#!/bin/bash
# Test Claude Code integration validation (DUAL-33)
# Validates wrapper behavior in subprocess contexts and multi-shell environments
# Prerequisites: spike-intercept container running, conductor-intercept-posix.sh in experiments/experiment-3/

set -e

RESULTS_FILE="/tmp/dual-33-integration-test.txt"
WRAPPER_PATH="experiments/experiment-3/conductor-intercept-posix.sh"
TESTS_PASSED=0
TESTS_FAILED=0

# Helper: Run test and record result
run_test() {
  local test_num="$1"
  local test_name="$2"
  local test_cmd="$3"

  echo "Test $test_num: $test_name" | tee -a "$RESULTS_FILE"

  if eval "$test_cmd" 2>/dev/null; then
    echo "✅ PASS" | tee -a "$RESULTS_FILE"
    TESTS_PASSED=$((TESTS_PASSED + 1))
  else
    echo "❌ FAIL" | tee -a "$RESULTS_FILE"
    TESTS_FAILED=$((TESTS_FAILED + 1))
  fi
  echo "" >> "$RESULTS_FILE"
}

echo "=== DUAL-33: Claude Code Integration Validation ===" | tee "$RESULTS_FILE"
echo "Testing wrapper in subprocess contexts and real Claude Code patterns" | tee -a "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

# Test 1: Subprocess initialization (bash -c pattern)
run_test "1a" "Subprocess: pnpm routing (bash -c)" \
  "bash -c \"source $WRAPPER_PATH && pnpm --version\" 2>/dev/null | grep -q '10.5.2'"

run_test "1b" "Subprocess: node routing (bash -c)" \
  "bash -c \"source $WRAPPER_PATH && node --version\" 2>/dev/null | grep -q 'v20'"

# Test 2: Multi-shell compatibility
run_test "2a" "Multi-shell: bash compatibility" \
  "bash -c \"source $WRAPPER_PATH && pnpm --version\" 2>/dev/null | grep -q '10.5.2'"

run_test "2b" "Multi-shell: zsh compatibility" \
  "zsh -c \"source $WRAPPER_PATH && pnpm --version\" 2>/dev/null | grep -q '10.5.2'"

run_test "2c" "Multi-shell: sh compatibility" \
  "sh -c \"source $WRAPPER_PATH && pnpm --version\" 2>/dev/null | grep -q '10.5.2'"

# Test 3: Complex command patterns
run_test "3a" "Pattern: Piped command (echo | node)" \
  "bash -c \"source $WRAPPER_PATH && echo 'test' | node -e \\\"let d=''; process.stdin.on('data',c=>d+=c); process.stdin.on('end',()=>console.log(d.toUpperCase()))\\\"\" 2>/dev/null | grep -q 'TEST'"

run_test "3b" "Pattern: Command substitution" \
  "bash -c \"source $WRAPPER_PATH && node -e \\\"console.log('nested')\\\" && echo Done\" 2>/dev/null | grep -q 'nested'"

run_test "3c" "Pattern: Redirection (> file)" \
  "bash -c \"source $WRAPPER_PATH && pnpm --version > /tmp/version.txt && cat /tmp/version.txt\" 2>/dev/null | grep -q '10.5.2'"

run_test "3d" "Pattern: Exit code checking" \
  "bash -c \"source $WRAPPER_PATH && node -e \\\"process.exit(42)\\\"; echo \\\"Exit: \\\$?\\\"\" 2>/dev/null | grep -q 'Exit: 42'"

# Test 4: Large output handling
run_test "4a" "Large output: 100K character string" \
  "bash -c \"source $WRAPPER_PATH && node -e \\\"console.log('x'.repeat(100000))\\\"\" 2>/dev/null | wc -c | grep -qE '^[0-9]{6}'"

run_test "4b" "Large output: 1000 lines" \
  "bash -c \"source $WRAPPER_PATH && node -e \\\"for(let i=0;i<1000;i++) console.log('line ' + i)\\\"\" 2>/dev/null | wc -l | grep -q '1000'"

# Test 5: Rapid successive commands
run_test "5" "Rapid commands: 10 sequential node invocations" \
  "bash -c \"source $WRAPPER_PATH && for i in {1..10}; do node -e \\\"console.log('cmd ' + \\\$i)\\\" || exit 1; done\" 2>/dev/null | wc -l | grep -q '10'"

# Test 6: Nested subprocesses
run_test "6" "Nested subprocess: child_process.execSync" \
  "bash -c \"source $WRAPPER_PATH && node -e \\\"require('child_process').execSync('echo nested')\\\"\" 2>/dev/null | grep -q 'nested'"

# Test 7: Environment variable propagation
run_test "7a" "Env vars: User-defined variable" \
  "bash -c \"export MY_VAR='test-value' && source $WRAPPER_PATH && node -e \\\"console.log(process.env.MY_VAR)\\\"\" 2>/dev/null | grep -q 'test-value'"

run_test "7b" "Env vars: NODE_ENV propagation" \
  "bash -c \"export NODE_ENV='production' && source $WRAPPER_PATH && node -e \\\"console.log(process.env.NODE_ENV)\\\"\" 2>/dev/null | grep -q 'production'"

# Test 8: Error handling and recovery
run_test "8a" "Error handling: Command failure" \
  "bash -c \"source $WRAPPER_PATH && (node -e \\\"process.exit(1)\\\" || true) && echo 'recovered'\" 2>/dev/null | grep -q 'recovered'"

run_test "8b" "Error handling: Multiple commands with failure" \
  "bash -c \"source $WRAPPER_PATH && node -e \\\"console.log('start')\\\" && (pnpm nonexistent 2>/dev/null || true) && node -e \\\"console.log('end')\\\"\" 2>/dev/null | grep 'start' | grep -q 'start'"

# Test 9: Real Claude Code CLI (if available)
if command -v claude &> /dev/null; then
  run_test "9a" "Real Claude CLI: pnpm --version" \
    "source $WRAPPER_PATH && claude code 'pnpm --version' 2>/dev/null | grep -q '10.5.2'"

  run_test "9b" "Real Claude CLI: node --version" \
    "source $WRAPPER_PATH && claude code 'node --version' 2>/dev/null | grep -q 'v20'"
else
  echo "Test 9: Real Claude Code CLI - SKIPPED (claude not in PATH)" | tee -a "$RESULTS_FILE"
  echo "" >> "$RESULTS_FILE"
fi

# Summary
echo "=== Test Summary ===" | tee -a "$RESULTS_FILE"
echo "Passed: $TESTS_PASSED" | tee -a "$RESULTS_FILE"
echo "Failed: $TESTS_FAILED" | tee -a "$RESULTS_FILE"

if [ $TESTS_FAILED -eq 0 ]; then
  echo "Status: ✅ All tests passed" | tee -a "$RESULTS_FILE"
else
  echo "Status: ❌ Some tests failed" | tee -a "$RESULTS_FILE"
fi

echo "" >> "$RESULTS_FILE"
cat "$RESULTS_FILE"
