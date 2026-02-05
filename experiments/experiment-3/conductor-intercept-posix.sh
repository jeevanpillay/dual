#!/bin/sh
# conductor-intercept-posix.sh — POSIX-compatible transparent command interception
#
# Works in: bash, zsh, dash, sh
# Uses only POSIX shell features - no arrays, no +=

CONDUCTOR_CONTAINER="spike-intercept"
CONDUCTOR_WORKDIR="/workspace"

# Commands that should route to the container
CONTAINER_COMMANDS="pnpm npm node npx curl wget python python3 pip"

_conductor_exec() {
  local cmd="$1"
  shift

  # Build docker exec with NODE_ENV forwarding if set
  if [ -n "$NODE_ENV" ]; then
    docker exec -i -w "$CONDUCTOR_WORKDIR" -e "NODE_ENV=$NODE_ENV" "$CONDUCTOR_CONTAINER" "$cmd" "$@"
  else
    docker exec -i -w "$CONDUCTOR_WORKDIR" "$CONDUCTOR_CONTAINER" "$cmd" "$@"
  fi
  return $?
}

# Override each command
pnpm() { _conductor_exec pnpm "$@"; }
npm() { _conductor_exec npm "$@"; }
node() { _conductor_exec node "$@"; }
npx() { _conductor_exec npx "$@"; }
curl() { _conductor_exec curl "$@"; }
wget() { _conductor_exec wget "$@"; }
python() { _conductor_exec python "$@"; }
python3() { _conductor_exec python3 "$@"; }
pip() { _conductor_exec pip "$@"; }

echo "Conductor interception layer loaded (POSIX-compatible)."
echo "Intercepting: $CONTAINER_COMMANDS"
echo "Forwarding: NODE_ENV (if set)"
echo "Works in: bash, zsh, dash, sh"
