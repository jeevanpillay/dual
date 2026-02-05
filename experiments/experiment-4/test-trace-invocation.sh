#!/bin/bash
# Trace how Claude Code actually invokes commands
# Uses shell tracing, strace, and custom logging to understand the call pattern

set -e

WRAPPER_DEBUG="experiments/experiment-4/conductor-intercept-debug.sh"
TRACE_DIR="/tmp/claude-invocation-trace"
mkdir -p "$TRACE_DIR"

echo "=== Tracing Claude Code Command Invocation ==="
echo "Output directory: $TRACE_DIR"
echo ""

# Test 1: Direct invocation in current shell
echo "Test 1: Direct invocation (interactive shell)"
echo "=================================="
rm -f /tmp/conductor-debug.log
bash -c "source $WRAPPER_DEBUG && pnpm --version" 2>/dev/null
echo ""
echo "Debug log:"
cat /tmp/conductor-debug.log
cp /tmp/conductor-debug.log "$TRACE_DIR/test-1-direct.log"
echo ""

# Test 2: With shell tracing enabled
echo "Test 2: Shell trace (bash -x)"
echo "=================================="
{
  bash -x -c "source $WRAPPER_DEBUG 2>/dev/null && pnpm --version" 2>&1
} | tee "$TRACE_DIR/test-2-trace.log"
echo ""

# Test 3: How Claude Code actually invokes (subprocess without tty)
echo "Test 3: Non-TTY subprocess invocation (likely Claude pattern)"
echo "=================================="
rm -f /tmp/conductor-debug.log
bash -c "source $WRAPPER_DEBUG && pnpm --version" </dev/null 2>/dev/null
echo ""
echo "Debug log:"
cat /tmp/conductor-debug.log
cp /tmp/conductor-debug.log "$TRACE_DIR/test-3-nontty.log"
echo ""

# Test 4: Test with strace (if available)
if command -v strace &>/dev/null; then
  echo "Test 4: System call trace (strace)"
  echo "=================================="
  {
    strace -e execve,clone,fork,vfork bash -c "source $WRAPPER_DEBUG 2>/dev/null && pnpm --version" 2>&1 | head -100
  } | tee "$TRACE_DIR/test-4-strace.log"
  echo ""
else
  echo "Test 4: strace - SKIPPED (not installed)"
  echo ""
fi

# Test 5: Multiple commands in sequence
echo "Test 5: Multiple commands (understanding state/inheritance)"
echo "=================================="
rm -f /tmp/conductor-debug.log
bash -c "
  source experiments/experiment-4/conductor-intercept-debug.sh 2>/dev/null
  pnpm --version
  node --version
  pnpm --version
" </dev/null 2>/dev/null
echo ""
echo "Debug log:"
cat /tmp/conductor-debug.log
cp /tmp/conductor-debug.log "$TRACE_DIR/test-5-multiple.log"
echo ""

# Test 6: Environment inspection
echo "Test 6: Environment at invocation time"
echo "=================================="
bash -c "
  source experiments/experiment-4/conductor-intercept-debug.sh 2>/dev/null
  echo 'Environment when wrapper sourced:'
  echo \"  \$SHELL: \$SHELL\"
  echo \"  \$0: \$0\"
  echo \"  \$BASH_VERSION: \$BASH_VERSION\"
  echo \"  \$ZSH_VERSION: \$ZSH_VERSION\"
  echo \"  \$-: \$-\"
  echo ''
  pnpm --version
" </dev/null 2>/dev/null | tee "$TRACE_DIR/test-6-env.log"
echo ""

# Test 7: Real Claude Code invocation (if claude available)
echo "Test 7: Real Claude Code CLI invocation"
echo "=================================="
if command -v claude &>/dev/null; then
  rm -f /tmp/conductor-debug.log
  export CONDUCTOR_DEBUG_WRAPPER="$WRAPPER_DEBUG"

  # Try to invoke through Claude
  # Note: This depends on how Claude Code sources the wrapper
  bash -c "source $WRAPPER_DEBUG && pnpm --version" 2>/dev/null || true

  echo "Debug log:"
  cat /tmp/conductor-debug.log || echo "No debug log captured"
  cp /tmp/conductor-debug.log "$TRACE_DIR/test-7-claude.log" 2>/dev/null || true
else
  echo "SKIPPED - claude CLI not available"
fi
echo ""

# Test 8: Compare invocation methods
echo "Test 8: Comparison - Different invocation methods"
echo "=================================="
echo "Method A: bash -c with stdin"
rm -f /tmp/conductor-debug.log
bash -c "source $WRAPPER_DEBUG && pnpm --version" </dev/null 2>/dev/null
echo "  Parent process (from log): $(grep 'Parent process' /tmp/conductor-debug.log | tail -1)"

echo ""
echo "Method B: bash -c without redirect"
rm -f /tmp/conductor-debug.log
bash -c "source $WRAPPER_DEBUG && pnpm --version" 2>/dev/null
echo "  Parent process (from log): $(grep 'Parent process' /tmp/conductor-debug.log | tail -1)"

echo ""
echo "Method C: bash -i (interactive)"
rm -f /tmp/conductor-debug.log
bash -i -c "source $WRAPPER_DEBUG && pnpm --version" </dev/null 2>/dev/null || true
echo "  Parent process (from log): $(grep 'Parent process' /tmp/conductor-debug.log | tail -1 || echo 'N/A')"

echo ""

# Summary
echo "=== Trace Analysis Summary ==="
echo ""
echo "Log files created:"
ls -lh "$TRACE_DIR"
echo ""
echo "To analyze further:"
echo "  cat $TRACE_DIR/test-2-trace.log        # See shell trace"
echo "  cat $TRACE_DIR/test-3-nontty.log       # See non-TTY invocation"
echo "  cat $TRACE_DIR/test-5-multiple.log     # See state/inheritance"
echo "  cat $TRACE_DIR/test-6-env.log          # See environment"
