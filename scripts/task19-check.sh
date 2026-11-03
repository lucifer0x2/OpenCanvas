#!/usr/bin/env bash
# Fresh, disposable PostgreSQL only. Credentials remain in a private temporary file.
set -euo pipefail
export CARGO_BUILD_JOBS=2
cd "$(dirname "$0")/.."
task19_dir=$(mktemp -d "${TMPDIR:-/tmp}/opencanvas-task19.XXXXXX")
task19_suffix=$(basename "$task19_dir" | tr '[:upper:]' '[:lower:]')
task19_pg="${task19_suffix}-pg"
task19_stage=prepare
task19_docker() {
  local task19_seconds=$1
  shift
  perl -e 'alarm shift; exec @ARGV or die "Cannot start Docker\n"' "$task19_seconds" docker "$@"
}
cleanup() {
  local task19_status=$?
  if ((task19_status)); then
    printf 'Task19 failed during %s (exit %s).\n' "$task19_stage" "$task19_status" >&2
  fi
  task19_docker 20 rm -fv "$task19_pg" >/dev/null 2>&1 || true
  rm -rf "$task19_dir"
}
trap cleanup EXIT
trap 'exit 130' INT TERM
umask 077
task19_password=$(openssl rand -hex 24)
printf 'POSTGRES_PASSWORD=%s\nPOSTGRES_DB=task19\nPOSTGRES_USER=task19\n' "$task19_password" > "$task19_dir/postgres.env"
printf '%s\n' 'POSTGRES_INITDB_ARGS=--locale-provider=icu --icu-locale=en-US' >> "$task19_dir/postgres.env"
task19_stage=postgres-start
printf 'Task19: starting fresh PostgreSQL container.\n'
task19_pg_args=(postgres)
if [[ "${TASK19_BENCH_STATS:-0}" == 1 ]]; then
  task19_pg_args+=(-c shared_preload_libraries=pg_stat_statements)
fi
task19_docker 180 run -d --name "$task19_pg" --label opencanvas.task=task19 \
  --env-file "$task19_dir/postgres.env" -p 127.0.0.1::5432 postgres:16-alpine \
  "${task19_pg_args[@]}" >/dev/null
task19_stage=port-mapping
task19_pg_port=''
for task19_attempt in {1..30}; do
  task19_pg_port=$(task19_docker 5 port "$task19_pg" 5432/tcp 2>/dev/null | cut -d: -f2) || task19_pg_port=''
  if [[ "$task19_pg_port" =~ ^[0-9]+$ ]]; then break; fi
  sleep 0.1
done
[[ "$task19_pg_port" =~ ^[0-9]+$ ]]
task19_stage=service-readiness
printf 'Task19: waiting for PostgreSQL.\n'
for task19_attempt in {1..60}; do
  if task19_docker 5 exec "$task19_pg" pg_isready -h 127.0.0.1 -U task19 -d task19 >/dev/null 2>&1; then break; fi
  sleep 0.5
done
task19_docker 5 exec "$task19_pg" pg_isready -h 127.0.0.1 -U task19 -d task19 >/dev/null
export TASK19_POSTGRES_URL="postgresql://task19:${task19_password}@127.0.0.1:${task19_pg_port}/task19"
export TASK19_POSTGRES_CONTAINER="$task19_pg"
printf 'Task19: fresh PostgreSQL ready on a random loopback port; cleanup armed.\n'
task19_stage=tests
if (($#)); then
  "$@"
else
  cargo test --offline -p opencanvas-storage --test postgres_contract \
    --test postgres_domain --test postgres_transactions --test postgres_resources --test postgres_cache \
    --test postgres_cache_failures --test postgres_namespace --test postgres_fidelity \
    -- --ignored --test-threads=1
  cargo test --offline -p opencanvas-server --test postgres_backend -- --ignored --test-threads=1
fi
