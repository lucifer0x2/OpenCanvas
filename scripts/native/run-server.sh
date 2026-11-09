#!/bin/bash
set -euo pipefail
config=${1:?usage: run-server.sh SERVER_ENV}
set -a
# Private installer-generated shell assignments, supplied explicitly by launchd.
# shellcheck source=/dev/null
source "$config"
set +a
# shellcheck source=/dev/null
source "$OPENCANVAS_ROOT/runtime/scripts/runtime-supervise.sh"
export PATH="$OPENCANVAS_RELEASE/bin:/usr/bin:/bin:/usr/sbin:/sbin"
export PLAYWRIGHT_BROWSERS_PATH="$OPENCANVAS_RELEASE/.runtime/browsers"
export OPENCANVAS_LAYOUT_BIN="$OPENCANVAS_RELEASE/bin/layout-step"
for ((attempt=0; attempt<30; attempt++)); do
  /opt/homebrew/opt/postgresql@16/bin/pg_isready -h 127.0.0.1 -p 2703 -q && break
  sleep 1
done
cd "$OPENCANVAS_RELEASE"
supervise INT "$OPENCANVAS_RELEASE/bin/opencanvas-server" \
  --workspace "$OPENCANVAS_RELEASE" --data-root "$OPENCANVAS_ROOT/runtime/unused-sqlite" \
  --host 127.0.0.1 --port "$OPENCANVAS_PORT" \
  --public-viewer-port "$OPENCANVAS_PUBLIC_PORT" --public-viewer-origin "$OPENCANVAS_PUBLIC_ORIGIN"
