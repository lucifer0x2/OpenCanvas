# G2 native text comparison — 2026-11-04

**G2 geometry passes its original corpus, not global Task10/11 AC.**
H1 below records its historical script/mark blockers; H2 fixes all 131 geometry
failures in that unchanged stress capture. Strict ink equality is still false.
No tolerance, numeric rounding or expanded fallback exclusion is used. Full
scene/Taffy bounds and PNG pixels are not measured by this harness. Ink is still
non-identical and is reported separately, including genuine mark-placement gaps.

## Reproduce (repository root)

Keep the G1 capture (`out/oracle.json`, exact original `out/fonts/` bytes); the
adapter checks every original SHA256 before decoding. To replace the capture,
use the pinned-browser commands in README, including `--sf-trak` for six faces.

```sh
cargo run -p opencanvas-text --example oracle
node spikes/native-render-parity/compare.mjs
node spikes/native-render-parity/probe.mjs
cargo run -p opencanvas-text --example oracle -- \
  spikes/native-render-parity/out/probe-oracle.json \
  spikes/native-render-parity/out/probe-native.json
node spikes/native-render-parity/compare.mjs \
  spikes/native-render-parity/out/probe-oracle.json \
  spikes/native-render-parity/out/probe-native.json \
  spikes/native-render-parity/out/probe-comparison.json
```

CLI exit status checks **measurement geometry + metadata + errors only**, not
ink or whole-scene acceptance. Each output also contains explicit independent
gates. The comparator requires the native report's oracle hash to match the
exact input capture. Rust measurements serialize through `serde_json::Value`,
preserving exact f32→JS Number values; possible f32-only differences are labeled,
never rounded away or counted as equal. Line text/x/y/width, box width/height and
ascent are compared independently of baseline-relative ink x/y/width/height.

Ignored outputs preserve every request/result, raw run advances, source hashes,
errors, missing metadata, exclusions and every mismatch by font/group/case/key.
No proprietary SF binary or generated JSON is committed.

## Results

macOS arm64; Chromium **156.0.8078.4** / Playwright **1.64.1**, scene-3/web-4.
Same exact source bytes/manifests as G1 (README has hashes/licenses).

| Face | Primary requests | Actual missing excluded | Exact geometry | Supplemental exact |
|---|---:|---:|---:|---:|
| Noto Sans | 1,698 | 8 | 1,690/1,690 | 360/360 |
| Lato Regular | 346 | 8 | 338/338 | 360/360 |
| Lato Bold | 346 | 8 | 338/338 | 360/360 |
| Lato Italic | 346 | 8 | 338/338 | 360/360 |
| Inter variable | 343 | 12 | 331/331 | 360/360 |
| SF byte probe (`trak`) | 343 | 12 | 331/331 | 360/360 |
| **Total** | **3,422** | **56** | **3,366/3,366** | **2,160/2,160** |

Maximum geometry error **0**, zero native errors, zero missing/coverage metadata
mismatches. All 1,232 legacy advance requests and 120 wrap requests remain
present. Supplemental 540 originals + 1,620 ceil64 ±1/64/exact boundaries test
12/14/16/17.5/24/32 px, tracking 0/−2/+0.5, kerning/ligatures, Vietnamese,
mixed scripts, WJ/ZWSP. It caught 288 tracking failures before the correction.

The SF file really contains `trak` and is loaded by original bytes through a
private FontFace, not an installed family. No manual trak delta is added: the
captured Canvas widths match the shared shaper at these sizes/spacing settings
after automatic optical sizing. This does not establish arbitrary SF/OS/text
coverage or a general CoreText replacement.

Only **actual missing scalars** exclude a request. `declared-platform-fallback`
alone cannot exclude a native-covered request. `comparison.json.excluded` lists
all 56 with original text, browser missing and native missing arrays; these
arrays agree. No fallback exception was silently added. No font-load failure is
classified as missing glyphs.

## Evidence-driven corrections

1. Canvas shapes U+2028/U+2029 as U+0020 while preserving line source and byte
   clusters; TAB/LF/CR/FF receive the same Canvas shaping normalization.
2. CSS capitalize uses simple titlecase: ß/ﬃ do not expand to Ss/Ffi. Full
   uppercase/lowercase expansion and final sigma remain unchanged.
