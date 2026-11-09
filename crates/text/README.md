# Native text foundation

Pure Rust, caller-owned bytes. No filesystem/network access, system font discovery,
thread spawning, Tokio, or implicit fallback. Layout is used **only for wire types**;
layout does not depend on this crate. Nothing changes the default Chromium backend.

## Integration API

```rust,ignore
let font = Font::from_bytes(upload_bytes, &manifest.hash)?
    .with_face(&manifest.family, manifest.weight, &manifest.style)?;
let mut fonts = FontSet::new();
fonts.insert(&manifest.id, font)?;
let measurement = fonts.measure(&request)?; // layout::Measurement, no adapter

let font = fonts.get(&scene_font_id).ok_or(TextError::FontMissing)?;
let run = font.positioned_run(&line.text, font_size, letter_spacing)?;
let scaler = ScalerFont::unhinted(font.shaping_font_at_size(font_size)?, None);
// scaler.outline(glyph.id).draw(DrawMemory::Scratch, &mut sink)
// Sink transforms FONT-UNIT outlines with size / units_per_em, flips outline Y,
// then adds glyph.x/y (already px, Y-down), line origin and alphabetic baseline.
```

Exact public signatures (all types available from `opencanvas_text`):

```text
Font::from_bytes(bytes: impl AsRef<[u8]>, expected_hash: &str) -> Result<Font, TextError>
Font::with_face(self, family: impl Into<String>, weight: u16,
                style: impl Into<String>) -> Result<Font, TextError>
Font::hash(&self) -> &str
Font::face(&self) -> Option<&Face>
Font::sfnt_bytes(&self) -> &[u8]
Font::shaping_font(&self) -> &harfrust::Font
Font::shaping_font_at_size(&self, size: f32) -> Result<harfrust::Font, TextError>
Font::positioned_run(&self, text: &str, size: f32, spacing: f32)
    -> Result<PositionedRun, TextError>
Font::metrics(&self, size: f32) -> Result<FontMetrics, TextError>
Font::ink_bounds(&self, run: &PositionedRun, size: f32) -> Result<Bounds, TextError>
Font::missing_codepoints(&self, text: &str) -> Result<Vec<u32>, TextError>
FontSet::new() -> FontSet
FontSet::insert(&mut self, manifest_id: impl Into<String>, font: Font)
    -> Result<(), TextError>
FontSet::get(&self, manifest_id: &str) -> Option<&Font>
FontSet::measure(&self, request: &MeasureRequest) -> Result<Measurement, TextError>
```

`Face { family: String, weight: u16, style: String }` contains explicit manifest
descriptors, matching CSS FontFace labels, **not** OS-installed family names or a
requirement that a font's internal family name equal its manifest alias. Weights
100–900 and normal/italic are validated; every request must match all descriptors,
original hash and ID exactly. No synthetic bold/italic, nearest-face lookup or
fallback. Rebinding an ID to different bytes/descriptors is rejected. Font/FontSet
are cheaply cloneable. `FontSet` is default-constructible.

`PositionedRun { glyphs: Vec<PositionedGlyph>, advance: f64 }`;
`PositionedGlyph { id: GlyphId, cluster: u32, x: f64, y: f64,
x_advance: f64, y_advance: f64 }`. Coordinates/advances are logical pixels, Y-down;
clusters index UTF-8 bytes in the supplied transformed line. Advance includes
tracking, including trailing character spacing. Do not apply tracking again when
painting. Canvas accumulates script-item widths as float; `advance` can therefore
differ from an f64 sum of glyph advances. Positions already include that item-origin
rounding. Use `shaping_font().units_per_em()` for unscaled outline conversion.
The exact matching `ScalerFont`, `DrawMemory`, `PathSink`, `GlyphId` are re-exported.
`shaping_font()` supplies the manifest-weight instance without automatic optical
size (an unbound font remains the default instance). **Painters must use
`shaping_font_at_size(size)`** for the automatic optical-size instance shared by
positioned runs; otherwise Inter/SF outlines differ even when advances match.

