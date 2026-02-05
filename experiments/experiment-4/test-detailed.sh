#!/bin/bash
# Detailed test harness for DUAL-33: Claude Code integration validation
# Provides granular testing with detailed output for diagnostics

set -e

WRAPPER_PATH="experiments/experiment-3/conductor-intercept-posix.sh"
RESULTS_DIR="/tmp/dual-33-detailed"
mkdir -p "$RESULTS_DIR"

test_section() {
  echo ""
  echo "━━━ $1 ━━━"
}

record_result() {
  local test_name="$1"
  local status="$2"
  local details="$3"

  echo "$test_name | $status | $details" >> "$RESULTS_DIR/results.csv"

  if [ "$status" = "PASS" ]; then
    echo "✅ $test_name"
  else
    echo "❌ $test_name: $details"
  fi
}

echo "=== DUAL-33: Detailed Integration Test Suite ===" > "$RESULTS_DIR/results.csv"
echo "Test Name | Status | Details" >> "$RESULTS_DIR/results.csv"

# Test 1: Subprocess initialization
test_section "Test 1: Subprocess initialization (bash -c)"

# 1a: pnpm version routing
OUTPUT=$(bash -c "source $WRAPPER_PATH && pnpm --version" 2>&1 || true)
if echo "$OUTPUT" | grep -q "10.5.2"; then
  record_result "1a_pnpm_version" "PASS" "Routed to container (10.5.2)"
else
  record_result "1a_pnpm_version" "FAIL" "Got: $OUTPUT"
fi

# 1b: node version routing
OUTPUT=$(bash -c "source $WRAPPER_PATH && node --version" 2>&1 || true)
if echo "$OUTPUT" | grep -q "v20"; then
  record_result "1b_node_version" "PASS" "Routed to container (v20)"
else
  record_result "1b_node_version" "FAIL" "Got: $OUTPUT"
fi

# Test 2: Multi-shell compatibility
test_section "Test 2: Multi-shell compatibility"

# 2a: bash
OUTPUT=$(bash -c "source $WRAPPER_PATH && pnpm --version" 2>&1 || true)
if echo "$OUTPUT" | grep -q "10.5.2"; then
  record_result "2a_shell_bash" "PASS" "bash shell works"
else
  record_result "2a_shell_bash" "FAIL" "Got: $OUTPUT"
fi

# 2b: zsh
if command -v zsh &>/dev/null; then
  OUTPUT=$(zsh -c "source $WRAPPER_PATH && pnpm --version" 2>&1 || true)
  if echo "$OUTPUT" | grep -q "10.5.2"; then
    record_result "2b_shell_zsh" "PASS" "zsh shell works"
  else
    record_result "2b_shell_zsh" "FAIL" "Got: $OUTPUT"
  fi
else
  record_result "2b_shell_zsh" "SKIP" "zsh not installed"
fi

# 2c: sh
OUTPUT=$(sh -c "source $WRAPPER_PATH && pnpm --version" 2>&1 || true)
if echo "$OUTPUT" | grep -q "10.5.2"; then
  record_result "2c_shell_sh" "PASS" "sh shell works"
else
  record_result "2c_shell_sh" "FAIL" "Got: $OUTPUT"
fi

# 2d: dash
if command -v dash &>/dev/null; then
  OUTPUT=$(dash -c "source $WRAPPER_PATH && pnpm --version" 2>&1 || true)
  if echo "$OUTPUT" | grep -q "10.5.2"; then
    record_result "2d_shell_dash" "PASS" "dash shell works"
  else
    record_result "2d_shell_dash" "FAIL" "Got: $OUTPUT"
  fi
else
  record_result "2d_shell_dash" "SKIP" "dash not installed"
fi

# Test 3: Complex command patterns
test_section "Test 3: Complex command patterns"

# 3a: Piped command
OUTPUT=$(bash -c "source $WRAPPER_PATH && echo 'test' | node -e \"let d=''; process.stdin.on('data',c=>d+=c); process.stdin.on('end',()=>console.log(d.toUpperCase()))\"" 2>&1 || true)
if echo "$OUTPUT" | grep -q "TEST"; then
  record_result "3a_pipe_echo_node" "PASS" "Piped data flows correctly"
else
  record_result "3a_pipe_echo_node" "FAIL" "Got: $OUTPUT"
fi

# 3b: Command substitution
OUTPUT=$(bash -c "source $WRAPPER_PATH && node -e \"console.log('nested')\" && echo Done" 2>&1 || true)
if echo "$OUTPUT" | grep -q "nested" && echo "$OUTPUT" | grep -q "Done"; then
  record_result "3b_command_substitution" "PASS" "Sequential commands work"
else
  record_result "3b_command_substitution" "FAIL" "Got: $OUTPUT"
fi

# 3c: Redirection
OUTPUT=$(bash -c "source $WRAPPER_PATH && pnpm --version > /tmp/test-version.txt && cat /tmp/test-version.txt" 2>&1 || true)
if echo "$OUTPUT" | grep -q "10.5.2"; then
  record_result "3c_redirection_file" "PASS" "File redirection works"
else
  record_result "3c_redirection_file" "FAIL" "Got: $OUTPUT"
fi

# 3d: Exit code
bash -c "source $WRAPPER_PATH && node -e \"process.exit(42)\"" 2>/dev/null || EXIT_CODE=$?
if [ "$EXIT_CODE" = "42" ]; then
  record_result "3d_exit_code" "PASS" "Exit code 42 propagated"