3. A zero line height is accepted and uses the same font-bbox baseline formula.
4. Automatic optical sizing selects `opsz = CSS px`, clamped by fvar; avar1
   interpolation is done before one nearest-2.14 conversion. The dependency's
   16.16→2.14 double rounding made Inter's 2/18 coordinate 1821 instead of 1820,
   producing real 1/64 widths/wrap flips. Fractional HVAR nominal advances are
   retained instead of passing through an unscaled integer HarfBuzz advance.
5. Default-ignorable-only graphemes (WJ/ZWSP/BOM) do not get separate tracking.

No per-font hashes/names, case-specific width patches, epsilon, system fonts,
CoreText calls, browser calls or I/O were added to the runtime library.

**Painter integration:** existing public signatures are unchanged. New additive
`Font::shaping_font_at_size(size)` supplies the same optical instance for
`ScalerFont::unhinted`; using old `shaping_font()` directly for variable outlines
is now known incorrect. There is still one positioned-run implementation.

## G2 baseline ink (before H1; retained evidence)

| Face | Primary exact ink / eligible | Primary max abs error (px) |
|---|---:|---:|
| Noto | 92/1,690 | 0.0001220703125 |
| Lato Regular | 8/338 | 0.00005173683166503906 |
| Lato Bold | 8/338 | 0.000020757317543029785 |
| Lato Italic | 8/338 | 0.000034809112548828125 |
| Inter | 128/331 | **3.7978153228759766** |
| SF | 326/331 | 0.0000705718994140625 |

Primary: **570 exact / 2,796 non-identical ink**. Small geometric float errors
are still mismatches, not an invented ink tolerance. Inter's larger difference
is real: `current/inter/86` (`lowercase/23`), line `ς i̇`, width of ink expected
15.574615478515625 vs native 19.3724308013916. The entire LTR run is script-guessed
as Greek; it lacks browser-like script itemization/mark attachment for Latin i̇.
`current/inter/87` and `current/inter/84/ceil64-below` also retain this gap.
Supplemental mixed-script probes retain it (max ink error **6.609375 px**);
SF supplemental ink max is **0.00909423828125 px**. Full rows are retained.

These are unhinted geometric bounds versus Canvas ink metrics, **not raster
pixel differences**. Do not erase them with a text-box mask or declare all text
paint equivalent. Whole-scene bounds, render/export integration, raster review,
paragraph bidi/script itemization, avar2, transformed-glyf WOFF2 and successful
CFF OTF fixture coverage remain unproven here. No global Rust acceptance follows.

## Focused checks

```sh
cargo test -p opencanvas-text
cargo clippy -p opencanvas-text --all-targets -- -D warnings
cargo fmt -p opencanvas-text --check
cargo build -p opencanvas-text --target wasm32-unknown-unknown
node --test spikes/native-render-parity/oracle.test.mjs spikes/native-render-parity/compare.test.mjs
git diff --check -- crates/text spikes/native-render-parity
```

22 Rust tests and 9 JS tests pass; strict Clippy, fmt, diff check and WASM build
pass. WASM **buildability only**, not native↔WASM measurement/pixel execution.
New/modified Rust source files are at most 136 lines; native adapter/examples
and JS comparator/probe each stay under 200 lines. No root manifest/lock manual
edits, layout/server/render edits, commit, push, deploy or user-data access.

## H1 — focused LTR script/mark fix, 2026-11-04

**The Inter attachment bug is fixed; broader stress parity is still blocked.**
The G2 baseline above remains recorded: geometry 3,366 + 2,160 exact, primary
ink max 3.7978153228759766, supplemental ink max 6.609375. Original G1/G2 output
files were not replaced; H1 writes separate ignored `out/h1-*` reports.

`src/itemize.rs` uses HarfRust's existing public Unicode Script classifier,
coalesces extended graphemes, resolves leading Common/Inherited to the first
strong script and other neutrals to the preceding script. Matched Common
brackets recover their opener's script (including nested/non-ASCII brackets).
The cached, pinned MIT/Apache-2.0 `unicode-bidi = 0.3.18` supplies bracket property
data only; no bidi algorithm or second shaper is invoked. Script_Extensions and
paragraph bidi remain outside this focused fix.

