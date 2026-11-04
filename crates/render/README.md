# Pure native/WASM scene renderer

`opencanvas-render` paints **already-computed** `scene-3` / `web-4` scenes with
tiny-skia. It does not compute layout, wrap/transform text, discover OS fonts,
fetch URLs, access files, spawn threads, or depend on Tokio. Chromium remains the
application default; adding this workspace member does not select a backend.

## Public integration API

```rust,ignore
use opencanvas_render::{render, encode_png, PaintScene, Resources, ResourceBytes, RenderOptions};

// Deserialize the server's scene directly, NOT through layout::Scene:
// server diagnostics can be objects; layout::Scene expects Vec<String>.
let scene: PaintScene = serde_json::from_value(scene_json)?;
let mut resources = Resources::default();
resources.fonts.insert(font.id.clone(), ResourceBytes {
    hash: &font.hash, bytes: &font_bytes,
});
resources.assets.insert(asset.id.clone(), ResourceBytes {
    hash: &asset.hash, bytes: &asset_bytes,
});
let cancelled = || cancellation_token.is_cancelled() || clock.now() >= deadline;
let options = RenderOptions {
    scale: 1.0,
    strict: true,
    cancelled: Some(&cancelled),
    ..RenderOptions::default()
};
let raster = render(&scene, &resources, &options)?;
let png_bytes = encode_png(&raster)?;
```

Exact public signatures:

```text
render(&PaintScene, &Resources<'_>, &RenderOptions<'_>) -> Result<Raster, RenderError>
encode_png(&Raster) -> Result<Vec<u8>, RenderError>
inspect_resources(&PaintScene, &Resources<'_>, &RenderOptions<'_>) -> Result<Vec<ResourceReadiness>, RenderError>
PaintScene: From<opencanvas_layout::Scene> + Clone + Serialize + Deserialize
Resources<'a>: Default
RenderOptions<'a>: Default
```

- `PaintScene { schema_version: u32, coordinate_space: String,
  provenance: layout::Provenance, nodes: Vec<layout::SceneNode>,
  fonts: Vec<layout::Font>, assets: Vec<layout::Asset>, diagnostics: Vec<Value>,
  runtime_provenance: BTreeMap<String, Value> }`. Unknown provenance fields
  deserialize into `runtime_provenance` and serialize back into `provenance`.
  Opaque scene diagnostics are retained, never interpreted as load failures.
- `ResourceBytes<'a> { hash: &'a str, bytes: &'a [u8] }` uses SHA256 of the
  **original file**, before WOFF2 expansion. Each resource's caller hash and
  actual bytes are checked against its manifest. Resource maps are keyed by ID.
- `Resources<'a> { fonts: BTreeMap<String, ResourceBytes<'a>>,
  assets: BTreeMap<String, ResourceBytes<'a>>, readiness_diagnostics: Vec<Value> }`.
  The last field contains upstream load failures only, not missing-glyph advisories.
- `RenderOptions<'a> { scale: f64, strict: bool, max_layer_pixels: u64,
  cancelled: Option<&'a dyn Fn() -> bool>, text_coverage_gamma: f32,
  snap_glyph_origins: bool }`. Defaults: 1, true, 16,000,000, None, 1, false.
  Experimental glyph options enable independent coverage/snap comparisons; no
  improvement over CoreText is claimed. Gamma must be finite in [0.25,4].
- `Raster { width: u32, height: u32, rgba: Vec<u8>, draw: DrawStats,
  source_scene_key: String, diagnostics: Vec<Value>, readiness: Vec<ResourceReadiness> }`.
  RGBA is **premultiplied sRGB RGBA8**, row-major, stride `width * 4`. PNG encoding
  demultiplies and writes an sRGB tag; it does not rerender.
- `DrawStats { drawn_lines: usize, opacity_layers: usize,
  opacity_layer_fallbacks: usize }` serializes to camelCase, matching worker draw.
- `ResourceReadiness { id: String, kind: String, status: String,
  code: Option<String> }` uses font/asset, ready/failed, and optional failure code.
