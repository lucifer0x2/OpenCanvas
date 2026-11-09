#!/bin/bash
# Whole-database custom archive, verified before atomic publication/retention.
set -euo pipefail
umask 077
HERE=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=scripts/native/backup-common.sh
source "$HERE/backup-common.sh"
[[ $# -ge 1 && $# -le 2 ]] || backup_die "Usage: backup.sh CONFIG [--if-due]"
backup_load_config "$1"
[[ ${2:-} == '' || ${2:-} == --if-due ]] || backup_die "Unknown backup option"
[[ ! -L "$BACKUP_DIR" ]] || backup_die "Backup destination cannot be a symlink"
mkdir -p "$BACKUP_DIR"
chmod 700 "$BACKUP_DIR"
lock="$BACKUP_DIR/.backup.lock"
if ! mkdir "$lock" 2>/dev/null; then backup_die "Another backup is active, or a stale lock needs inspection"; fi
staging=''
cleanup() {
  local status=$?
  trap - EXIT HUP INT TERM
  [[ -z "$staging" ]] || rm -rf "$staging"
  rm -f "$lock/pid"
  rmdir "$lock"
  exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
printf '%s\n' "$$" > "$lock/pid"

# RunAtLoad catches missed schedules. One successful backup per local calendar day.
if [[ ${2:-} == --if-due ]]; then
  for complete in "$BACKUP_DIR"/backup-*/COMPLETE; do
    if [[ -f "$complete" && ! -L "$complete" ]] &&
      [[ $(cat "$complete") == opencanvas-native-backup-v1 ]] &&
      awk -v today="$(date +%Y-%m-%d)" '$0=="completed_local_date=" today { found=1 } END { exit !found }' \
        "${complete%/COMPLETE}/metadata.txt"; then
      printf 'OpenCanvas backup: already completed today\n'
      exit 0
    fi
  done
fi

staging=$(mktemp -d "$BACKUP_DIR/.pending.XXXXXXXX")
backup_app pg_dump --dbname="$PGDATABASE" --format=custom --no-owner --no-acl \
  --file="$staging/database.dump"
"$HERE/restore-verify.sh" "$BACKUP_CONFIG" "$staging" --record-fingerprint
{
  printf 'format=opencanvas-native-backup-v1\ncompleted_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'completed_local_date=%s\n' "$(date +%Y-%m-%d)"
  printf 'database=%s\nrole=%s\nport=%s\n' "$PGDATABASE" "$PGUSER" "$PGPORT"
  "$PG_BIN/pg_dump" --version
} > "$staging/metadata.txt"
(cd "$staging" && backup_sha database.dump fingerprint.tsv metadata.txt > SHA256SUMS)
printf 'opencanvas-native-backup-v1\n' > "$staging/COMPLETE"
destination="$BACKUP_DIR/backup-$(date -u +%Y%m%dT%H%M%SZ)-${staging##*.}"
[[ ! -e "$destination" ]] || backup_die "Backup destination collision"
mv "$staging" "$destination"
staging=''

# Retention runs only after successful restore verification and publication.
completed=()
for directory in "$BACKUP_DIR"/backup-*; do
  if [[ -d "$directory" && ! -L "$directory" && -f "$directory/COMPLETE" && ! -L "$directory/COMPLETE" ]] &&
    [[ $(cat "$directory/COMPLETE") == opencanvas-native-backup-v1 ]]; then completed+=("$directory"); fi
done
if [[ ${#completed[@]} -gt "$BACKUP_KEEP" ]]; then
  count=0
  while IFS= read -r directory; do
    count=$((count + 1))
    if [[ "$count" -le $((${#completed[@]} - BACKUP_KEEP)) ]]; then rm -rf "$directory"; fi
  done < <(printf '%s\n' "${completed[@]}" | LC_ALL=C sort)
fi
printf 'OpenCanvas backup: published one verified backup; retaining %s successful sets\n' "$BACKUP_KEEP"
