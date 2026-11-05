# Task10 Chromium text oracle and native comparison

`capture.mjs` captures **current `scene-3` / `web-4`** measurements only. The
separate native adapter/comparator below uses that immutable capture. Neither
changes the default backend or uses an app server, project, database or user data.
See **[PARITY.md](./PARITY.md)** for results, exact scope and remaining gaps.

## Rerun from the repository root

Use the repository's root `node_modules` (the supplied workspace symlink works).
Required pins: **Playwright 1.64.1**, Chromium **156.0.8078.4**, revision **1248**.
The CLI fails on another installed/actual version. Node 24.21.0, macOS arm64 was
used for the focused capture checks; exact runtime details go into each report.

```sh
node --test spikes/native-render-parity/oracle.test.mjs
node spikes/native-render-parity/capture.mjs
# Optional explicit, local-only SF byte probe (fails if absent or without trak):
node spikes/native-render-parity/capture.mjs --sf-trak
```

`PLAYWRIGHT_BROWSERS_PATH` defaults to `$PWD/.runtime/browsers` resolved from the
repository, not the caller's working directory; an explicit override is honored
but must still provide the pinned version. No downloads/install of Chromium are
performed by this harness. Inter's first download requires public HTTPS access;
later runs use hash-checked copies under ignored `out/fonts/`.

The CLI builds **only** the renderer via `npm run build:renderer` if its bundle
is missing. After changing renderer sources, explicitly rerun that command before
capture. An in-memory esbuild check rejects stale bundles. Renderer and harness
source hashes are recorded. An additional ignored esbuild
bundle exposes the actual `renderer/text-transform.ts` helper for diagnostics.
No renderer source or root manifest is edited.

The HTTP listener binds `127.0.0.1:0`, serves an explicit file allowlist, and closes
in `finally`, including failed captures; Playwright closes its browser/profile.
Font download responses are bounded to 2 MB and 30 seconds, hashes are mandatory,
HTTP/font-readiness failures are never silently skipped. Each font goes
through the real `renderer.loadResources` hash checks, `FontFace` load and
`document.fonts.ready`, using its hash/weight/style private family. The document
has a rendered body for CSS `innerText` casing (not `display:none`). No installed
family is selected. Browser missing-glyph fallback can still happen and is labeled.

## Generated handoff (all ignored; do not commit)

* `out/oracle.json`: requests, exact face manifests and **unmodified full
  `measureBatch` results**, keyed by request `key`; raw Canvas diagnostics too.
* `out/report-schema.json`: exact JSON Schema, draft 2020-12, exported from the
  authored `schema.mjs`. The schema is the adapter contract, not a sketch.
* `out/fonts/*.ttf`, `*-license.txt`: exact original bytes and licenses for a
  later native adapter. `Face.sourceBytes` / `Face.license.file` are relative to
  the report directory. Retained intentionally, not temporary process residue.
* `out/transform.js`: generated diagnostic helper. A failed rerun removes the
  prior `oracle.json`; a successful report is published by atomic rename.

Top-level schema version is `1`; `kind` is
`chromium-measurement-oracle-only`. For the native adapter:

```js
const report = JSON.parse(await readFile("spikes/native-render-parity/out/oracle.json"));
for (const request of report.requests) {
  // Exact fields: key,text,fontId,fontHash,family,weight,style,fontSize,
  // lineHeight,width (number|null),letterSpacing,textTransform.
  const face = report.fonts.find((f) => f.manifest.id === request.fontId);
  // Read bytes relative to oracle.json, verify SHA256 === face.manifest.hash.
  const expected = report.browser.measurements[request.key];
  // width,height,lines[{text,x,y,width,inkBounds:{x,y,width,height}}],
  // ascent,missing:[Unicode scalar numbers],coverageUnavailable:boolean.
  const eligibility = report.browser.eligibility[request.key];
  // Only status:"eligible" is a parity candidate, NOT evidence of native success.
}
```

`browser.probes[key]` preserves the actual CSS-transformed text, CSS font and
tracking string, empty-string/full-string/per-line Canvas `TextMetrics`. This
includes font bounding ascent/descent and ink extents, without an extra rounding
step. Measurement ink bounds are baseline-relative, not box-relative. `cases[key]`
records the group, label, declared fallback and optional `thresholdOf` parent.
`browser.eligibility` lists every exclusion and reason; the original `missing`
array and `coverageUnavailable` flag are never rewritten. Rebase manifest URL
paths to the new local server if replaying the browser; native consumes bytes.

## Corpus and semantics

The legacy corpus is **read-only** from
`spikes/rust-export/data/corpus.json` (SHA recorded): all **1,232** string×size
and **120** wrap cases, remeasured using the current renderer, not old reports.
Authored small generators add all faces, CR/CRLF/LF and trailing/empty lines,
TAB/FF/VT collapse, NBSP/U+202F/U+2060/U+FEFF nonbreaking groups, Unicode spaces,
Vietnamese composed/decomposed marks, graphemes, ligatures, kerning with zero and
positive/negative tracking, fractional and undersized line heights, uppercase
expansions (ß/ﬃ), Greek final sigma lowercasing, and CSS capitalize boundaries.
Second-pass requests test exact measured width and ±1/64 around that boundary.

