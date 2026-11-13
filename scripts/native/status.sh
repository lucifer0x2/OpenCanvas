#!/bin/bash
# Read-only status: releases, process-compose processes, health, schema, backups, history.
set -euo pipefail
umask 077
HERE=$(cd "$(dirname "$0")" && pwd -P)
# shellcheck source=scripts/native/deploy-common.sh
source "$HERE/deploy-common.sh"

if [[ "${1:-}" == --help || "${1:-}" == -h ]]; then
  cat <<'EOF'
Usage: status.sh [--history N]

Shows the current and previous release, process-compose process states, HTTP health of the
local and public listeners, database schema versus the current release, the newest backup
and the last N (default 5) deploy-history lines. Changes nothing. Exits 1 when the server
is not ready or the local listener does not answer.
Root: OPENCANVAS_ROOT, default ~/Library/Application Support/OpenCanvas.
EOF
  exit 0
fi
lines=5
if [[ "${1:-}" == --history ]]; then lines=${2:?--history needs a number}; shift 2; fi
[[ $# == 0 && "$lines" =~ ^[0-9]+$ ]] || die "Usage: status.sh [--history N] (see --help)"
root_init
healthy=true

describe() {
  local name=$1
  if [[ -z "$name" ]]; then printf 'none\n'; return; fi
  printf '%s (built %s, schema v%s)\n' "$name" "$(release_meta "$name" built_utc || printf 'legacy release')" \
    "$(release_meta "$name" database_schema_version || printf '?')"
}

say "OpenCanvas runtime: $ROOT"
current=$(current_release)
note "current:  $(describe "$current")"
note "previous: $(describe "$(previous_release)")"
[[ -n "$current" ]] || { note "runtime/current is missing: not migrated to process-compose"; healthy=false; }

say "process-compose (socket $PC_SOCKET)"
if [[ -x "$PC_BIN" ]] && listing=$(pc process list --output json 2>/dev/null); then
  jq -r '(if type == "object" then .data else . end)[]
    | "    \(.name): \(.status), \(.is_ready), pid \(.pid), restarts \(.restarts)"' <<< "$listing"
  read -r status ready _ <<< "$(pc_state server)"
  [[ "${status:-}" == Running && "${ready:-}" == Ready ]] || healthy=false
else
  note "not reachable"
  healthy=false
fi

say "Health"
local_port=$(port_value OPENCANVAS_PORT "$SERVER_ENV")
public_port=$(port_value OPENCANVAS_PUBLIC_PORT "$SERVER_ENV")
if http_ok "$local_port"; then note "local  127.0.0.1:$local_port  ok"; else note "local  127.0.0.1:$local_port  not answering"; healthy=false; fi
code=$(curl -s --max-time 5 -o /dev/null -w '%{http_code}' "http://127.0.0.1:$public_port/" 2>/dev/null || true)
note "public 127.0.0.1:$public_port  HTTP ${code:-000} (000 = not listening; host allowlist may answer 4xx)"

say "Database"
if (load_native_conf && db_schema_read && printf '    registry v%s, newest project schema v%s\n' "$DB_SCHEMA" "$DB_SCENE_MAX") 2>/dev/null; then :
else note "not reachable"; fi
newest=$(find "$ROOT/backups" -maxdepth 2 -name COMPLETE -path '*/backup-*' 2>/dev/null | LC_ALL=C sort | tail -n 1)
note "newest backup: ${newest:+$(basename "$(dirname "$newest")")}"

say "LaunchAgent org.opencanvas.compose"
if "${OPENCANVAS_LAUNCHCTL:-launchctl}" print "gui/$(id -u)/org.opencanvas.compose" >/dev/null 2>&1; then note "loaded"; else note "not loaded"; fi

say "Deploy history (last $lines)"
if [[ -f "$HISTORY" ]]; then tail -n "$lines" "$HISTORY" | sed 's/^/    /'; else note "no history yet"; fi
[[ "$healthy" == true ]]
