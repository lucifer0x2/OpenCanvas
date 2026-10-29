# Read-only browser viewer

The viewer lives in `web/` and imports the shared Canvas2D renderer from `renderer/`. React and Konva manage only the editor shell, local pan/zoom, and selection overlays. The browser never computes layout or wraps text; a Konva `Shape` paints the scene with the shared `paintScene` and the resources from `loadResources`, using precomputed text lines. A blue outline highlights selection without resize handles.

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

The canvas paints the scene as vectors on every draw: a Konva `Shape` whose `sceneFunc` calls the
shared `paintScene` under the stage transform (zoom times `devicePixelRatio`). Text and edges are
therefore rasterized at the size they are shown, during a wheel or trackpad gesture as well as at
rest, up to the 4x maximum zoom, and the picture does not change once a gesture stops. Nothing is
re-rasterized or cached per zoom level: the Konva layer stays the size of the canvas pane, so a
zoom allocates no canvas and costs one paint of the scene per frame (about 1.4 ms p95 for an
857-node, 716-line scene in headless Chromium). Images are drawn with
`imageSmoothingQuality = "high"`, which only affects the viewer, never the worker PNG.

`paintScene` is the same code the worker PNG runs through `drawScene`, which only validates the
pipeline, sizes and clears the target and sets its scale before painting. The viewer still draws
the scene once at 1:1 with `drawScene` when a job arrives: that offscreen canvas checks the
pipeline and stays byte-identical to the worker PNG, which is what the acceptance run compares.
Each node paints fill, then image or text; a node's `stroke` paints after its whole subtree
(`renderer/stroke.ts`), under ancestor clips and opacity but not its own clip, so an outside
stroke of a clipping frame stays visible.
`npm --prefix web run test:zoom` measures sharpness against a `drawScene` reference mid-gesture
and at rest, canvas allocation, and frame cost on 800×600, 1440×900 and 2880×1800 fixtures.

## Opacity

`opacity` composites like CSS (`renderer/layers.ts`, `paintTree`): the scene is walked as a tree,
and a node below opacity 1 whose subtree issues two or more paints renders into an offscreen layer
at alpha 1, which is then drawn once at the accumulated opacity, so overlapping children do not
show through each other. A node's fill and its stroke count as two paints, so a faded stroked box
fades as one image like a CSS border. A single paint (a leaf, or a group with one painting child) folds the
alpha instead, which is identical and keeps such scenes byte-identical to the old per-node alpha.
A layer covers the device-pixel box of what its subtree paints (`renderer/scene-tree.ts`; text is
padded for glyph overhang; outside/center strokes widen it past bounds, and a node's own clip never
cuts its own stroke), clipped to the target and to clipping ancestors, offset only by whole
pixels so antialiasing matches direct painting. Layers are `HTMLCanvasElement`s: Chromium's
`OffscreenCanvas` clips without antialiasing. Live layers share a 16M-pixel budget; a group over it
folds its alpha like before and counts in `opacityLayerFallbacks` of `drawScene`'s result, next to
`opacityLayers`. The viewer allocates its layers on every draw (no pool): a 2880×1800 scene with
56 opacity groups measured about 3.4 ms p95 per zoom frame against 1.6 ms without them.

## Consistency

Selection persists in local storage. Reload validates the project, page, frame, and node against the current snapshot, falling back to a valid frame when a target disappears. A visible message explains removed frame/layer fallbacks.

The first snapshot request and the event stream start in the same effect pass; every stream open refetches the snapshot, so a commit between the two is never lost. The stream cursor is based on the last accepted HTTP snapshot. Duplicates and foreign project commits are ignored; a commit, gap, reset, or reconnect refetches the authoritative snapshot. Reconnect uses bounded exponential backoff with jitter. Concurrent refreshes coalesce without losing a subsequent refresh. HTTP snapshots never move revision backwards.

Each render request has an effect lifetime and validates project/revision/frame, expected job ID, viewport, theme/state, and source scene key. A new snapshot or viewport immediately hides the old scene and computed inspector. Late resources and jobs cannot publish into a newer request. Pending jobs are polled at 500 ms and time out after 45 seconds. The top-level job error is distinct from snapshot/connection error; missing fonts and assets display labeled shared-renderer placeholders and explicit status text. Scene and authored revisions are displayed separately.

## Live workspace

The viewer keeps **exactly one `EventSource` per tab**. The server speaks HTTP/1.1 without TLS, so
a browser allows about six connections per origin; two streams per tab would starve snapshot, job
and asset requests after three tabs. With a project selected the tab uses the project stream
(`/api/v1/projects/{id}/events`); with none (an empty workspace) it uses the workspace stream
(`/api/v1/events`). Switching closes the previous stream before opening the next, and a failed
stream is closed before its backoff timer opens a replacement.

- **Live project list.** `project_created` inserts or updates the project in the selector by
  `projectId`, in the list API's slug order, without a reload. While the workspace is empty the
  viewer selects the new project, as on first load, and switches to its project stream;
  otherwise a short notice names the new project.
- **Activity marks.** `project_activity` from another project bumps its revision in the list,
  marks it with `●` in the selector, shows a count next to the selector and a short notice with
  the project's name. It never refetches the snapshot in view. Selecting the project clears its
  mark and loads its newest revision.
