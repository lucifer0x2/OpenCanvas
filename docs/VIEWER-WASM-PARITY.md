# Native server ↔ browser WASM scene parity

## Checked result

Run **2026-11-05 03:21:09 UTC**, macOS Darwin 25.4.0 arm64, Apple M5,
Node v24.21.0, Playwright **1.64.1**, Chromium **156.0.8078.4** revision **1248**.
The release WASM module in a real Chromium **dedicated module Worker** matched
the actual native server's complete shared Scene contract in **20/20 cases**:
**1,164 node observations, 420 text observations, 476 final text lines, zero
mismatches**. Geometry, advances, baselines and ink use **zero tolerance**.
All 400 warm calls also returned exactly their own first result.

This includes the missing-glyph control at all four sizes. No frame, node,
invisible text, clipped text, line or missing glyph was removed from the gate.
Both sides emitted the same four `MISSING_GLYPH` observations for
`Missing ⌘ 🦄 一`: **U+2318, U+4E00, U+1F984**. Both use the same supplied font
and `.notdef` policy; this is equality of that policy, not a claim of glyph coverage.
The gate also compared **44 TEXT_CONTENT_OUTSIDE_BOX** advisories and
**7 TEXT_OVERFLOW_SUMMARY** records, including evidence geometry and order.
There were no native paint errors in the captured jobs. WASM painting and pixel
parity are outside this verification.

## Oracle, fixtures and exact contract

`crates/server/tests/viewer_wasm_capture.rs` starts the real server router on an
ephemeral loopback port with `Config::for_backend(..., RenderBackend::Rust)`.
The existing native harness supplies an empty worker workspace and nonexistent
Node executable. A temporary SQLite store receives actual MCP `create_project`,
`import_asset` and `apply_batch` calls. Every oracle is the exact requested
`render_frame` job fetched over HTTP by job ID after it settles. Revision, frame,
viewport, default state, engine and source job key are checked. Native capture
passed **1 test, 38.54 s**. Its temporary data root is deleted at test exit;
the real application and user data are not used.

Each frame runs at its numeric authored width/height, **1440×900**, **768×1024**
and **390×844**, default state only:

| Source, imported without translating fixture logic | Actual frame ID | Authored viewport |
| --- | --- | --- |
| `samples/welcome/design.mjs::welcomeOperations` | `welcome-dashboard` | 1040×720 |
| `samples/welcome-stacks/design.mjs::stacksOperations` | `welcome-dashboard-stacks` | 1040×720 |
| `native_layout_support::fixtures::repo_wrap` | `frame` | 1040×844 |
| `native_layout_support::fixtures::typography` | `typography` | 1040×1200 |
| `native_layout_support::fixtures::missing_control` | `missing-control` | 1040×1200 |

The original Rust typography and font upload helpers are linked directly into
the integration test. The wrap helper imports the existing repository fixture.
The capture checks every native scene's ID set against the complete authored
frame subtree. The JS comparator then checks exact node **order** and every
shared field recursively: bounds, parent/kind/theme, all line text/x/y/width,
all typography/decoration, every ink rectangle, paints, resources, diagnostics,
schema and provenance. `TextLine.y` is the baseline; no per-line height exists.
There is no rounding, epsilon, visible-only filter or generic unknown-field mask.

The WASM caller supplies the **native scene's exact font/asset URL maps before
compute**. Fonts, assets and both derived manifest hashes therefore stay in the
full equality gate. Font downloads use captured original bytes, verified against
their native manifest SHA-256 and again by `SceneComputer.load_font`.

Only these explicit fields are excluded, with original values retained by path
in the generated ledger:

- `provenance.sourceSceneKey`: the server replaces the layout key with its
  pipeline/job cache key; the pure browser core has no server job/pipeline.
- `provenance.engine`, `engineVersion`, `runtimeVersion`: the server adds native
  compute/paint labels (`native`, `opencanvas-render-0.1.0/tiny-skia-0.12.0`,
  `rust-native`); the pure browser Scene does not claim that painter/runtime.
- Exact copies of those same fields in overflow diagnostic `sourceSceneKey`
  and summary `provenance`. Advisory codes, count/order, severity, intent,
  examples, ink/advance evidence and remaining provenance all remain compared.

The outer server job envelope (`jobId`, transport/readiness, metrics, image/PNG,
error/status) is not part of the Scene and is preserved separately. There are no
unexplained field exclusions or remaining mismatches.

## Chromium Worker timings

All numbers below are milliseconds, displayed to 0.1 ms; raw samples retain full
`performance.now()` values in the ignored ledger. Each case gets a **new Worker
and SceneComputer**, discovers fonts, fetches/validates/decodes exact bytes, then
makes its **first compute** and **20 warm computes**. `compute` includes JS snapshot
JSON serialization, Rust JSON parse, resolution/expansion, layout and text
measurement, scene serialization, shared advisory generation and JS scene parse.
It excludes Worker startup/message transfer, module loading, font discovery and
font fetch/decode. Font discovery is a separate resolver pass and is reported.
The per-case decode column is the sum of all required font SHA/decode calls.
Separate local font-fetch totals range **2.5–11.8 ms**; individual fetch/decode
observations for every font are retained in `comparison.json`.

