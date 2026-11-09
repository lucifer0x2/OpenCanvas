#!/bin/bash
# launchd must signal this wrapper, which translates TERM to the child's graceful signal.
supervise() {
  local graceful=$1 child result=0
  shift
  "$@" &
  child=$!
  trap 'kill -'"$graceful"' "$child" 2>/dev/null || true' TERM INT
  while kill -0 "$child" 2>/dev/null; do
    wait "$child" && result=0 || result=$?
  done
  trap - TERM INT
  return "$result"
}
