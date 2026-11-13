#!/bin/bash
# Builds one complete native release from one git commit. No Docker, no patch overlays.
set -euo pipefail
umask 077
HERE=$(cd "$(dirname "$0")" && pwd -P)
# shellcheck source=scripts/native/deploy-common.sh
source "$HERE/deploy-common.sh"

usage() {
  cat <<'EOF'
Usage: build-release.sh <git-ref> <output-dir>

Resolves <git-ref> to a full commit, archives that exact tree into a private source
directory and builds natively: scene WASM, renderer bundle, viewer, release server and
layout-step. Copies Node (for verify-runtime.mjs), then writes RENDER_BACKEND, REVISION,
PROVENANCE and SHA256SUMS. The release runs the Rust render backend: run-server.sh exports
its RENDER_BACKEND, and no Chromium is copied. <output-dir> must not exist; it appears only
after a complete build.

Environment (optional):
  CARGO_TARGET_DIR     Cargo cache (default <repo>/.runtime/native/target)
  CARGO_BUILD_JOBS     Parallel Cargo jobs
  WASM_BINDGEN         wasm-bindgen matching Cargo.lock (default: .runtime/native/tools,
                       target/scene-wasm/tools, then PATH)
  OPENCANVAS_NODE      Node binary matching the Dockerfile pin (default nvm Node)
  OPENCANVAS_RENDER_BACKEND
                       rust (default) or chromium. chromium also copies the Playwright
                       browser from OPENCANVAS_BROWSERS (default <repo>/.runtime/browsers).
Builds run offline (cargo --offline, npm ci --offline); dependencies must be cached.
EOF
}

