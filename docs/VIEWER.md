# Read-only browser viewer

The viewer lives in `web/` and imports the shared Canvas2D renderer from `renderer/`. React and Konva manage only the editor shell, local pan/zoom, and selection overlays. The browser never computes layout or wraps text; `Konva.Image` displays the canvas produced by shared `loadResources` and `drawScene` with precomputed text lines. A blue outline highlights selection without resize handles.

## Run

```sh
cd web
npm ci
npm run dev
```

The Vite server proxies `/api` to `http://127.0.0.1:3847`. Set `OPENCANVAS_API_URL` to override that target for an isolated test server. `npm run build` creates `web/dist` for the production backend to serve. No demo data is created by starting the viewer.

Project → page → frame selectors use authored snapshots; layers are sorted by numeric storage `sortKey`. Viewport choices are frame size, desktop 1440×900, tablet 768×1024, and mobile 390×844. Frame size uses authored numeric width/height with an explicit 800×600 fallback for nonnumeric values. These choices request computed scenes and never change authored data. The inspector separates authored properties from frame-local computed bounds and records the scene revision.

Drag to pan; wheel or the zoom buttons to zoom; Fit or `F` to fit. Wheel zoom is proportional to the wheel delta, so a mouse notch still steps about 1.1x while a trackpad glides in small increments. The frame name and zoom controls float over the canvas, which fills the pane edge to edge. Focus the canvas and use arrow keys to pan and `+`/`-` to zoom. Every layer is a keyboard accessible native button. The header buttons toggle layers and inspector, including at narrow widths. No editing, resizing, chat, terminal, theme toggle, or mutation request is provided.

## Zoom rendering

Scaling one 1:1 bitmap to every zoom level softens text, so the viewer rasterizes the scene again
at the displayed device scale (zoom times `devicePixelRatio`) about 140 ms after a zoom settles,
through the same shared `drawScene`. A settled raster lands pixel for pixel on the display, and the
stage origin is snapped to whole device pixels so no fractional offset resamples it. The scale is
clamped to the renderer's 4x and pixel-budget limits. Past that cap the capped raster is still
displayed, because scaling it up stays far sharper than scaling the 1:1 canvas; a refused render
keeps the previous raster.

While a zoom gesture is in flight the viewer previews with half-size copies of the 1:1 canvas,
because Canvas2D downscales in one bilinear step and speckles text at arbitrary ratios, and
`imageSmoothingQuality` does not change that. The 1:1 canvas itself is never resampled and stays
byte-identical to the worker PNG, which is what the acceptance run compares.

## Consistency

Selection persists in local storage. Reload validates the project, page, frame, and node against the current snapshot, falling back to a valid frame when a target disappears. A visible message explains removed frame/layer fallbacks.

The event stream is opened before the first snapshot request. Its cursor is based on the last accepted HTTP snapshot. Duplicates and foreign project events are ignored; a commit, gap, reset, or reconnect refetches the authoritative snapshot. Reconnect uses bounded exponential backoff with jitter. Concurrent refreshes coalesce without losing a subsequent refresh. HTTP snapshots never move revision backwards.

Each render request has an effect lifetime and validates project/revision/frame, expected job ID, viewport, theme/state, and source scene key. A new snapshot or viewport immediately hides the old scene and computed inspector. Late resources and jobs cannot publish into a newer request. Pending jobs are polled at 500 ms and time out after 45 seconds. The top-level job error is distinct from snapshot/connection error; missing fonts and assets display labeled shared-renderer placeholders and explicit status text. Scene and authored revisions are displayed separately.

## Verification — 2026-10-28

Commands actually run from the workspace:

```sh
npm --prefix web run typecheck
npm --prefix web test
npm --prefix web run build
node web/tests/smoke.mjs
```

- TypeScript check passed. Production Vite build passed; its approximately 574 kB JavaScript bundle currently emits the normal 500 kB chunk-size warning.
- Four state tests passed: numeric layer sorting, valid/deleted selection reconciliation, SSE duplicate/gap decisions, and rejection of old/mislabeled jobs and viewport/source-key races.
- Playwright Chromium 156.0.8078.4 smoke passed: GET-only requests; selection persistence over reload; zoom and keyboard fit; no old canvas during new revision pending; late job R after R+1; SSE gap/reset/reconnect; missing font and missing asset; failed worker state; deleted layer fallback; and no horizontal overflow at 390 px. No browser page errors occurred.
- Actual UI screenshots and machine-readable evidence are under `artifacts/viewer/`: `desktop.png`, `narrow.png`, `missing-resources.png`, `evidence.json`. Screenshots use an explicitly named in-memory **Viewer test fixture**, never a real user project. Its temporary API server uses a random localhost port and shuts down after the test. Port 3847 remains free for the backend.

The smoke test verifies the frontend HTTP contract against a temporary fixture; it does not prove live Rust/MCP integration, persistence, or PNG equality. Those require the backend integration acceptance run. No claim is made here that M1/M2 are complete end to end.

All hand-written viewer source and test files were split by responsibility and are below 250 lines after normal formatting. `package-lock.json` is generated. No separate design-system project was created.

## Live backend acceptance — resumed run

The later production E2E closes the live-integration gap described above. The
worker ran `npm run test:e2e`; root independently rebuilt the final backend and
ran `node spikes/e2e/run.mjs` successfully on 2026-10-28. Actual MCP calls created
the temporary SQLite project and resources; the production viewer received the
same computed scene/line list/provenance as strict PNG. Decoded RGBA differences
were zero at 640×480 for revisions 2/3 and after process restart, and at 390×844.
The browser updated through production SSE after a token mutation without manual
refresh. Restart/reload preserved IDs, contents, resource references and selection.
Selection, keyboard pan, zoom, Fit, narrow layout and GET-only viewer API traffic
were checked. Root inspected actual desktop/mobile screenshots. All temporary
data and harness processes were cleaned up; no project used `~/.opencanvas`.

See `artifacts/e2e/evidence.json`, `spikes/e2e/README.md` and `docs/ACCEPTANCE.md`
for exact evidence and limits. The earlier fixture checks remain evidence for
controlled stale-job/reconnect/failure UI scenarios; the live run does not claim
cross-browser pixel equality or replace backend/storage failure suites.