`Font::positioned_run` remains the **one** measurement/paint implementation and
all public signatures are unchanged. Each item sets its script explicitly;
glyph positions accumulate in source order, clusters retain original global
UTF-8 byte offsets, and tracking ends at the item's boundary rather than the
whole text. Optical coordinates, nominal truncation and kerning math are intact.
The adapter hashes the new module too. No font-name/hash/string branches, width
epsilon, system-font discovery or runtime I/O were introduced.

The exact captured Inter regression now has base/mark clusters `[0,2,3,3]`,
Latin base x **13.47137451171875**, dot x **13.484390258789063**, dot advance **0**,
and run advance **17.282150268554688**. Ink width is now **15.574670791625977**,
not the old **19.3724308013916**; Canvas remains **15.574615478515625**. The small
remaining discrepancy is asserted as non-equal, not clamped or tolerated away.
The opt-in test loads exact original bytes only through test filesystem I/O;
it does not bundle ignored captured fonts. Test-only serde_json `float_roundtrip`
ensures captured reference numbers are parsed exactly.

### H1 results

| Capture | Exact geometry / eligible | Excluded actual missing | Exact ink | Max ink error (px) |
|---|---:|---:|---:|---:|
| Existing primary | **3,366/3,366** | 56 | 570 | **0.007822036743164062** |
| Existing supplemental | **2,160/2,160** | 0 | 450 | **0.00909423828125** |
| New H1 stress + boundaries | **5,089/5,220** | 72 | 1,378 | **41.84185791015625** |

Existing primary and supplemental have zero native errors, coverage mismatches
or geometry error. Direct comparison against retained G2 native reports found
**0 changed probes out of 3,422 primary + 2,160 supplemental**: full transformed
source, raw full/line advances and font metrics are exactly unchanged. Primary
ink max is now Inter `current/inter/53` line 9 height, not the attachment case.
Inter supplemental ink max is 0.00780487060546875; overall max is SF's retained
0.00909423828125. All ink non-equalities remain strict mismatches.

New H1 capture contains 26 strings × six faces × three sizes × three tracking
values = 1,404 originals plus 3,888 ceil64 below/exact/above probes. It includes
Greek/Latin/Cyrillic transitions, neutral spaces/punctuation, nested brackets,
inherited/stacked/isolated marks, script-specific marks, invisibles, astral text,
and a 24-repeat stress line. Its comparator **exits 1**, not a passing gate:

- **120 geometry failures** are `crossScriptMark` (`α a҃ α`) and its boundaries.
  U+0483 has a different strong script from its Latin base. H1 intentionally
  keeps the grapheme intact; Canvas adds another tracking unit at this script
  boundary. Original width deltas are +2 / −0.5 px at tracking −2 / +0.5, with
  downstream wrap/height differences. No blanket per-mark correction was added.
- **11 geometry failures** are long `stress` originals/boundaries. The Inter
  16px/−2 original crosses ceil64 by 1/64; full raw advance is
  1004.2100830078125 vs Canvas 1004.2103881835938. The precise accumulation cause
  needs separate investigation; it was not patched with an epsilon/f32 clamp.
- Geometry max 44px is a wrap/line-height consequence, not a glyph displacement.
  Unwrapped new-probe ink max is **3.5px** (isolated marks with negative tracking);
  all-lines max **41.84185791015625px** includes differing wraps. These remain
  explicit gaps, not float-noise claims. All 72 exclusions are actual missing
  scalars, with matching browser/native metadata and zero native errors.

### H1 reproduction and checks

