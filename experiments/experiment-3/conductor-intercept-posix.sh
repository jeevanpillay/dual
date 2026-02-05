#!/bin/sh
# Transparent command interception wrapper (POSIX-compatible).
# Routes pnpm, npm, node, npx, curl, wget, python to container via docker exec.
# Works in bash, zsh, dash, sh. See DUAL-30 for details.

CONDUCTOR_CONTAINER="spike-intercept"
CONDUCTOR_WORKDIR="/workspace"

_conductor_exec() {
  local cmd="$1"
  shift
  if [ -n "$NODE_ENV" ]; then
    docker exec -i -w "$CONDUCTOR_WORKDIR" -e "NODE_ENV=$NODE_ENV" "$CONDUCTOR_CONTAINER" "$cmd" "$@"
  else
    docker exec -i -w "$CONDUCTOR_WORKDIR" "$CONDUCTOR_CONTAINER" "$cmd" "$@"
  fi
  return $?
}

pnpm() { _conductor_exec pnpm "$@"; }
npm() { _conductor_exec npm "$@"; }
node() { _conductor_exec node "$@"; }
npx() { _conductor_exec npx "$@"; }
curl() { _conductor_exec curl "$@"; }
wget() { _conductor_exec wget "$@"; }
python() { _conductor_exec python "$@"; }
python3() { _conductor_exec python3 "$@"; }
pip() { _conductor_exec pip "$@"; }
