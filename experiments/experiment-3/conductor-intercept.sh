#!/bin/bash
# conductor-intercept.sh — Transparent command interception wrapper (v2)
#
# This wrapper allows commands to be transparently routed through docker exec
# without the calling process detecting any difference in behavior.
#
# Validates the core architectural invariant: Claude Code must never know
# it is executing inside a container.
#
# Usage: source this script in your shell to enable interception
#   source conductor-intercept.sh
#   pnpm dev          # Routes to container
#   git status        # Runs on host
#
# Configuration:
#   CONDUCTOR_CONTAINER  - Docker container name to route to
#   CONDUCTOR_WORKDIR    - Working directory inside container
#   FORWARD_VARS         - Environment variables to forward

CONDUCTOR_CONTAINER="spike-intercept"
CONDUCTOR_WORKDIR="/workspace"

# Commands that should route to the container
CONTAINER_COMMANDS="pnpm npm node npx curl wget python python3 pip"

# Environment variables to forward to the container
FORWARD_VARS="NODE_ENV PATH HOME USER"

_conductor_exec() {
  local cmd="$1"
  shift

  # Build docker exec command array
  local docker_args=(-i -w "$CONDUCTOR_WORKDIR")

  # Forward selected environment variables
  for var in $FORWARD_VARS; do
    if [ -n "${!var}" ]; then
      docker_args+=(-e "$var=${!var}")
    fi
  done

  docker_args+=("$CONDUCTOR_CONTAINER" "$cmd")

  # Execute docker exec with all remaining args
  docker exec "${docker_args[@]}" "$@"
  return $?
}

# Wrapper functions for each command
pnpm() { _conductor_exec pnpm "$@"; }
npm() { _conductor_exec npm "$@"; }
node() { _conductor_exec node "$@"; }
npx() { _conductor_exec npx "$@"; }
curl() { _conductor_exec curl "$@"; }
wget() { _conductor_exec wget "$@"; }
python() { _conductor_exec python "$@"; }
python3() { _conductor_exec python3 "$@"; }
pip() { _conductor_exec pip "$@"; }

echo "Conductor interception layer loaded."
echo "Intercepting: $CONTAINER_COMMANDS"
echo "Forwarding env vars: $FORWARD_VARS"
echo "All other commands run on host."
