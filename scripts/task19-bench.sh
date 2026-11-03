#!/usr/bin/env bash
# Benchmark only fresh Task19 PostgreSQL; source runner owns cleanup/credentials.
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_BUILD_JOBS=2
if [[ "${1:-}" != --services ]]; then
  export TASK19_BENCH_OUTPUT_DIR
  TASK19_BENCH_OUTPUT_DIR=$(mktemp -d "${TMPDIR:-/tmp}/opencanvas-task19-bench.XXXXXX")
  printf 'Task19b output: %s\n' "$TASK19_BENCH_OUTPUT_DIR"
  cargo test --offline --release -p opencanvas-server --test runtime_storage_benchmark --no-run -j2
  export TASK19_BENCH_STATS=1
  exec bash scripts/task19-check.sh bash scripts/task19-bench.sh --services
fi
[[ -n "${TASK19_BENCH_OUTPUT_DIR:-}" && -d "$TASK19_BENCH_OUTPUT_DIR" ]]
[[ "$TASK19_POSTGRES_CONTAINER" == opencanvas-task19.*-pg ]]
docker update --cpus 2 --memory 2g --memory-swap 2g "$TASK19_POSTGRES_CONTAINER" >/dev/null
docker exec "$TASK19_POSTGRES_CONTAINER" psql -X -v ON_ERROR_STOP=1 -U task19 -d task19 \
  -c 'CREATE EXTENSION pg_stat_statements' >/dev/null
task19_bench_fingerprint() {
  git ls-files --cached --others --exclude-standard -- crates Cargo.toml Cargo.lock scripts/task19-bench.sh scripts/task19-check.sh \
    | LC_ALL=C sort -u | while IFS= read -r path; do
      if [[ -f "$path" ]]; then shasum -a 256 "$path"; fi
    done | shasum -a 256
}
task19_bench_fingerprint_before=$(task19_bench_fingerprint)
{
  printf 'Timestamp UTC: '; date -u '+%Y-%m-%dT%H:%M:%SZ'
  printf '\nBuild: cargo test --offline --release -p opencanvas-server --test runtime_storage_benchmark -j2\n'
  printf 'PostgreSQL pool=4; pg_stat_statements enabled for all PG cells\n'
  printf 'Moka TTL=3600s, 64MiB total and 16MiB entry JSON-byte weight limits\n'
  printf 'Host: '; uname -sm
  rustc -vV
  cargo --version
  printf '\nGit HEAD: '; git rev-parse HEAD
  printf 'Git branch: '; git branch --show-current
  printf 'Rust/Cargo/runner source fingerprint (sha256 of sorted path checksums):\n'
  printf 'Before: %s\n' "$task19_bench_fingerprint_before"
  printf '\nDocker host/resources:\n'; docker info --format '{{.OSType}} {{.Architecture}} CPUs={{.NCPU}} Memory={{.MemTotal}}'
  docker inspect --format 'Image={{.Config.Image}} ImageID={{.Image}} CPUs={{.HostConfig.NanoCpus}} Memory={{.HostConfig.Memory}} Swap={{.HostConfig.MemorySwap}}' "$TASK19_POSTGRES_CONTAINER"
  task19_bench_image=$(docker inspect --format '{{.Image}}' "$TASK19_POSTGRES_CONTAINER")
  docker image inspect --format 'RepoDigests={{json .RepoDigests}}' "$task19_bench_image"
  printf '\nPostgres version:\n'
  docker exec "$TASK19_POSTGRES_CONTAINER" postgres --version
} > "$TASK19_BENCH_OUTPUT_DIR/metadata.txt"
task19_bench_status=0
cargo test --offline --release -p opencanvas-server --test runtime_storage_benchmark -j2 \
  -- --ignored --nocapture --test-threads=1 2>&1 | tee "$TASK19_BENCH_OUTPUT_DIR/run.log" || task19_bench_status=$?
task19_bench_fingerprint_after=$(task19_bench_fingerprint)
printf '\nAfter: %s\nRun exit: %s\n' "$task19_bench_fingerprint_after" "$task19_bench_status" >> "$TASK19_BENCH_OUTPUT_DIR/metadata.txt"
[[ "$task19_bench_fingerprint_before" == "$task19_bench_fingerprint_after" ]]
exit "$task19_bench_status"
