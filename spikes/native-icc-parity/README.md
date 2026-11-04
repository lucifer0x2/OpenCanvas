# Native ↔ Chromium ICC image parity (I1)

An isolated, scene-only ICC comparison. Chromium uses the **actual** TypeScript
`loadResources` / `drawScene`; native uses public `opencanvas-render::render` /
`encode_png` through G3's read-only CLI adapter. No product source, G3 source or
G3 output is written. No server/app data, OS color service, download or deploy.

## Reproduce from repository root

Requires the existing `node_modules` and `.runtime/browsers`, Rust/Cargo and
cached dependencies. Nothing is installed. Commands are intentionally separate:

```sh
node spikes/native-icc-parity/run.mjs
node --test --test-concurrency=1 spikes/native-icc-parity/harness.test.mjs
for file in spikes/native-icc-parity/*.mjs; do node --check "$file" || exit; done
CARGO_TARGET_DIR="$PWD/spikes/native-icc-parity/out/target" cargo test --manifest-path spikes/native-icc-parity/Cargo.toml --offline --bin icc-fixtures
CARGO_TARGET_DIR="$PWD/spikes/native-icc-parity/out/target" cargo clippy --manifest-path spikes/native-icc-parity/Cargo.toml --offline --package native-icc-parity --all-targets -- -D warnings
rustfmt --edition 2021 --check spikes/native-icc-parity/fixtures/main.rs spikes/native-icc-parity/fixtures/cmyk.rs spikes/native-icc-parity/fixtures/profile.rs
```

The first command seeds **its own ignored** Cargo.lock from the product lock,
builds an isolated `[workspace]` offline, checks every resolved registry version
against the product lock, copies the binaries, then generates and captures the
originals. It exits **0** only for a complete pass, **2** for failed parity,
reference/effect/probe checks or inconclusive source drift, **1** for harness errors.
Do not convert a failing exit into an acceptance claim. Run the capture command
before standalone Cargo commands to establish the product-pinned ignored lock.

All profiles, originals, fixtures JSON, requests, locks, build logs/targets,
PNGs/diffs, binaries and reports stay in ignored `out/`. Immutable run directories
are timestamped; **only this harness's** `out/latest.json` is updated. Each case's
strip is `chrome-native-diff.png`, in that order. Small original fixture and
comparator tests are authored sources, not checked-in binary goldens.

## Exact comparison contract

- Pinned Chromium **156.0.8078.4**, Playwright **1.64.1**, `scene-3` / `web-4`;
  `--force-color-profile=srgb`, device scale factor 1.
- Integer, **1:1** image destination covering the entire output, scale 1, no
  corners, padding, text, masks, crop or resampling. Default image sampling is
  retained; integer identity geometry eliminates its interpolation ambiguity.
- Straight RGBA8, **all output pixels** in the denominator. A pixel fails if
  **any channel Δ >8**; the failing fraction must be **strictly <0.005**. Alpha
  always participates. Ignore RGB only when **both** alphas equal zero.
- G3's comparator is imported unchanged. Raw straight and white-composite metrics
  remain diagnostic only; neither replaces the visible-straight gate.
- Same original flat `/resources/<file>` bytes and SHA256 go to both engines.
  Chromium fetch SHA256 is independently recorded before renderer loading;
  the renderer and public native API validate the same manifest hashes again.
- Report records original/profile hashes and license/origin, source hashes
  **before build and after capture**, copied native/generator binary hashes,
  dependency versions/licenses and lock hash, rustc/Node/OS, renderer bundle,
  actual executable/arguments and Chromium/V8 runtime. CDP supplies the runtime
  and spawned browser PID; `ps` reads only that PID's executable/arguments (the
  pinned build disables `Browser.getBrowserCommandLine`). The nominal Playwright
  executable is distinguished from the actual headless executable.
- Changes to render (including `icc.rs` / `images.rs`), text, layout, manifests,
  root lock or borrowed G3 sources invalidate the run as
  **INCONCLUSIVE_SOURCE_DRIFT**. No polling another worker or deploying to settle it.

## Fixtures, validity and license

All pixel patterns and CMYK JPEG segments are repository-authored MIT test data;
RGB/gray profile primaries/TRCs use moxcms's mathematical/public-domain defaults.
Dependency licenses are preserved in report metadata. No proprietary printer
profile or external fixture was downloaded. Generated profiles have fixed dates
and four-byte aligned tag payloads; generation is deterministic.