```sh
# Reuse the exact G1 out/oracle.json and out/fonts; preserve G2 output files.
cargo run -p opencanvas-text --example oracle -- spikes/native-render-parity/out/oracle.json spikes/native-render-parity/out/h1-primary-native.json
node spikes/native-render-parity/compare.mjs spikes/native-render-parity/out/oracle.json spikes/native-render-parity/out/h1-primary-native.json spikes/native-render-parity/out/h1-primary-comparison.json
cargo run -p opencanvas-text --example oracle -- spikes/native-render-parity/out/probe-oracle.json spikes/native-render-parity/out/h1-probe-native.json
node spikes/native-render-parity/compare.mjs spikes/native-render-parity/out/probe-oracle.json spikes/native-render-parity/out/h1-probe-native.json spikes/native-render-parity/out/h1-probe-comparison.json
node spikes/native-render-parity/h1-audit.mjs
node spikes/native-render-parity/h1-probe.mjs
cargo run -p opencanvas-text --example oracle -- spikes/native-render-parity/out/h1-oracle.json spikes/native-render-parity/out/h1-native.json
# Expected exit 1 until the explicitly retained stress failures are fixed.
node spikes/native-render-parity/compare.mjs spikes/native-render-parity/out/h1-oracle.json spikes/native-render-parity/out/h1-native.json spikes/native-render-parity/out/h1-comparison.json
cargo test -p opencanvas-text
cargo test -p opencanvas-text --test captured_scripts -- --ignored
cargo clippy -p opencanvas-text --all-targets -- -D warnings
cargo fmt -p opencanvas-text --check
cargo build -p opencanvas-text --target wasm32-unknown-unknown
node --test spikes/native-render-parity/oracle.test.mjs spikes/native-render-parity/compare.test.mjs
```

27 regular Rust tests pass (one captured test normally ignored), plus that one
captured test explicitly passes; nine existing JS tests pass. Strict Clippy,
fmt and WASM build pass. H1 Rust modules/tests/example are 30–141 lines; JS
probes/audit are 13/60/25 lines, all under 200. No full workspace suite was run. WASM is
buildability only. Paragraph bidi, Script_Extensions, exact ink, full-scene bounds,
raster acceptance, avar2, transformed-glyf WOFF2 and a successful CFF OTF fixture
remain unproven. No global Canvas/Task10/Task11 acceptance is claimed. No
server/render/API/other paint-spike edits, commit, push, deploy or user-data writes.

## H2 — existing H1 stress corrections, 2026-11-04

**All 131 original eligible H1 geometry failures are fixed.** No H1/G2/root
capture was replaced. Measurement geometry is exact in all retained corpora;
strict ink equality and whole-scene/raster acceptance are **not** established.

### Root causes and bounded corrections

- **120 strong-script-mark failures:** shaping items now follow logical Unicode
  Script boundaries, including a Cyrillic U+0483 after a Latin base. Inherited
  marks still join their base. Tracking counts visible local graphemes within
  each shaping item, not one extra unit for every combining mark. The wrap
  implementation is unchanged and still preserves the complete extended
  grapheme, even when a shaping item begins inside it. Original UTF-8 byte
  clusters and full original/transformed source strings remain intact.
- **11 long-run/boundary failures:** Blink stores each script-run width and
  accumulates those widths as float (`ShapeResult::InsertRun` and
  `ApplySpacingOrExpansion`). H1's all-f64 sum lost that operation order. H2
  retains exact fixed-point per-glyph advances and applies float accumulation
  only at script-item boundaries, also using those origins for positioned glyphs.
  Inter 16px/−2 stress is now exactly **1004.2103881835938**, not
  1004.2100830078125. The existing fractional HVAR, nominal advance truncation,
  optical coordinates and GPOS arithmetic are unchanged. This is not epsilon,
  ceil64 bias or an advance correction selected by font/string.
- **Isolated marks with negative tracking:** inspection of Blink
  `PlainTextNode::Shape` identified conservative overflow metrics, not a 3.5px
  mark displacement. For negative measured advance, Canvas extends the left
  bound to that signed advance while keeping the right edge. `ink_bounds` now
  reproduces that general rule for nonempty ink. Glyph x/y/outlines remain
  unchanged, and no pixels are filled into that metric-only area. Tests compare
  actual glyph positions before/after tracking separately from this expansion.

Runtime remains pure Rust with one shared positioned run and no added dependency,
OS/font discovery, browser call, I/O, thread, font-name/hash branch or public API
change. `FontSet` and `shaping_font_at_size` remain unchanged.

### H2 results against exact retained captures

| Capture | Exact geometry / eligible | Actual missing excluded | Exact ink | Max ink error (px) |
|---|---:|---:|---:|---:|
| Existing primary | **3,366/3,366** | 56 | 570 | **0.007822036743164062** |
| Existing supplemental | **2,160/2,160** | 0 | 450 | **0.00909423828125** |
| Unchanged original H1 stress + boundaries | **5,220/5,220** | 72 | 1,398 | **0.006938934326171875** |
| New bounded H2 root-cause regressions | **420/420** | 12 | 30 | **0.002593994140625** |

