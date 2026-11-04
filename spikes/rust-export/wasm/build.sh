#!/usr/bin/env bash
# Task 09: release .wasm in two variants (no SIMD, +simd128) × three opt-levels (release = 3,
# release-s = "s", release-z = "z"), copied to ../out/wasm/ as rust_spike-<simd>-<profile>.wasm, then
# raw / gzip -9 / brotli -q 11 sizes in bytes. Each variant has its own target dir so RUSTFLAGS do
# not force rebuilds. Run from anywhere; needs the wasm32-unknown-unknown rustup target.
set -euo pipefail
cd "$(dirname "$0")"
out=../out/wasm
mkdir -p "$out"
for simd in nosimd simd128; do
  flags=""
  [ "$simd" = simd128 ] && flags="-C target-feature=+simd128"
  for profile in release release-s release-z; do
    RUSTFLAGS="$flags" cargo build --quiet --profile "$profile" --target wasm32-unknown-unknown \
      --target-dir "target/$simd"
    cp "target/$simd/wasm32-unknown-unknown/$profile/rust_spike_wasm.wasm" "$out/rust_spike-$simd-$profile.wasm"
  done
done
printf '%-36s %10s %10s %10s\n' file raw gzip-9 brotli-11
for f in "$out"/rust_spike-*.wasm; do
  printf '%-36s %10d %10d %10d\n' "$(basename "$f")" "$(wc -c <"$f")" \
    "$(gzip -9 -c "$f" | wc -c)" "$(brotli -q 11 -c "$f" | wc -c)"
done