`FontMetrics { ascent: f32, descent: f32 }` uses integer-rounded hhea line extents
of the same optical-size instance, not `head`'s all-glyph bounding box. This matches
the pinned macOS Chromium corpus's empty-string font bbox. Baseline
is ascent + floor((lineHeight − ascent − descent)/2). Ink extents use unhinted
outlines at the shared run's positions, not rasterized CoreText/Skia bounds. Like
Canvas, a negative-advance run conservatively extends its left bound to that
signed advance; glyphs are not moved or painted into this extra metric area.

## Identity, decoding, limits and errors

- SHA256 is checked against **original upload bytes before** WOFF2 decoding. The
  caller supplies a 64-character lower-case hex hash. `hash()` retains this hash;
  `sfnt_bytes()` exposes decompressed, structurally validated single-face SFNT.
- TTF/OTF SFNT: directory range/overlap/duplicate-tag and required-table checks.
  This is not an OpenType Sanitizer security certification or a rewrite of tables.
- WOFF2: pure-Rust `wuff`, bounded fixed-output Brotli decode and directory-derived
  expansion/reconstruction cap 128 MiB; compression ratio ≤100. Upload cap 32 MiB.
  WOFF1/collections are deliberately rejected, not silently treated as TTF.
- Authored request text ≤64 KiB UTF-8 (storage's existing bound); transformed runs
  ≤192 KiB. Missing scalar values are sorted/unique and bounded by this text limit.
  Fit candidate cache ≤256 strings of ≤4096 bytes; no quadratic retained prefixes.
- Glyph absence is **not an error**: `.notdef` is shaped/advanced; Measurement's
  `missing` feeds layout's existing `MISSING_GLYPH` diagnostics. NFC-per-grapheme
  coverage excludes whitespace, controls and Unicode default-ignorables.
- `TextError::code()` maps hash mismatch to `RESOURCE_HASH_MISMATCH`, decode or
  absent exact face to `FONT_MISSING`, bad hash/face/request to `VALIDATION_FAILED`,
  size limits to `RESOURCE_LIMIT`, shaping failure to `TEXT_SHAPING_FAILED`.
  No error contains font bytes or filesystem paths. Missing coverage is never
  reported as ready through a fabricated fallback font.

## Semantics and remaining parity work

- One HarfRust positioned run applies float nominal-advance truncation to 16.16
  **per glyph**, retains scaled kerning/offsets, and drives both measurement/paint.
  Nonzero tracking disables optional liga/clig; visible logical script clusters
  receive spacing, not standalone default-ignorables such as WJ/ZWSP/BOM.
- Width/fit uses **ceil64**, not the old spike round64. LF/CR normalization, only
  TAB/FF/VT/ASCII-space collapse, NBSP/NNBSP/WJ/BOM word gluing, and emergency
  grapheme breaks match the current renderer algorithm. Empty paragraphs persist.
- Full upper/lower casing includes expansions and contextual Greek final sigma;
  capitalize uses simple single-scalar titlecase (ß/ﬃ stay unchanged), without
  lowercasing word tails. Canvas-only segment-separator normalization happens in
  shaping, not in authored or reported line strings. Zero line height is valid.
- Manifest `weight` selects a real `wght` axis when present; static faces are not
  emboldened. Weight and automatic `opsz` use clamped fvar values, f64 avar1
  mapping and one nearest-2.14 conversion. Optical selection preserves every
  other instance coordinate. Measurement, font metrics, ink and outline painting
  use the same instance through the unchanged public APIs. Complete user-space
  coordinates are retained for the dependency's avar2 inter-axis path.
- Automatic `opsz` uses CSS px and a single nearest-2.14 normalization after f64
  interpolation of avar1. The library's 16.16 intermediate can double-round Inter
  and flip ceil64 fit. No font-name/hash exceptions, OS calls or width epsilon.
  avar2 retains the dependency's implementation and is not corpus-verified.
- Fresh Chromium corpus: **3,366/3,366** eligible measurement geometries exact;
  56 actual missing-glyph samples listed separately. Supplemental size/tracking/
  invisible probes: **2,160/2,160** exact, no exclusions. H2 also makes the original
  H1 stress corpus's **5,220/5,220** eligible geometries exact (72 actual missing),
  including all 131 former failures. Four focused Rust tests and a 420-eligible-case
  targeted capture cover the causes. All source strings remain intact.
  See `spikes/native-render-parity/PARITY.md` for reproduction and ink mismatches.
  This does **not** establish full-scene bounds, PNG or global Task11 acceptance.
- LTR script itemization uses HarfRust's Unicode Script properties, resolves
  leading/trailing Common/Inherited text and matching bracket scripts. Greek/Latin
  `i̇` now gets Latin GPOS attachment;
  original byte clusters, tracking order and optical instance remain shared.
  This is not paragraph bidi or full Script_Extensions resolution. A mark with
  a strong script different from its base begins a logical shaping item, even inside
  an extended grapheme. Tracking uses each item's local graphemes; wrapping still
  preserves the full extended grapheme. Float script-width accumulation fixes the
  long mixed-script ceil64/wrap differences without changing HVAR/optical math.
  See PARITY's H2 section, not a global pass.
  Arbitrary authored variable axes and synthetic styles are not implemented.
  True ital/slnt-axis selection has no captured fixture here and remains unverified;
  descriptor-only italic controls on Inter/static Lato do not synthesize outlines.
  V1 adds **567/567** exact variable/control/boundary geometries and repairs the
  original six-request weight probe; all **11,166** prior eligible geometries and
  their raw probes/ink are unchanged. See PARITY's V1 section. Ink remains
  strict/non-identical; H1 ink max is now 0.006938934326171875 px, not 3.5 px.
- WOFF2 success tests encode the bundled OFL TTF with untransformed tables. The
  decoder's transformed-glyf path and a successful CFF OTF fixture still need
  integration corpus coverage; malformed inputs are explicitly tested.

## Dependency pins and licenses

| Dependency | Version | License |
| --- | --- | --- |
| harfrust | =0.15.0 | MIT |
| skrifa | =0.49.0 | MIT OR Apache-2.0 |
| read-fonts | =0.46.0 | MIT OR Apache-2.0 |
| wuff | =0.2.9 | MIT |
| sha2 | 0.10 (lock: 0.10.9) | MIT OR Apache-2.0 |
| unicode-segmentation | =1.13.3 | MIT OR Apache-2.0 |
| unicode-normalization | =0.1.26 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| unicode-case-mapping | =1.0.0 | Apache-2.0 |
| unicode-bidi (bracket property data only) | =0.3.18 | MIT OR Apache-2.0 |
| brotli (test-only) | =8.0.2 | BSD-3-Clause AND MIT |

HarfRust/skrifa/read-fonts match the spike and share **one** read-fonts version.
The already-cached unicode-bidi supplies Unicode paired-bracket classification
for Common script resolution, not a second shaper or paragraph bidi engine.
Test-only serde_json enables `float_roundtrip` to read exact captured references.
The bundled font remains `renderer/fonts/NotoSans.ttf` under OFL-1.1; tests include
its bytes, never read user data or generate assets in `~/.opencanvas`.

```sh
cargo test -p opencanvas-text
cargo test -p opencanvas-text --test captured_scripts -- --ignored
cargo test -p opencanvas-text --test captured_h2 -- --ignored
cargo test -p opencanvas-text --test captured_v1 -- --ignored
cargo clippy -p opencanvas-text --all-targets -- -D warnings
cargo fmt -p opencanvas-text --check
cargo build -p opencanvas-text --target wasm32-unknown-unknown
```