`import/init` separates JS module import from WASM fetch/compile/instantiate.
This is a release build (`opt-level=z`, LTO, one codegen unit, panic abort) served
as raw uncompressed bytes on local HTTP with `no-store`. The enclosing browser
page is 1440×900; scene layout uses each row's logical viewport. No CPU/network
throttling or device scale transform affects logical sizes. Workers report
10 logical processors and `crossOriginIsolated=false`; timer resolution is
therefore browser-limited. Workers are fresh, but browser/OS/compiler caches are
not cleared between rows; these are first-compute timings, not network-cold claims.
Warm verification compares JSON outside the timed interval.

Frame aliases: **welcome**=`welcome-dashboard`, **stacks**=`welcome-dashboard-stacks`,
**wrap**=`frame`, **type**=`typography`, **missing**=`missing-control`.

| Frame | Viewport | Nodes/texts/lines | JS import/WASM init | Discover | Font decode | First compute | Warm min/median/p95/max (n=20) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| welcome | 1040×720 | 71/41/41 | 5.9/8.8 | 8.8 | 10.2 | 17.8 | 4.7/5.4/7.5/8.7 |
| welcome | 1440×900 | 71/41/41 | 0.9/7.0 | 7.2 | 10.1 | 18.0 | 4.9/5.3/7.4/9.2 |
| welcome | 768×1024 | 71/41/41 | 0.9/6.1 | 7.2 | 13.4 | 18.5 | 5.2/5.9/7.6/9.0 |
| welcome | 390×844 | 71/41/41 | 0.7/6.6 | 7.6 | 10.9 | 19.5 | 5.4/5.9/8.2/9.4 |
| stacks | 1040×720 | 181/41/41 | 0.8/6.6 | 12.7 | 11.5 | 42.9 | 21.7/22.8/25.5/26.1 |
| stacks | 1440×900 | 181/41/41 | 1.0/6.5 | 12.8 | 12.2 | 46.8 | 22.4/23.3/27.1/29.4 |
| stacks | 768×1024 | 181/41/41 | 0.9/6.5 | 13.6 | 12.0 | 47.7 | 23.9/26.5/28.4/29.4 |
| stacks | 390×844 | 181/41/49 | 1.1/6.8 | 15.5 | 13.9 | 52.6 | 20.4/23.3/29.8/32.4 |
| wrap | 1040×844 | 8/3/9 | 0.8/6.5 | 6.7 | 11.4 | 14.8 | 3.1/3.7/4.8/5.7 |
| wrap | 1440×900 | 8/3/7 | 1.0/6.6 | 6.8 | 12.1 | 16.2 | 3.2/3.8/5.3/5.8 |
| wrap | 768×1024 | 8/3/7 | 0.8/6.4 | 6.9 | 12.2 | 13.0 | 2.4/2.7/3.7/4.3 |
| wrap | 390×844 | 8/3/8 | 0.8/6.9 | 7.2 | 12.8 | 16.7 | 3.1/3.9/5.6/6.2 |
| type | 1040×1200 | 29/19/24 | 1.0/7.4 | 7.5 | 25.8 | 25.0 | 6.4/7.1/9.1/10.5 |
| type | 1440×900 | 29/19/32 | 1.2/7.3 | 8.6 | 27.8 | 29.8 | 7.5/8.0/10.3/11.7 |
| type | 768×1024 | 29/19/24 | 1.1/7.8 | 9.0 | 30.1 | 28.3 | 7.0/8.5/12.0/12.1 |
| type | 390×844 | 29/19/25 | 0.9/7.8 | 8.5 | 29.8 | 29.5 | 7.0/7.8/11.1/12.7 |
| missing | 1040×1200 | 2/1/1 | 1.3/8.1 | 8.5 | 15.5 | 12.3 | 1.0/1.3/3.2/3.3 |
| missing | 1440×900 | 2/1/1 | 1.2/7.5 | 8.5 | 15.0 | 12.4 | 1.1/1.5/2.0/2.1 |
| missing | 768×1024 | 2/1/1 | 1.1/7.7 | 8.5 | 15.4 | 12.7 | 1.1/1.4/2.0/2.2 |
| missing | 390×844 | 2/1/1 | 1.0/7.5 | 8.6 | 15.6 | 12.1 | 1.1/1.4/2.0/2.2 |

## Reproduce and artifact policy

From the isolated worktree, with the pinned local Playwright browser installed:

```sh
# Original Inter fixture bytes must already exist; missing/wrong bytes fail, never skip.
# See spikes/native-render-parity/README.md for the pinned font preparation.
WASM_BINDGEN=/private/tmp/opencanvas-scene-wasm-tools/bin/wasm-bindgen CARGO_NET_OFFLINE=true npm run build:scene-wasm
cargo test -p opencanvas-server --test viewer_wasm_capture --locked --offline -j 2 -- --ignored --nocapture --test-threads=1
node spikes/viewer-wasm-scene/run.mjs

node --test spikes/viewer-wasm-scene/compare.test.mjs
cargo clippy -p opencanvas-server --test viewer_wasm_capture --locked --offline -j 2 --no-deps -- -D warnings
rustfmt --check --edition 2021 --config skip_children=true crates/server/tests/viewer_wasm_capture.rs crates/server/tests/viewer_wasm_support/capture.rs
node_modules/.bin/prettier --check spikes/viewer-wasm-scene/*.mjs
```

