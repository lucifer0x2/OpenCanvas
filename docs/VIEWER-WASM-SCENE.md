# Browser-computed frame scenes

Implemented on `feat/viewer-wasm-scene`, based on `feat/task11-native-png`
at `ce30a958295aa1c7ac67b6de921000ad98c27845`, in an isolated worktree.
Changes are committed locally in dependency-ordered steps. No push, deployment
or user-data migration.

## What moved

- `crates/scene-input`: authored models/errors, Candidate validation, tokens and
  scoped themes, styles, instance expansion, property catalogs and frame-subtree
  input assembly. Storage reexports the same types/functions; server `make_input`
  still attaches its controlled resource URLs. There is no dependency cycle.
- Native storage enables optional `sqlite-errors` solely to preserve the existing
  `From<rusqlite::Error>` conversion and exact error type identity. The default
  WASM dependency graph contains no storage, rusqlite, PostgreSQL, Tokio, mio,
  tiny-skia or render crate.
- `crates/text::FontSetMeasurer`: shared FontSet adapter, with caller checks so
  native cancellation/deadlines and measurement counts retain their behavior.
- `scene-input::scene_diagnostics`: unchanged bounded overflow and note-intent
  logic shared with native jobs; the WASM JSON boundary includes these advisories.
- `crates/scene-wasm`: native-testable `SceneEngine` plus wasm-bindgen
  `SceneComputer`. Exports font discovery, verified font loading, supplied URL
  maps and `compute_scene(snapshotJson, frameId, width, height, theme)`.

`web/src/scene/` separates flag selection, resource manifests, font-byte cache,
worker protocol/client, worker initialization and server fallback. `useScene`
receives the already accepted snapshot. SSE hints use the existing snapshot
refresh; no separate snapshot request is added by the compiler. Default-state
frame scenes use the existing `scene-3`/`web-4` contract and Canvas2D painter.
The preview change is type-only compatibility for its independent token types.

## Build and enable

Prerequisites: existing Rust WASM target, pinned npm dependencies and
wasm-bindgen **0.2.129**. From this worktree:

```sh
npm ci
npm --prefix web ci
npm run build:scene-wasm -- --install-tools
npm run build
```

Tool installation is explicit and project-local under ignored `target/`.
An existing exact-version binary can instead be supplied through `WASM_BINDGEN`.
Generated ESM/WASM/declarations are ignored under `web/public/scene-wasm/` and
copied into the production viewer by Vite. Ordinary server-mode builds work
without these artifacts. A pre-existing tracked machine-specific `node_modules`
symlink was removed; dependencies are installed locally and ignored.

Open a frame with **`?scene=wasm`**. A saved preference is also supported:
`localStorage["opencanvas.scene"] = "wasm"`; explicit `?scene=server` wins.
Server computation remains the default. The module initializes lazily in a
dedicated Worker. Failure falls back to the existing scene job route and shows
a notice even when ordinary notices are hidden. Aborting a request settles only
that caller while the Worker and loaded fonts remain warm. Late success replies
can acknowledge registered fonts without publishing an obsolete scene. Coded
Rust errors reject only their caller; infrastructure errors and execution
timeouts restart the Worker. A failed initialization promise is cleared. If
bootstrap fails for an aborted caller, current queued requests replay against
a fresh Worker with complete snapshots and fonts.

Execution timers start when the Worker acknowledges beginning a message, so
queued requests do not spend their timeout behind obsolete computation. Module
startup and initialization remain bounded even after their caller aborts.

Snapshots are structured-cloned once per cached `(projectId, revision)` and
encoded in the Worker, outside the main thread. Preparation and computation
reference that revision. Both sides retain the same four-revision FIFO and clear
it on a project switch; a worker restart resends the snapshot. Changing Retry's
key also invalidates the Worker cache even at the same revision. Rust still
parses the cached JSON separately for discovery and computation.

