#!/bin/bash
set -euo pipefail
# Private installer-generated shell assignments, supplied explicitly by launchd.
# shellcheck source=/dev/null
source "${1:?usage: run-postgres.sh NATIVE_CONF}"
# shellcheck source=/dev/null
source "$OPENCANVAS_ROOT/runtime/scripts/runtime-supervise.sh"
export LC_ALL=C LANG=C
# PostgreSQL SIGINT is a fast, clean shutdown (disconnects clients and checkpoints).
supervise INT "$PG_BIN/postgres" -D "$OPENCANVAS_ROOT/postgres"
