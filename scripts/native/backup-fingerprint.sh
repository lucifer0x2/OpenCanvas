#!/bin/bash
# Canonical archive content inventory. It never changes database contents.
set -euo pipefail
umask 077
HERE=$(cd "$(dirname "$0")" && pwd)
# shellcheck source=scripts/native/backup-common.sh
source "$HERE/backup-common.sh"
[[ $# == 2 ]] || backup_die "Usage: backup-fingerprint.sh CONFIG DATABASE"
backup_load_config "$1"
database=$2
export PGOPTIONS='-c timezone=UTC -c datestyle=ISO,YMD -c extra_float_digits=3 -c bytea_output=hex'

# Schema-only SQL is an independent description of all dumpable objects.
# Remove pg_dump comments/blank lines, which contain server/tool versions.
schema_hash=$(backup_admin pg_dump --dbname="$database" --schema-only --no-owner \
  --no-acl --quote-all-identifiers --no-comments | sed '/^--/d; /^$/d; /^\\restrict /d; /^\\unrestrict /d' |
  shasum -a 256 | awk '{print $1}')
printf 'format\topencanvas-pg-fingerprint-v1\nschema\t%s\n' "$schema_hash"

tables=$(backup_psql --dbname="$database" --field-separator=$'\t' --command="
  SELECT c.oid, encode(convert_to(n.nspname,'UTF8'),'hex'),
         encode(convert_to(c.relname,'UTF8'),'hex')
  FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
  WHERE c.relkind IN ('r','m') AND n.nspname NOT IN ('pg_catalog','information_schema')
    AND n.nspname NOT LIKE 'pg_toast%'
  ORDER BY n.nspname COLLATE \"C\", c.relname COLLATE \"C\"")
if [[ -n "$tables" ]]; then
  while IFS=$'\t' read -r oid schema table; do
    [[ "$oid" =~ ^[0-9]+$ ]] || backup_die "Invalid catalog object"
    count_sql=$(backup_psql --dbname="$database" --command="SELECT format('SELECT count(*) FROM %s', $oid::regclass)")
    count=$(backup_psql --dbname="$database" --command="$count_sql")
    copy_sql=$(backup_psql --dbname="$database" --command="SELECT format(
      'COPY (SELECT row_to_json(row_value)::text FROM %s AS row_value ORDER BY row_to_json(row_value)::text COLLATE \"C\") TO STDOUT', $oid::regclass)")
    digest=$(backup_psql --dbname="$database" --command="$copy_sql" | shasum -a 256 | awk '{print $1}')
    printf 'table\t%s\t%s\t%s\t%s\n' "$schema" "$table" "$count" "$digest"
  done <<< "$tables"
fi

# Include sequence state: identity generation must continue after recovery.
sequences=$(backup_psql --dbname="$database" --field-separator=$'\t' --command="
  SELECT c.oid, encode(convert_to(n.nspname,'UTF8'),'hex'),
         encode(convert_to(c.relname,'UTF8'),'hex')
  FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
  WHERE c.relkind='S' AND n.nspname NOT IN ('pg_catalog','information_schema')
  ORDER BY n.nspname COLLATE \"C\", c.relname COLLATE \"C\"")
if [[ -n "$sequences" ]]; then
  while IFS=$'\t' read -r oid schema sequence; do
    sql=$(backup_psql --dbname="$database" --command="SELECT format('SELECT last_value, is_called FROM %s', $oid::regclass)")
    state=$(backup_psql --dbname="$database" --command="$sql")
    printf 'sequence\t%s\t%s\t%s\n' "$schema" "$sequence" "$state"
  done <<< "$sequences"
fi

# pg_dump includes PostgreSQL large objects as well as application BYTEA rows.
large_count=$(backup_psql --dbname="$database" --command='SELECT count(*) FROM pg_largeobject_metadata')
large_hash=$(backup_psql --dbname="$database" --command="COPY (
  SELECT loid, pageno, encode(data,'hex') FROM pg_largeobject ORDER BY loid,pageno
  ) TO STDOUT" | shasum -a 256 | awk '{print $1}')
printf 'large_objects\t%s\t%s\n' "$large_count" "$large_hash"