Only required font bytes are fetched and SHA-verified, then shared with both
WASM and the existing FontFace loader. They are not transferred/detached. The
immutable byte cache coalesces loads and evicts failures for retry, bounded by
32 entries and 64 MiB of retained completed buffers. Oversized loads serve active
callers without being retained. Only fonts not yet acknowledged by `(id, hash)`
are sent to the Worker; project switches and restarts clear that registry.
Coded request errors invalidate the client mirror because a failed request may
already have replaced a font in WASM; the actual Worker/font cache remains warm.
Painting, hit testing and page overview behavior remain on their existing paths.

Both native and browser preparation use `scene-input::check_viewport`: finite,
positive sides at most 100,000, finite scale in `(0, 4]`, and at most 16 million
scaled pixels. Browser scenes use scale 1; native assembly preserves valid
downscaled requests. Rejection is `INVALID_REQUEST` before font/layout work,
and the browser's existing server fallback preserves the server error outcome.

## Size and performance

Release: `opt-level=z`, LTO, one codegen unit, panic abort. Sizes are bytes:

| Artifact | Raw | gzip, level 9 | Brotli, quality 11 |
| --- | ---: | ---: | ---: |
| WASM | 3,628,098 | 949,322 | 687,528 |
| ESM glue | 14,563 | 3,593 | 3,159 |

Original timing baseline, Chromium 156.0.8078.4 on Apple M5/macOS arm64,
real dedicated Workers before the review corrections:
first compute **12.1–52.6 ms/frame**; warm medians **1.3–26.5 ms/frame**
(20 warm calls per case). These include snapshot/scene JSON conversion,
resolution, layout, text and advisory generation. They exclude module loading,
font discovery/fetch/decode and worker message transfer; those costs are listed
separately in the [full timing/parity report](VIEWER-WASM-PARITY.md).
These measurements do not involve WASM painting.

## Verification

- Native-server/WASM parity: **20/20 scenes**, **1,164 node observations**,
  **420 texts**, **476 lines**, **0 mismatches**. Includes both welcome samples
  and every native-layout fixture at authored size, 1440×900, 768×1024 and
  390×844. All 400 warm calls are deterministic.
- Exact comparison includes geometry, baselines, ink, resources, manifest hashes
  and structured diagnostics. Only native runtime/paint labels and the server
  pipeline job key differ; their exact paths/values are documented. Raw HTTP
  snapshots/scenes avoid decimal parse/reencode artifacts in the oracle.
- Parent independently recomputed all 20 captured comparisons and verified every
  recorded source/artifact hash: zero differences.
- Final native workspace suite: **529 passed, 0 failed, 36 ignored**, exit 0:
  `cargo test --workspace --locked --offline --no-fail-fast -j 2 -- --test-threads=1`.
  The ignored PostgreSQL/oracle/artifact tests retain their explicit prerequisites;
  the new native-capture/Chromium WASM matrix was executed separately in full.
- All **37 web unit tests**, web/renderer typechecks, production web build, existing
  browser/continuity/actions smoke and generated-binding smoke passed. The new
  WASM smoke proves no `/scene` call, one shared font download, accepted-snapshot
  SSE ordering, unused-font isolation, module/hash fallback and repaired-font retry.
- A built production viewer against a real temporary backend rendered an expanded
  component instance on Canvas2D with zero `/scene` requests; WASM MIME was
  `application/wasm`. Worker reply-error recovery has separate preparation/compute
  tests proving termination, clean pending rejection and successful fresh-worker retry.
- Native library Clippy for scene-input/scene-wasm/text/storage/server passed with
  warnings denied. The four compute crates passed final WASM-target checks.
- Scoped Rust formatting passed. Existing stale `effectiveTheme` golden fields
  and the instance-stroke catalog expectation were corrected, the deadline test
  uses an isolated semaphore, and notice smoke tests explicitly select their
  expected preference; production defaults stay intact.

