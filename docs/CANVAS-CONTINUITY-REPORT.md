# Canvas continuity — 2026-10-31

## Result and cause

Continuous agent edits now retain the last ready scene, resources and mounted Konva Stage
while a newer revision loads or fails. Previously `useScene` removed its scene immediately
on every revision request; the Stage unmounted and the camera fit effect ran again when the
scene returned. That effect also fitted on every pane or frame dimension change.

`useScene` now tracks the exact request/retry separately from its stable
`[projectId, frameId, viewportPreset]` scope. Only a same-scope refresh retains content;
navigation clears incompatible content before effects run. An older request is never
reported ready. Existing abort/effect-lifetime guards and `fetchSceneJob` validation of job,
revision, viewport, provenance and source key remain in force.

The camera fits once per scope after the first ready scene and nonzero pane measurement.
Revision, authored dimension and pane/window changes preserve it. Fit/F still deliberately
fits the currently displayed scene. Drag position is synchronized on drag movement, so a
scene swap cannot restore the position from before an unfinished drag.

A compact status names the displayed scene revision and requested revision while loading
or failed; only Retry captures pointer input. The canvas region exposes `aria-busy` while
pending. Computed inspector geometry is suppressed until the exact current scene is ready,
while current authored properties remain inspectable.

## Validation

Executed successfully:

- `npm --prefix web run typecheck`.
- `npm --prefix web test`: 25 tests passed, including existing job/provenance validation.
- `npm --prefix web run test:continuity`: Chromium browser regression passed. It checks
  retained Stage/canvas identity across rapid revisions; zoom while pending; a ready swap
  during a native active drag with continued movement; authored dimension changes; panel and
  window resize; failure/retry; a mislabeled response; obsolete job completion; Fit/F;
  frame/project/preset navigation with incompatible content hidden; colliding frame IDs
  across projects; computed inspector suppression with a positive pre-refresh assertion;
  GET-only requests and no page errors.
- `npm --prefix web run test:browser`: existing browser smoke passed, including SSE
  gap/reset/reconnect, stale jobs, missing resources, selection fallback and narrow layout.
  Its old pending assertion now requires a retained canvas with a displayed-revision label.
- `CONTINUITY_BASELINE=1 npm --prefix web run test:continuity`: expected failure at
  “revision pending retains the mounted Stage and canvas”, using the committed original
  `useScene.ts` served only to the isolated test page. No workspace rollback was used.
- `npm --prefix web run build`: production output refreshed in `web/dist`; the existing
  chunk warning remains (634.05 kB JavaScript, 197.21 kB gzip).
- Prettier check for changed source/tests/package configuration and `git diff --check`.

The main session independently reran the continuity browser regression and inspected the
pending banner visually. Browser fixture checks use Chromium 156.0.8078.4; they verify the
frontend HTTP/SSE contract, not a new live Rust/MCP integration run or cross-browser parity.
The 45-second timeout itself was not waited out; controlled failures and Retry were tested.

## Files and scope

- `web/src/hooks/useScene.ts`: request lifetime, retained scene and explicit stable scope.
- `web/src/hooks/useCanvasTransform.ts`: measured first fit and preserved local camera.
- `web/src/components/CanvasView.tsx`: scope integration, drag synchronization and busy state.
- `web/src/components/CanvasMessage.tsx`, `web/src/canvas.css`: nonblocking refresh/error status.
- `web/src/App.tsx`: pass the viewport preset and suppress stale computed inspector data.
- `web/tests/canvas-continuity{,-fixture,-harness}.mjs`: controlled fixture and browser regression.
- `web/tests/smoke.mjs`, `web/package.json`: updated expectation and `test:continuity` script.
- `docs/VIEWER.md`: camera/consistency contract; this report records evidence and limits.

Changed handwritten source/test files are 62–224 lines each; none exceeds 250 lines.
The existing App remains focused on composition; camera behavior and scene lifecycle stay
in their own hooks, status presentation stays in its component/CSS, and fixture lifecycle
is separated from regression assertions.

All temporary fixture/Vite processes and browser instances were closed after successful
and baseline-failure runs. Fixture APIs and the new Vite harness use random loopback ports.
No user data, `~/.opencanvas`, native server process, Git branch/history or remote was changed.
The production server reads the refreshed `web/dist` dynamically; existing tabs need refresh
to load the new JavaScript. AGENTS/LOG updates belong to the main session.
