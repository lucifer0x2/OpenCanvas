#!/bin/bash
# Deploy/rollback acceptance on a disposable root: fresh PG16 cluster, free ports and
# process-compose started directly (not through launchd). One built release directory is
# reused under several commit names; a "broken" copy's server exits at start.
# Usage: deploy-integration.sh RELEASE_DIR   (a directory produced by build-release.sh)
set -euo pipefail
umask 077
HERE=$(cd "$(dirname "$0")" && pwd -P)
NATIVE=$(cd "$HERE/.." && pwd -P)
TEST_RELEASE=$(cd "${1:?usage: deploy-integration.sh RELEASE_DIR}" && pwd -P)
# shellcheck source=scripts/native/tests/native-fixture.sh
source "$HERE/native-fixture.sh"
# shellcheck source=scripts/native/deploy-common.sh
source "$NATIVE/deploy-common.sh"
trap fixture_cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM HUP
PC=${OPENCANVAS_PROCESS_COMPOSE:-$(command -v process-compose || true)}
[[ -x "$PC" ]] || fail "process-compose is required (brew install process-compose)"
export OPENCANVAS_PROCESS_COMPOSE=$PC OPENCANVAS_READY_TIMEOUT=${OPENCANVAS_READY_TIMEOUT:-60}
# Every release is installed in advance; any build attempt fails at once.
export OPENCANVAS_BUILD_RELEASE=/usr/bin/false
repo=$(cd "$NATIVE/../.." && pwd -P)
commit_a=$(head -n 1 "$TEST_RELEASE/REVISION")
commit_b=$(git -C "$repo" rev-parse "$commit_a~1")
commit_c=$(git -C "$repo" rev-parse "$commit_a~2")
legacy="$commit_a+legacy-overlay"

fixture_create
root_init
fixture_release "$legacy" "$commit_a"
# Like the live overlay releases: no schema versions in PROVENANCE (resolved through git).
grep -v '_schema_version=' "$TEST_RELEASE/PROVENANCE" > "$RELEASES/$legacy/PROVENANCE"
fixture_release "$commit_a" "$commit_a"
fixture_release "$commit_b" "$commit_b"
fixture_release "$commit_c" "$commit_c" broken
install -m 700 "$NATIVE/run-compose.sh" "$RUNTIME/scripts/run-compose.sh"
ln -s "releases/$legacy" "$RUNTIME/current"
history_append migrate - "$legacy" ok "test fixture"
"$NATIVE/compose-config.sh" >/dev/null
fixture_spawn "$RUNTIME/scripts/run-compose.sh" "$ROOT" "$PC" >> "$ROOT/logs/compose.stdout.log" 2>&1
compose_pid=${FIXTURE_PIDS[0]}
wait_until 120 http_up || fail "server did not start under process-compose"
[[ -S "$PC_SOCKET" ]] || fail "process-compose API socket missing"
! lsof -nP -a -p "$compose_pid" -iTCP -sTCP:LISTEN >/dev/null 2>&1 || fail "process-compose listens on TCP"
# process-compose marks readiness at its next probe period, shortly after HTTP answers.
pc_ready() { [[ "$(pc_state server | cut -d ' ' -f 1-2)" == "Running Ready" ]]; }
wait_until 30 pc_ready || fail "process-compose reports server: $(pc_state server)"
fixture_seed "$legacy"
pass "installed and started under process-compose (UDS API only, no TCP listener)"

current() { current_release; }
last() { tail -n 1 "$HISTORY" | cut -f "$1"; }
server_pid() { pc_state server | awk '{ print $3 }'; }
snapshot() { { current; cat "$HISTORY"; ls -a "$RUNTIME" "$RELEASES"; server_pid; } | shasum -a 256; }

"$NATIVE/deploy.sh" "$commit_a" --yes > "$FIXTURE_TMP/deploy-a.log" 2>&1 || { cat "$FIXTURE_TMP/deploy-a.log"; fail "deploy A"; }
[[ "$(current)" == "$commit_a" && "$(last 5)" == ok ]] || fail "deploy A not recorded"
pass "deploy release A: backup, switch, server-only restart, verify"
"$NATIVE/deploy.sh" "$commit_a" --yes | grep -q 'nothing to do' || fail "same SHA was not a no-op"
pass "deploying the current SHA is a no-op"

"$NATIVE/deploy.sh" "$commit_b" --yes > "$FIXTURE_TMP/deploy-b.log" 2>&1 || { cat "$FIXTURE_TMP/deploy-b.log"; fail "deploy B"; }
[[ "$(current)" == "$commit_b" && "$(previous_release)" == "$commit_a" ]] || fail "deploy B state"
pass "deploy release B"

"$NATIVE/rollback.sh" --yes > "$FIXTURE_TMP/rollback.log" 2>&1 || { cat "$FIXTURE_TMP/rollback.log"; fail "rollback"; }
[[ "$(current)" == "$commit_a" && "$(last 2)" == rollback ]] || fail "rollback did not return to A"
pass "rollback to the previous release A"