[[ "${1:-}" != --help && "${1:-}" != -h ]] || { usage; exit 0; }
[[ $# == 2 ]] || { usage >&2; exit 2; }
ref=$1
output=$2
[[ "$output" == /* ]] || output="$PWD/$output"
[[ ! -e "$output" ]] || die "Output already exists: $output"
parent=$(dirname "$output")
[[ -d "$parent" ]] || die "Output parent directory does not exist: $parent"
sha=$(git -C "$NATIVE_REPO" rev-parse --verify --quiet "$ref^{commit}") || die "Unknown git ref: $ref"
backend=${OPENCANVAS_RENDER_BACKEND:-rust}
[[ "$backend" == rust || "$backend" == chromium ]] || die "OPENCANVAS_RENDER_BACKEND must be rust or chromium"
cache="$NATIVE_REPO/.runtime/native"
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$cache/target}
export CARGO_NET_OFFLINE=true
started=$SECONDS
src="$cache/source/$sha"
staging=''
cleanup() {
  local status=$?
  trap - EXIT
  [[ -z "$staging" ]] || rm -rf "$staging"
  ((status == 0)) || printf 'error: build of %s failed; nothing was written to %s\n' "$sha" "$output" >&2
  exit "$status"
}
trap cleanup EXIT

say "Archiving $sha ($ref)"
mkdir -p "$cache/source"
rm -rf "$src"
mkdir -p "$src"
git -C "$NATIVE_REPO" archive "$sha" | tar -x -C "$src"

say "Checking toolchain against the Dockerfile pins"
node=${OPENCANVAS_NODE:-$HOME/.nvm/versions/node/v24.21.0/bin/node}
node_pin=$(sed -n 's/^ARG NODE_IMAGE=node:\([0-9.]*\)-.*/\1/p' "$src/Dockerfile")
rust_pin=$(sed -n 's/^ARG RUST_IMAGE=rust:\([0-9.]*\)-.*/\1/p' "$src/Dockerfile")
[[ -x "$node" && "$("$node" --version)" == "v$node_pin" ]] || die "Node v$node_pin required (set OPENCANVAS_NODE)"
node_dir=$(dirname "$node")
export PATH="$node_dir:$PATH"
rustc_version=$(rustc --version)
[[ "$rustc_version" == "rustc $rust_pin "* ]] || warn "Dockerfile pins Rust $rust_pin; building with $rustc_version"
rustup target list --installed 2>/dev/null | grep -qx wasm32-unknown-unknown ||
  die "Rust target wasm32-unknown-unknown is not installed"
bindgen_pin=$(awk '$0 == "name = \"wasm-bindgen\"" { getline; gsub(/version = |"/, ""); print }' "$src/Cargo.lock")
bindgen=${WASM_BINDGEN:-}
# Project-local installs (this cache, or npm run build:scene-wasm -- --install-tools), then PATH.
for candidate in "$cache/tools/bin/wasm-bindgen" "$NATIVE_REPO/target/scene-wasm/tools/bin/wasm-bindgen" \
  "$(command -v wasm-bindgen || true)"; do
  [[ -n "$bindgen" || -z "$candidate" || ! -x "$candidate" ]] || bindgen=$candidate
done
[[ -n "$bindgen" && "$("$bindgen" --version 2>/dev/null)" == "wasm-bindgen $bindgen_pin" ]] ||
  die "wasm-bindgen $bindgen_pin required: set WASM_BINDGEN or install it project-locally with
  cargo install wasm-bindgen-cli --version $bindgen_pin --locked --root .runtime/native/tools"
browsers=${OPENCANVAS_BROWSERS:-$NATIVE_REPO/.runtime/browsers}

say "Building scene WASM"
(cd "$src" && WASM_BINDGEN="$bindgen" sh scripts/build-scene-wasm.sh "$src/web/public/scene-wasm" "$CARGO_TARGET_DIR/scene-wasm")
(cd "$src" && node scripts/compress-scene-wasm.mjs web/public/scene-wasm)

say "Installing locked npm dependencies from the local cache"
(cd "$src" && npm ci --offline --no-audit --no-fund && npm --prefix web ci --offline --no-audit --no-fund)

say "Building renderer bundle and viewer"
(cd "$src" && npm run build:renderer && npm --prefix web run build)

say "Building layout-step (debug, as in the Dockerfile) and the release server"
cargo build --offline --locked --manifest-path "$src/Cargo.toml" -p opencanvas-layout --bin layout-step
cargo build --offline --locked --release --manifest-path "$src/Cargo.toml" -p opencanvas-server --bin opencanvas-server

chromium_revision=$(node -p 'require(process.argv[1]).browsers.find((b) => b.name === "chromium-headless-shell").revision' \
  "$src/node_modules/playwright-core/browsers.json")
[[ "$backend" == rust || -d "$browsers/chromium_headless_shell-$chromium_revision" ]] ||
  die "Playwright browser chromium_headless_shell-$chromium_revision not found under $browsers"
db_schema=$(parse_db_schema < "$src/$DB_SCHEMA_SOURCE")
scene_schema=$(parse_scene_schema < "$src/$SCENE_SCHEMA_SOURCE")
[[ "$db_schema" =~ ^[0-9]+$ && "$scene_schema" =~ ^[0-9]+$ ]] ||
  die "Cannot determine the supported schema versions from $DB_SCHEMA_SOURCE and $SCENE_SCHEMA_SOURCE"

say "Assembling release"
staging=$(mktemp -d "$parent/.build-${sha:0:12}.XXXXXX")
mkdir -p "$staging/bin" "$staging/renderer" "$staging/web" "$staging/node_modules" "$staging/.runtime"
cp "$CARGO_TARGET_DIR/debug/layout-step" "$CARGO_TARGET_DIR/release/opencanvas-server" "$staging/bin/"
cp "$node" "$staging/bin/node"
cp -R "$src/worker" "$staging/worker"
cp -R "$src/renderer/dist" "$src/renderer/fonts" "$staging/renderer/"
cp -R "$src/web/dist" "$staging/web/dist"
cp -R "$src/node_modules/playwright" "$src/node_modules/playwright-core" "$staging/node_modules/"
cp "$src/package.json" "$src/package-lock.json" "$src/Cargo.lock" "$staging/"
printf '%s\n' "$backend" > "$staging/RENDER_BACKEND"
if [[ "$backend" == chromium ]]; then
  # APFS clones keep per-release browser copies cheap; fall back to a plain copy elsewhere.
  cp -cR "$browsers" "$staging/.runtime/browsers" 2>/dev/null || {
    rm -rf "$staging/.runtime/browsers"
    cp -R "$browsers" "$staging/.runtime/browsers"
  }
fi
printf '%s\n' "$sha" > "$staging/REVISION"
{
  printf 'commit=%s\nref=%s\nbuilt_utc=%s\nbuild_seconds=%s\n' "$sha" "$ref" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$((SECONDS - started))"
  printf 'builder=scripts/native/build-release.sh\nsource=git-archive\n'
  printf 'database_schema_version=%s\nscene_schema_version=%s\n' "$db_schema" "$scene_schema"
  printf 'rustc=%s\ncargo=%s\n' "$rustc_version" "$(cargo --version)"
  printf 'node=%s\nnpm=%s\nwasm_bindgen=%s\n' "$(node --version)" "$(npm --version)" "$bindgen_pin"
  printf 'playwright=%s\nchromium_headless_shell_revision=%s\n' \
    "$(node -p 'require(process.argv[1]).version' "$src/node_modules/playwright/package.json")" "$chromium_revision"
  printf 'layout_step_profile=debug\nserver_profile=release\nrender_backend=%s\n' "$backend"
} > "$staging/PROVENANCE"
(cd "$staging" && find bin worker renderer web node_modules package.json package-lock.json Cargo.lock RENDER_BACKEND -type f -print0 |
  LC_ALL=C sort -z | xargs -0 shasum -a 256 > SHA256SUMS)
mv "$staging" "$output"
staging=''
# The Cargo target directory is the reusable cache; a failed build keeps its source for diagnosis.
rm -rf "$src"
say "Release $sha built in $((SECONDS - started))s: $output"
