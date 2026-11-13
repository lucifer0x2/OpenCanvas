#!/bin/bash
# Internal step shared by deploy.sh and rollback.sh, which hold the deploy lock: switch
# runtime/current, restart only the server through process-compose, wait for readiness and
# verify. On any failure switch back, restart, verify, record the failure and exit non-zero.
set -euo pipefail
umask 077
HERE=$(cd "$(dirname "$0")" && pwd -P)
# shellcheck source=scripts/native/deploy-common.sh
source "$HERE/deploy-common.sh"

if [[ "${1:-}" == --help || "${1:-}" == -h ]]; then
  printf 'Usage: switch-release.sh deploy|rollback RELEASE\nInternal; run deploy.sh or rollback.sh instead.\n'
  exit 0
fi
[[ $# == 2 && ( "$1" == deploy || "$1" == rollback ) ]] || die "Usage: switch-release.sh deploy|rollback RELEASE"
action=$1
target=$2
root_init
[[ "$(cat "$LOCK/pid" 2>/dev/null || true)" == "$PPID" ]] ||
  die "switch-release.sh runs only from deploy.sh or rollback.sh, which hold the deploy lock"
# Ctrl-C must not leave a half-switched runtime: it takes effect after this step finishes
# or switches back (the ignored disposition also reaches the verify and client processes).
trap '' INT
local_port=$(port_value OPENCANVAS_PORT "$SERVER_ENV")
from=$(current_release)
[[ -n "$from" ]] || die "runtime/current is missing; run migrate-to-process-compose.sh first"
[[ -n "$(pc_state server)" ]] || die "process-compose is not reachable at $PC_SOCKET; check status.sh"

activate() {
  local release=$1 old_pid
  old_pid=$(pc_state server | awk '{ print $3 }')
  switch_current "$release"
  say "runtime/current -> releases/$release; restarting only the server"
  if ! pc process restart server >/dev/null 2>&1; then
    pc process start server >/dev/null 2>&1 || { warn "process-compose could not restart the server"; return 1; }
  fi
  wait_server_ready "${old_pid:-0}" "$local_port" || return 1
  say "Server ready; verifying (snapshots, MCP, PNG render, scene WASM)"
  verify_release "$local_port"
}

if activate "$target"; then
  history_append "$action" "$from" "$target" ok
  say "$action complete: $from -> $target"
  exit 0
fi
warn "$action to $target failed; switching back to $from"
if activate "$from"; then
  history_append "$action" "$from" "$target" failed "restored $from"
  die "$action to $target failed; $from is serving again and verified (see $ROOT/logs/server.log)"
fi
history_append "$action" "$from" "$target" failed "restore of $from also failed"
printf 'error: %s to %s failed and restoring %s also failed; run status.sh and inspect %s\n' \
  "$action" "$target" "$from" "$ROOT/logs/server.log" >&2
exit 2
