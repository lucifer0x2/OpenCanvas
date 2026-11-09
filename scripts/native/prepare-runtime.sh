#!/bin/bash
# Build macOS binaries from the live image revision; preserve its web/worker artifacts.
set -euo pipefail
umask 077
repo=$(cd "$(dirname "$0")/../.." && pwd -P)
revision=${1:?usage: prepare-runtime.sh REVISION CONTAINER [OUTPUT]}
container=${2:?missing source container}
output=${3:-$repo/.runtime/native/release-$revision}
source_dir="$repo/.runtime/native/source/$revision"
docker=(docker --context orbstack)
actual=$("${docker[@]}" inspect "$container" --format '{{index .Config.Labels "org.opencontainers.image.revision"}}')
[[ "$actual" == "$revision" ]] || { echo 'Source image revision mismatch' >&2; exit 1; }
[[ ! -e "$output" ]] || { echo 'Output already exists' >&2; exit 1; }
mkdir -p "$source_dir" "$output/bin" "$output/node_modules" "$output/renderer" "$output/web" "$output/.runtime"
git -C "$repo" archive "$revision" | tar -x -C "$source_dir"
export CARGO_TARGET_DIR="$repo/.runtime/native/target"
cargo build --offline --locked --manifest-path "$source_dir/Cargo.toml" -p opencanvas-layout --bin layout-step
cargo build --offline --locked --release --manifest-path "$source_dir/Cargo.toml" -p opencanvas-server --bin opencanvas-server
cp "$CARGO_TARGET_DIR/debug/layout-step" "$output/bin/layout-step"
cp "$CARGO_TARGET_DIR/release/opencanvas-server" "$output/bin/opencanvas-server"
cp /Users/luis/.nvm/versions/node/v24.21.0/bin/node "$output/bin/node"
for item in worker renderer/fonts renderer/dist web/dist node_modules/playwright node_modules/playwright-core package.json package-lock.json Cargo.lock; do
  "${docker[@]}" cp "$container:/app/$item" "$output/$item"
done
cp -R "$repo/.runtime/browsers" "$output/.runtime/browsers"
printf '%s\n' "$revision" > "$output/REVISION"
printf '%s\n' "$("${docker[@]}" inspect "$container" --format '{{.Image}}')" > "$output/SOURCE_IMAGE"
(cd "$output" && find bin worker renderer web node_modules -type f -exec shasum -a 256 {} + > SHA256SUMS)
echo "Native release prepared: $output"