Native's confirmed missing-glyph policy is **`MISSING_GLYPH` + `.notdef` advance**,
not OS lookup. Browser fallback samples (⌘, CJK, unsupported scalars and emoji)
remain captured but excluded. Renderer cmap checks prove scalar coverage only,
not complete cluster/variation/emoji shaping. No missing font is skipped. The
`declared-platform-fallback` exclusion conservatively covers authored emoji
probes even if a scalar cmap happens to accept them.

## Font provenance / licenses

All manifests include source URL, byte count, SHA-256, private family, SFNT table
inventory, and a license record with its own source URL/hash/copy. Noto and Inter
are variable source files measured at the renderer's declared 400 normal face;
no custom axes, OS installation or synthetic bold/italic are introduced.

| Face | Bytes SHA-256 | Source / license |
| --- | --- | --- |
| Bundled Noto Sans 400 | `bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d` | `renderer/fonts/manifest.json`, bundled `OFL.txt`; SIL OFL 1.1 |
| Lato Regular 400 | `d636e4683231f931eda222d588e944d082bfd3bdba02f928bee461c0f185b251` | Existing `fixtures/fonts/README.md`, Google Fonts commit `f3b885d5590e307e02542f1a724cec55d567fdaa`; SIL OFL 1.1 |
| Lato Bold 700 | `8a0aace75d33794eece4b28187bfc1df0bbd2888b5d8a56e01788c8d65d16be1` | Same pinned static fixture source/license |
| Lato Italic 400 | `e399c44efe1387100531d26c7e4800c5d12251b890d6654a3098c7c679cb1786` | Same pinned static fixture source/license |
| Inter 400 | `4989b125924991b90d05b2d16e0e388c48f7d5bb8b30539bbf9c755278d0ccaf` | [Official Inter v4.1 bytes](https://raw.githubusercontent.com/rsms/inter/e3a3d4c57d5ecc01453a575621882a384c1995a3/docs/font-files/InterVariable.ttf); SIL OFL 1.1 |

Inter v4.1 resolves to commit `e3a3d4c57d5ecc01453a575621882a384c1995a3`.
[Its original license](https://raw.githubusercontent.com/rsms/inter/e3a3d4c57d5ecc01453a575621882a384c1995a3/LICENSE.txt)
has SHA-256 `262481e844521b326f5ecd053e59b98c8b2da78c8ee1bdbb6e8174305e54935a`.
Inter is a real uploaded-style external file served by the same bytes pipeline,
not a Google Fonts CSS reference or invented font record.

`--sf-trak` reads **only** the explicit `/System/Library/Fonts/SFNS.ttf` file,
requires the `trak` table and serves those bytes through the same private
FontFace path. It does not enumerate system fonts. These proprietary bytes stay
in ignored local output; **never share/archive/commit `sf-probe.ttf`**. The report
uses `system:SFNS.ttf` rather than a private path. Its hash is computed per host.
The probe succeeding says nothing about native CoreText/advance parity.

## Native comparison still to do

This was the capture worker's handoff checklist; it is now exercised by G2's
adapter/comparator. Full-scene bounds and raster comparison remain separate.

* Keep ceil64/clamp width for **both fit and reported width**; old nearest-grid
  spike conclusions are not today's AC. Blink per-glyph advance quantization,
  tracking's interaction with kerning/ligatures and CoreText `trak` need checking.
* Baseline uses empty-string **font bounding** ascent/descent plus
  `floor((lineHeight-ascent-descent)/2)`, then JS round-to-1/64 per line. It is not
  `fontSize + leading/2`; negative half-tie JS rounding can differ from Rust.
* Preserve offscreen CSS transform semantics (Greek final sigma, expansions,
  capitalize), current grapheme and nonbreaking whitespace rules; do not port
  the old spike's `trim`/`\s` wrapper.
* Compare line text/width/x/y, height/ascent, ink geometry and missing metadata.
  Shared advances do not prove shared outlines, hinting, optical sizing or ink.
  Variable Noto/Inter/SF defaults and static Lato faces must remain distinguishable.

No timing/performance statistics or native parity counts are produced here.

## Focused checks performed

All eight authored JS modules passed `node --check`; six unit tests passed.
Default capture completed with **3,079 requests** (3,035 eligible candidates,
44 excluded). With `--sf-trak`, **3,422 requests** completed (3,366 candidates,
56 excluded), including an actually loaded SF font with a `trak` table.
The latter report was checked against the exported schema and every copied font
hash; per-line ceil64 widths/font-bbox baselines were checked against raw Canvas
metrics. A second capture had identical requests, font manifests, measurements,
raw probes and exclusion policy. These are oracle reproducibility checks only.
Generated check evidence is local under `out/`; native comparison is still pending.
