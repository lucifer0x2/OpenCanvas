#!/bin/bash
# LaunchAgent entry point: process-compose supervises PostgreSQL and the server headless.
# launchd SIGTERM reaches process-compose (exec), which stops the server, then PostgreSQL.
set -euo pipefail
umask 077
root=${1:?usage: run-compose.sh ROOT [PROCESS_COMPOSE]}
pc=${2:-/opt/homebrew/bin/process-compose}
socket="$root/run/process-compose.sock"
export PC_CLIENT_LOG_FILE="$root/logs/process-compose-client.log"
if [[ -S "$socket" ]]; then
  if "$pc" --use-uds --unix-socket "$socket" process list >/dev/null 2>&1; then
    echo "process-compose is already running for this root" >&2
    exit 1
  fi
  rm -f "$socket" # stale socket left by a crash
fi
cd "$root"
# The API listens only on the private Unix socket, never on the default TCP :8080.
exec "$pc" up --config "$root/process-compose.yaml" --tui=false --use-uds --unix-socket "$socket" \
  --log-file "$root/logs/process-compose.log" --log-no-color --disable-dotenv --ordered-shutdown --keep-project