All four comparisons: zero geometry error, native errors or coverage metadata
mismatches. All comparator CLIs exit **0**. Their independent gates are
`measurementGeometry:true`, **`inkExact:false`**, `fullSceneBounds:not-tested`,
`rasterPixels:not-tested`. Every nonzero ink discrepancy remains a mismatch;
there is no tolerance or clamp to claim ink/raster equivalence.

The bounded H2 capture is seven known-root-cause strings × six original faces ×
three tracking values at 16px: 126 originals + 306 eligible ceil64 boundaries.
It checks same-script vs cross-script marks, inherited/stacked marks, inherited
marks following a strong-script mark, grapheme-atomic wrapping and negative ink.
The 12 exclusions are SF's actual missing U+0483 originals, not a new policy.
No random stress or additional shaper was introduced.

Float operation order intentionally changes raw probes: 179/3,422 primary
and 279/5,292 H1 reports differ from H1; 0/2,160 supplemental probes change.
**Zero transformed source strings change.** Raw-advance equality is not a global
claim: the retained Noto 16px/0 `joiner` probe remains
21.887954711914062 vs Canvas 21.887969970703125, as before H2, with exact ceil64
geometry. All eligible unwrapped root-cause raw advances are asserted exactly.

### Reproduce H2 without overwriting H1/G2/root files

```sh
cargo run -p opencanvas-text --example oracle -- spikes/native-render-parity/out/oracle.json spikes/native-render-parity/out/h2-primary-native.json
node spikes/native-render-parity/compare.mjs spikes/native-render-parity/out/oracle.json spikes/native-render-parity/out/h2-primary-native.json spikes/native-render-parity/out/h2-primary-comparison.json
cargo run -p opencanvas-text --example oracle -- spikes/native-render-parity/out/probe-oracle.json spikes/native-render-parity/out/h2-probe-native.json
node spikes/native-render-parity/compare.mjs spikes/native-render-parity/out/probe-oracle.json spikes/native-render-parity/out/h2-probe-native.json spikes/native-render-parity/out/h2-probe-comparison.json
cargo run -p opencanvas-text --example oracle -- spikes/native-render-parity/out/h1-oracle.json spikes/native-render-parity/out/h2-native.json
node spikes/native-render-parity/compare.mjs spikes/native-render-parity/out/h1-oracle.json spikes/native-render-parity/out/h2-native.json spikes/native-render-parity/out/h2-comparison.json
node spikes/native-render-parity/h2-probe.mjs
cargo run -p opencanvas-text --example oracle -- spikes/native-render-parity/out/h2-oracle.json spikes/native-render-parity/out/h2-targeted-native.json
node spikes/native-render-parity/compare.mjs spikes/native-render-parity/out/h2-oracle.json spikes/native-render-parity/out/h2-targeted-native.json spikes/native-render-parity/out/h2-targeted-comparison.json
cargo test -p opencanvas-text
cargo test -p opencanvas-text --test captured_h2 -- --ignored
cargo test -p opencanvas-text --test captured_scripts -- --ignored
cargo clippy -p opencanvas-text --all-targets -- -D warnings
cargo fmt -p opencanvas-text --check
cargo build -p opencanvas-text --target wasm32-unknown-unknown
node --test spikes/native-render-parity/oracle.test.mjs spikes/native-render-parity/compare.test.mjs
git diff --check -- crates/text spikes/native-render-parity
```

Executed: **31 regular Rust tests**, both new opt-in captured H2 tests, the retained
H1 captured regression, nine JS tests, strict Clippy, fmt and WASM build pass.
WASM is buildability only. New/changed Rust modules and tests remain ≤139 lines;
the H2 capture script is 62 lines. No full workspace test was run. Paragraph bidi,
full Script_Extensions resolution, avar2, transformed-glyf WOFF2, successful CFF
OTF coverage, exact platform ink, whole-scene bounds and raster acceptance remain
outside this focused correction. No server/render/layout/paint edits, root
manifest/dependency changes, commit, push, deploy or user-data writes were made.

