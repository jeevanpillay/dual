#!/bin/bash
# Architecture Loop - Stop Hook
#
# This hook implements the Ralph pattern for /architecture_loop:
# - Only activates when loop is explicitly started
# - Intercepts Claude's exit attempt
# - Checks if architecture design is complete
# - Re-injects the command if not complete
#
# Loop control:
#   Start loop: touch .claude/.architecture-loop-active
#   Stop loop:  rm .claude/.architecture-loop-active (or output ARCHITECTURE_COMPLETE)
#
# JSON Output:
#   { "decision": "block", "reason": "..." } = Continue looping
#   exit 0 with no output = Allow exit

# Read hook input from stdin
INPUT=$(cat)

# Get project directory
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-.}"
LOOP_FLAG="$PROJECT_DIR/.claude/.architecture-loop-active"
ARCH_FILE="$PROJECT_DIR/thoughts/ARCHITECTURE.md"

# Check if loop is active
if [[ ! -f "$LOOP_FLAG" ]]; then
    # Loop not active - allow normal exit
    exit 0
fi

# Get the transcript path to check Claude's output
TRANSCRIPT_PATH=$(echo "$INPUT" | jq -r '.transcript_path // empty')

# Check for completion signal in the transcript
if [[ -n "$TRANSCRIPT_PATH" && -f "$TRANSCRIPT_PATH" ]]; then
    if tail -100 "$TRANSCRIPT_PATH" | grep -q "ARCHITECTURE_COMPLETE"; then
        # Architecture complete - deactivate loop and allow exit
        rm -f "$LOOP_FLAG"
        echo '{"systemMessage": "Architecture loop complete. Loop deactivated."}'
        exit 0
    fi
fi

# Check for explicit stop request (safety valve)
if [[ -n "$TRANSCRIPT_PATH" && -f "$TRANSCRIPT_PATH" ]]; then
    if tail -50 "$TRANSCRIPT_PATH" | grep -q "STOP_LOOP"; then
        rm -f "$LOOP_FLAG"
        echo '{"systemMessage": "Loop stopped by user request."}'
        exit 0
    fi
fi

# Check iteration count to prevent runaway loops
MAX_ITERATIONS="${ARCHITECTURE_LOOP_MAX_ITERATIONS:-20}"

if [[ -f "$ARCH_FILE" ]]; then
    ITERATION_COUNT=$(grep -c "^- \[" "$ARCH_FILE" 2>/dev/null || echo "0")

    if [[ "$ITERATION_COUNT" -ge "$MAX_ITERATIONS" ]]; then
        rm -f "$LOOP_FLAG"
        echo '{"systemMessage": "Max iterations ('"$MAX_ITERATIONS"') reached. Loop deactivated."}'
        exit 0
    fi
fi

# Continue the loop - block exit and instruct to continue
cat << 'EOF'
{
  "decision": "block",
  "reason": "Architecture loop is active. Continue with /architecture_loop to process the next open question in thoughts/ARCHITECTURE.md. Output ARCHITECTURE_COMPLETE when all questions are answered, or STOP_LOOP to exit early."
}
EOF
