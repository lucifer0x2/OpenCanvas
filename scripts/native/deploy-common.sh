#!/bin/bash
# Shared helpers for the native release build, deploy, rollback, status and migration.
# Sourced only. Never prints configuration contents or secrets.
# shellcheck disable=SC2034 # Globals set here are read by the sourcing scripts.
NATIVE_SCRIPTS=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
NATIVE_REPO=$(cd "$NATIVE_SCRIPTS/../.." && pwd -P)
DEFAULT_ROOT="$HOME/Library/Application Support/OpenCanvas"

say() { printf '==> %s\n' "$*"; }
note() { printf '    %s\n' "$*"; }
warn() { printf 'warning: %s\n' "$*" >&2; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

# Reads one KEY=VALUE line (last wins) without sourcing the file; for plain values only.
env_value() { awk -v key="$1" 'index($0, key "=") == 1 { value = substr($0, length(key) + 2) } END { print value }' "$2"; }

port_value() {
  local value
  value=$(env_value "$1" "$2")
  if [[ ! "$value" =~ ^[1-9][0-9]{0,4}$ ]] || ((value > 65535)); then die "Missing or invalid $1 in ${2##*/}"; fi
  printf '%s\n' "$value"
}

# Replaces exactly one KEY= line with a shell-quoted value; keeps the file private.
set_env_line() {
  local file=$1 key=$2 temporary
  [[ $(awk -v key="$key" 'index($0, key "=") == 1 { n++ } END { print n + 0 }' "$file") == 1 ]] ||
    die "Expected exactly one $key line in ${file##*/}"
  temporary=$(mktemp "$(dirname "$file")/.${file##*/}.XXXXXX")
  NEW_LINE="$key=$(printf '%q' "$3")" awk -v key="$key" \
    'index($0, key "=") == 1 { print ENVIRON["NEW_LINE"]; next } { print }' "$file" > "$temporary"
  chmod 600 "$temporary"
  mv "$temporary" "$file"
}

# Supported schema versions, parsed from source text on stdin.
parse_db_schema() { sed -n 's/.*row\.get::<_, i32>(0) != \([0-9][0-9]*\).*/\1/p' | head -n 1; }
parse_scene_schema() { sed -n 's/^pub const SCHEMA_VERSION: u32 = \([0-9][0-9]*\);.*/\1/p' | head -n 1; }
DB_SCHEMA_SOURCE=crates/storage/src/postgres/schema.rs
SCENE_SCHEMA_SOURCE=crates/scene-input/src/model.rs

# Sets ROOT, RUNTIME, RELEASES, HISTORY, IS_LIVE_ROOT and private config paths.
root_init() {
  local root=${OPENCANVAS_ROOT:-$DEFAULT_ROOT} live=''
  [[ "$root" == /* ]] || die "OPENCANVAS_ROOT must be an absolute path"
  [[ -d "$root" ]] || die "Runtime root not found: $root"
  ROOT=$(cd "$root" && pwd -P)
  [[ ! -d "$DEFAULT_ROOT" ]] || live=$(cd "$DEFAULT_ROOT" && pwd -P)
  IS_LIVE_ROOT=false
  [[ "$ROOT" != "$live" ]] || IS_LIVE_ROOT=true
  RUNTIME="$ROOT/runtime"
  RELEASES="$RUNTIME/releases"
  HISTORY="$RUNTIME/deploy-history.log"
  NATIVE_CONF="$ROOT/config/native.conf"
  SERVER_ENV="$ROOT/config/server.env"
  PC_SOCKET="$ROOT/run/process-compose.sock"
  PC_BIN=${OPENCANVAS_PROCESS_COMPOSE:-$(command -v process-compose || printf /opt/homebrew/bin/process-compose)}
  LOCK="$RUNTIME/.deploy.lock"
}

# Asks before changing the live root; --yes (ASSUME_YES=true) skips the question.
confirm_live() {
  [[ "$IS_LIVE_ROOT" == true && "${ASSUME_YES:-false}" != true ]] || return 0
  [[ -t 0 ]] || die "Refusing to change the live root without --yes (no terminal to ask)"
  local answer
  printf '%s\nType yes to continue: ' "$1"
  read -r answer
  [[ "$answer" == yes ]] || die "Cancelled"
}

load_native_conf() {
  # shellcheck source=scripts/native/backup-common.sh
  source "$NATIVE_SCRIPTS/backup-common.sh"
  backup_load_config "$NATIVE_CONF"
  [[ "$(cd "$OPENCANVAS_ROOT" && pwd -P)" == "$ROOT" ]] || die "native.conf belongs to another root"
}

# Database schema versions: DB_SCHEMA (registry) and DB_SCENE_MAX (newest project schema).
db_schema_read() {
  local registered
  registered=$(backup_psql --dbname="$PGDATABASE" \
    -c "SELECT to_regclass('opencanvas_runtime_v1.schema_registry') IS NOT NULL")
  DB_SCHEMA=0 DB_SCENE_MAX=0
  [[ "$registered" == t ]] || return 0
  DB_SCHEMA=$(backup_psql --dbname="$PGDATABASE" \
    -c "SELECT version FROM opencanvas_runtime_v1.schema_registry WHERE singleton")
  DB_SCENE_MAX=$(backup_psql --dbname="$PGDATABASE" \
    -c "SELECT COALESCE(max(schema_version), 0) FROM opencanvas_runtime_v1.projects")
}

release_meta() { [[ -f "$RELEASES/$1/PROVENANCE" ]] && env_value "$2" "$RELEASES/$1/PROVENANCE"; }

# REL_DB_SCHEMA/REL_SCENE_SCHEMA: from PROVENANCE, else from the REVISION commit in this
# repository (older overlay releases), else empty (unknown).
release_schema() {
  local name=$1 revision
  REL_DB_SCHEMA=$(release_meta "$name" database_schema_version || true)
  REL_SCENE_SCHEMA=$(release_meta "$name" scene_schema_version || true)
  [[ -z "$REL_DB_SCHEMA" || -z "$REL_SCENE_SCHEMA" ]] || return 0
  revision=$(head -n 1 "$RELEASES/$name/REVISION" 2>/dev/null || true)
  [[ "$revision" =~ ^[0-9a-f]{40}$ ]] && git -C "$NATIVE_REPO" cat-file -e "$revision^{commit}" 2>/dev/null || return 0
  REL_DB_SCHEMA=$(git -C "$NATIVE_REPO" show "$revision:$DB_SCHEMA_SOURCE" 2>/dev/null | parse_db_schema)
  REL_SCENE_SCHEMA=$(git -C "$NATIVE_REPO" show "$revision:$SCENE_SCHEMA_SOURCE" 2>/dev/null | parse_scene_schema)
}

current_release() {
  local target
  [[ -L "$RUNTIME/current" ]] || return 0
  target=$(readlink "$RUNTIME/current")
  [[ "$target" == releases/* && "$target" != */*/* ]] || die "runtime/current must point to releases/<name>"
  printf '%s\n' "${target#releases/}"
}

# The release that was current before the current one: the "from" of the last successful
# switch that made the current release current.
previous_release() {
  local current
  current=$(current_release)
  [[ -n "$current" && -f "$HISTORY" ]] || return 0
  awk -F '\t' -v current="$current" '$5 == "ok" && $4 == current { from = $3 } END { if (from != "-") print from }' "$HISTORY"
}

history_append() {
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$1" "${2:--}" "${3:--}" "$4" "${5:-}" >> "$HISTORY"
}

# Atomic switch: rename(2) of a new symlink over runtime/current.
switch_current() {
  [[ -d "$RELEASES/$1" && ! -L "$RELEASES/$1" ]] || die "Release not found: $1"
  rm -f "$RUNTIME/.current.$$"
  if ! { ln -s "releases/$1" "$RUNTIME/.current.$$" && mv -fh "$RUNTIME/.current.$$" "$RUNTIME/current"; }; then
    rm -f "$RUNTIME/.current.$$"
    die "Could not switch runtime/current to $1"
  fi
}

release_valid() {
  local dir="$RELEASES/$1"
  [[ -f "$dir/REVISION" && -f "$dir/SHA256SUMS" && -x "$dir/bin/opencanvas-server" && -x "$dir/bin/node" ]] || return 1
  (cd "$dir" && shasum -a 256 --check --quiet SHA256SUMS >/dev/null 2>&1)
}

lock_acquire() {
  local holder
  if ! mkdir "$LOCK" 2>/dev/null; then
    holder=$(cat "$LOCK/pid" 2>/dev/null || true)
    if [[ "$holder" =~ ^[0-9]+$ ]] && ! kill -0 "$holder" 2>/dev/null; then
      warn "Removing stale deploy lock of exited process $holder"
      rm -f "$LOCK/pid"
      rmdir "$LOCK"
      mkdir "$LOCK"
    else
      die "Another deploy or rollback is running (pid ${holder:-unknown}); lock: $LOCK"
    fi
  fi
  printf '%s\n' "$$" > "$LOCK/pid"
  LOCK_HELD=true
}

lock_release() {
  [[ "${LOCK_HELD:-false}" == true ]] || return 0
  rm -f "$LOCK/pid"
  rmdir "$LOCK" 2>/dev/null || true
  LOCK_HELD=false
}

# process-compose client over the private Unix socket; never the default TCP :8080.
pc() {
  [[ -x "$PC_BIN" ]] || die "process-compose not found (set OPENCANVAS_PROCESS_COMPOSE)"
  PC_CLIENT_LOG_FILE="$ROOT/logs/process-compose-client.log" \
    "$PC_BIN" --use-uds --unix-socket "$PC_SOCKET" "$@"
}

# Prints "status ready pid restarts" for one process, or nothing when unreachable.
pc_state() {
  pc process list --output json 2>/dev/null | jq -r --arg name "$1" \
    '(if type == "object" then .data else . end)[] | select(.name == $name)
     | "\(.status) \(.is_ready | gsub(" "; "_")) \(.pid) \(.restarts)"' 2>/dev/null || true
}

http_ok() { curl -fsS --max-time 5 -o /dev/null "http://127.0.0.1:$1/api/v1/viewer" 2>/dev/null; }

# Waits until process-compose reports the server Running and Ready under a new pid and the
# local listener answers.
wait_server_ready() {
  local old_pid=$1 port=$2 deadline=$((SECONDS + ${OPENCANVAS_READY_TIMEOUT:-120})) state status ready pid
  while ((SECONDS < deadline)); do
    state=$(pc_state server)
    read -r status ready pid _ <<< "${state:-none - 0 0}"
    if [[ "$status" == Running && "$ready" == Ready && "$pid" != "$old_pid" && "$pid" != 0 ]] && http_ok "$port"; then
      return 0
    fi
    sleep 1
  done
  warn "Server did not become ready within ${OPENCANVAS_READY_TIMEOUT:-120}s (last state: ${state:-unreachable})"
  return 1
}

# Read-only acceptance against the local listener with the (current) release's own Node.
verify_release() {
  local port=$1 node=${2:-$RUNTIME/current}/bin/node output status=0
  output=$(mktemp -d "$ROOT/logs/.verify.XXXXXX")
  "$node" "$NATIVE_SCRIPTS/verify-runtime.mjs" "http://127.0.0.1:$port" "$output" || status=$?
  [[ ! -f "$output/result.json" ]] || cp "$output/result.json" "$ROOT/logs/verify-last.json"
  rm -rf "$output"
  return "$status"
}