- `RenderError { code: String, message: String, readiness: Vec<ResourceReadiness> }`
  implements Error/Display/serde. Strict decode failures preserve observed readiness;
  the vector is empty for failures before resource preparation. Scale uses f64 for
  exact public ceil dimensions; only device transforms narrow to f32.

### Server handoff: deadline and errors

The callback is pure and caller-owned: the server should capture its existing
cancellation flag and monotonic deadline and map `CANCELLED` to its job outcome.
Checks occur between nodes/phases, resource decodes, glyphs, ICC rows, blur
scanlines, and bounded pixel chunks. A single third-party codec, shaping, hash,
outline draw or tiny-skia operation cannot be interrupted internally. Therefore a
server timeout must still retain concurrency permits until actual CPU work ends;
dropping an await is not cancellation. PNG encoding is separate and bounded but
has no callback. No thread/runtime policy is imposed by this crate.

Stable failures: `PIPELINE_MISMATCH` (versions/resource identities), `PIXEL_BUDGET`,
`RESOURCE_NOT_READY`, `SCENE_INVALID`, `CANCELLED`, `UNSUPPORTED_FEATURE`,
`RESOURCE_LIMIT`, `TEXT_SHAPING_FAILED`, `VALIDATION_FAILED`, `ENCODE_FAILED`.
Strict checks **all manifest resources**, including unused entries; missing or
invalid bytes fail readiness. `MISSING_GLYPH` is advisory in strict mode: the
shared text engine positions `.notdef`, without system fallback. Non-strict mode
uses recognizable self-owned bitmap labels, diagnosed as
`PLACEHOLDER_BITMAP_LABEL`; it does not pretend to reproduce a system-font label.
`inspect_resources` shares hash/version checks and preparation with `render`, but
does not allocate a raster or enforce a PNG viewport budget. SVG-only native
exports use it to retain decode readiness and reject unsupported resources.

## Drawing coverage and bounds

- Transparent target; finite scale `(0,4]`; ceil dimensions; checked 16M target
  pixels before allocation. Scenes are bounded to 2048 nodes/resources, depth 128,
  3 MiB transformed line text; malformed radii/cycles/colors fail, never panic.
- CSS corner reduction, elliptical border radii, inside/center/outside rings,
  multi-color sides, transparent-first side order and AA seam pushes. Strokes
  paint after descendants, outside the node's own clip, within ancestor clips.
- Linear/radial elliptical gradients, stable hard stops, premultiplied-sRGB stop
  correction; inherited AA clips. CSS group/text-overlap opacity uses conservative
  subtree extents and integer-device offsets. Simultaneous live opacity layers
  are capped at min(option,16M) pixels; overflow folds alpha and increments the
  fallback counter rather than pretending group compositing succeeded.
- Reverse-order outer/inset shadows, spread-radius correction, conservative
  extents and device-space separable three-box Gaussian approximation. Shadow
  scratch work is independently capped at 16M pixels (not included in opacity
  layer stats); oversized scratch work fails explicitly with `PIXEL_BUDGET`.
- Text uses `Font::positioned_run`, `shaping_font_at_size`, `ScalerFont`, `DrawMemory`, `PathSink` from
  `opencanvas-text`; positions include Blink-quantized advances and tracking.
  Only scene lines are drawn, at their supplied alphabetic origins. Decoration
  rounding follows JS Math.round. No duplicate shaper, wrapping, transformation,
  font lookup, synthetic bold/italic, or spike adapter. The size-specific font
  instance matches the shared shaper's automatic optical-size axis selection.
- PNG/JPEG/WebP decode, preserved alpha, EXIF orientation, bilinear fit/contain/
  cover and normalized crop. Embedded RGB/gray ICC profiles convert to sRGB;
  CMYK/YCCK JPEG ICC uses original conventional ink channels, then converts to
  sRGB (never applying a CMYK profile to already-decoded RGB).
- Uploaded image bytes ≤64 MiB, decoded image ≤16M pixels, profile ≤4 MiB;
  decoder allocation limit 128 MiB (codec-dependent best effort). Font decoding
  has the stricter limits documented in `../text/README.md`.

