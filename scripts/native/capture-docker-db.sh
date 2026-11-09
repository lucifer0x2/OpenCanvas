#!/bin/bash
# The fingerprint and custom dump share one exported PostgreSQL snapshot.
set -euo pipefail
umask 077
container=${1:?usage: capture-docker-db.sh PG_CONTAINER OUTPUT FINGERPRINT_SQL}
output=${2:?missing output directory}
sql=${3:?missing fingerprint SQL}
[[ ! -e "$output" ]] || { echo 'Capture output already exists' >&2; exit 1; }
mkdir -p "$output"
work=$(mktemp -d "$output/.capture.XXXXXX")
keeper=
cleanup() {
  if [[ -n "$keeper" ]]; then
    exec 3>&-
    kill "$keeper" 2>/dev/null || true
    wait "$keeper" 2>/dev/null || true
  fi
  rm -rf "$work"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
docker=(docker --context orbstack exec)
psql=(psql -X -q -U opencanvas -d opencanvas -v ON_ERROR_STOP=1)
mkfifo "$work/stdin"
"${docker[@]}" -i "$container" "${psql[@]}" -At < "$work/stdin" > "$work/snapshot" &
keeper=$!
exec 3> "$work/stdin"
printf 'BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY;\nSELECT pg_export_snapshot()\n\\g | cat\n' >&3
snapshot=
for ((attempt=0; attempt<100; attempt++)); do
  if IFS= read -r snapshot < "$work/snapshot"; then break; fi
  kill -0 "$keeper"
  sleep 0.1
done
[[ "$snapshot" =~ ^[0-9A-F]+-[0-9A-F]+-[0-9]+$ ]]
"${docker[@]}" -i "$container" "${psql[@]}" -v "snapshot=$snapshot" -f - < "$sql" > "$output/fingerprint.tsv"
[[ $(awk -F '\t' '$1 == "large_objects" {print $2}' "$output/fingerprint.tsv") == 0 ]]
"${docker[@]}" "$container" pg_dump -U opencanvas -d opencanvas --format=custom \
  --lock-wait-timeout=5s --snapshot="$snapshot" > "$output/database.dump"
"${docker[@]}" -i "$container" "${psql[@]}" -v sequence_only=1 -f - < "$sql" > "$work/sequences-after.tsv"
awk -F '\t' '$1 == "sequence"' "$output/fingerprint.tsv" > "$work/sequences-before.tsv"
[[ -s "$work/sequences-before.tsv" ]]
cmp "$work/sequences-before.tsv" "$work/sequences-after.tsv"
printf 'ROLLBACK;\n\\q\n' >&3
exec 3>&-
wait "$keeper"
keeper=
/opt/homebrew/opt/postgresql@16/bin/pg_restore --list "$output/database.dump" > "$output/database.toc"
/opt/homebrew/opt/postgresql@16/bin/pg_restore --file=/dev/null "$output/database.dump"
(cd "$output" && shasum -a 256 database.dump > database.sha256)
echo 'Consistent Docker database capture complete.'