## V1 — manifest variable weight instances, 2026-11-04

**The concrete variable-weight bug is fixed, not tolerated.** The root's six
original requests previously failed 4/6 geometries (max 42.390625px); they now
match 6/6. Original G1/G2/H1/H2/root reports remain unchanged; all V1 outputs use
ignored `out/v1-*.json`. This is not global Task10/11 or raster acceptance.

### Cause, instance contract and controls

`Font::with_face` previously bound descriptor labels without selecting `wght`.
Then optical sizing allocated an all-zero coordinate vector, resetting every
axis except `opsz`. Both problems are corrected in `src/font.rs`/`src/optical.rs`:
manifest weight selects the actual axis, fvar clamps user values, f64 avar1
mapping precedes **one** nearest-2.14 rounding, and optical selection preserves
the other instance coordinates. Full user-space locations are retained for the
dependency's avar2 cross-axis path; avar2 parity remains unverified.

The public signatures are unchanged. `shaping_font()` now correctly exposes
manifest weight without automatic optical size; painters continue to use
`shaping_font_at_size(size)`. There is still **one** positioned-run implementation,
used by measurement/ink/paint, with identical weight+optical selection for
unhinted outlines and font metrics. No width adjustment, epsilon, hash/name/case
patch, OS lookup, CoreText/browser call, runtime I/O or dependency was added.

The capture uses actual renderer `loadResources`/`fontCSS`, exact FontFace
descriptors and private hash/weight/style families. Every source SHA is checked;
Inter is pinned to G1 SHA256
`4989b125924991b90d05b2d16e0e388c48f7d5bb8b30539bbf9c755278d0ccaf`.
It reuses G1's original OFL Inter/Noto/Lato bytes and licenses without downloading
or committing font binaries. Runtime descriptors, CSS strings, table inventory,
fvar ranges, avar maps, MVAR tags and hhea/OS2/head facts are retained in V1 JSON.

Inter has `opsz` 14/14/32 and `wght` 100/400/900, with a nonlinear avar1 weight
map. At weights 100/400/500/700/900 the normalized weight bits are
−16384/0/2949/8847/16384; these survive optical-size bits 1820/9102/16384 at
16/24/32px. Tests collect actual outline commands at every weight/size, proving
that outlines change as well as advances. Another unit test independently
preserves a nondefault unrelated axis and checks clamping. Rebinding to weight
400 returns to the default instance without stale coordinates.

Inter has no ital/slnt axis, and the three Lato faces are static. **81 browser
controls** show exact unchanged geometry/raw metrics/ink for Inter's descriptor-only
italic aliases and regular Lato bytes labeled weight700/italic (`F2` control).
Native outline-command tests preserve those bytes without faux bold/italic.
Actual ital/slnt-axis selection was not established or added: probing a licensed
true variable-italic font through this same local FontFace harness is a future
option, not a claim of coverage or an excuse to synthesize static outlines.

Inter MVAR records are `stro/strs/undo/unds/xhgt`; Noto has `stro/xhgt`, neither
varies hhea ascent/descent. All 567 captured empty-string font bbox metrics match.
A synthetic OFL unit fixture renames an existing MVAR record to `hasc`, exercises
real nonzero hhea variation deltas and verifies that `Font::metrics` uses the
selected instance before integer pixel rounding. This fixture is a plumbing
test, not new browser evidence about arbitrary MVAR fonts.

### Exact results, including retained strict differences

| Capture | Exact geometry / eligible | Actual missing excluded | Exact ink | Max ink error (px) |
|---|---:|---:|---:|---:|
| Root's original variable probe | **6/6** | 0 | 2 | **0.007770538330078125** |
| New V1 bounded sweep + boundaries | **567/567** | 0 | 76 | **0.013336181640625** |
| Unchanged primary | **3,366/3,366** | 56 | 570 | **0.007822036743164062** |
| Unchanged supplemental | **2,160/2,160** | 0 | 450 | **0.00909423828125** |
| Unchanged H1 stress + boundaries | **5,220/5,220** | 72 | 1,398 | **0.006938934326171875** |
| Unchanged H2 targeted | **420/420** | 12 | 30 | **0.002593994140625** |

