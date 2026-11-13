#!/bin/bash
# migrate-to-process-compose.sh and --revert on a disposable root that mimics the live layout:
# an overlay-style release named in server.env, the two old LaunchAgents (run by a launchctl
# test double, never launchd), private config and its own PG16 cluster on free ports.
# Usage: migrate-integration.sh RELEASE_DIR   (a directory produced by build-release.sh)
set -euo pipefail
umask 077
HERE=$(cd "$(dirname "$0")" && pwd -P)
NATIVE=$(cd "$HERE/.." && pwd -P)
TEST_RELEASE=$(cd "${1:?usage: migrate-integration.sh RELEASE_DIR}" && pwd -P)
# shellcheck source=scripts/native/tests/native-fixture.sh
source "$HERE/native-fixture.sh"
# shellcheck source=scripts/native/deploy-common.sh
source "$NATIVE/deploy-common.sh"
PC=${OPENCANVAS_PROCESS_COMPOSE:-$(command -v process-compose || true)}
[[ -x "$PC" ]] || fail "process-compose is required (brew install process-compose)"
export OPENCANVAS_PROCESS_COMPOSE=$PC OPENCANVAS_READY_TIMEOUT=${OPENCANVAS_READY_TIMEOUT:-90}
export OPENCANVAS_LAUNCHCTL="$HERE/fake-launchctl.sh"
launchd_cleanup() {
  local label
  for label in org.opencanvas.compose org.opencanvas.server org.opencanvas.postgres; do
    "$OPENCANVAS_LAUNCHCTL" bootout "gui/$(id -u)/$label" >/dev/null 2>&1 || true
  done
  fixture_cleanup
}
trap launchd_cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM HUP

commit=$(head -n 1 "$TEST_RELEASE/REVISION")
legacy="$commit+native-overlay.test"
fixture_create
root_init
export OPENCANVAS_LAUNCH_AGENTS=$AGENTS_DIR FAKE_LAUNCHD_STATE="$FIXTURE_TMP/launchd"
fixture_release "$legacy" "$commit"
grep -v '_schema_version=' "$TEST_RELEASE/PROVENANCE" > "$RELEASES/$legacy/PROVENANCE"
fixture_release "$commit" "$commit"
set_env_line "$SERVER_ENV" OPENCANVAS_RELEASE "$RELEASES/$legacy"
# Older wrapper copies must come back byte for byte after --revert.
for script in "$RUNTIME/scripts/"*.sh; do printf '# installed copy before process-compose\n' >> "$script"; done
for service in postgres server; do
  config=$NATIVE_CONF
  [[ "$service" == postgres ]] || config=$SERVER_ENV
  plutil -create xml1 "$AGENTS_DIR/org.opencanvas.$service.plist"
  plutil -insert Label -string "org.opencanvas.$service" "$AGENTS_DIR/org.opencanvas.$service.plist"
  plutil -insert ProgramArguments -array "$AGENTS_DIR/org.opencanvas.$service.plist"
  for argument in /bin/bash "$RUNTIME/scripts/run-$service.sh" "$config"; do
    plutil -insert ProgramArguments -string "$argument" -append "$AGENTS_DIR/org.opencanvas.$service.plist"
  done
  for key in RunAtLoad KeepAlive; do plutil -insert "$key" -bool true "$AGENTS_DIR/org.opencanvas.$service.plist"; done
  plutil -insert ExitTimeOut -integer 20 "$AGENTS_DIR/org.opencanvas.$service.plist"
  plutil -insert StandardOutPath -string "$ROOT/logs/$service.stdout.log" "$AGENTS_DIR/org.opencanvas.$service.plist"
  plutil -insert StandardErrorPath -string "$ROOT/logs/$service.stderr.log" "$AGENTS_DIR/org.opencanvas.$service.plist"