pg_before=$(pc_state postgres | awk '{ print $3 }')
expect_fail "$NATIVE/deploy.sh" "$commit_c" --yes
[[ "$(current)" == "$commit_a" ]] || fail "broken deploy left $(current) current"
[[ "$(last 5)" == failed && "$(last 6)" == "restored $commit_a" ]] || fail "broken deploy not recorded"
http_up || fail "A is not serving after the failed deploy"
"$RUNTIME/current/bin/node" "$NATIVE/verify-runtime.mjs" "http://127.0.0.1:$LOCAL_PORT" "$FIXTURE_TMP/verify-a" >/dev/null ||
  fail "A does not verify after the failed deploy"
[[ "$(pc_state postgres | awk '{ print $3 }')" == "$pg_before" ]] || fail "PostgreSQL was restarted"
pass "broken release B' fails non-zero; A serves and verifies; PostgreSQL untouched"

"$NATIVE/deploy.sh" "$commit_b" --yes > "$FIXTURE_TMP/deploy-concurrent.log" 2>&1 &
first=$!
wait_until 30 test -d "$LOCK" || fail "first deploy never took the lock"
expect_fail "$NATIVE/deploy.sh" "$commit_c" --yes
grep -q 'Another deploy or rollback is running' "$FIXTURE_TMP/expected-failure.log" || fail "second deploy not refused by the lock"
expect_fail "$NATIVE/rollback.sh" "$legacy" --yes
grep -q 'Another deploy or rollback is running' "$FIXTURE_TMP/expected-failure.log" || fail "rollback not refused by the lock"
wait "$first" || { cat "$FIXTURE_TMP/deploy-concurrent.log"; fail "first deploy failed"; }
[[ "$(current)" == "$commit_b" && ! -e "$LOCK" ]] || fail "lock test end state"
pass "a concurrent deploy and rollback are refused by the lock"

before=$(snapshot)
"$NATIVE/deploy.sh" "$commit_a" --dry-run --keep 2 > "$FIXTURE_TMP/dry-deploy.log"
"$NATIVE/rollback.sh" --dry-run > "$FIXTURE_TMP/dry-rollback.log"
[[ "$(snapshot)" == "$before" ]] || fail "--dry-run changed the runtime"
grep -q 'Dry run' "$FIXTURE_TMP/dry-deploy.log" || fail "deploy dry run printed no plan"
pass "--dry-run of deploy and rollback changes nothing"

load_native_conf
backup_psql --dbname=opencanvas -c 'UPDATE opencanvas_runtime_v1.schema_registry SET version=2 WHERE singleton'
expect_fail "$NATIVE/rollback.sh" "$commit_a" --yes
grep -q 'database schema is newer' "$FIXTURE_TMP/expected-failure.log" || fail "schema guard message missing"
[[ "$(current)" == "$commit_b" ]] || fail "schema guard switched anyway"
"$NATIVE/rollback.sh" "$commit_a" --dry-run --force > "$FIXTURE_TMP/forced.log" 2>&1 || fail "--force did not pass the guard"
backup_psql --dbname=opencanvas -c 'UPDATE opencanvas_runtime_v1.schema_registry SET version=1 WHERE singleton'
"$NATIVE/rollback.sh" "$legacy" --dry-run > "$FIXTURE_TMP/legacy.log" || fail "legacy release schema not resolved through git"
pass "schema guard refuses an incompatible rollback (registry v2 > supported v1)"

"$NATIVE/deploy.sh" "$commit_a" --yes --keep 2 > "$FIXTURE_TMP/keep.log" 2>&1 || { cat "$FIXTURE_TMP/keep.log"; fail "deploy --keep"; }
[[ -d "$RELEASES/$commit_a" && -d "$RELEASES/$commit_b" ]] || fail "--keep removed current or previous"
[[ ! -e "$RELEASES/$legacy" && ! -e "$RELEASES/$commit_c" ]] || fail "--keep 2 kept old releases"
pass "--keep 2 prunes old releases, keeps current and previous"

"$NATIVE/status.sh" > "$FIXTURE_TMP/status.log" || { cat "$FIXTURE_TMP/status.log"; fail "status.sh unhealthy"; }
grep -q "current:  $commit_a" "$FIXTURE_TMP/status.log" || fail "status.sh does not show the current release"
pass "status.sh reports a healthy runtime"

stops=$(grep -c 'Stopping' "$ROOT/logs/server.log" || true)
kill -TERM "$compose_pid"
wait "$compose_pid" || true
(($(grep -c 'Stopping' "$ROOT/logs/server.log") > stops)) || fail "server did not stop gracefully"
grep -q 'database system is shut down' "$ROOT/logs/postgres.log" || fail "PostgreSQL did not shut down cleanly"
[[ ! -e "$ROOT/postgres/postmaster.pid" ]] || fail "PostgreSQL still running"
pass "SIGTERM to process-compose stops the server gracefully, then PostgreSQL"
printf 'PASS: deploy-integration\n'
