#!/bin/bash
# Sourced by native integration tests: a disposable runtime root (with spaces) mirroring
# install-runtime.sh, its own fresh PG16 cluster and free ports. Never touches a real root.
FIXTURE_NATIVE=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
PG_BIN=${PG_BIN:-/opt/homebrew/opt/postgresql@16/bin}
FIXTURE_TMP='' FIXTURE_PIDS=()

fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf 'ok - %s\n' "$*"; }
expect_fail() { if "$@" > "$FIXTURE_TMP/expected-failure.log" 2>&1; then fail "unexpectedly succeeded: $*"; fi; }

# Distinct free loopback ports outside every port used by live or other local services.
fixture_ports() {
  local chosen=" 2703 2704 3857 3858 3867 3868 3877 8080 " name port
  for name in PG_PORT LOCAL_PORT PUBLIC_PORT; do
    while :; do
      port=$((20000 + RANDOM % 20000))
      [[ "$chosen" != *" $port "* ]] || continue
      ! lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1 || continue
      chosen="$chosen$port "
      printf -v "$name" '%s' "$port"
      break
    done
  done
}

# ROOT_DIR with config/, backups/, logs/, runtime/{releases,scripts}, an initialized cluster
# owned by this user (peer) plus the SCRAM app role, and installed backup tools.
fixture_create() {
  FIXTURE_TMP=$(mktemp -d /private/tmp/opencanvas-native-test.XXXXXXXX)
  ROOT_DIR="$FIXTURE_TMP/Application Support/OpenCanvas"
  AGENTS_DIR="$FIXTURE_TMP/LaunchAgents"
  fixture_ports
  mkdir -p "$ROOT_DIR"/{runtime/releases,runtime/scripts,postgres,run,config,backups,logs} "$AGENTS_DIR"
  chmod 700 "$ROOT_DIR" "$ROOT_DIR"/{postgres,run,config,backups,logs}
  "$PG_BIN/initdb" -D "$ROOT_DIR/postgres" --encoding=UTF8 --locale=C --auth-local=peer \
    --auth-host=scram-sha-256 > "$ROOT_DIR/logs/initdb.log"
  cat > "$ROOT_DIR/postgres/pg_hba.conf" <<EOF
local all $(id -un) peer
local all all scram-sha-256
host all all 127.0.0.1/32 scram-sha-256
host all all ::1/128 scram-sha-256
EOF
  cat >> "$ROOT_DIR/postgres/postgresql.conf" <<EOF
listen_addresses = '127.0.0.1'
port = $PG_PORT
unix_socket_directories = '$ROOT_DIR/run'
password_encryption = 'scram-sha-256'
logging_collector = off
EOF
  local password
  password=$(openssl rand -hex 32)
  {
    printf 'OPENCANVAS_ROOT=%q\nPG_BIN=%q\n' "$ROOT_DIR" "$PG_BIN"
    printf 'PGHOST=%q\nPGPORT=%s\nPGDATABASE=opencanvas\nPGUSER=opencanvas\n' "$ROOT_DIR/run" "$PG_PORT"
    printf 'PGPASSFILE=%q\nPGADMIN_USER=%q\nPGADMIN_HOST=%q\nBACKUP_KEEP=14\n' \
      "$ROOT_DIR/config/pgpass" "$(id -un)" "$ROOT_DIR/run"
  } > "$ROOT_DIR/config/native.conf"
  printf '*:%s:*:opencanvas:%s\n' "$PG_PORT" "$password" > "$ROOT_DIR/config/pgpass"
  {
    printf 'OPENCANVAS_ROOT=%q\nOPENCANVAS_RELEASE=%q\n' "$ROOT_DIR" "$ROOT_DIR/runtime/current"
    printf 'DATABASE_PROVIDER=postgresql\nDATABASE_URL=%q\n' "postgresql://opencanvas:$password@127.0.0.1:$PG_PORT/opencanvas"
    printf 'OPENCANVAS_PORT=%s\nOPENCANVAS_PUBLIC_PORT=%s\n' "$LOCAL_PORT" "$PUBLIC_PORT"
    printf 'OPENCANVAS_PUBLIC_ORIGIN=https://viewer.example.invalid\nOPENCANVAS_PUBLIC_TRUSTED_PROXY_IP=127.0.0.1\n'
  } > "$ROOT_DIR/config/server.env"
  chmod 600 "$ROOT_DIR/config/"*
  LC_ALL=C "$PG_BIN/pg_ctl" -D "$ROOT_DIR/postgres" -l "$ROOT_DIR/logs/postgres-install.log" -w start >/dev/null
  printf "CREATE ROLE opencanvas LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE PASSWORD '%s';\n" "$password" |
    "$PG_BIN/psql" -X -q -h "$ROOT_DIR/run" -p "$PG_PORT" -d postgres -v ON_ERROR_STOP=1 >/dev/null
  "$PG_BIN/createdb" -h "$ROOT_DIR/run" -p "$PG_PORT" -O opencanvas opencanvas
  "$PG_BIN/pg_ctl" -D "$ROOT_DIR/postgres" -m fast -w stop >/dev/null
  "$FIXTURE_NATIVE/install-backup.sh" "$ROOT_DIR/config/native.conf" "$AGENTS_DIR" >/dev/null
  install -m 700 "$FIXTURE_NATIVE"/{runtime-supervise,run-postgres,run-server}.sh "$ROOT_DIR/runtime/scripts/"
  export OPENCANVAS_ROOT="$ROOT_DIR"
}