Four profiles: Display-P3 RGB, sRGB RGB control, linear-gamma gray, synthetic CMYK
constant-XYZ input profile. RGB/gray each have opaque and alpha (128/64/192/0)
versions, and identical-sample **untagged controls**. There are **14 captures**:
six RGB/gray tagged, six corresponding controls, CMYK tagged and its control.
The gray PNG retains original gray+alpha channels. CMYK is a four-component
single-MCU 8×8 JPEG; original decoded inks are `[127,127,127,127]`, never RGBA.

`fixtures/cmyk.rs` starts from render's CMS unit fixture's 2×2×2×2 constant
70/255 PCS LUT math. It uses canonical **lut16**, not the unit fixture's lut8:
moxcms 0.8.1 `writer.rs::write_lut_entry` emits two lut16 table-count fields even
for lut8, shifting its tables. That serialization is **not** used as a valid
success fixture here. Tests check the reparsed lut16 tables equal the authored
tables, embedded JPEG profile bytes equal the original and original inks match.

Profiles are parsed by native moxcms; embedded PNG profile bytes are checked
exactly. Browser decode alone is **not** evidence of ICC interpretation: each
engine must also match an independent arithmetic color reference and show a
nontrivial tagged-versus-untagged effect for P3/gray/CMYK; sRGB must remain unchanged.
Chromium exposes no ICC parser API. These observable checks, not a fabricated
"browser ICC parse passed", are the browser-side validity evidence.

Arithmetic reference: P3→sRGB primary matrix plus sRGB transfer; gray gamma1→sRGB;
CMYK lut16 XYZ PCS values `70*257/32768`, D50→D65 Bradford→sRGB. Its separate
maximum-channel tolerance **4** allows alpha-surface quantization (minimum
nonzero alpha 64); **alpha must match exactly**. This is NOT the parity gate.
A failed reference check never becomes a parity pass just because engines agree.

One **malformed ICC error-policy probe** is reported separately, not among the
14 parity captures: native rejects `RESOURCE_NOT_READY`, while Chromium currently
ignores the invalid profile and decodes. No invalid profile is counted as success.

## Recorded result — 2026-11-04

Run location: this directory's [out/latest.json](out/latest.json), then
`<path>/report.json`. The completed capture on this source has:

| Cases | Pixel gate | Largest any-channel Δ | All-pixel fraction >8 |
| --- | --- | ---: | ---: |
| Display-P3 opaque tagged/control | PASS | 0 | 0 |
| Display-P3 alpha tagged/control | PASS | 4 / 0 | 0 |
| sRGB opaque+alpha tagged/controls | PASS | 0 | 0 |
| Gray opaque tagged/control | PASS | 0 | 0 |
| Gray alpha tagged/control | PASS | 2 / 0 | 0 |
| Untagged CMYK | PASS | 0 | 0 |
| CMYK constant-XYZ ICC | **FAIL** | **58** | **64/64 = 100%** |

**13 PASS, 1 FAIL**, no source drift, no draw-counter mismatches. All RGB/gray
arithmetic-reference checks pass with exact alpha; maximum reference Δ is 2.
Observed P3 tagged/control max differences reach 58 (opaque), 60/56
(Chromium/native alpha); gray reaches 69 opaque and 68 alpha. sRGB difference 0.
Thus passing RGB/gray cases actually exercised color management, not just decoding.

### CMYK investigation handoff, not a production patch

`cmyk-icc` outputs Chromium **[145,141,158,255]** versus native
**[198,193,216,255]** at every pixel. Native matches the independent lut16 XYZ
reference; Chromium does not. Both engines decode and visibly apply the embedded
profile, but disagree on its PCS behavior. This is a real browser-parity failure,
**not yet an identified native CMS bug** and not resolved by the native unit test.
Inspect `crates/render/src/icc.rs::transform/cmyk` and the moxcms-versus-Chromium
lut16 PCS interpretation before changing production. Re-run command above after
any authorized fix; do not halve PCS, reinterpret K as alpha or relax the gate
inside the harness to manufacture a pass.

Remaining bounds: no real printer characterization, CMYK Adobe/YCCK inversion,
varying-ink CLUTs, rendering intents, WebP ICC, orientation+ICC or scaled-fit test.
Native supports RGB/gray ICC and original CMYK **JPEG** only; other ICC spaces
and CMYK profiles on non-JPEG sources must remain explicit `UNSUPPORTED_FEATURE`
limitations, never an untagged fallback. These bounded captures do not establish
global ICC support or Task11 paint acceptance.

Module checks: 8 Node comparator/reference/cleanup tests and 4 Rust fixture tests;
strict own-workspace Clippy, own Rust formatting and JS syntax checks are rerunnable
above. Every authored source module is below 200 lines; no size exception needed.