## Exact direct dependencies / licenses

| Dependency | Pin | License |
| --- | --- | --- |
| opencanvas-layout / opencanvas-text | local 0.1.0 | repository MIT |
| tiny-skia, std/simd, **no default features** | 0.12.0 | BSD-3-Clause |
| image, png/jpeg/webp, **no default features** | 0.25.10 | MIT OR Apache-2.0 |
| jpeg-decoder, platform_independent, **no rayon/default features** | 0.3.2 | MIT OR Apache-2.0 |
| moxcms, lut, **no default features** | 0.8.1 | BSD-3-Clause OR Apache-2.0 |
| png | 0.18.1 | MIT OR Apache-2.0 |
| serde, derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 |
| sha2 | 0.10.9 | MIT OR Apache-2.0 |

Font shaping/outline versions remain owned by `opencanvas-text` (HarfRust 0.15.0,
Skrifa 0.49.0); this crate consumes its public reexports. Tests reuse the existing
OFL NotoSans fixture; image/JPEG/profile fixtures are constructed in test code.
No generated locks, images, JSON, logs, or WASM binaries belong in this crate.

## Focused verification and remaining gates

```sh
cargo test -p opencanvas-render --locked --offline
cargo clippy -p opencanvas-render --all-targets --locked --offline -- -D warnings
cargo build -p opencanvas-render --target wasm32-unknown-unknown --release --locked --offline
rustfmt --edition 2021 --check crates/render/src/*.rs crates/render/tests/unit/*.rs crates/render/tests/parity/fixture.rs
node crates/render/tests/parity/compare.mjs
```

The source-only parity harness creates a temporary validation crate under the
approved parent temporary directory, generates its lock offline, then uses locked
builds. It runs the **real render crate** natively and in Node WebAssembly, compares
geometry/layer counters and every premultiplied RGBA byte for four fixtures:
gradient/corner/multicolor stroke/clip, opacity/shadows, positioned text/decoration,
and ICC PNG/crop. Generated artifacts stay outside the repository.

This is **not Chromium parity acceptance**. The outside-text `Δ>8 <0.5%` gate and
representative Chrome | Rust | diff text images belong to independent parity work.
Server integration is exercised by `crates/server/tests/native_*.rs`, including an
actual built binary with empty PATH/workspace, text/image PNG downloads, node
exports and strict readiness failures. Default-Chromium regressions remain in
`render.rs`, `export_nodes.rs` and `export_svg.rs` in the same tests directory. Do not mark
Task11 complete from server tests, crate unit tests or native/WASM
agreement alone. AA edge coverage, Gaussian approximation, image downsampling,
and CoreText glyph rasterization may still need measured adjustments.

Executed 2026-11-04: 28/28 unit tests, strict focused Clippy, native compilation,
WASM release build, Rust formatting, JS syntax and scoped diff checks passed.
Native↔WASM: four fixtures, 69,120 pixels, max premultiplied RGBA delta **0**;
dimensions and opacity counters also identical. No Chromium pixel gate was run
by this implementer. The parity harness does not benchmark WASM speed.

Explicit unsupported cases: ICC channel spaces other than RGB/gray or CMYK JPEG;
fonts with COLR/CBDT/sbix/SVG color tables (rejected, never painted monochrome),
visible non-outline glyphs; text gradient scenes (not a valid current
scene-3 authored feature). Such inputs must fail rather than silently omit paint.
No OS color/font fallback is attempted. Missing-glyph `.notdef` remains supported.

### Native server integration API delta

Scale is now f64 end-to-end until canvas transforms: 40×2.4 is 96, 320×0.3 is 96,
and a genuinely fractional 133.5×3 still rounds up to 401. No general epsilon or
round-to-nearest workaround is applied. Decode errors retain observed readiness
in `RenderError.readiness`; SVG-only exports call `inspect_resources` using the
same preparation, with no PNG target allocation. The server retains original
hash-checked binaries for a whole export job, but the pure crate still reparses
fonts/images on each render/inspection call (no Prepared resource reuse API).