Commands and generated evidence live in the parity report and ignored
`artifacts/viewer-wasm-scene/`. New/modified handwritten source and tests are all
at most 250 lines; no source-size exception is required. The largest touched
source/test is 236 lines. The existing viewer reference, `docs/VIEWER.md`, remains
254 lines; its overview was corrected in place and this feature's details live
in the separate reports. Cargo.lock is generated dependency metadata.

### Follow-up verification — 2026-11-05

The independent review's stale renderer-source lookup and three source comments
now point to `scene-input`; no `storage/src` references remain in the requested
Rust/TypeScript/Node/shell source search. Abort/cache regressions also cover
concurrent callers, late replies, revision eviction, project switches and restart.

- `cargo test -p opencanvas-scene-input -p opencanvas-scene-wasm -p opencanvas-text
  -p opencanvas-storage -p opencanvas-server --locked --offline -j 2 -- --test-threads=2`:
  **448 passed, 0 failed, 36 ignored**. Breakdown: scene-input 7, scene-wasm 7,
  text 38, storage 153, server 243. Ignored tests retain their stated prerequisites.
- Web unit tests **43/43**, renderer tests **11/11**, both typechecks, production
  web build and real Chromium `test:scene-wasm` passed. The smoke verifies one
  snapshot transfer per revision and reference-only compute messages.
- `node spikes/viewer-wasm-scene/run.mjs`: **20/20, 0 mismatches**, unchanged
  WASM SHA-256 `7d7a053189af3e4070c4dc77f15e1d0fd76af219ef6df8a4afe4b243038f6a3f`.
  Comparator regressions **2/2** and strict library Clippy for all five crates passed.
- Changed handwritten source/tests remain at most **236 lines**. Generated
  WASM, evidence, dependency directories and native targets stay ignored.

### Review corrections — 2026-11-05

Initialization now clears rejected promises, and an infrastructure error from an
aborted request restarts the Worker and replays current queued requests without
settling those callers as failed. Execution acknowledgements give each message
one initialization-plus-compute budget, excluding obsolete queue wait. Coded
Rust errors preserve their code and leave the Worker warm. Retry invalidates
snapshot state; acknowledged font identities avoid repeated transfers, and the
font-byte cache now enforces both count and retained-byte limits.

The shared viewport guard and scaled native assembly preserve downscale behavior
while rejecting invalid browser inputs before discovery/layout. The measurer
error is runtime-neutral, and the two requested server-reference paths now point
to `scene-input`.

Final Rust suite: **456 passed, 0 failed, 36 ignored** across scene-input **11**,
scene-wasm **9**, text **38**, storage **153**, and server **245**, using the
five-crate test command above. Ignored tests retain their explicit prerequisites;
native capture was separately executed with `--ignored` before parity.

Final web tests **64/64**, renderer **11/11**, generated binding **2/2**, both
typechecks, production build, WASM browser smoke and strict five-crate library
Clippy passed. Fresh native captures and the rebuilt WASM still compare exactly:
**20/20 scenes, 1,164 nodes, 420 texts, 476 lines, 0 mismatches**.
Release WASM raw/gzip/Brotli: **3,628,098/949,322/687,528 bytes**; SHA-256
`e7f5b74384bca6628b32c06db50b2a247b5c7b41f67e9018551210d4be2ca6c2`.
Generated captures and binaries remain ignored. Changed source/tests are at most
206 lines; the existing 538-line server reference received a path-only edit.

## Remaining gaps

- Default state only in browser computation; hover/pressed/focus/disabled stay
  outside this change. The existing server interaction support is preserved.
- Missing glyphs emit diagnostics and use the shared `.notdef` policy. No system
  font or missing-glyph fallback is added; missing-control scenes remain in parity.
- Linux and other browser/host combinations were not measured or accepted here.
- Cold startup includes extra resource/module costs, and font discovery is a
  separate resolver pass. Timings describe this bounded fixture set, not a
  general performance guarantee or paint/pixel parity result.
