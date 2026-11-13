#!/usr/bin/env bash
# Task24 large-project baseline. Reads the source design with HTTP GET from the
# loopback native server only; measures in a temporary SQLite root and a fresh,
# disposable PostgreSQL (scripts/task19-check.sh). Never writes ~/.opencanvas.
# Output: an OS temporary directory with cell/viewer JSON, logs and summary.md.
# The fetched design and response bodies are private and deleted on exit.
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_BUILD_JOBS=2
: "${TASK24_SOURCE_ORIGIN:=http://127.0.0.1:3857}"
: "${TASK24_SOURCE_PROJECT:=axis-training}" # ID or slug
: "${TASK24_COPIES_LIST:=1 5 15 30}"
: "${TASK24_SHARED_COPIES=15}" # empty: skip the shared-master variant
: "${TASK24_ENGINES:=sqlite postgres}"
export TASK24_COPIES_LIST TASK24_SHARED_COPIES

task24_cell() { # engine variant copies; a failed cell is recorded, the run continues
  local name body=/dev/null status=0
  name="$1-$2-$(printf '%02d' "$3")"
  # Only SQLite cells keep the object body, for the viewer step (same design bytes).
  if [[ "$1" == sqlite ]]; then body="$TASK24_OUTPUT_DIR/bodies/body-${name#sqlite-}.json"; fi
  printf 'Task24: cell %s\n' "$name"
  TASK24_ENGINE=$1 TASK24_VARIANT=$2 TASK24_COPIES=$3 TASK24_BODY_FILE=$body \
    /usr/bin/time -l "$TASK24_BIN" --ignored --exact large_project_cell --nocapture \
    --test-threads=1 >"$TASK24_OUTPUT_DIR/run-$name.log" 2>"$TASK24_OUTPUT_DIR/time-$name.txt" ||
    status=$?
  if ((status)); then printf '%s exit %s\n' "$name" "$status" | tee -a "$TASK24_OUTPUT_DIR/failures.txt"; fi
}

task24_cells() { # engine
  local copies
  for copies in $TASK24_COPIES_LIST; do task24_cell "$1" own "$copies"; done
  if [[ -n "$TASK24_SHARED_COPIES" ]]; then task24_cell "$1" shared "$TASK24_SHARED_COPIES"; fi
}

if [[ "${1:-}" == --pg-cells ]]; then
  [[ "$TASK19_POSTGRES_CONTAINER" == opencanvas-task19.*-pg ]]
  docker update --cpus 2 --memory 2g --memory-swap 2g "$TASK19_POSTGRES_CONTAINER" >/dev/null
  docker exec "$TASK19_POSTGRES_CONTAINER" postgres --version >>"$TASK24_OUTPUT_DIR/metadata.txt"
  task24_cells postgres
  exit 0
fi

[[ "$TASK24_SOURCE_ORIGIN" =~ ^http://127\.0\.0\.1:[0-9]+$ ]] || { echo 'Source must be loopback' >&2; exit 2; }
umask 077
export TASK24_OUTPUT_DIR
TASK24_OUTPUT_DIR=$(mktemp -d "${TMPDIR:-/tmp}/opencanvas-task24.XXXXXX")
export TASK24_SOURCE_DIR="$TASK24_OUTPUT_DIR/source"
trap 'rm -rf "$TASK24_SOURCE_DIR" "$TASK24_OUTPUT_DIR/bodies"' EXIT
mkdir -p "$TASK24_SOURCE_DIR/assets" "$TASK24_OUTPUT_DIR/bodies"
printf 'Task24 output: %s\n' "$TASK24_OUTPUT_DIR"

# Read-only source: one snapshot GET plus one GET per asset (font files are assets).
project_url="$TASK24_SOURCE_ORIGIN/api/v1/projects/$TASK24_SOURCE_PROJECT"
curl -fsS --get "$project_url/snapshot" -o "$TASK24_SOURCE_DIR/snapshot.json"
node -e 'for (const a of JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).assets) console.log(a.id)' \
  "$TASK24_SOURCE_DIR/snapshot.json" | while IFS= read -r asset; do
  [[ "$asset" =~ ^[A-Za-z0-9_-]+$ ]]
  curl -fsS --get "$project_url/assets/$asset" -o "$TASK24_SOURCE_DIR/assets/$asset"
done

build=(cargo test --offline --release -p opencanvas-server --features large-project-bench
  --test large_project_benchmark --no-run -j2)
TASK24_BIN=$("${build[@]}" 2>&1 | sed -n 's/.*Executable .*(\(.*\))$/\1/p' | tail -1)
[[ -x "$TASK24_BIN" ]]
export TASK24_BIN
{
  printf 'Timestamp UTC: '; date -u '+%Y-%m-%dT%H:%M:%SZ'
  printf 'Build: %s\n' "${build[*]}"
  printf 'Engines: %s; copies: %s; shared-master variant: %s\n' "$TASK24_ENGINES" "$TASK24_COPIES_LIST" "$TASK24_SHARED_COPIES"
  printf 'Host: '; uname -sm; sysctl -n machdep.cpu.brand_string hw.ncpu hw.memsize 2>/dev/null || true
  rustc -vV; cargo --version; node --version
  printf 'Git HEAD: '; git rev-parse HEAD
  printf 'Git branch: '; git branch --show-current
  printf 'Dirty paths: %s\n' "$(git status --porcelain | wc -l | tr -d ' ')"
  docker info --format 'Docker {{.OSType}} {{.Architecture}} CPUs={{.NCPU}} Memory={{.MemTotal}}'
} >"$TASK24_OUTPUT_DIR/metadata.txt"

if [[ " $TASK24_ENGINES " == *" sqlite "* ]]; then task24_cells sqlite; fi
if [[ " $TASK24_ENGINES " == *" postgres "* ]]; then
  TASK19_BENCH_STATS=0 bash scripts/task19-check.sh bash scripts/task24-bench.sh --pg-cells
fi
for body in "$TASK24_OUTPUT_DIR"/bodies/body-*.json; do
  [[ -e "$body" ]] || continue
  base=$(basename "$body" .json)
  node --expose-gc --max-old-space-size=16384 spikes/large-project/viewer-cost.mjs "$body" 1 5 \
    >"$TASK24_OUTPUT_DIR/viewer-${base#body-}.json"
done
node spikes/large-project/summarize.mjs "$TASK24_OUTPUT_DIR" | tee "$TASK24_OUTPUT_DIR/summary.md"
printf 'Task24 output: %s\n' "$TASK24_OUTPUT_DIR"
