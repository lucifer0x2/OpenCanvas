# Browser scene computation

`SceneEngine` is the native-testable, I/O-free snapshot → scene API. The wasm-bindgen
`SceneComputer` class exposes it to a dedicated module Worker. It shares resolution,
component expansion, responsive layout and native deterministic text measurement.
It does not link storage, painting, an HTTP client, Tokio or system fonts.

Build with a `wasm32-unknown-unknown` Rust target and wasm-bindgen **0.2.129**:

```sh
npm run build:scene-wasm -- --install-tools
# Or supply an existing exact-version tool without installing anything:
WASM_BINDGEN=/path/to/wasm-bindgen npm run build:scene-wasm
```

Tool installation is opt-in and local to `target/scene-wasm/tools/`. The script uses
Cargo.lock, a pinned glue version, size optimization and LTO, and prints raw/gzip/
Brotli sizes for the JS and WASM. Generated JS/WASM/`.d.ts` live in the ignored
`web/public/scene-wasm/` directory. They are regenerated, never hand-edited or
committed. Source-owned Worker types describe the narrow API, allowing ordinary
flag-off Vite builds and typechecks without generating WASM first.

Targeted checks:

```sh
cargo test --locked --offline -p opencanvas-scene-wasm --tests -j 2
node --test crates/scene-wasm/tests/bindings.test.mjs
```

The module is an ESM file served unchanged by Vite's public directory. Import its
runtime URL with `/* @vite-ignore */`, then initialize with an explicit WASM URL:

```js
const base = new URL("scene-wasm/", viewerBaseUrl);
const wasm = await import(/* @vite-ignore */ new URL("opencanvas_scene_wasm.js", base).href);
await wasm.default({ module_or_path: new URL("opencanvas_scene_wasm_bg.wasm", base) });
const computer = new wasm.SceneComputer();
// required_fonts(snapshotJSON, frameId, width, height, theme) returns the exact
// resolved Font[] manifest JSON, allowing a fetch of only used fonts first.
computer.load_font(font.id, font.contentHash, new Uint8Array(await response.arrayBuffer()));
computer.set_resource_urls(JSON.stringify({ fonts: fontUrlsById, assets: assetUrlsById }));
const scene = JSON.parse(computer.compute_scene(JSON.stringify(snapshot), frameId, width, height, theme));
```

`load_font` validates original-byte SHA-256 before decoding TTF/OTF/WOFF2. Loading a
new valid hash replaces that ID atomically; a rejected load preserves the old font.
Each compute binds bytes to the exact snapshot family/weight/style. Missing fonts,
hash mismatch, invalid requests and layout failures throw plain structured objects
with pipeline `code`, `message`, `retryable` and optional validation context fields.
The compute state is fixed to `default`, with `scene-3`/`web-4` engine versions.

The JSON API appends the same bounded measured text overflow advisories as native
server jobs, including note-based intentional overflow for resolved instances and
ancestors. The pure implementation lives in `scene-input::scene_diagnostics`;
it includes geometry and source scene provenance, never authored text or notes.
The typed `compute()` API retains layout's `Scene` with string diagnostics; object
advisories are added only at the JSON boundary after a single resolve/compute pass.

URLs are transport labels: the caller constructs and supplies maps, replaced by
each `set_resource_urls` call. With no map, manifests have empty URLs. No URL is
constructed or followed by the pure core. URLs enter the layout input before
compute, so manifest hashes and scene key describe the returned manifests. Native
parity requires identical caller-provided transport URLs. Geometry, line lists,
diagnostics and authored resolution have no parity exclusions. Server-only
readiness, elapsed-time metrics and paint/engine provenance added outside the
shared layout call are not produced here. Image decode/readiness remains a browser
rendering concern; this class only computes scenes and text metrics.
