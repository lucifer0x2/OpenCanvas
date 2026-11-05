# Task11 bounded paint-parity corrections — P1 → P2 takeover

## Outcome: improved, NOT accepted

Final unchanged G3 run:
[`out/2026-11-04T09-02-57.333Z/report.json`](out/2026-11-04T09-02-57.333Z/report.json).
**88 captures: 71 non-text PASS, 4 non-text FAIL, 13 REVIEW_TEXT.**
The command exited **2**, correctly: four isolated cases still fail the gate.
The original G3 run had 47 PASS / 28 FAIL / 13 REVIEW_TEXT.

The gate is unchanged: straight RGBA8, any channel delta **>8**, divided by
**all output pixels**, with fraction **strictly <0.005**. Alpha always counts;
RGB is ignored only where both alphas are zero. No masks, padding, exclusions,
white-composite substitution, fixture-name branches or threshold changes.
An atlas pass does not override an isolated failure.

Final source hashes before/after are identical (`sourceChangedDuringRun:false`).
All draw counters match. All seven error probes reject on both engines. The
existing hash-mismatch mapping remains Chrome `RESOURCE_NOT_READY` versus native
`PIPELINE_MISMATCH`; this work does not silently equate their codes.
The report contains source/binary/resource hashes and pinned runtime/dependencies.
This conclusion document was added after capture; no paint sources changed after it.

## Retained/corrected production behavior

P2 continued P1's in-progress sources rather than resetting them. No dependency,
server, text-engine, renderer TypeScript, harness-source, product-default or user
data changes were made by this takeover. No commit, push or deployment.

- **Gradients:** explicit device-center sampling, renderer's premultiplied-stop
  expansion, straight stop interpolation followed by premultiplication, and
  ordered RGB dithering before RGBA8 storage. Alpha is not dithered. The formerly
  failing alpha-linear and elliptical-radial cases now have **zero pixels >8**
  at both scales. Hard stops and transparent-stop colors remain semantic.
- **Image filtering:** premultiplied bilinear samples with four-bit weights and
  a single final truncation, then opacity. Sampling is clamped to the decoded
  image; transparent source colors do not create halos. The shader is sampled
  once, not resampled by a second bilinear pattern.
- **Image destination:** an axis-aligned `drawImage` destination has hard
  pixel-center coverage. Its rounded-node and ancestor clips remain separately
  antialiased. The former second AA rectangle caused full-column fringes in
  contain cases. Example: a 65×65 box containing a 2×1 source has destination
  y=[16.25,48.75]; rows 16 and 48 are full coverage, not 75% coverage.
- **EXIF:** decode still physically orients the pixels, but retains which output
  sampling axes reverse the encoded axes. Reversed four-bit weights use the
  opposite rounding direction. Without this, rotating the pixmap changes the
  filter grid by one weight step. All orientation-6 fit/cover/contain cases now
  pass, including both scales; no filename/media-specific workaround.
- **Coverage:** P1's bounded 128×128 tiles integrate 4× mask coverage to A8,
  without supersampling color shaders or allocating a 16× full target. Existing
  clip multiplication, stroke-after-subtree order and opacity-layer behavior
  remain. This improves precision, but does **not** reproduce Skia's analytic AA.
- **Shadows:** P1's three-box plan retains equal first/second windows (opposite
  half-pixel offsets for even windows), an odd final window, A8 quantization
  between axes, and coverage applied to a quantized premultiplied shadow color.
  Existing spread/radius correction, clipping and reverse paint order remain.
- **Cancellation:** tiled coverage now checks cancellation and returns errors
  rather than panicking through an `expect`. Cancellation reaches root, opacity
  layers and shadow scratch targets; mask combination and blur quantization
  loops check periodically. Shader/decode/output/layer limits remain bounded.

Public contracts retained: `RenderOptions.scale: f64`, `RenderError.readiness`,
`inspect_resources`, premultiplied `Raster.rgba`, strict resource/hash/version
validation, no OS-font/CoreText/Chromium fallback, and no crate I/O or threads.

## Final feature results

| Group | PASS / FAIL | Detail |
| --- | --- | --- |
| Solid alpha/opaque + fractional-ceil scale | 7 / 0 | Fractional 2.4 case remains 96×96. |
| Linear/alpha/hard-stop/radial gradients | 8 / 0 | Alpha-linear and radial: zero pixels >8. |
| Independent corners + uniform/per-side strokes | 13 / 1 | Outside multicolor stroke at 1× fails. |
| Nested alpha/clip/post-child stroke | 1 / 1 | 1× fails; 2× passes. |
| Overlapping group opacity | 2 / 0 | Counters also match. |
| Outer/inset/ordered shadows | 12 / 2 | Positive-spread outer and ordered at 1× fail. |
| PNG/JPEG/WebP/orientation/crop, all fits | 26 / 0 | Both scales pass. |
| Full non-text atlas | 2 / 0 | 0.270761% / 0.148329%; not a substitute for isolated cases. |
| Text | 13 REVIEW_TEXT | Never automatic acceptance; consult the strips/H2 review. |