All comparisons have zero geometry error, native errors or metadata mismatches.
V1 has 432 originals: 16 descriptors × three sizes × three spacings × three
semantic strings; 135 ceil64 below/exact/above boundaries cover the root string
at all five Inter normal weights/sizes/spacings. Weights are 100/400/500/700/900,
sizes 16/24/32, tracking 0/−2/+0.5. Noto checks `wght` without `opsz`; controls
include default weight, actual static regular/bold/italic, and alias descriptors.

The root string `AVATAR office Tiếng Việt` now gives width16/width32 of
187/345.078125 (400), 195.1875/367.96875 (700), 202.15625/387.46875 (900).
`v1-audit.json` also proves **zero changed complete measurements or raw probes**
against retained H2 outputs for all 11,306 old requests, including missing-glyph
ones (11,166 eligible). No original source strings were altered.

Strict ink remains **false**: V1 has 491 non-identical ink cases; max is Noto700.
Raw advances are not universally exact either: 324/432 new originals are exact,
108 differ, max **0.0002288818359375px**, despite exact ceil64 geometry and tested
boundaries. These values are retained, not rounded away or excluded. No raster
pixels or whole-scene layout were tested by this focused V1 harness.

### Reproduce and focused checks

```sh
node spikes/native-render-parity/v1-capture.mjs
cargo run -p opencanvas-text --example oracle -- spikes/native-render-parity/out/v1-oracle.json spikes/native-render-parity/out/v1-native.json
node spikes/native-render-parity/compare.mjs spikes/native-render-parity/out/v1-oracle.json spikes/native-render-parity/out/v1-native.json spikes/native-render-parity/out/v1-comparison.json
cargo run -p opencanvas-text --example oracle -- spikes/native-render-parity/out/root-variable-oracle.json spikes/native-render-parity/out/v1-root-variable-native.json
node spikes/native-render-parity/compare.mjs spikes/native-render-parity/out/root-variable-oracle.json spikes/native-render-parity/out/v1-root-variable-native.json spikes/native-render-parity/out/v1-root-variable-comparison.json
# Preserve every pre-V1 capture/output; write only v1-* reports.
for pair in primary:oracle supplemental:probe-oracle h1:h1-oracle h2:h2-oracle; do
  name=${pair%%:*}; input=${pair#*:}
  cargo run -p opencanvas-text --example oracle -- "spikes/native-render-parity/out/$input.json" "spikes/native-render-parity/out/v1-$name-native.json" || exit
  node spikes/native-render-parity/compare.mjs "spikes/native-render-parity/out/$input.json" "spikes/native-render-parity/out/v1-$name-native.json" "spikes/native-render-parity/out/v1-$name-comparison.json" || exit
done
node spikes/native-render-parity/v1-audit.mjs
cargo test -p opencanvas-text
cargo test -p opencanvas-text --test captured_v1 -- --ignored
cargo test -p opencanvas-text --test captured_h2 -- --ignored
cargo test -p opencanvas-text --test captured_scripts -- --ignored
cargo clippy -p opencanvas-text --all-targets -- -D warnings
cargo fmt -p opencanvas-text --check
cargo build -p opencanvas-text --target wasm32-unknown-unknown
node --test spikes/native-render-parity/oracle.test.mjs spikes/native-render-parity/compare.test.mjs spikes/native-render-parity/v1.test.mjs
git diff --check -- crates/text spikes/native-render-parity
```

Executed: **36 regular Rust tests** (31 retained + five new), **five explicitly
run captured tests** (three retained + two V1), **11 JS tests**, strict Clippy,
fmt and WASM build pass. WASM is buildability only; no full workspace suite.
Modified/new Rust modules/tests stay below 200 lines; V1 JS modules are below
100 lines. No root Cargo/lock, layout/server/render, original paint spikes or
agent-log edits, subagent spawning, commit, push, deploy or user-data writes.
Licenses/dependencies are unchanged. Paragraph bidi, full Script_Extensions,
avar2 parity, true ital/slnt-axis coverage, transformed-glyf WOFF2 and successful
CFF OTF fixture coverage remain unproven; exact platform ink and raster/global
Task10/Task11 acceptance are not claimed. Parent session must recheck final
source and update its own progress log.