# Copies (APFS clone) a built release as runtime/releases/NAME labelled with COMMIT.
# With "broken", the server binary exits at start (SHA256SUMS regenerated to match).
fixture_release() {
  local name=$1 commit=$2 dir="$ROOT_DIR/runtime/releases/$1"
  cp -cR "$TEST_RELEASE" "$dir" 2>/dev/null || cp -R "$TEST_RELEASE" "$dir"
  printf '%s\n' "$commit" > "$dir/REVISION"
  awk -v commit="$commit" '/^commit=/ { print "commit=" commit; next } { print }' "$TEST_RELEASE/PROVENANCE" > "$dir/PROVENANCE"
  if [[ "${3:-}" == broken ]]; then
    rm "$dir/bin/opencanvas-server"
    printf '#!/bin/bash\necho "broken test release: exiting at start" >&2\nexit 3\n' > "$dir/bin/opencanvas-server"
    chmod 700 "$dir/bin/opencanvas-server"
    (cd "$dir" && find bin worker renderer web node_modules package.json package-lock.json Cargo.lock -type f -print0 |
      LC_ALL=C sort -z | xargs -0 shasum -a 256 > SHA256SUMS)
  fi
}

# Starts a command in the background in its own process group, remembered for cleanup.
# Job control keeps SIGINT deliverable, as under launchd (async jobs otherwise ignore it).
fixture_spawn() {
  set -m
  "$@" &
  FIXTURE_PIDS+=("$!")
  set +m
}

pg_ready() { "$PG_BIN/pg_isready" -q -h "$ROOT_DIR/run" -p "$PG_PORT"; }
http_up() { curl -fsS --max-time 5 -o /dev/null "http://127.0.0.1:$LOCAL_PORT/api/v1/viewer" 2>/dev/null; }
wait_until() { # SECONDS COMMAND...
  local deadline=$((SECONDS + $1))
  shift
  until "$@"; do ((SECONDS < deadline)) || return 1; sleep 1; done
}

# One project with a page, a frame and text, created through MCP (verify-runtime needs a frame).
fixture_seed() {
  "$ROOT_DIR/runtime/releases/$1/bin/node" "$FIXTURE_NATIVE/tests/seed-project.mjs" "http://127.0.0.1:$LOCAL_PORT" \
    > "$FIXTURE_TMP/seed.json" || fail "seeding a project failed"
}

fixture_cleanup() {
  local pid
  # Expansion form safe for an empty array under set -u in macOS bash 3.2.
  for pid in ${FIXTURE_PIDS[@]+"${FIXTURE_PIDS[@]}"}; do kill -TERM "$pid" 2>/dev/null || true; done
  for pid in ${FIXTURE_PIDS[@]+"${FIXTURE_PIDS[@]}"}; do wait "$pid" 2>/dev/null || true; done
  if [[ -n "$FIXTURE_TMP" && -f "$ROOT_DIR/postgres/postmaster.pid" ]]; then
    "$PG_BIN/pg_ctl" -D "$ROOT_DIR/postgres" -m immediate -w stop >/dev/null 2>&1 || true
  fi
  # Anything still running from this root (server, worker, Chromium, process-compose) is ours.
  [[ -z "$FIXTURE_TMP" ]] || pkill -KILL -f "$FIXTURE_TMP" 2>/dev/null || true
  [[ -z "$FIXTURE_TMP" ]] || rm -rf "$FIXTURE_TMP"
}
