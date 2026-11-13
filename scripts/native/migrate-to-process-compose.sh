#!/bin/bash
# One-time cutover from the org.opencanvas.{postgres,server} LaunchAgents to one
# process-compose LaunchAgent, and its exact revert. Run by the user, never by an agent.
set -euo pipefail
umask 077
HERE=$(cd "$(dirname "$0")" && pwd -P)
# shellcheck source=scripts/native/deploy-common.sh
source "$HERE/deploy-common.sh"

usage() {
  cat <<'EOF'
Usage: migrate-to-process-compose.sh [--revert] [--dry-run] [--yes] [--force]

Cutover: backs up config, LaunchAgents, wrapper scripts and the database; stops the
org.opencanvas.server and org.opencanvas.postgres agents (server first); installs the
wrappers and org.opencanvas.compose; points config/server.env at runtime/current (initially
the release that is live now); starts and verifies. A failed start or verification reverts.

--revert stops org.opencanvas.compose and restores the two old agents, server.env and the
wrapper scripts byte for byte from the migration backup, then starts and verifies them.
It refuses when the database schema is newer than the original release supports (--force).

  --dry-run  Print the plan and change nothing.   --yes  Do not ask on the live root.
Root: OPENCANVAS_ROOT. Test hooks: OPENCANVAS_LAUNCH_AGENTS, OPENCANVAS_LAUNCHCTL.
EOF
}

MODE=forward DRY_RUN=false ASSUME_YES=false FORCE=false
for argument in "$@"; do
  case $argument in
    -h | --help) usage; exit 0 ;;
    --revert) MODE=revert ;;
    --dry-run) DRY_RUN=true ;;
    --yes) ASSUME_YES=true ;;
    --force) FORCE=true ;;
    *) die "Unknown argument: $argument (see --help)" ;;
  esac
done
root_init
AGENTS=${OPENCANVAS_LAUNCH_AGENTS:-$HOME/Library/LaunchAgents}
LAUNCHCTL=${OPENCANVAS_LAUNCHCTL:-launchctl}
DOMAIN="gui/$(id -u)"
RECORD="$RUNTIME/process-compose-migration"
OLD_AGENTS=(org.opencanvas.server org.opencanvas.postgres)
COMPOSE=org.opencanvas.compose
local_port=$(port_value OPENCANVAS_PORT "$SERVER_ENV")
public_port=$(port_value OPENCANVAS_PUBLIC_PORT "$SERVER_ENV")
load_native_conf

agent_loaded() { "$LAUNCHCTL" print "$DOMAIN/$1" >/dev/null 2>&1; }
port_free() { ! lsof -nP -iTCP:"$1" -sTCP:LISTEN >/dev/null 2>&1; }
wait_for() { # SECONDS DESCRIPTION COMMAND...
  local deadline=$((SECONDS + $1)) what=$2
  shift 2
  until "$@"; do ((SECONDS < deadline)) || { warn "Timed out waiting for $what"; return 1; }; sleep 1; done
}
server_stopped() { port_free "$local_port" && port_free "$public_port"; }
postgres_stopped() { [[ ! -e "$ROOT/postgres/postmaster.pid" ]] && port_free "$PGPORT"; }
postgres_ready() { "$PG_BIN/pg_isready" -q -h "$PGADMIN_HOST" -p "$PGPORT"; }
agent_stop() {
  agent_loaded "$1" || return 0
  say "Stopping $1"
  "$LAUNCHCTL" bootout "$DOMAIN/$1" || true
  wait_for 30 "$1 to unload" agent_unloaded "$1"
}
agent_unloaded() { ! agent_loaded "$1"; }
agent_start() {
  ! agent_loaded "$1" || return 0
  say "Starting $1"
  "$LAUNCHCTL" bootstrap "$DOMAIN" "$AGENTS/$1.plist"
}
# Prints only OPENCANVAS_RELEASE from a private env file, evaluated in an empty environment.
# shellcheck disable=SC2016 # Expanded by the inner shell.
release_of_env() { env -i /bin/bash -c 'set -a; source "$1"; printf "%s" "$OPENCANVAS_RELEASE"' _ "$1"; }
xml_escape() { sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g' -e 's/"/\&quot;/g' -e "s/'/\&apos;/g"; }

write_compose_plist() {
  local root_xml pc_xml temporary
  root_xml=$(printf '%s' "$ROOT" | xml_escape)
  pc_xml=$(printf '%s' "$PC_BIN" | xml_escape)
  temporary=$(mktemp "$AGENTS/.opencanvas-compose.XXXXXX") || return 1
  # awk replacement strings interpret ampersands; index/substr avoid substitution.
  if awk -v root="$root_xml" -v pc="$pc_xml" '{ line = $0
      while (p = index(line, "@@ROOT@@")) line = substr(line, 1, p - 1) root substr(line, p + 8)
      while (p = index(line, "@@PROCESS_COMPOSE@@")) line = substr(line, 1, p - 1) pc substr(line, p + 19)
      print line }' "$HERE/compose.launchd.plist.in" > "$temporary" &&
    plutil -lint "$temporary" >/dev/null && chmod 600 "$temporary"; then
    mv "$temporary" "$AGENTS/$COMPOSE.plist"
  else
    rm -f "$temporary"
    return 1
  fi
}

