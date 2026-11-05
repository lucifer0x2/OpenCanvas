#!/bin/sh
# Shared by the local Node wrapper and the Rust-only Docker build stage.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
output=${1:-"$root/web/public/scene-wasm"}
target_dir=${2:-"$root/target/scene-wasm"}
bindgen=${WASM_BINDGEN:-wasm-bindgen}

cd "$root"
CARGO_PROFILE_RELEASE_OPT_LEVEL=z \
CARGO_PROFILE_RELEASE_LTO=true \
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
CARGO_PROFILE_RELEASE_PANIC=abort \
    cargo build --locked --release -p opencanvas-scene-wasm \
        --target wasm32-unknown-unknown --target-dir "$target_dir"
mkdir -p "$output"
"$bindgen" "$target_dir/wasm32-unknown-unknown/release/opencanvas_scene_wasm.wasm" \
    --target web --out-dir "$output" --out-name opencanvas_scene_wasm
# Vite serves/copies these as ESM from public/, not as an npm CommonJS package.
rm -f "$output/package.json"