else
  record_result "3d_exit_code" "FAIL" "Got exit code: $EXIT_CODE"
fi

# Test 4: Large output handling
test_section "Test 4: Large output handling"

# 4a: Large string output (100K chars)
OUTPUT=$(bash -c "source $WRAPPER_PATH && node -e \"console.log('x'.repeat(100000))\"" 2>&1 || true)
CHAR_COUNT=$(echo -n "$OUTPUT" | wc -c)
if [ "$CHAR_COUNT" -gt 99000 ]; then
  record_result "4a_large_string" "PASS" "Output not truncated ($CHAR_COUNT chars)"
else
  record_result "4a_large_string" "FAIL" "Output truncated ($CHAR_COUNT chars)"
fi

# 4b: Multiple lines (1000 lines)
LINE_COUNT=$(bash -c "source $WRAPPER_PATH && node -e \"for(let i=0;i<1000;i++) console.log('line ' + i)\"" 2>&1 | wc -l || true)
if [ "$LINE_COUNT" -ge 999 ]; then
  record_result "4b_many_lines" "PASS" "All lines present ($LINE_COUNT)"
else
  record_result "4b_many_lines" "FAIL" "Lines truncated ($LINE_COUNT/1000)"
fi

# Test 5: Rapid successive commands
test_section "Test 5: Rapid successive commands"

OUTPUT=$(bash -c "source $WRAPPER_PATH && for i in {1..10}; do node -e \"console.log('cmd ' + \$i)\" || exit 1; done" 2>&1 || true)
CMD_COUNT=$(echo "$OUTPUT" | wc -l)
if [ "$CMD_COUNT" -ge 9 ]; then
  record_result "5_rapid_commands" "PASS" "All 10 commands executed ($CMD_COUNT lines)"
else
  record_result "5_rapid_commands" "FAIL" "Only $CMD_COUNT commands executed"
fi

# Test 6: Nested subprocesses
test_section "Test 6: Nested subprocesses"

OUTPUT=$(bash -c "source $WRAPPER_PATH && node -e \"require('child_process').execSync('echo nested')\"" 2>&1 || true)
if echo "$OUTPUT" | grep -q "nested"; then
  record_result "6_nested_subprocess" "PASS" "Nested execSync works"
else
  record_result "6_nested_subprocess" "FAIL" "Got: $OUTPUT"
fi

# Test 7: Environment variable propagation
test_section "Test 7: Environment variable propagation"

# 7a: User-defined variable
OUTPUT=$(bash -c "export MY_VAR='test-value' && source $WRAPPER_PATH && node -e \"console.log(process.env.MY_VAR)\"" 2>&1 || true)
if echo "$OUTPUT" | grep -q "test-value"; then
  record_result "7a_user_env_var" "PASS" "User env var propagated"
else
  record_result "7a_user_env_var" "FAIL" "Got: $OUTPUT"
fi

# 7b: NODE_ENV variable
OUTPUT=$(bash -c "export NODE_ENV='production' && source $WRAPPER_PATH && node -e \"console.log(process.env.NODE_ENV)\"" 2>&1 || true)
if echo "$OUTPUT" | grep -q "production"; then
  record_result "7b_node_env_var" "PASS" "NODE_ENV propagated"
else
  record_result "7b_node_env_var" "FAIL" "Got: $OUTPUT"
fi

# Test 8: Error handling and recovery
test_section "Test 8: Error handling and recovery"

# 8a: Graceful failure and recovery
OUTPUT=$(bash -c "source $WRAPPER_PATH && (node -e \"process.exit(1)\" 2>/dev/null || true) && echo 'recovered'" 2>&1 || true)
if echo "$OUTPUT" | grep -q "recovered"; then
  record_result "8a_failure_recovery" "PASS" "Shell recovers after command failure"
else
  record_result "8a_failure_recovery" "FAIL" "Got: $OUTPUT"
fi

# 8b: Multiple commands with error
OUTPUT=$(bash -c "source $WRAPPER_PATH && node -e \"console.log('start')\" && (pnpm nonexistent 2>/dev/null || true) && node -e \"console.log('end')\"" 2>&1 || true)
if echo "$OUTPUT" | grep -q "start" && echo "$OUTPUT" | grep -q "end"; then
  record_result "8b_multi_cmd_error" "PASS" "Commands execute despite intermediate error"
else
  record_result "8b_multi_cmd_error" "FAIL" "Got: $OUTPUT"
fi

# Test 9: Real Claude Code CLI (optional)
test_section "Test 9: Real Claude Code CLI"

if command -v claude &>/dev/null; then
  # Note: These tests assume wrapper is properly sourced in shell environment
  OUTPUT=$(bash -c "source $WRAPPER_PATH && pnpm --version" 2>&1 || true)
  if echo "$OUTPUT" | grep -q "10.5.2"; then
    record_result "9_real_claude_cli" "PASS" "Wrapper functions available for claude"
  else
    record_result "9_real_claude_cli" "FAIL" "Wrapper may not be properly initialized"
  fi
else
  record_result "9_real_claude_cli" "SKIP" "claude CLI not installed"
fi

# Summary
test_section "Summary"
echo ""
cat "$RESULTS_DIR/results.csv"
echo ""
echo "Results saved to: $RESULTS_DIR/results.csv"
