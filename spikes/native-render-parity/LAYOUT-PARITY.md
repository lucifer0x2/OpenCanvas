# Task10 full-scene layout parity — independent L1 acceptance

## Result and scope

The focused integration test compares **real Chromium and native server scenes**,
not captured text measurements or a reconstructed layout oracle. The geometry
gate passed on 16 scene pairs: **296 node observations, 176 text observations,
256 final text lines**, with **0 geometry mismatches**. Ink is not identical:
194 line observations differ, maximum **0.0008525848388671875 logical px**.
All observed ink differences are <=1 px; none are >1 px. These are reporting
buckets, not tolerances applied to geometry or an ink-parity claim.

This closes only the bounded full-scene fixture gap. It does not re-run the
separate corpus oracle, prove universal font parity, or accept native PNG quality.
No product source, existing helper, manifest, lock, runtime deployment or user
data was changed by this acceptance worker. No commit/push was performed.

## Reproduce

From the repository root, with the existing pinned browser/runtime installed:

```sh
cargo test -p opencanvas-server --test native_layout_parity -- --nocapture --test-threads=1
```

The required real Inter variable bytes must already exist at ignored
`spikes/native-render-parity/out/fonts/inter.ttf`, from the documented G1 capture.
Absent or incorrect bytes fail explicitly; nothing is silently skipped or
downloaded. The test checks Playwright 1.64.1 / Chromium 156.0.8078.4 revision
1248 and rejects a renderer bundle that differs from a fresh in-memory esbuild.
`PLAYWRIGHT_BROWSERS_PATH` may select an installed copy of that same pin.

Detailed local output is ignored at `out/layout-parity.log`: `INK` records retain
fixture, viewport, state, node ID, line index, literal line text, original values
and exact deltas. `PROVENANCE` and `SOURCE_MANIFEST` preserve runtime/source
identity separately. Generated logs, JSON and font bytes are not versioned.

## Actual protocol and comparison contract

- One temporary SQLite store is shared by two independently configured AppStates
  on different ephemeral loopback ports. Fonts and authored nodes are created via
  actual MCP `import_asset` / `apply_batch`; HTTP snapshot equality checks both
  backends see exactly the same authored revision (5).
- Chromium uses the repository's real worker, layout-step and renderer. Native
  calls `Config::for_backend(..., Rust)` **before** fingerprinting, with an empty
  workspace and nonexistent Node path. Its scene must identify engine `native`,
  without a browser version. Pipeline identities and every compared scene key
  must differ; repeating each identical request must reuse its cached job ID.
- Each exact MCP `render_frame` job is awaited until no longer pending, then
  fetched by its own HTTP job ID. The test checks revision, frame, viewport,
  state and `sourceSceneKey`; no latest-scene lookup can substitute another job.
- Every job is also read using actual MCP `inspect_layout`, unfiltered and
  untruncated. Every returned ID/bounds/line list must equal its own HTTP scene.
- Both complete node-ID sets must equal the authored frame subtree. Clipped
  off-box text and opacity-zero text are retained; no visible-only filtering.
- Geometry compares raw numeric values exactly: all nodes' x/y/width/height,
  parent/kind/theme, final line count, text, x/y/width, fontSize, lineHeight and
  tracking. `TextLine.y` is the baseline. Scene-3 has no per-line height member;
  text box height and declared lineHeight are checked instead of inventing one.
  Numeric JSON integers and floats are compared as the same exact finite number,
  without rounding, epsilon, masking or normalization of measured values.
- Ink x/y/width/height is measured and compared in a separate non-gating ledger.
  The test never asserts that whole text objects are identical when ink differs.
  Strict PNG errors are reported separately and do not hide measurable scenes.
- Font IDs/hashes/families/weights/styles must agree. Resource URLs, derived font
  manifest hashes, source scene keys, runtime identity and readiness URLs are
  reported separately: the two server ports legitimately differ.

## Fixture coverage

Each row below runs both `default` and `hover`. Max geometry delta is **0** for
every row; observations include both states, rather than unique authored nodes.

| Frame / width (height 1200) | Nodes/state | Texts/state | Lines/state | Ink-different lines/state |
| --- | ---: | ---: | ---: | ---: |
| `frame` / 1040 | 8 | 3 | 9 | 6 |
| `frame` / 721 | 8 | 3 | 7 | 6 |
| `frame` / 720 | 8 | 3 | 7 | 5 |
| `frame` / 390 | 8 | 3 | 8 | 7 |
| `typography` / 1040 | 29 | 19 | 24 | 18 |
| `typography` / 721 | 29 | 19 | 24 | 18 |
| `typography` / 720 | 29 | 19 | 24 | 18 |
| `typography` / 390 | 29 | 19 | 25 | 19 |

