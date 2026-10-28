# Task04 proposal probe

Historical prototype evidence is retained as `evidence.json` and `text-*.png`.
With native row wrap now in the source tree, the same published command copies
and builds the native crate **without injecting duplicate DTO/mapping fields**.
It retains production guards: the no-minimum control must reject, rather than
silently reverting to prototype behavior. Native runs write `native-evidence.json`
and `native-text-*.png`, preserving historical files. The native-copy rerun passed
15 cases (14 scenes + the expected validation rejection), cleanup=true.
Production acceptance is separate: `npm run test:pipeline` and
`npm run test:e2e`, with results in `docs/TASK04-REPORT.md`.
The remaining description below records the original pre-implementation probe.

This is evidence for a proposed row-wrap design, **not implemented application
support**. Read [the approval proposal](../../docs/TASK04-PROPOSAL.md) first.

From the repository root, with existing Cargo offline dependencies, renderer
bundle, `target/debug/layout-step`, Playwright and bundled Chromium available:

```sh
node spikes/responsive-proposal/run.mjs
```

`prepare.mjs` copies the layout crate and Cargo lock into an OS temporary
directory. It asserts exact unique source anchors, adds a boolean `wrap` DTO
field and maps it to Taffy Wrap + align-content Start **only in that copy**.
An independent Cargo workspace/target builds offline with two compiler jobs.
`fixture.mjs` authors three cards with definite finite width, minimum280px,
row height hug, optional browser-measured Vietnamese/long-grapheme text.
`run.mjs` uses the existing real worker/renderer with the temporary layout binary.
Its own environment override is restored; browser/server and the complete
temporary directory are closed/removed in finally, including failed attempts.
It never calls the backend or creates a project; port3847 and user data are unused.

Generated `evidence.json` retains all scene bounds/lines/provenance and metrics.
`text-{1040,640,390}.png` are real strict renders; desktop/mobile images were
opened and inspected. Generated JSON exceeds250 lines intentionally; handwritten
modules are fixture60, prepare63, run169 lines after Prettier.

Successful run2026-10-28T08:44:23.372Z:15 cases, Node24.21.0,
Chromium156.0.8078.4. Geometry3→2→1 columns, hug heights and exact line boundaries,
real text line-width/height/gap checks, no-wrap/no-min/oversized-min controls,
and omitted-default scene-node equality against the existing production binary
passed. Text repetition uses0 measurement RPCs and identical scenes. An initial
prototype compile type-inference error was fixed in the copy recipe; a later
sandbox attempt hit loopback EPERM. Only the approved escalated rerun passed.

No new storage/direct semantic validation is implemented by this prototype.
No MCP, persistence, viewer, full-pipeline or application feature acceptance is
claimed. The missing guards and required implementation checks are specified
in the proposal. No dependency/model/auth/config change, commit or push occurred.

Checks completed after the successful probe:

```sh
node --check spikes/responsive-proposal/fixture.mjs
node --check spikes/responsive-proposal/prepare.mjs
node --check spikes/responsive-proposal/run.mjs
./node_modules/.bin/prettier --check spikes/responsive-proposal/*.mjs
git diff --check
wc -l spikes/responsive-proposal/*.mjs
```
