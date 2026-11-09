#!/bin/bash
# Restore only into a freshly created scratch database; never overwrite one.
set -euo pipefail
umask 077
HERE=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=scripts/native/backup-common.sh
source "$HERE/backup-common.sh"
[[ $# -ge 2 && $# -le 3 ]] || backup_die "Usage: restore-verify.sh CONFIG BACKUP_SET [--record-fingerprint]"
backup_load_config "$1"
backup_set=$2
mode=${3:-verify}
[[ "$mode" == verify || "$mode" == --record-fingerprint ]] || backup_die "Unknown restore mode"
[[ -d "$backup_set" && ! -L "$backup_set" && -f "$backup_set/database.dump" && ! -L "$backup_set/database.dump" ]] ||
  backup_die "Backup set must contain a regular database.dump"

if [[ "$mode" == verify ]]; then
  for file in database.dump fingerprint.tsv metadata.txt SHA256SUMS COMPLETE; do
    [[ -f "$backup_set/$file" && ! -L "$backup_set/$file" ]] || backup_die "Incomplete backup set"
  done
  [[ $(cat "$backup_set/COMPLETE") == opencanvas-native-backup-v1 ]] || backup_die "Unsupported backup set"
  # Only these fixed relative names may be read by the checksum tool.
  awk 'BEGIN { valid=1 } !/^[0-9a-f]+  (database.dump|fingerprint.tsv|metadata.txt)$/ { valid=0 }
    { if (length($1)!=64 || seen[$2]++) valid=0 }
    END { exit !(valid && NR==3 && seen["database.dump"] && seen["fingerprint.tsv"] && seen["metadata.txt"]) }' \
    "$backup_set/SHA256SUMS" || backup_die "Invalid checksum manifest"
  (cd "$backup_set" && backup_sha --check SHA256SUMS >/dev/null)
else
  [[ ! -e "$backup_set/COMPLETE" && ! -e "$backup_set/fingerprint.tsv" ]] ||
    backup_die "Fingerprint recording is only allowed for an unpublished staging set"
fi

# Neither a caller-selected destination nor the live database is accepted.
scratch="opencanvas_verify_$(date -u +%Y%m%d%H%M%S)_$$_${RANDOM}"
[[ "$scratch" =~ ^opencanvas_verify_[0-9_]+$ && "$scratch" != "$PGDATABASE" ]] ||
  backup_die "Unsafe scratch database name"
owned=false
creating=false
pending_signal=0
fingerprint=$(mktemp "${TMPDIR:-/tmp}/opencanvas-fingerprint.XXXXXXXX")
cleanup() {
  local status=$?
  trap - EXIT HUP INT TERM
  rm -f "$fingerprint"
  if [[ "$owned" == true ]]; then
    if ! backup_admin dropdb --if-exists --maintenance-db="$PGADMIN_DATABASE" "$scratch"; then
      printf 'OpenCanvas backup: scratch cleanup failed for %s\n' "$scratch" >&2
      status=1
    fi
  fi
  exit "$status"
}
trap cleanup EXIT
handle_signal() {
  # Bash runs traps after a foreground createdb returns. Record its result and
  # ownership before exiting, so a successful CREATE cannot escape cleanup.
  if [[ "$creating" == true ]]; then pending_signal=$1; else exit "$1"; fi
}
trap 'handle_signal 130' INT
trap 'handle_signal 143' HUP TERM
exists=$(backup_psql --dbname="$PGADMIN_DATABASE" --set=scratch="$scratch" \
  --command="SELECT 1 FROM pg_database WHERE datname='$scratch'")
[[ -z "$exists" ]] || backup_die "Scratch destination already exists"
# createdb is itself atomic; a collision never sets the cleanup ownership flag.
creating=true
if backup_admin createdb --maintenance-db="$PGADMIN_DATABASE" --template=template0 --owner="$PGUSER" "$scratch"; then
  owned=true
else
  status=$?
  creating=false
  exit "$status"
fi
creating=false
[[ "$pending_signal" == 0 ]] || exit "$pending_signal"
backup_admin pg_restore --dbname="$scratch" --role="$PGUSER" --single-transaction \
  --exit-on-error --no-owner --no-acl "$backup_set/database.dump"
"$HERE/backup-fingerprint.sh" "$BACKUP_CONFIG" "$scratch" > "$fingerprint"
if [[ "$mode" == --record-fingerprint ]]; then
  mv "$fingerprint" "$backup_set/fingerprint.tsv"
else
  cmp -s "$fingerprint" "$backup_set/fingerprint.tsv" || backup_die "Restored schema/data fingerprint mismatch"
fi
printf 'OpenCanvas backup: fresh scratch restore and content verification passed\n'