# Each step returns explicitly: errexit does not apply inside a function used in a condition.
# Subshells contain helpers that exit through die.
install_compose_files() {
  local label script
  for label in "${OLD_AGENTS[@]}"; do rm -f "$AGENTS/$label.plist" || return 1; done
  say "Installing wrappers, runtime/current and $COMPOSE"
  for script in runtime-supervise.sh run-postgres.sh run-server.sh run-compose.sh; do
    install -m 700 "$HERE/$script" "$RUNTIME/scripts/$script" || return 1
  done
  (switch_current "$1") || return 1
  (set_env_line "$SERVER_ENV" OPENCANVAS_RELEASE "$RUNTIME/current") || return 1
}

start_compose() {
  "$HERE/compose-config.sh" --yes || return 1
  write_compose_plist || return 1
  agent_start "$COMPOSE" || return 1
  wait_server_ready none "$local_port" || return 1
  say "Server ready under process-compose; verifying"
  verify_release "$local_port"
}

backup_files() { # Lists the files that revert restores, relative to the backup directory.
  (cd "$1" && find server.env LaunchAgents scripts -type f | LC_ALL=C sort)
}

forward() {
  [[ ! -e "$RECORD" && ! -e "$RUNTIME/current" ]] || die "Already migrated (runtime/current or $RECORD exists)"
  for label in "${OLD_AGENTS[@]}"; do [[ -f "$AGENTS/$label.plist" ]] || die "Missing $AGENTS/$label.plist"; done
  [[ ! -e "$AGENTS/$COMPOSE.plist" ]] || die "$AGENTS/$COMPOSE.plist already exists"
  [[ -x "$PC_BIN" ]] || die "process-compose not found at $PC_BIN (brew install process-compose)"
  [[ -x "$ROOT/backup-tools/backup.sh" ]] || die "backup-tools/backup.sh is not installed"
  local release name
  release=$(release_of_env "$SERVER_ENV")
  name=${release##*/}
  [[ -d "$RELEASES/$name" && "$(cd "$release" && pwd -P)" == "$RELEASES/$name" ]] ||
    die "server.env does not select a release under runtime/releases"
  [[ $(grep -c '^OPENCANVAS_RELEASE=' "$SERVER_ENV") == 1 ]] || die "server.env needs exactly one OPENCANVAS_RELEASE line"
  say "Root: $ROOT"
  note "live release: $name; local port $local_port, public port $public_port, PostgreSQL $PGPORT"
  if [[ "$DRY_RUN" == true ]]; then
    say "Dry run: nothing will be changed. Plan:"
    note "1. back up server.env, both plists and runtime/scripts; run a verified database backup"
    note "2. stop org.opencanvas.server, then org.opencanvas.postgres; move their plists into the backup"
    note "3. install wrappers, runtime/current -> releases/$name, OPENCANVAS_RELEASE=runtime/current"
    note "4. generate process-compose.yaml, install and start $COMPOSE, verify (revert on failure)"
    return 0
  fi
  confirm_live "Move the live OpenCanvas services at $ROOT to process-compose?"
  local backup
  backup="$ROOT/backups/pc-migration-$(date -u +%Y%m%dT%H%M%SZ)"
  say "Backing up configuration to $backup"
  mkdir -m 700 "$backup" "$backup/LaunchAgents"
  cp -p "$SERVER_ENV" "$backup/server.env"
  cp -Rp "$RUNTIME/scripts" "$backup/scripts"
  for label in "${OLD_AGENTS[@]}"; do cp -p "$AGENTS/$label.plist" "$backup/LaunchAgents/"; done
  (cd "$backup" && backup_files . | xargs shasum -a 256 > MANIFEST.sha256)
  say "Backing up the database"
  "$ROOT/backup-tools/backup.sh" "$NATIVE_CONF"
  # Line 1: backup directory; line 2: original release (deploy --keep never prunes it).
  printf '%s\n%s\n' "${backup##*/}" "$name" > "$RECORD"
  # From here on, an interruption or any failure restores the two agents.
  trap 'revert_now; exit 130' INT
  trap 'revert_now; exit 143' TERM HUP
  for label in "${OLD_AGENTS[@]}"; do agent_stop "$label" || { revert_now; die "Could not stop $label; reverted"; }; done
  if ! { wait_for 30 "server listeners to close" server_stopped && wait_for 60 "PostgreSQL to stop" postgres_stopped; }; then
    revert_now
    die "Old services did not stop; reverted"
  fi
  if ! { install_compose_files "$name" && start_compose; }; then
    history_append migrate - "$name" failed "reverting to the two agents"
    revert_now
    die "process-compose start or verification failed; reverted to the two agents"
  fi
  trap - INT TERM HUP
  history_append migrate - "$name" ok "org.opencanvas.compose"
  say "Migrated: $COMPOSE supervises PostgreSQL and the server on $name"
}

# A separate process keeps errexit semantics for the revert steps.
revert_now() {
  warn "Reverting to org.opencanvas.postgres and org.opencanvas.server"
  "$HERE/migrate-to-process-compose.sh" --revert --yes --force || warn "Revert failed; see docs/NATIVE-RUNTIME.md"
}

revert() {
  [[ -f "$RECORD" ]] || die "No migration record at $RECORD"
  local backup original current
  backup="$ROOT/backups/$(head -n 1 "$RECORD")"
  [[ -f "$backup/MANIFEST.sha256" && -f "$backup/server.env" ]] || die "Migration backup incomplete: $backup"
  original=$(release_of_env "$backup/server.env")
  original=${original##*/}
  [[ -d "$RELEASES/$original" ]] || die "The original release runtime/releases/$original no longer exists"
  current=$(current_release)
  say "Root: $ROOT"
  note "restoring release $original (current: ${current:-none}) from ${backup##*/}"
  if postgres_ready; then
    db_schema_read
    release_schema "$original"
    if [[ -z "$REL_DB_SCHEMA" ]] || ((DB_SCHEMA > REL_DB_SCHEMA || DB_SCENE_MAX > ${REL_SCENE_SCHEMA:-0})); then
      [[ "$FORCE" == true ]] || die "The database schema is newer than $original supports (or unknown); use --force or restore a backup"
    fi
  fi
  if [[ "$DRY_RUN" == true ]]; then
    say "Dry run: nothing will be changed. Plan:"
    note "stop $COMPOSE; restore server.env, both plists and runtime/scripts; remove runtime/current"
    note "start org.opencanvas.postgres, then org.opencanvas.server; verify"
    return 0
  fi
  confirm_live "Revert the live OpenCanvas services at $ROOT to the two LaunchAgents?"
  if agent_loaded "$COMPOSE"; then
    agent_stop "$COMPOSE" || true
    wait_for 60 "process-compose to stop the server" server_stopped
    wait_for 90 "process-compose to stop PostgreSQL" postgres_stopped
  fi
  rm -f "$AGENTS/$COMPOSE.plist" "$ROOT/process-compose.yaml" "$RUNTIME/current"
  cp -p "$backup/server.env" "$SERVER_ENV"
  rm -rf "$RUNTIME/scripts"
  cp -Rp "$backup/scripts" "$RUNTIME/scripts"
  cp -p "$backup/LaunchAgents/"*.plist "$AGENTS/"
  (cd "$backup" && while read -r hash file; do
    case $file in
      server.env) target=$SERVER_ENV ;;
      LaunchAgents/*) target=$AGENTS/${file#LaunchAgents/} ;;
      scripts/*) target=$RUNTIME/$file ;;
      *) die "Unexpected manifest entry: $file" ;;
    esac
    [[ "$(shasum -a 256 "$target" | awk '{ print $1 }')" == "$hash" ]] || die "Restored $file differs from the backup"
  done < MANIFEST.sha256)
  rm -f "$RECORD"
  agent_start org.opencanvas.postgres
  wait_for 60 "PostgreSQL" postgres_ready
  agent_start org.opencanvas.server
  wait_for 90 "the server" http_ok "$local_port"
  verify_release "$local_port" "$RELEASES/$original"
  history_append revert "${current:-}" "$original" ok "org.opencanvas.postgres+server"
  say "Reverted: the two LaunchAgents serve $original; files match the migration backup"
}

if [[ "$MODE" == forward ]]; then forward; else revert; fi