Use any existing **wasm-bindgen 0.2.129** binary via `WASM_BINDGEN`; the path above
identifies the checked local tool. If needed, the build script's explicit
`--install-tools` option installs it under the ignored project target directory.
The capture test is intentionally **ignored in ordinary cargo test**: it is an
explicit artifact producer for the subsequent real Chromium gate, requires the
prepared original Inter bytes, and the browser gate requires the generated
release WASM/JS artifacts. The two commands above execute the full matrix;
missing inputs fail explicitly. Loopback capture/browser execution required the
environment's sandbox network approval; both approved executions passed.

Generated JSON, font bytes and raw response captures live only under ignored
`artifacts/viewer-wasm-scene/`. `native.json` references all 20 raw HTTP job files,
three raw snapshot files and hash-addressed fonts. `comparison.json` preserves
all WASM scenes, exact mismatch/exclusion ledgers, per-call timings, runtime and
source/artifact SHA-256 identities. Nothing generated is committed. The handwritten
sources are small: capture93, helper130, comparator88, comparator tests39,
runner182, Worker102 lines; all are below200 and each has one responsibility.
Focused Clippy, Rust formatting, JS formatting and both comparator tests passed.

An earlier capture parsed native HTTP JSON through reqwest/serde_json and then
serialized it again. That harness path introduced **445 one-ULP ink differences**
(largest **5.684341886080802e-14 px**, also copied into advisory evidence).
For example raw native HTTP and WASM both had `13.899999618530273`, while the
reencoded capture had `13.899999618530272`. The fix preserves **raw snapshot and
job HTTP bytes** and parses them once in JavaScript, matching a real browser.
The old complete ledger is retained locally as `comparison-initial-reencoded.json`.
No measured field was rounded, filtered or given a tolerance to obtain the final
zero-mismatch result. This verification task changed no product implementation.

Checked artifact SHA-256:

- Native capture executable: `15820f4b85234740cee4061b35e05207bfdc9fb51fba1072503e0f3b2b86a3d0`.
- Release WASM: `7d7a053189af3e4070c4dc77f15e1d0fd76af219ef6df8a4afe4b243038f6a3f`.
- JS glue: `c4921525c610a86a77291ef648bfce7e0c617d754a62c91b8f84b6944577163f`.
- Cargo.lock: `340f4143a5ad1956329e0aeccff9fcc2fc99dfedf4a2f3d2dc874344ac58be5a`.

Follow-up rerun on 2026-11-05 after worker cancellation/snapshot-cache changes:
`node spikes/viewer-wasm-scene/run.mjs` completed **20 comparisons, 0 mismatches**;
the release WASM hash above is unchanged. This harness measures the shared WASM
engine directly. The viewer's separate `test:scene-wasm` also passed and asserts
one structured snapshot transfer per revision; client/runtime unit regressions
verify abort reuse, bounded snapshot retention and project/restart invalidation.

Review-correction rerun on 2026-11-05 rebuilt the shared viewport guard and neutral
measurer wording, captured native jobs again, and passed **20/20 comparisons with
0 mismatches** (1,164 nodes, 420 texts, 476 lines). The new release WASM SHA-256 is
`e7f5b74384bca6628b32c06db50b2a247b5c7b41f67e9018551210d4be2ca6c2`;
native capture executable SHA-256 is
`37063c28fa4d15bf1b585181f2b4bb360662264557da40ad8792f908b9d2cb66`.
WASM raw/gzip/Brotli sizes are **3,628,098/949,322/687,528 B**. The viewer smoke
passed after the final Worker changes; web regressions are now **64/64**, including
real off-thread obsolete work longer than the current request's execution budget.

The release build succeeded; JS raw/gzip/Brotli sizes are **14,563/3,593/3,159 B**,
WASM **3,627,161/948,871/687,689 B**. This is a bounded fixture acceptance on a
concurrently developed worktree, not a global commit/source-stability or universal
text-engine proof. Additional per-file hashes are in the generated comparison.

Pinned original font SHA-256 (the four upload pins reuse the original Rust helper):

| Font | SHA-256 |
| --- | --- |
| Bundled Noto Sans | `bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d` |
| Inter variable 400 normal | `4989b125924991b90d05b2d16e0e388c48f7d5bb8b30539bbf9c755278d0ccaf` |
| Lato Regular | `d636e4683231f931eda222d588e944d082bfd3bdba02f928bee461c0f185b251` |
| Lato Bold | `8a0aace75d33794eece4b28187bfc1df0bbd2888b5d8a56e01788c8d65d16be1` |
| Lato Italic | `e399c44efe1387100531d26c7e4800c5d12251b890d6654a3098c7c679cb1786` |