done
launch() { "$OPENCANVAS_LAUNCHCTL" "$@"; }
loaded() { launch print "gui/$(id -u)/$1" >/dev/null 2>&1; }
launch bootstrap "gui/$(id -u)" "$AGENTS_DIR/org.opencanvas.postgres.plist"
wait_until 60 pg_ready || fail "old postgres agent did not start"
launch bootstrap "gui/$(id -u)" "$AGENTS_DIR/org.opencanvas.server.plist"
wait_until 90 http_up || fail "old server agent did not start"
fixture_seed "$legacy"
pass "simulated live layout: two agents serving overlay release $legacy"

fingerprint() {
  { shasum -a 256 "$SERVER_ENV" "$AGENTS_DIR"/org.opencanvas.{postgres,server}.plist "$RUNTIME/scripts/"*.sh
    stat -f '%Lp %m %N' "$SERVER_ENV" "$AGENTS_DIR"/org.opencanvas.{postgres,server}.plist; } 2>&1
}
original=$(fingerprint)

"$NATIVE/migrate-to-process-compose.sh" --dry-run > "$FIXTURE_TMP/dry.log"
[[ "$(fingerprint)" == "$original" && ! -e "$RUNTIME/current" ]] || fail "--dry-run changed files"
pass "--dry-run changes nothing"

OPENCANVAS_PROCESS_COMPOSE=/usr/bin/false expect_fail "$NATIVE/migrate-to-process-compose.sh" --yes
[[ "$(fingerprint)" == "$original" && ! -e "$RUNTIME/current" && ! -e "$AGENTS_DIR/org.opencanvas.compose.plist" ]] ||
  fail "failed cutover did not restore files"
if ! { loaded org.opencanvas.server && loaded org.opencanvas.postgres && http_up; }; then
  fail "failed cutover did not restart old agents"
fi
pass "a cutover whose process-compose start fails reverts automatically"

"$NATIVE/migrate-to-process-compose.sh" --yes > "$FIXTURE_TMP/migrate.log" 2>&1 || { cat "$FIXTURE_TMP/migrate.log"; fail "migrate"; }
if ! { loaded org.opencanvas.compose && ! loaded org.opencanvas.server && ! loaded org.opencanvas.postgres; }; then
  fail "agent states after cutover"
fi
[[ ! -e "$AGENTS_DIR/org.opencanvas.server.plist" && ! -e "$AGENTS_DIR/org.opencanvas.postgres.plist" ]] || fail "old plists remain"
plutil -lint "$AGENTS_DIR/org.opencanvas.compose.plist" >/dev/null
[[ "$(current_release)" == "$legacy" ]] || fail "runtime/current is not the live release"
# shellcheck disable=SC2016 # Expanded by the inner shell.
release_env=$(env -i /bin/bash -c 'set -a; source "$1"; printf "%s" "$OPENCANVAS_RELEASE"' _ "$SERVER_ENV")
[[ "$release_env" == "$RUNTIME/current" ]] || fail "server.env does not point at runtime/current"
[[ "$(pc_state server | cut -d ' ' -f 1-2)" == "Running Ready" ]] || fail "server not supervised by process-compose"
pass "cutover: compose agent serves the live release through runtime/current"

export OPENCANVAS_BUILD_RELEASE=/usr/bin/false
"$NATIVE/deploy.sh" "$commit" --yes > "$FIXTURE_TMP/deploy.log" 2>&1 || { cat "$FIXTURE_TMP/deploy.log"; fail "deploy after cutover"; }
pass "deploy after cutover switches to $commit"

"$NATIVE/migrate-to-process-compose.sh" --revert --yes > "$FIXTURE_TMP/revert.log" 2>&1 || { cat "$FIXTURE_TMP/revert.log"; fail "revert"; }
[[ "$(fingerprint)" == "$original" ]] || fail "revert did not restore server.env, plists and scripts exactly"
[[ ! -e "$RUNTIME/current" && ! -e "$AGENTS_DIR/org.opencanvas.compose.plist" && ! -e "$ROOT/process-compose.yaml" ]] ||
  fail "revert left process-compose files"
if loaded org.opencanvas.compose || ! { loaded org.opencanvas.postgres && loaded org.opencanvas.server && http_up; }; then
  fail "agent states after revert"
fi
pass "--revert restores both agents, config and wrappers exactly and serves $legacy"
printf 'PASS: migrate-integration\n'
