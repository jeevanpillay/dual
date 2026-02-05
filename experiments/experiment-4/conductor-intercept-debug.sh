#!/bin/sh
# Debug version of conductor-intercept-posix.sh
# Logs all invocations to understand how Claude Code calls commands

CONDUCTOR_CONTAINER="spike-intercept"
CONDUCTOR_WORKDIR="/workspace"
DEBUG_LOG="/tmp/conductor-debug.log"

# Initialize log
{
  echo "=== Conductor Debug Log ==="
  echo "Started: $(date)"
  echo "Shell: $SHELL"
  echo "Parent PID: $$"
  echo ""
} >> "$DEBUG_LOG"

_conductor_exec() {
  local cmd="$1"
  shift

  # Log the invocation
  {
    echo "[$(date '+%H:%M:%S')] Called: _conductor_exec"
    echo "  Command: $cmd"
    echo "  Arguments: $*"
    echo "  Argument count: $#"
    echo "  NODE_ENV: ${NODE_ENV:-<unset>}"
    echo "  Parent process: $(ps -o comm= -p $PPID 2>/dev/null || echo 'unknown')"
    echo "  SHELL: $SHELL"
    echo "  Invocation context:"
    ps -ef | grep $$ | grep -v grep || true
    echo ""
  } >> "$DEBUG_LOG"

  # Execute (same as production)
  if [ -n "$NODE_ENV" ]; then
    docker exec -i -w "$CONDUCTOR_WORKDIR" -e "NODE_ENV=$NODE_ENV" "$CONDUCTOR_CONTAINER" "$cmd" "$@"
  else
    docker exec -i -w "$CONDUCTOR_WORKDIR" "$CONDUCTOR_CONTAINER" "$cmd" "$@"
  fi
  return $?
}

# Override commands with logging
pnpm() {
  {
    echo "[$(date '+%H:%M:%S')] Function called: pnpm"
    echo "  Arguments: $*"
  } >> "$DEBUG_LOG"
  _conductor_exec pnpm "$@"
}

npm() {
  {
    echo "[$(date '+%H:%M:%S')] Function called: npm"
    echo "  Arguments: $*"
  } >> "$DEBUG_LOG"
  _conductor_exec npm "$@"
}

node() {
  {
    echo "[$(date '+%H:%M:%S')] Function called: node"
    echo "  Arguments: $*"
  } >> "$DEBUG_LOG"
  _conductor_exec node "$@"
}

npx() {
  {
    echo "[$(date '+%H:%M:%S')] Function called: npx"
    echo "  Arguments: $*"
  } >> "$DEBUG_LOG"
  _conductor_exec npx "$@"
}

curl() {
  {
    echo "[$(date '+%H:%M:%S')] Function called: curl"
    echo "  Arguments: $*"
  } >> "$DEBUG_LOG"
  _conductor_exec curl "$@"
}

wget() {
  {
    echo "[$(date '+%H:%M:%S')] Function called: wget"
    echo "  Arguments: $*"
  } >> "$DEBUG_LOG"
  _conductor_exec wget "$@"
}

python() {
  {
    echo "[$(date '+%H:%M:%S')] Function called: python"
    echo "  Arguments: $*"
  } >> "$DEBUG_LOG"
  _conductor_exec python "$@"
}

python3() {
  {
    echo "[$(date '+%H:%M:%S')] Function called: python3"
    echo "  Arguments: $*"
  } >> "$DEBUG_LOG"
  _conductor_exec python3 "$@"
}

pip() {
  {
    echo "[$(date '+%H:%M:%S')] Function called: pip"
    echo "  Arguments: $*"
  } >> "$DEBUG_LOG"
  _conductor_exec pip "$@"
}

# Log wrapper sourcing
{
  echo "[$(date '+%H:%M:%S')] Wrapper sourced"
  echo "  Functions available: pnpm npm node npx curl wget python python3 pip"
  echo ""
} >> "$DEBUG_LOG"
