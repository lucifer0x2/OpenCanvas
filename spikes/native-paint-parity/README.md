# Independent Chromium ↔ native PAINT parity

Scene-only correctness harness for Task11; no server, layout, UI, deployment,
real project data, alternative painter or system-font fallback. The native
adapter calls the public `opencanvas-render::{render, encode_png}` API. Chromium
calls the actual bundled TypeScript renderer's `loadResources` and `drawScene`.

## Reproduce

From repository root with the existing `node_modules` symlink and
`.runtime/browsers` installed:

```sh
node --test --test-concurrency=1 spikes/native-paint-parity/*.test.mjs
node spikes/native-paint-parity/run.mjs
node spikes/native-paint-parity/run.mjs --native
rustfmt --edition 2021 --check spikes/native-paint-parity/native/main.rs
CARGO_TARGET_DIR="$PWD/spikes/native-paint-parity/out/clippy-target" cargo clippy --manifest-path spikes/native-paint-parity/native/Cargo.toml --offline -- -D warnings
```

The first capture command is explicitly **NATIVE_PENDING**, never a parity pass.
The second builds the real native crate offline in an isolated `[workspace]`.
It exits **2** for any non-text gate failure, draw-counter mismatch, source drift,
native/comparison error or probe failure; infrastructure errors exit **1**.
Text is always **REVIEW_TEXT**, even when its numeric fraction is below 0.5%.
All commands leave product defaults and other workers' sources unchanged.

Captures, copied original fonts/licenses, generated image bytes, requests, Cargo
lock/build output, PNGs and report JSON stay ignored under `out/<timestamp>/`.
`out/latest.json` points to the latest run. Runs never overwrite old evidence.
Native adapter accepts only flat `/resources/<filename>` fixture URLs inside its
explicit output root; it cannot fetch URLs or resolve system fonts. It passes
the same original bytes/hash to the crate that Chromium receives over loopback.
Scale deserialization is inferred from `RenderOptions`, not hardcoded to f32.

## Pins and resources

- Chromium **156.0.8078.4**, Playwright **1.64.1**, scene **scene-3**, renderer
  **web-4** are asserted. Browser color profile is sRGB, deviceScaleFactor is 1;
  scene scales are 1/2, with a 2.4 fractional-ceil and 3x small-text probe.
- Actual renderer bundle hash, source hashes before/after, native binary hash,
  dependency versions/licenses, rustc/Node, OS/architecture and resource hashes
  are saved in `report.json`. Native lock is copied into each native run.
- Noto uses `renderer/fonts/manifest.json` and bundled `OFL.txt` with its pinned
  SHA256. Static Lato regular/bold/italic use `fixtures/fonts/README.md` and OFL
  from google/fonts commit `f3b885d5590e307e02542f1a724cec55d567fdaa`.
- Inter is **read-only** from the explicitly authorized G2 capture
  `spikes/native-render-parity/out/fonts/inter.ttf`. Its pinned SHA256 is
  `4989b125924991b90d05b2d16e0e388c48f7d5bb8b30539bbf9c755278d0ccaf`.
  Origin: rsms/inter commit `e3a3d4c57d5ecc01453a575621882a384c1995a3`,
  `docs/font-files/InterVariable.ttf`; OFL is copied and hash-checked as well.
  Missing G2 original bytes cause a visible harness error, never substitution.
- PNG/JPEG/WebP originals are small harness-authored pixel patterns encoded by
  pinned Chromium; they are saved before capture. A fourth resource inserts a
  self-authored minimal EXIF orientation=6 segment into the JPEG. No downloads.
- **ICC coverage is still missing:** no bounded licensed ICC original is in this
  harness's authorized repository fixtures. Do not infer ICC parity from these
  PNG/JPEG/WebP or orientation cases.

## Cases and comparison contract

Isolated cases cover opaque/alpha fills, oblique/alpha/hard-stop linear and
elliptical radial gradients, independent corners, uniform/per-side color strokes
at all alignments, group opacity, nested clips and stroke-after-children, negative/
zero/positive shadow spread, outer/inset/multiple shadows, every image fit and
normalized crop, saved text line lists, tracking, decoration, true static faces
and overlapping text opacity. Full non-text/text atlases supplement isolated
cases; an atlas pass cannot override a failing isolated feature.

The Inter lowercase fixture copies G2 `current/inter/86` (`lowercase/23`)
positions/advances, including the exact `ς i̇` line at baseline 132, width
17.296875, size 16/line-height 23. A separate mixed Greek/Latin/combining-dot
case makes mark attachment inspectable. Paint never transforms or wraps again.
Geometry agreement is not evidence of text raster/attachment agreement.

