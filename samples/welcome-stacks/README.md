# Welcome dashboard authored with stacks

This sample keeps the original `samples/welcome` design and creates a separate
`welcome-dashboard-stacks` frame. Authoring uses actual MCP `create_project` and
one atomic `apply_batch`; no database or user data directory is written directly.
The original sample and the server on port 3847 remain untouched by verification.

Run from the repository root with the existing server/layout binaries, renderer
bundle, Playwright package and bundled Chromium already built:

```sh
node samples/welcome-stacks/verify.mjs
```

Verification starts its own backend with a temporary data root and random port,
authors both the baseline and stacks projects, calls `render_frame` and
`inspect_layout` at 1040×720, 800×720 and 390×844, and saves PNGs, scenes,
inspection results, authored snapshot and paginated semantic context in
`artifacts/sample-stacks/`. Its `finally` block closes the MCP session, stops the
backend process group (including worker/Chromium), and removes the temporary root.
Existing user projects are neither created nor changed by this command.

After the user approves creating the real sample, the authoring command is:

```sh
node samples/welcome-stacks/author.mjs http://127.0.0.1:3847
```

That creates slug `opencanvas-welcome-stacks`, revision 1, and refuses to duplicate
an existing project with that slug. The user approved and this was run on
2026-10-28: project `p_ae68646852a94fddbb9837277b7c88ef`, revision 1, frame
`welcome-dashboard-stacks`. Open `http://127.0.0.1:3847` and select
**OpenCanvas · Mẫu stacks**. Live readback/render/viewer evidence is saved separately
under `artifacts/sample-stacks/live/`; the original welcome project remains at
revision 1. Do not rerun authoring to inspect the existing sample.

Modules have explicit responsibilities: `design.mjs` composes the frame;
`sidebar.mjs`, `sections.mjs` and `chart.mjs` author its sections;
`helpers.mjs` adapts original properties and declares the comparison threshold;
`author.mjs` performs MCP mutations; `verify.mjs` drives temporary verification;
`pixels.mjs` compares PNG pixels in Chromium. None changes the layout engine.

## Desktop comparison

The threshold was declared in `helpers.mjs` and written to `threshold.json`
**before** rendering: changed RGBA pixel ratio ≤0.005 (0.5%) and mean absolute
channel error ≤0.25 on the 0–255 scale. A pixel counts as changed if any RGBA
channel differs. Comparison uses a newly authored baseline on the same runtime,
and separately checks that baseline against `artifacts/sample/dashboard.png`.

Final result: **0 / 748,800 changed pixels, mean absolute channel error 0**.
The historical baseline also has **0 changed channels**. There are no remaining
desktop pixel differences to explain. `desktop-diff.png` is the all-clear mask.
The dashboard text, colors, card geometry and spacing remain unchanged.

## Computed bounds

All values come from actual MCP `inspect_layout`; width is in logical pixels.

| Node | 1040×720 | 800×720 | 390×844 |
| --- | ---: | ---: | ---: |
| sidebar | 176 | 176 | 176 |
| body | 864 | 624 | 480 |
| body-content / stats / today-tasks | 784 | 544 | 400 |
| each stat card | 248 | 168 | 120 |
| weekly-chart | 498 | 262 | 220 |
| weekly-goal | 266 | 262 | 160 |

Sidebar height follows the viewport (720/720/844); its footer remains bottom
anchored using a fill spacer. Body content starts at x=208 in every viewport.
The 800px render proves genuine shrinking within the feasible layout range.
At 390px the body stops at its authored 480px minimum: sidebar 176 + body 480
requires **656px**. Content beyond the root frame is clipped, as intended by the
declared minima. This is a min-constraint probe, not a finished mobile layout.

## Limits encountered

- **Historical no-wrap fixture:** this sample does not opt in to Task04 row wrap;
  its original authored nodes and known-overflow baseline stay unchanged.
  The three stat cards stay in one row; below
  656px the whole dashboard cannot fit. There is no sidebar collapse, stacked
  cards or alternative mobile navigation. New row wrap can be authored separately;
  sidebar/body minima and breakpoint navigation still need separate decisions.
- **Padding is one scalar, with no per-edge padding or alignment controls:**
  preserving original left/top offsets needs explicit nested stacks and spacer
  nodes. Structural wrappers are intentional; the semantic context is paginated.
- **Fill has equal growth, without a weight or percentage:** chart/goal are
  498/266px on desktop through maxWidth caps, but become 262/262px at 800px.
  This is constrained flex sizing, not a maintained proportional split.
- **No percentage progress sizing:** the progress fill uses `fill` with
  `maxWidth:178`, and the track uses `maxWidth:222`. It preserves the desktop
  80% picture, but at the 160px goal minimum it fills the available 138px track;
  it does not express a semantic 80% relationship at every width.
- **The chart plot has authored data coordinates:** only `chart-plot` is `free`
  so bars, grid lines and axis labels can overlap; chart header/title/subtitle
  and legend use stacks. At narrower widths, plot coordinates do not rescale,
  so bars/lines can overflow their card and later cards cover them. The image
  evidence deliberately retains this limitation rather than changing the engine.
- **Desktop heights remain fixed:** width-driven text wrapping at the minimum
  can exceed the original card/header heights (for example the completed-note
  wraps below its 112px card, and the chart title overlaps its subtitle at 390px).
  A separate narrow composition or different hug
  section sizing is needed for polished small-screen behavior.
- The other `free` group, `logo-overlay`, is a real glyph-over-tile decoration.
  Ordinary shell, navigation, content, cards, goal, task rows and controls are
  row/column stacks. No layout feature was added to `crates/` or `web/`.

The final PNGs have been opened and visually inspected at all three viewports.
`verification.json` records actual bounds, scene keys, readiness, success and
temporary-root cleanup; each `*-job.json` retains renderer/browser/font provenance.
