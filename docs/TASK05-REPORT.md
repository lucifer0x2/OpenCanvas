# Task05 — actual multi-viewport acceptance

Completed by the Task05 CLI worker, 2026-10-28. Shared workspace/branch main;
no commit, reset, checkout, push, child agent or configuration/auth changes.
Root owns final independent acceptance and task status.

## Result

`npm run test:e2e` **passed, exit0, 15 check groups**, preserving all original12.
It built renderer, offline Rust layout/server and production viewer/TypeScript,
then ran actual MCP → SQLite → HTTP/SSE → viewer → PNG → process restart.
Run08:29:13.873Z–08:29:19.906Z; Node24.21.0, Chromium156.0.8078.4,
MCP2025-11-25. Evidence: `artifacts/e2e/evidence.json` and `responsive.json`.

| Viewport | Free pixelPass | Free FRAME overflow | Free reflow from1040 | Stacks pixelPass | Stacks FRAME overflow | Stacks reflow from1040 |
| --- | --- | ---: | --- | --- | ---: | --- |
| 1040×720 | true,0 channels differ | 0 | baseline | true,0 channels differ | 0 | baseline |
| 800×720 | true,0 | 21 | false,0/70 changed | true,0 | 0 | true,106/180 changed |
| 390×844 | true,0 | 52 | false,0/70 changed | true,0 | 104 | true,116/180 changed |

Every sample is authored through `create_project`/`apply_batch` in temporary
projects using the existing welcome/stacks operation factories. All70 free and
180 stacks descendants are checked against authored IDs, actual ready scene and
untruncated MCP `inspect_layout`. Bounds are not clipped. Overflow uses x<0 and
x+width>viewport.width, reporting each ID and left/right/total logical pixels.
Reflow compares all descendants and x/y/width/height with epsilon1e-6; root is
excluded. The report includes full bounds, changedNodeIds, changed dimensions,
before/after values, viewport dimensions/counts and scene/image provenance.

PNG repeats are fresh render jobs after backend/worker restart, rather than
repeated reads of one cached image. Job IDs differ; scene keys/bounds match.
Each sample compares2,995,200 /2,304,000 /1,316,640 RGBA channels at the three
widths with tolerance0, all0 differences. The original viewer/PNG checks at
640×480 r2/r3/restart and390×844 are retained and pass unchanged.

## Design findings and limits

Free keeps all descendant bounds unchanged at all three sizes. Sidebar remains
176×720; focus card remains x744/y148/248×112. At390, date overflows right238px,
greeting418px, new-task-button602px and focus602px. These assertions ensure the
detector cannot pass while silently ignoring fixed nodes. The sample has no
node named c3: its third stat card is `focus`. No reflow is a design finding,
not a renderer failure.

Stacks body widths864→624→480 and stat cards248→168→120 prove real width changes
at800 with the same720px height, separately from sidebar height following844.
At800, FRAME overflow is0, while10 descendants overflow weekly-chart CARD;
chart-grid-0 is212px past the card. At390, body minimum480 plus sidebar176
requires656px, so body overflows FRAME266px;104 descendants exceed the frame.
The narrow chart grid exceeds its CARD254px. Vertical text wrapping/overlap
also remains visible in PNGs and is outside the horizontal-overflow check.
Viewed actual `free-390.png`, `stacks-800.png`, `stacks-390.png`; clipping and
chart/text limitations agree with the numeric report. Neither sample passes
mobile-design acceptance. Reflow alone and pixelPass alone do not imply fit.

`known-overflow.json` freezes all IDs AND pixels from prior reviewed Task02
inspection evidence (free desktop projected onto narrow widths, stacks actual
per-viewport evidence). This run confirms those values through actual renders
at every viewport. It does not regenerate the fixture; any new, removed or
changed overflow fails. Known free overflow is explained by fixed authored
positions; known stacks390 overflow is explained by min-width and fixed plot
coordinates. No blanket clip/parent/node-kind exemption exists. Card overflow
is reported separately and never counted as FRAME overflow.

Pixel0 at narrow width proves deterministic rendering, including deterministic
clipping. It does not show complete content, responsive reflow or usability.
No engine/schema/web/sample change was made to improve these findings.

## Commands and cleanup

- `node --test --test-concurrency=1 spikes/e2e/bounds.test.mjs`:3/3 passed.
  Covers negative/right overflow, root-only changes, all four dimensions,
  tolerance and rejection of missing/empty scopes.
- `npm run test:e2e`: first attempt built successfully but sandbox denied
  loopback bind with EPERM; no test success claimed for that attempt. Repeated
  same command using approved sandbox escalation: exit0,15 check groups.
  Existing web chunk warning576.70kB remains; TypeScript/build succeed.
- `node --check` on every `spikes/e2e/*.mjs`, `git diff --check`: passed.
- Prettier formatted the five changed/new JS modules. Line counts after
  formatting: bounds60, bounds.test47, responsive-render116, responsive240,
  run222; every handwritten harness module≤250 lines. The240-line responsive
  module coordinates one acceptance scenario with fixture assertions; render
  I/O and pure bounds math are already separate. No size exception needed.
- Cleanup evidence `temporaryDataRemoved:true`; `finally` awaited MCP/browser/
  backend closure and removed the temporary root. Backend group termination
  includes its worker/Chromium. Read-only process inspection after completion
  found only the pre-existing user backendPID12919 and workerPID13009; no E2E
  backend/worker remained. `lsof -nP -iTCP:3847 -sTCP:LISTEN` still showedPID12919.
  No command called, stopped, started or restarted the server on3847.

Changed ownership: `spikes/e2e/`, regenerated `artifacts/e2e/`,
`docs/ACCEPTANCE.md`, this report. Root's AGENTS/task status, all Task01/02/03
source/sample deltas and existing hook cache were preserved. No blocker remains
within Task05; root should independently review and accept the result.

## Root independent acceptance

Root reviewed all new modules and the frozen overflow fixture, then ran
`node --test --test-concurrency=1 spikes/e2e/bounds.test.mjs` (**3/3 passed**)
and `node spikes/e2e/run.mjs` (**15 groups passed**) against the final builds.
Root evidence completed2026-10-28T08:33:57.842Z; temporaryDataRemoved=true.
The free/stacks comparison exactly matched the table above. Root opened actual
free390/stacks800/stacks390 PNGs and confirmed the clipping/chart/text findings.
Post-check process inspection found only the intentionally running user backend
PID12919 on3847 and its workerPID13009; no E2E process remained. No required
finding remained; syntax and final diff checks passed. No commit/push.
