#!/bin/bash
# Runs one built release directly (no process-compose, no launchd) on a disposable root with
# a fresh PG16 cluster and free ports, seeds one project and runs verify-runtime.mjs.
# Usage: release-smoke.sh RELEASE_DIR   (a directory produced by build-release.sh)
set -euo pipefail
umask 077
HERE=$(cd "$(dirname "$0")" && pwd -P)
TEST_RELEASE=${1:?usage: release-smoke.sh RELEASE_DIR}
TEST_RELEASE=$(cd "$TEST_RELEASE" && pwd -P)
# shellcheck source=scripts/native/tests/native-fixture.sh
source "$HERE/native-fixture.sh"
trap fixture_cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM HUP

commit=$(head -n 1 "$TEST_RELEASE/REVISION")
fixture_create
fixture_release "$commit" "$commit"
ln -s "releases/$commit" "$ROOT_DIR/runtime/current"
pass "disposable root, PG16 cluster on $PG_PORT, server ports $LOCAL_PORT/$PUBLIC_PORT"

fixture_spawn "$ROOT_DIR/runtime/scripts/run-postgres.sh" "$ROOT_DIR/config/native.conf" \
  >> "$ROOT_DIR/logs/postgres.log" 2>&1
wait_until 60 pg_ready || fail "PostgreSQL did not start"
fixture_spawn "$ROOT_DIR/runtime/scripts/run-server.sh" "$ROOT_DIR/config/server.env" \
  >> "$ROOT_DIR/logs/server.log" 2>&1
wait_until 90 http_up || { tail -n 20 "$ROOT_DIR/logs/server.log" >&2; fail "server did not answer"; }
pass "release $commit serves on loopback $LOCAL_PORT"

fixture_seed "$commit"
"$ROOT_DIR/runtime/current/bin/node" "$HERE/../verify-runtime.mjs" "http://127.0.0.1:$LOCAL_PORT" \
  "$FIXTURE_TMP/verify" | tee "$FIXTURE_TMP/verify.log"
grep -q '"ok":true' "$FIXTURE_TMP/verify.log" || fail "verify-runtime did not report ok"
pass "verify-runtime.mjs: snapshots, MCP, PNG render and scene WASM"

kill -TERM "${FIXTURE_PIDS[1]}"
wait "${FIXTURE_PIDS[1]}" || true
grep -q 'Stopping' "$ROOT_DIR/logs/server.log" || fail "server did not stop gracefully"
pass "wrapper SIGTERM reached the server as a graceful SIGINT"
printf 'PASS: release-smoke %s\n' "$commit"