- **Hints are not durable.** On every stream open and every `reset_required` the viewer refetches
  both the snapshot and `GET /api/v1/projects`; the list merges by `projectId` and never drops a
  hinted project or moves a revision backwards. Only the first page (100 projects) is read.
- **Follow agent** (status bar checkbox). When on, `project_created` or `project_activity` moves
  the viewer to that project. It is **off by default** (`FOLLOW_AGENT_DEFAULT` in
  `web/src/hooks/useFollowAgent.ts`), because jumping away from the project in view loses
  context, and the choice persists in local storage (`opencanvas.followAgent`) when available.

Code: `useProjectList` (list and refetch), `useProjectStream` (the single stream),
`useSnapshot` (authoritative snapshot), `useFollowAgent`, composed by `useProject`; pure reducers
`mergeProjects`, `bumpRevision`, `markActivity`, `clearActivity`, `pickProject` and
`followTarget` live in `web/src/state.ts` with unit tests. The viewer still sends only GET.

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

## Motion preview

`/preview?project=<projectId>&frame=<frameId>[&width=&height=]` plays a frame's declared
`props.animations`, `props.states` and `props.links` in the browser. The header's **Preview**
button opens it for the selected frame in a new tab; the server's SPA fallback serves it, and
`web/src/main.tsx` picks `PreviewApp` for paths under `/preview`. It is a DOM preview, not a
time-sampled render (P07c/P07d stay out of scope), and it sends only GET requests.

**Data.** The authored snapshot gives animations, links, states, tokens/styles and the root
frame picker. The default-state scene (`state=default`, default theme) gives bounds, fill,
opacity, radius, stroke, text lines, fonts and assets. For each state declared in the frame
(including masters of its instances) the scene is also fetched with `state=<name>`; per-node
differences against the default scene become the state visuals, so the server keeps owning
theme/token resolution. Scene jobs share `fetchSceneJob` (`web/src/sceneJob.ts`) with
`useScene`. Width/height default to the authored frame size (800×600 fallback).

**DOM.** One absolutely positioned element per scene node (`.oc-node` → `.oc-box` for fill,
radius, clip and children, then `.oc-stroke`, a border overlay outside the node's own clip,
sized like `renderer/stroke.ts`). Text is SVG `<text>` at the scene's baseline positions, so the
browser never re-wraps; images use the shared `placeImage` with crop. The 1:1 frame is scaled
down to fit the window. At rest it differs from the worker PNG in under 1% of pixels (text
antialiasing) on the smoke fixture.

**Motion semantics** (`web/src/preview/keyframes.ts`, unit tested):

- `fillMode` defaults to `both`; times are milliseconds; `"infinite"` iterations map to
  `Infinity`. The effect is linear and each keyframe carries `easing ?? animation.easing ?? ease`
  for the segment starting at it, as in CSS.
- Offsets spread like WAAPI (lone keyframe without offset = 1). Every animated property missing
  at offset 0 or 1 gets an explicit keyframe with the node's resting scene value.
- `translateX/Y`, `scale`, `rotate` map to the CSS `translate`, `scale`, `rotate` properties
  (around the bounds center), one CSS property per validator channel, so two animations on one
  node animating different channels never clobber a shared `transform`.
- `fill` animates `background-color` of shapes and `color` of text (SVG text uses
  `currentColor`). `{tokenId}`/`{styleId}` resolve at the default theme from the snapshot; an
  unresolvable reference skips that property and is listed under the stage as a warning.
- `load` plays when the frame mounts; **Replay** resets every animation and restarts the load
  ones. `hover` plays forward on `pointerenter` and reverses from its current time on
  `pointerleave` (no snap). `click` restarts on every click.
- **Reduced motion** starts from `prefers-reduced-motion` and can be toggled: triggers jump to
  their end state (`finish()`), an infinite animation stays at rest, hover leave returns to rest.

**States.** Rules are generated per declared state with `!important` on `:hover`, `:active`
(pressed), `:focus-visible` (focus; such nodes get `tabindex=0`) and `[data-oc-disabled]`
(the **Disabled state** toggle, shown only when declared). They therefore beat the resting
inline styles. Animations own `translate`/`scale`/`rotate`; for `opacity` and `fill`, an active
state that declares the property wins over the animated value, and the animation shows again
when the state ends. A state stroke replaces the whole stroke overlay.

**Links.** A click link `navigate` pushes `?frame=<to>` and shows that frame; `back` goes back
through preview history (a toast when there is none). `overlay`/`close` show a "not supported in
preview" toast; hover (overlay) links are listed as warnings.

**Instances (P24).** Scene IDs of instance children are virtual (`<instanceId>/<masterChildId>`).
State visuals come from the per-state scenes, so expanded master states work without ID
mapping. Component subtrees and instances cannot author animations or links (storage rejects
them), so there is nothing else to map.

**Known limitations.** Default design theme only; no overlay/close navigation; states are
plain CSS pseudo-classes (hovering a child also hovers its ancestors, focus needs keyboard
focus); the preview loads one snapshot and does not follow live SSE commits (use Reload); a
`width`/`height` in the URL is kept across link navigation.

Tests: `npm --prefix web test` (keyframes, model and state-style units) and
`npm --prefix web run test:preview`, a Playwright smoke against a temporary server
(`target/debug` binaries, `renderer/dist`, `web/dist` must be built) with a temporary data root,
which creates its fixture through MCP and removes everything afterwards.