All remaining failures are 1×, each with denominator **38,400 pixels**:

| Isolated case | Pixels >8 | Fraction | Pixels with alpha delta >8 |
| --- | ---: | ---: | ---: |
| `stroke-outside-sides` | 244 | **0.635417%** | 194 |
| `nested-alpha-clip-stroke-after-children` | 220 | **0.572917%** | 136 |
| `shadow-outer-spread-6` | 284 | **0.739583%** | 64 |
| `shadow-order-multiple` | 250 | **0.651042%** | 16 |

Zero-spread outer shadow at 1× passes narrowly (0.492188%); ordered shadows at
2× also pass narrowly (0.476563%). These are measured passes, not robustness
claims for arbitrary scenes or untested scales.

## Remaining blocker and bounded investigation

Pixel probes and the unchanged strips locate stroke/nested errors on curved
outlines and clip seams, including real alpha discrepancies, not merely hidden
RGB. Example outside-stroke pixel (180,21): Chrome `[239,67,67,193]`, native
`[239,68,68,235]`. The current rounded paths are cubic approximations rasterized
by tiny-skia plus integrated coverage; Chromium uses Skia's conic/analytic
scan-conversion/clip machinery. Higher geometric accuracy alone is not an exact
Skia AA implementation. The precise contribution of curve subdivision versus
edge quantization still needs a controlled rasterizer-level investigation.

Shadow errors combine that source-edge coverage difference with A8 blur/color
quantization. Small mask changes are amplified by straight-alpha unpremultiplying:
positive spread pixel (191,20) is Chrome `[20,20,39,13]`, native `[20,39,59,13]`.
In the positive-spread case 117/284 failing pixels have both alphas below 16;
64 still have alpha delta >8. Ordered shadows have 97/250 such low-alpha pixels,
but still 16 alpha failures. These failures cannot be dismissed using white
composites or a low-alpha exception.

A bounded small-sigma true-Gaussian experiment was measured and **discarded**:
it worsened ordered shadows at 1× from 0.651042% to 1.122396%. It is not retained
or claimed as an improvement. No further empirical offset/mask adjustments
were made. Next work should isolate Skia-compatible rounded-path/clip coverage
and blur-mask quantization generically; the remaining four gates block Task11
pixel acceptance. Do not treat this handoff as Task11 complete.

## Checks and reproduction

```sh
cargo test -p opencanvas-render --offline
cargo clippy -p opencanvas-render --all-targets --offline -- -D warnings
cargo fmt -p opencanvas-render --check
cargo build -p opencanvas-render --target wasm32-unknown-unknown --release --offline --locked
node spikes/native-paint-parity/run.mjs --native
```

- Tests: **30 existing unit tests + 7 new public-API paint regression tests pass**.
  `crates/render/tests/paint_pixels.rs` covers gradient premul/dither, bilinear
  quantization, EXIF axes, contain edges, transparent filtering/rounded clip,
  and cancellation during tiled coverage without partial-success publication.
- Strict crate Clippy and crate fmt check pass. `git diff --check -- crates/render`
  is clean, but the entire crate is untracked in the parent worktree; this alone
  is not a diff check of its contents. Fmt/Clippy inspect the actual source.
- WASM release **build passes**. Native↔WASM execution/pixel delta was **not run**
  by P2; a successful build is not the max-delta correctness gate or a speed gate.
- `AGENTS.md`: 69 lines / 9,476 bytes, unchanged by P2. Parent owns progress logs.
- Every render source is ≤250 lines (largest `validation.rs`: 246, unchanged
  semantically here). Changed paint modules are ≤190 lines; new regression file
  is 163 lines. Helpers remain single-purpose: coverage 55, gradient 116,
  image sampling 34. No line-count exceptions needed.

No new dependencies/licenses. Existing pins remain tiny-skia 0.12.0,
image 0.25.10, jpeg-decoder 0.3.2, moxcms 0.8.1, png 0.18.1; full dependency
license metadata is in the run report. **ICC Chromium parity remains uncovered**:
the harness has no licensed bounded ICC original. Existing synthetic ICC unit
tests pass, but do not fill that oracle-fixture gap. Text remains REVIEW_TEXT;
neither missing-glyph behavior nor font shaping/raster parity is newly accepted.
