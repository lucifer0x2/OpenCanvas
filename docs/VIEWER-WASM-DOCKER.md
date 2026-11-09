# Browser scene WASM in Docker

Verified on `feat/viewer-wasm-scene`, 2026-11-05. The Docker build produces its
own WASM; host output under `web/public/scene-wasm` is excluded from its context.

## Build and serving

The `scene-wasm` stage uses the configured Rust image. Its first cached layer
installs `wasm32-unknown-unknown` and `wasm-bindgen-cli 0.2.129 --locked` into
`/opt/wasm-bindgen`, before copying project sources. Registry, CLI compilation
and WASM compilation use BuildKit caches; installed tools and `/out/scene-wasm`
are ordinary stage output, independent of cache-mount contents.

`scripts/build-scene-wasm.sh` owns the release settings for both Docker and the
local Node wrapper: optimization `z`, LTO, one codegen unit and panic abort.
It generates web bindings and removes their `package.json`. The web stage copies
these files into `web/public/scene-wasm`, writes Brotli/gzip sidecars using the
same compression helper as local builds, then runs the regular Vite build.

Only the static `ServeDir` negotiates precompressed files. Responses retain the
original MIME type, including `application/wasm`, and vary on `Accept-Encoding`.
API, asset-download and SSE routes retain their existing response behavior.
The public viewer shares this static service and retains its host guard.

The optional Worker backstop bounds a live request to twice its execution
timeout from posting, including queue wait. Aborting clears that caller's
backstop; infrastructure replay preserves the original deadline. The existing
execution timer still starts when the Worker acknowledges beginning work.

## Build measurements

Measured on the existing local OrbStack builder with cached base images and
native dependencies. These are observed builds, not an uncached benchmark.
The baseline used the previous Dockerfile with an explicit removal of host WASM
before Vite, representing a clean source build rather than contaminated output.

| Measurement | Before | After |
| --- | ---: | ---: |
| `docker image inspect .Size`, bytes | 356,870,912 | 359,472,633 |
| Initial measured full build, seconds | 66.09 | 104.35 |

Image growth is **2,601,721 bytes** (about 2.48 MiB). The new stage spent
**58.9 seconds** installing the target/CLI and **27.8 seconds** compiling/binding
WASM, about **86.7 seconds** total before copying output. It overlaps native
compilation: measured full-build growth was 38.26 seconds. Sidecar compression
took about 6.6 seconds within the web stage. A subsequent build with the final
static-serving source took 27.02 seconds, reused the tool installation layer,
and mostly rebuilt the native server. The final cache check took 10.49 seconds;
target/CLI installation was cached and the WASM build step took 0.8 seconds.
Build times depend on cache state/load. The pre-existing image tag before this
task was 354,279,699 bytes; it predates this branch's clean source baseline.

The final verified image was
`sha256:eed4139dce293a6af1dca060dc1913749857cee4334950eac30b172a7845d4cb`.
Existing running containers were not recreated with this image.

| Image artifact | Identity bytes | gzip bytes | Brotli bytes |
| --- | ---: | ---: | ---: |
| `opencanvas_scene_wasm_bg.wasm` | 3,625,802 | 948,086 | 687,020 |
| `opencanvas_scene_wasm.js` | 14,563 | 3,593 | 3,159 |

The local Rust toolchain produces slightly different WASM bytes from the pinned
Linux Docker toolchain; both use the shared release settings. The local build
and existing browser smoke also passed. Both compressed variants round-trip
exactly to their respective identity files.

## Isolated container verification

Built with `docker build -t opencanvas:kubernetes-local .` from the worktree.
Used `compose.yaml` with project name `opencanvas-viewer-wasm-check`, an explicit
throwaway env file selecting port 43947, and a temporary override mapping only
a temporary data directory to `/data`. No production env, PostgreSQL volume or
`.runtime/compose-postgres` data was used. The canary became healthy.

The reusable probe expects an already running disposable server and creates a
sample project through its MCP transport:

```sh
node spikes/viewer-wasm-scene/docker-smoke.mjs http://127.0.0.1:43947
```

The probe passed against the built container with Chromium 156.0.8078.4:

- Identity, Brotli and gzip GETs for the glue/WASM returned the correct MIME and
  identical decoded bytes; the browser fetched Brotli WASM.
- A real Worker computed revision 1 of the welcome frame, 71 nodes, and the
  canvas painted. The viewer made **zero `/scene` requests**, with no fallback
  notice or page/Worker errors.
- Snapshot and SSE responses remained uncompressed.
- A host-only marker placed beside generated WASM was absent from the image;
  generated `package.json` was also absent.

The probe, local fixture smoke, parity and test outputs are generated evidence,
not committed artifacts. Test-owned data is disposable.

The final Rust rerun initially exhausted the host disk. Only this worktree's
generated incremental/old object caches were removed; the successful retry used
`CARGO_INCREMENTAL=0`. OrbStack subsequently stopped, and the user brought it
back. The throwaway Compose container/network were then removed; production
containers were never recreated by this verification.

## Regression checks

- Five-crate Rust gate: **460 passed, 0 failed, 36 ignored**. Breakdown:
  scene-input 11, scene-wasm 9, text 38, storage 153, server 249. Existing opt-in
  database/capture tests remain ignored. Four new static HTTP tests passed,
  including the public viewer, negotiation, raw decoded bytes and live SSE.
- Web tests **66/66**, typecheck passed; renderer tests **11/11**.
- Generated WASM binding tests **2/2** and `test:scene-wasm` passed.
- Existing parity runner: **20/20, 0 mismatches**.
- The targeted static test Clippy/fmt checks passed. Source/tests changed here
  stay at 250 lines or fewer; generated outputs are ignored.

The Rust command was:

```sh
CARGO_INCREMENTAL=0 cargo test \
  -p opencanvas-scene-input -p opencanvas-scene-wasm -p opencanvas-text \
  -p opencanvas-storage -p opencanvas-server --locked --offline -j 2 \
  -- --test-threads=2
```