PNG comparison uses **straight RGBA8**, not premultiplied channels. The denominator
is **all output pixels**, transparent background included, on each case/scale.
A pixel fails if **any channel Δ > 8**; the fraction must be **strictly < 0.005**.
Alpha always participates. Only when **both alphas are zero** is undefined RGB
excluded from the `visibleStraight` gate. Unmodified `rawStraight` and separately
white-composited metrics remain reported; white never replaces the gate. Low,
nonzero-alpha RGB differences are **not** relaxed. Thresholds cannot be overridden.

There are **no masks**, including no broad text-box masks. Text exports full
metrics, Chrome/native/diff and a side-by-side strip; no glyph mask is implemented
or claimed. Diff is opaque RGB, amplified 8x; strip order is Chrome | native |
diff, with original alpha preserved on its first two panels. Review against both
light/dark backgrounds when interpreting low-alpha differences.

`report-schema.mjs` describes `native-paint-parity/1`. Each case stores its
name/category/scale, exact request, scene/PNG hashes, draw counters, diagnostics,
readiness, image paths and verdict. Metrics include total/count/fraction/max Δ,
worst coordinate/channel deltas, per-channel counts and mean Δ. Resource failures
are not omitted: seven source probes cover wrong version, invalid scale, target
budget, missing image, wrong hash, corrupt image and corrupt font. Chrome/native
error codes are reported independently, since actual code mappings can differ.

## Current evidence

Final native run: [report](out/2026-11-04T08-35-37.476Z/report.json).
88 captures: **47 non-text PASS, 28 non-text FAIL, 13 text REVIEW_TEXT**.
All seven probes rejected on both engines; hash mismatch is Chrome
`RESOURCE_NOT_READY` vs native `PIPELINE_MISMATCH`. All draw counters matched;
source hashes did not change during that run. Exit status was **2**, as intended
for failing parity. The fractional scale case passed at **40 × 2.4 = 96** pixels.
This is **not acceptance**.

Representative local strips (same run):

- [Outer shadow](out/2026-11-04T08-35-37.476Z/shadow-outer-spread-0@1/chrome-native-diff.png) — 4.943% of all pixels >8.
- [Alpha gradient](out/2026-11-04T08-35-37.476Z/gradient-alpha@1/chrome-native-diff.png) — 0.737%.
- [Radial](out/2026-11-04T08-35-37.476Z/radial-elliptical@1/chrome-native-diff.png) — 1.628%.
- [PNG cover](out/2026-11-04T08-35-37.476Z/image-png-cover@1/chrome-native-diff.png) — 1.188%.
- [Inter mixed scripts](out/2026-11-04T08-35-37.476Z/text-inter-mixed-greek-latin-combining-dot@2/chrome-native-diff.png) — REVIEW_TEXT.
- [Exact G2 lowercase/23 at 3x](out/2026-11-04T08-35-37.476Z/text-inter-lowercase-expansion@3/chrome-native-diff.png) — REVIEW_TEXT, even though fraction is 0.462%.

Failing isolated cases (percentages at scale1 / scale2; a dash means that scale
passed):

| Feature | Fraction >8, % |
| --- | --- |
| Alpha linear / radial | 0.737 / 0.732; 1.628 / 1.622 |
| Outside multicolor stroke | 0.612 / — |
| Nested alpha/clip/stroke-after-children | 0.529 / — |
| Outer shadow spread -5 / 0 / 6 | 3.099 / 3.128; 4.943 / 5.227; 6.357 / 6.571 |
| Multiple ordered shadows | 3.865 / 3.621 |
| PNG contain / cover / fit | 1.333 / 1.322; 1.188 / 1.109; 1.630 / 1.363 |
| JPEG contain / EXIF-rotated contain | 0.648 / —; 0.531 / — |
| WebP contain | 0.664 / 0.580 |
| Normalized crop | 1.750 / 1.969 |
| Full non-text atlas | 0.936 / 0.879 |

Some low-alpha gradient/shadow straight-RGB failures disappear in the separately
reported white composite (e.g. alpha gradient: white 0%). **They still fail the
required straight-RGBA gate.** PNG/crop and outside-stroke differences also exceed
the threshold when composited white; no alpha policy change would fix those.

Harness checks: **13/13 tests**, all JS syntax checks, native adapter strict
Clippy and rustfmt check passed. Every new source file is ≤85 lines; no file
exceeds 250 lines. Root source, root log and other workers' evidence were not
modified. Re-run after independent paint/text fixes; do not edit the renderer
from this spike or silently turn current failures into a pass.
