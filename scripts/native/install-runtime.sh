#!/bin/bash
# Installs an isolated cluster and release. Does not start or replace the live app.
set -euo pipefail
umask 077
: "${OPENCANVAS_PUBLIC_ORIGIN:?Set OPENCANVAS_PUBLIC_ORIGIN to the public HTTPS origin}"
release=${1:?usage: install-runtime.sh PREPARED_RELEASE [ROOT]}
root=${2:-$HOME/Library/Application Support/OpenCanvas}
scripts=$(cd "$(dirname "$0")" && pwd -P)
pg_bin=/opt/homebrew/opt/postgresql@16/bin
[[ -s "$release/REVISION" && -x "$release/bin/opencanvas-server" ]]
[[ ! -e "$root" ]] || { echo 'Root already exists; refusing to overwrite' >&2; exit 1; }
[[ -z $(lsof -nP -iTCP:2703 -sTCP:LISTEN 2>/dev/null) ]] || { echo 'Port2703 is in use' >&2; exit 1; }
revision=$(cat "$release/REVISION")
mkdir -p "$root"/{runtime/releases,runtime/scripts,postgres,run,config,backups,logs}
chmod 700 "$root" "$root/"{postgres,run,config,backups,logs}
cp -R "$release" "$root/runtime/releases/$revision"
cp "$scripts/"{runtime-supervise,run-server,run-postgres}.sh "$root/runtime/scripts/"
chmod 700 "$root/runtime/scripts/"*.sh
"$pg_bin/initdb" -D "$root/postgres" --encoding=UTF8 --locale=C --auth-local=peer --auth-host=scram-sha-256 > "$root/logs/initdb.log"
# Only the cluster owner authenticates as itself over the private Unix socket.
# The application uses SCRAM on both Unix and TCP connections.
cat > "$root/postgres/pg_hba.conf" <<EOF
local all $(id -un) peer
local all all scram-sha-256
host all all 127.0.0.1/32 scram-sha-256
host all all ::1/128 scram-sha-256
EOF
cat >> "$root/postgres/postgresql.conf" <<EOF
listen_addresses = '127.0.0.1'
port = 2703
unix_socket_directories = '$root/run'
password_encryption = 'scram-sha-256'
logging_collector = off
EOF
password=$(openssl rand -hex 32)
{
  printf 'OPENCANVAS_ROOT=%q\nPG_BIN=%q\n' "$root" "$pg_bin"
  printf 'PGHOST=%q\nPGPORT=2703\nPGDATABASE=opencanvas\nPGUSER=opencanvas\n' "$root/run"
  printf 'PGPASSFILE=%q\nPGADMIN_USER=%q\nPGADMIN_HOST=%q\nBACKUP_KEEP=14\n' "$root/config/pgpass" "$(id -un)" "$root/run"
} > "$root/config/native.conf"
printf '*:2703:*:opencanvas:%s\n' "$password" > "$root/config/pgpass"
{
  printf 'OPENCANVAS_ROOT=%q\nOPENCANVAS_RELEASE=%q\n' "$root" "$root/runtime/releases/$revision"
  printf 'DATABASE_PROVIDER=postgresql\nDATABASE_URL=%q\n' "postgresql://opencanvas:$password@127.0.0.1:2703/opencanvas"
  printf 'OPENCANVAS_PORT=3867\nOPENCANVAS_PUBLIC_PORT=3868\n'
  printf 'OPENCANVAS_PUBLIC_ORIGIN=%q\n' "$OPENCANVAS_PUBLIC_ORIGIN"
  if [[ ${OPENCANVAS_PUBLIC_ALLOWED_ORIGINS+x} ]]; then
    printf 'OPENCANVAS_PUBLIC_ALLOWED_ORIGINS=%q\n' "$OPENCANVAS_PUBLIC_ALLOWED_ORIGINS"
  fi
  printf 'OPENCANVAS_PUBLIC_TRUSTED_PROXY_IP=127.0.0.1\n'
} > "$root/config/server.env"
chmod 600 "$root/config/"*
cleanup() {
  if [[ -f "$root/postgres/postmaster.pid" ]]; then
    "$pg_bin/pg_ctl" -D "$root/postgres" -m fast -w stop >/dev/null
  fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP
LC_ALL=C "$pg_bin/pg_ctl" -D "$root/postgres" -l "$root/logs/postgres-install.log" -w start >/dev/null
printf "CREATE ROLE opencanvas LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE PASSWORD '%s';\n" "$password" | \
  "$pg_bin/psql" -X -h "$root/run" -p 2703 -d postgres -v ON_ERROR_STOP=1 >/dev/null
"$pg_bin/createdb" -h "$root/run" -p 2703 -O opencanvas opencanvas
unset password
echo 'Isolated native runtime and cluster installed; app remains on rehearsal ports3867/3868.'