`frame` comes directly from `spikes/e2e/wrap-design.mjs::wrapOperations`, which
uses the existing responsive fixture: nested column/row, fill/hug, min280,
padding/gap, row wrap, Vietnamese copy and an unbroken emergency-wrap string.

`typography` is a small authored MCP fixture, not a layout engine: nested stacks,
five wrapping cards (min240), fixed/fill/hug, 100%/75%/50% sizing, fractional
font sizes/line heights, positive/negative tracking, center/right alignment,
uppercase expansions, CSS capitalize, CRLF/TAB, composed/decomposed Vietnamese,
Greek/Latin mixed-script marks, clipping and transparent text. A full-rule
breakpoint changes padding at 721/720; the test verifies descendant reflow,
the exact breakpoint x positions, and the actual hover fill preview.

Fonts: bundled Noto Sans plus real uploaded Inter variable 400 normal and static
Lato Regular 400, Bold 700, Italic 400. All four uploaded byte hashes are pinned
in `tests/native_layout_support/fonts.rs` to the same G1/original fixture sources.
No OS font lookup or SF/trak claim is introduced here.

## Ink and actual missing-glyph exception

Largest ink difference is in `typography`, `type-card-1-body` (uploaded Inter),
line 0, text `Một thiết kế có thể xuống dòng.`, `inkBounds.height`:
Chromium **18.1085205078125**, native **18.107667922973633**, absolute delta
**0.0008525848388671875**. The same height delta also occurs on its second
body line and repeats across states/viewports. Geometry/baselines remain exact.
Mixed-script node `mixed-script`, line `Việt á ṇ ΟΣ ος Alpha`, has only small
ink differences in this run (e.g. height delta **0.000003814697265625**).

A separate `missing-control` frame, width720/default, contains exactly
`missing-text`: `Missing ⌘ 🦄 一`. Both real backends must emit
`MISSING_GLYPH node=missing-text` for **U+2318, U+4E00, U+1F984**. Only this
explicit diagnostic-backed control is outside the geometry gate (2 requested
jobs). Every primary covered fixture rejects missing/unverified-coverage
diagnostics; no fallback label excludes any primary test.

## Checked run and quality checks

Final run: **2026-11-04 08:52:09–08:52:34 UTC**, macOS arm64, Node v24.21.0,
Chromium 156.0.8078.4. **1 passed, 0 failed**, 24.71 seconds. There were no
strict-paint errors in the primary scene pairs, but pixels were not compared.
The exact gate checked **1,184 bounds fields** and **1,552 text scalar/string
fields**, in addition to node identity/parent/theme and every line count.

This is a dirty, concurrently developed workspace, not a global commit claim:

- Actual test executable SHA-256:
  `645fc2be3987f2c206369902b22606f5914cc0ea799bfca64c00dede3bdfb688`.
- Sorted scene-computation source-manifest SHA-256 (92 files, unchanged across
  this run): `22c05b46491a5d3d6c0776dec9d9eb8d3fc2827692e2ac01836c963374ccd8db`.
  This covers text/layout, server job paths, state/fingerprint, lock, worker and
  current renderer sources/bundle. Full per-file hashes are in the ignored log.
  Separately owned `crates/render/src` is deliberately outside this geometry
  source-stability guard; the executable hash still identifies compiled paint.
- Checked native compute source (`crates/server/src/jobs/native.rs`):
  `452ff18cf6550c93e9da06b7b199821283e81d1d02063d6672b8d2fd3858ac68`.
- Checked scene construction (`crates/layout/src/scene_builder.rs`):
  `eb2dca36ee67d923d712f598e19487a08adcbe040c85927aaf1d18af89fe8fb2`.
- Chromium renderer bundle:
  `0744f097a465d8e530e2570c24839bd905dee91bb110e7b4bf3eb6cf6746cc8a`.

Focused strict Clippy passed:

```sh
cargo clippy -p opencanvas-server --test native_layout_parity --no-deps -- -D warnings
rustfmt --check --edition 2021 --config skip_children=true crates/server/tests/native_layout_parity.rs crates/server/tests/native_layout_support/*.rs
```

The earlier Clippy invocation *without* `--no-deps` was blocked by the separately
owned `crates/render/src/blur.rs:41`, `clippy::too_many_arguments` (8/7); this
worker did not modify or suppress that source warning. No full-suite lint pass
is claimed. New-file whitespace checks were clean. All six authored Rust files
are <=200 lines: main173, comparator192, fixtures102, fonts61, harness129,
runtime121. Each isolates one responsibility; no oversized-file exception.

An intermediate run correctly failed its then-broader source-stability guard
when the independent paint worker changed `crates/render/src/canvas.rs` during
execution. The final run above guards the actual scene-computation paths and
passed unchanged. No geometry source bug was found or fixed by this worker.
