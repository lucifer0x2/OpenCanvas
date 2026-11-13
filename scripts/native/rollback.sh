#!/bin/bash
# Rolls back to an installed release through the same switch/restart/verify path as deploy.
set -euo pipefail
umask 077
HERE=$(cd "$(dirname "$0")" && pwd -P)
# shellcheck source=scripts/native/deploy-common.sh
source "$HERE/deploy-common.sh"

usage() {
  cat <<'EOF'
Usage: rollback.sh [release] [--dry-run] [--yes] [--force]

Switches runtime/current to [release] (a directory name under runtime/releases), by
default the previous successful release recorded in runtime/deploy-history.log, restarts
only the server, waits for readiness and verifies. Any failure switches back.

Refuses when the database schema is newer than the target release supports, or the
release fails its checks: restore the pre-deploy backup instead (docs/NATIVE-BACKUP.md,
"Recover into a new database").

  --dry-run  Print the plan and change nothing.
  --yes      Do not ask for confirmation on the live root.
  --force    Override the schema and release checks (unsafe).

Root: OPENCANVAS_ROOT, default ~/Library/Application Support/OpenCanvas.
EOF
}

target='' DRY_RUN=false ASSUME_YES=false FORCE=false
while (($#)); do
  case $1 in
    -h | --help) usage; exit 0 ;;
    --dry-run) DRY_RUN=true ;;
    --yes) ASSUME_YES=true ;;
    --force) FORCE=true ;;
    -*) die "Unknown option: $1 (see --help)" ;;
    *) [[ -z "$target" ]] || die "Only one release is accepted"; target=$1 ;;
  esac
  shift
done
root_init
[[ -L "$RUNTIME/current" && -f "$ROOT/process-compose.yaml" ]] ||
  die "$ROOT is not running under process-compose yet; run migrate-to-process-compose.sh first"
current=$(current_release)
if [[ -z "$target" ]]; then
  target=$(previous_release)
  [[ -n "$target" ]] || die "No previous successful release in deploy-history.log; name one from runtime/releases"
fi
[[ "$target" != */* && "$target" != .* && -d "$RELEASES/$target" ]] || die "Release not found: runtime/releases/$target"
say "Root: $ROOT"
note "current release: $current"
note "rollback target: $target"
if [[ "$target" == "$current" ]]; then
  say "Release $target is already current; nothing to do"
  exit 0
fi

refuse() {
  if [[ "$FORCE" == true ]]; then warn "--force: $1"; return 0; fi
  die "$1"
}

say "Checking the database schema against $target"
load_native_conf
db_schema_read
release_schema "$target"
note "database: registry v$DB_SCHEMA, newest project schema v$DB_SCENE_MAX"
note "release supports: registry v${REL_DB_SCHEMA:-unknown}, project schema v${REL_SCENE_SCHEMA:-unknown}"
latest_backup=$(find "$ROOT/backups" -maxdepth 2 -name COMPLETE -path '*/backup-*' 2>/dev/null | LC_ALL=C sort | tail -n 1)
restore_hint="Restore the pre-deploy backup instead (docs/NATIVE-BACKUP.md, \"Recover into a new database\"); newest set: ${latest_backup:+$(basename "$(dirname "$latest_backup")")}"
if [[ -z "$REL_DB_SCHEMA" || -z "$REL_SCENE_SCHEMA" ]]; then
  refuse "Cannot determine which schema $target supports (no PROVENANCE versions and its REVISION is not in this repository). $restore_hint. Use --force to override."
elif ((DB_SCHEMA > REL_DB_SCHEMA || DB_SCENE_MAX > REL_SCENE_SCHEMA)); then
  refuse "The database schema is newer than $target supports; switching binaries back is unsafe. $restore_hint. Use --force to override."
fi
release_valid "$target" || refuse "runtime/releases/$target fails its checks (binaries or SHA256SUMS)."

if [[ "$DRY_RUN" == true ]]; then
  say "Dry run: nothing will be changed. Plan:"
  note "1. switch runtime/current: $current -> $target"
  note "2. restart only the server through process-compose and wait for readiness"
  note "3. run verify-runtime.mjs; on failure switch back to $current, restart and verify"
  exit 0
fi
confirm_live "Roll the live OpenCanvas runtime at $ROOT back to $target?"
lock_acquire
trap lock_release EXIT
trap 'exit 130' INT
trap 'exit 143' TERM HUP
[[ "$(current_release)" == "$current" ]] || die "runtime/current changed while waiting; rerun rollback.sh"
"$HERE/switch-release.sh" rollback "$target"
say "Rolled back to $target"
