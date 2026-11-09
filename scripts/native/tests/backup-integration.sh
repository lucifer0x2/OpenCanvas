#!/bin/bash
# Real PG16 integration only: isolated disposable cluster, no real data root.
set -euo pipefail
umask 077
HERE=$(cd "$(dirname "$0")/.." && pwd)
PG_BIN=${PG_BIN:-/opt/homebrew/opt/postgresql@16/bin}
test_root=$(mktemp -d '/private/tmp/opencanvas-backup-test.XXXXXXXX')
test_root="$test_root/Native Backup With Spaces"
mkdir -p "$test_root/run" "$test_root/config"
cluster_started=false
cleanup() {
  local status=$?
  trap - EXIT HUP INT TERM
  if [[ "$cluster_started" == true ]]; then "$PG_BIN/pg_ctl" -D "$test_root/postgres" -m immediate -w stop >/dev/null; fi
  rm -rf "${test_root%/*}"
  exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
expect_fail() { if "$@" > "$test_root/expected-failure.log" 2>&1; then fail 'command unexpectedly succeeded'; fi; }
"$PG_BIN/initdb" -D "$test_root/postgres" --auth=trust --encoding=UTF8 --locale=C >/dev/null
if ! "$PG_BIN/pg_ctl" -D "$test_root/postgres" -l "$test_root/postgres.log" \
  -o "-k '$test_root/run' -p 27039 -c listen_addresses=''" -w start >/dev/null; then
  cat "$test_root/postgres.log" >&2
  exit 1
fi
cluster_started=true
export PGHOST="$test_root/run" PGPORT=27039 PGUSER
PGUSER=$(id -un)
export PGPASSFILE="$test_root/config/pgpass"
: > "$PGPASSFILE"
"$PG_BIN/psql" -X -v ON_ERROR_STOP=1 -d postgres -q -c 'CREATE ROLE opencanvas LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE'
"$PG_BIN/createdb" --owner=opencanvas --template=template0 opencanvas
"$PG_BIN/psql" -X -v ON_ERROR_STOP=1 -d opencanvas -q <<'SQL'
SET ROLE opencanvas;
CREATE SCHEMA opencanvas_runtime_v1;
CREATE TABLE public.schema_registry (singleton boolean PRIMARY KEY, version integer, namespace text);
INSERT INTO public.schema_registry VALUES (true,1,'fixed-test-namespace');
SET search_path=opencanvas_runtime_v1;
CREATE TABLE projects (id text PRIMARY KEY, generation bigint GENERATED ALWAYS AS IDENTITY, revision bigint);
CREATE TABLE entities (id text, payload text);
CREATE TABLE binaries (id text, bytes bytea);
CREATE TABLE commits (revision bigint, operations text);
CREATE TABLE idempotency (key text, response text);
CREATE TABLE viewer_credentials (hash bytea);
CREATE TABLE viewer_sessions (token bytea, ip text);
INSERT INTO projects (id,revision) VALUES ('project1',42);
INSERT INTO entities VALUES ('node1','{"escaped":"\u0000","text":"日本語"}');
INSERT INTO binaries VALUES ('font1',decode('000102ff','hex'));
INSERT INTO commits VALUES (42,'[{"type":"set"}]');
INSERT INTO idempotency VALUES ('key1','{"revision":42}');
INSERT INTO viewer_credentials VALUES (decode('abcdef','hex'));
INSERT INTO viewer_sessions VALUES (decode('aa00bb','hex'),'127.0.0.1');
SELECT lo_from_bytea(812345, decode('ff000102','hex'));
SQL
config="$test_root/config/native.conf"
{
  printf 'OPENCANVAS_ROOT=%q\nPG_BIN=%q\n' "$test_root" "$PG_BIN"
  printf 'PGHOST=%q\nPGPORT=27039\nPGDATABASE=opencanvas\nPGUSER=opencanvas\n' "$PGHOST"
  printf 'PGPASSFILE=%q\nPGADMIN_HOST=%q\nPGADMIN_USER=%q\n' "$PGPASSFILE" "$PGHOST" "$(id -un)"
} > "$config"
"$HERE/backup.sh" "$config" > "$test_root/backup.log"
sets=("$test_root/backups"/backup-*)
[[ ${#sets[@]} == 1 ]] || fail 'first backup not published'
set_path=${sets[0]}
"$HERE/restore-verify.sh" "$config" "$set_path" > "$test_root/verify.log"
"$HERE/backup-fingerprint.sh" "$config" opencanvas > "$test_root/source-fingerprint.tsv"
cmp -s "$test_root/source-fingerprint.tsv" "$set_path/fingerprint.tsv" || fail 'quiescent source and archive differ'
[[ $(awk '$1=="table" {n++} END {print n}' "$set_path/fingerprint.tsv") == 8 ]] || fail 'missing table fingerprints'
[[ $(awk '$1=="large_objects" {print $2}' "$set_path/fingerprint.tsv") == 1 ]] || fail 'large objects not inventoried'
[[ $("$PG_BIN/psql" -XAt -d postgres -c "SELECT count(*) FROM pg_database WHERE datname LIKE 'opencanvas_verify_%'") == 0 ]] || fail 'scratch DB leaked'
for file in "$set_path"/*; do
  mode=$(stat -f '%Lp' "$file")
  [[ $((8#$mode & 077)) == 0 ]] || fail 'backup file readable by another user'
done

# Local calendar due policy and exclusive lock both skip work safely.
"$HERE/backup.sh" "$config" --if-due > "$test_root/due.log"
[[ $(find "$test_root/backups" -name COMPLETE | wc -l | tr -d ' ') == 1 ]] || fail 'due check created another backup'
mkdir "$test_root/backups/.backup.lock"
expect_fail "$HERE/backup.sh" "$config"
rmdir "$test_root/backups/.backup.lock"
expect_fail "$HERE/restore-verify.sh" "$config" "$set_path" --database=opencanvas

# TERM after CREATE succeeds must defer exit until cleanup owns the scratch DB.
mkdir "$test_root/signal-tools"
for tool in psql pg_dump pg_restore dropdb; do ln -s "$PG_BIN/$tool" "$test_root/signal-tools/$tool"; done
cat > "$test_root/signal-tools/createdb" <<'SCRIPT'
#!/bin/bash
set -euo pipefail
"$OPENCANVAS_TEST_REAL_CREATEDB" "$@"
kill -TERM "$PPID"
sleep 0.2
SCRIPT
chmod 700 "$test_root/signal-tools/createdb"
export OPENCANVAS_TEST_REAL_CREATEDB="$PG_BIN/createdb"
cp "$config" "$test_root/config/signal.conf"
printf 'PG_BIN=%q\n' "$test_root/signal-tools" >> "$test_root/config/signal.conf"
expect_fail "$HERE/restore-verify.sh" "$test_root/config/signal.conf" "$set_path"
[[ $("$PG_BIN/psql" -XAt -d postgres -c "SELECT count(*) FROM pg_database WHERE datname LIKE 'opencanvas_verify_%'") == 0 ]] || fail 'TERM after createdb leaked scratch DB'

# A simulated creation collision belongs to another invocation: never drop it.
cat > "$test_root/signal-tools/createdb" <<'SCRIPT'
#!/bin/bash
set -euo pipefail
"$OPENCANVAS_TEST_REAL_CREATEDB" "$@"
printf '%s\n' "${!#}" > "$OPENCANVAS_TEST_COLLISION_NAME"
"$OPENCANVAS_TEST_REAL_CREATEDB" "$@"
SCRIPT
export OPENCANVAS_TEST_COLLISION_NAME="$test_root/collision-name"
expect_fail "$HERE/restore-verify.sh" "$test_root/config/signal.conf" "$set_path"
collision=$(cat "$test_root/collision-name")
[[ $("$PG_BIN/psql" -XAt -d postgres -c "SELECT count(*) FROM pg_database WHERE datname='$collision'") == 1 ]] || fail 'creation collision was dropped'
"$PG_BIN/dropdb" "$collision"

# Corruption fails checksum validation. A rehashed wrong inventory fails after restore.
cp -R "$set_path" "$test_root/corrupt"
printf 'corrupt' >> "$test_root/corrupt/database.dump"
expect_fail "$HERE/restore-verify.sh" "$config" "$test_root/corrupt"
cp -R "$set_path" "$test_root/wrong-inventory"
printf 'unexpected\trow\n' >> "$test_root/wrong-inventory/fingerprint.tsv"
(cd "$test_root/wrong-inventory" && shasum -a 256 database.dump fingerprint.tsv metadata.txt > SHA256SUMS)
expect_fail "$HERE/restore-verify.sh" "$config" "$test_root/wrong-inventory"

# Seed completed history, then prove a failed dump cannot prune it.
for index in $(seq 1 14); do
  old="$test_root/backups/backup-20000101T000000Z-$(printf '%02d' "$index")"
  mkdir "$old"
  printf 'opencanvas-native-backup-v1\n' > "$old/COMPLETE"
done
mkdir "$test_root/backups/backup-incomplete"
mkdir "$test_root/failing-tools"
for tool in psql pg_restore createdb dropdb; do ln -s "$PG_BIN/$tool" "$test_root/failing-tools/$tool"; done
printf '#!/bin/bash\nexit 71\n' > "$test_root/failing-tools/pg_dump"
chmod 700 "$test_root/failing-tools/pg_dump"
cp "$config" "$test_root/config/failing.conf"
printf 'PG_BIN=%q\n' "$test_root/failing-tools" >> "$test_root/config/failing.conf"
expect_fail "$HERE/backup.sh" "$test_root/config/failing.conf"
[[ $(find "$test_root/backups" -name COMPLETE | wc -l | tr -d ' ') == 15 ]] || fail 'failed dump pruned backups'
rm "$test_root/failing-tools/pg_dump" "$test_root/failing-tools/pg_restore"
ln -s "$PG_BIN/pg_dump" "$test_root/failing-tools/pg_dump"
printf '#!/bin/bash\nexit 72\n' > "$test_root/failing-tools/pg_restore"
chmod 700 "$test_root/failing-tools/pg_restore"
expect_fail "$HERE/backup.sh" "$test_root/config/failing.conf"
[[ $(find "$test_root/backups" -name COMPLETE | wc -l | tr -d ' ') == 15 ]] || fail 'failed restore pruned backups'
[[ ! -d "$test_root/backups/.backup.lock" ]] || fail 'failed dump leaked lock'
[[ $(find "$test_root/backups" -name '.pending.*' | wc -l | tr -d ' ') == 0 ]] || fail 'failed dump leaked staging'
"$HERE/backup.sh" "$config" > "$test_root/retention.log"
[[ $(find "$test_root/backups" -name COMPLETE | wc -l | tr -d ' ') == 14 ]] || fail 'retention did not keep14 successful sets'
[[ -d "$test_root/backups/backup-incomplete" ]] || fail 'retention removed incomplete set'
[[ $("$PG_BIN/psql" -XAt -d postgres -c "SELECT count(*) FROM pg_database WHERE datname LIKE 'opencanvas_verify_%'") == 0 ]] || fail 'failed verify leaked scratch DB'
"$HERE/install-backup.sh" "$config" "$test_root/LaunchAgents With Spaces" > "$test_root/install.log"
plutil -lint "$test_root/LaunchAgents With Spaces/org.opencanvas.backup.plist" >/dev/null
printf 'PASS: PG16 wholeDB/bytea/history/auth/sequence/large-object restore, hashes, failure, TERM cleanup, collision safety, retention, lock, local-day catchup and plist install\n'
