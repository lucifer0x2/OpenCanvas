# All-frames board paint — 2026-11-10

Branch `feat/viewer-frame-paint` (from `main` @`0f42dd1`). Before this change, every new
revision in the All view re-indexed and repainted every frame's vectors, and an update
published once per frame event. Three fixes, measured on the same fixture before and after.

## What changed

1. **Reused frames keep their scene object.** `reuseFrame` (`web/src/scene/staleFrames.ts`)
   no longer restamps `provenance.revision` through `rebaseScene`; the frame keeps the
   identical `scene` and `resources` and carries the accepted revision as
   `FrameSceneView.revision`, so `CanvasFrame`'s `indexScene` memo and paint callback hold.
   Every consumer compares that field: `currentScene(frame, revision)` (not stale, exact
   revision) gates `boardHitTest` and the All-view inspector; `pageView` sets
   `SceneView.sceneRevision` and `shownRevision(view)` feeds the status bar, `CanvasMessage`
   and the inspector's revision labels. Single-frame scenes still answer their own request,
   so their `provenance.revision` remains the shown revision. The memory/IndexedDB cache path
   (`browserSceneCache.ts`) still restamps a content-key hit with `rebaseScene`: it builds a
   new scene object anyway (`decorateResources`) and its job must pass `acceptsJob` and
   `validateBrowserScene` against the request. Stale scenes are still never hit-tested or
   inspected as current.
2. **One publication per animation frame.** `publishScheduler` (`web/src/scene/publishScheduler.ts`)
   coalesces `usePageScenes`'s seed/reuse/frame events into one `setLoaded` per
   `requestAnimationFrame`, reading the latest entries at commit time. A 100 ms timer is the
   hidden-tab fallback (whichever fires first wins); the final result and errors flush at once;
   abort/unmount cancels and ignores later calls.
3. **Per-frame bitmap cache** (`web/src/paint/`, All board only). `FrameBitmap` rasterizes a
   ready frame's `paintScene` output into one detached canvas at the board's resting device
   scale and blits it on later layer draws. `BoardPaint` (one per `CanvasView`) tracks
   whether the zoom is at rest and the pixel budget, and asks the layer to redraw once a zoom
   settles. Label, white frame rect with its shadow, skeleton, error text and selection
   outline stay separate Konva nodes, outside the bitmap.

## Design decisions

- **Validity.** A bitmap belongs to the exact `scene`, `resources` and `SceneTree` objects
  and device scale (`m.a`, relative tolerance 1e-9). A different object or scale releases it
  at the next draw; unmount releases it (`useEffect` cleanup). No React state is involved.
- **Pixel-exact blit.** The bitmap box keeps the frame origin's subpixel phase
  (`bitmapBox`), and the blit uses an identity transform at a whole-pixel offset
  (`blitOrigin`). Stage pan is already snapped to device pixels, so at rest the bitmap lands
  on the same pixel grid as vector paint. The box covers the scene tree's paint extent
  (`SceneTree.extent` of the roots: overflow, strokes, shadows, glyph overhang) plus one
  antialiasing pixel, like renderer opacity layers. `imageSmoothingQuality = "high"` is set
  on both vector and bitmap targets.
- **Zoom: vectors during the gesture.** While the scale changes (until it has held for
  `ZOOM_SETTLE_MS` = 180 ms), frames paint vectors exactly as before, so a gesture is never
  blurrier than today and the existing "no change after rest" behaviour holds. The settle
  timer then redraws once and visible frames re-rasterize. Scaled stale bitmaps would be
  cheaper mid-gesture but blurry when zooming in. The first scale a board paints at counts as
  at rest, so the first ready paint already rasterizes.
- **Limits.** A bitmap must stay below 16,000,000 pixels and 16,384 per side (Safari: 16.7 M
  pixels per canvas), and no larger than the board canvas itself. A frame larger than the
  canvas stays vector: `paintTree` already skips its off-screen subtrees, so a whole-frame
  raster cost more than it saved (a first version did this and showed 58 ms raster spikes
  while panning at 100%). All bitmaps of a board share 32 Mi pixels (128 MiB RGBA). A
  frame over a limit paints vectors.
- **Viewport.** A frame whose extent misses the layer canvas paints nothing (nothing could
  show). Within one canvas size of the edges it keeps its bitmap; beyond that it releases it.
  Rotated/skewed transforms and unbounded extents (missing font/image placeholders) paint
  vectors.
- **Single-frame view** keeps vector paint (no `board` prop). It has one frame and no
  cross-frame invalidation, and `test:zoom` AC4 documents that a zoom allocates no canvas.

## Measurements

Harness: `npm --prefix web run test:repaint` (`web/tests/all-frames-repaint.mjs`, about 1 min).
It builds the given checkout's production viewer with test-only counters injected by a
build alias (wrappers of `paintScene`, `indexScene` and `pageView`; product code is
unchanged), runs `target/debug/opencanvas-server` on a temporary SQLite root and ephemeral
loopback ports, seeds through MCP one page of 12 frames 1440 wide (heights 1,800–4,000;
4,945 nodes, about 412 per frame: sections with strokes and radii, cards with shadow, stroke,
an image or tile, wrapped text and an outlined button, one card per section at opacity 0.8),
and drives headless Chromium 1440×900 at DPR 2. Each edit is one MCP `apply_batch` that
changes one text node in `frame-5`; wall time runs from the edit response to the status bar
showing that revision as current (`Scene rN`); counters cover that window plus 1 s.
Raw JSON and layer pixel dumps: ignored `artifacts/frame-paint/`.

"Before" is a pristine `git worktree` of `main` @`0f42dd1`; "after" this branch. Medians of
5 edits at Fit (7.7%, board fully visible); pan is 60 frames, one 40 px arrow-key step per
animation frame.

| Metric | Before | After |
| --- | ---: | ---: |
| Edit → current, wall ms (5 runs) | 197 (185–200) | 148 (141–153) |
| `paintScene` calls per edit (edited frame) | 24 (2) | 1 (1) |
| `paintScene` ms per edit | 39.3 | 1.8 |
| `indexScene` calls per edit | 12 | 1 |
| `setLoaded` publications per edit | 14 | 3 |
| React DOM / Konva reconciler commits per edit | 4 / 4 | 4 / 4 |
| Konva layer draws per edit, total ms | 2, 40.1 | 1, 3.3 |
| Long tasks (>50 ms) per edit | 0 | 0 |
| Pan at Fit: frame interval mean / max ms | 18.1 / 33.4 | 16.7 / 16.8 |
| Pan at Fit: layer draw mean / p95 / max ms | 7.4 / 17.8 / 17.9 | 0.32 / 0.30 / 6.2 |
| Pan at Fit: `paintScene` calls | 360 | 2 |
| Pan at 100%: frame interval mean / max ms | 16.9 / 33.3 | 16.7 / 16.8 |
| Pan at 100%: layer draw mean / p95 / max ms | 1.8 / 2.3 / 2.4 | 1.8 / 2.3 / 2.4 |
| Pan at 100%: `paintScene` calls | 360 | 52 |
| Edit at 100%: `paintScene` calls, layer ms | 24, 1.1 | 1, 0.7 |
| Cold All load: `paintScene` calls, layer draw ms | 78, 320 | 12, 177 |
| Warm reload (IndexedDB): wall ms, layer draw ms | 1,098, 327 | 819, 161 |
| Offscreen canvases retained at Fit | 0 px | 1.32 M px ≈ 5.3 MiB in 13 canvases (12 frame bitmaps) |
| Offscreen canvases retained at 100% | 0 px | 0 (frames larger than the canvas) |

Notes. React already batches same-task updates, so coalescing cut publications (and their
`pageView` work) from 14 to 3 but not commits in this scenario: a one-frame edit has four
separate tasks either way (snapshot, seed+reuse, computed frame, finish). Cold load stays
about one publication per frame (14 → 13) because every frame is computed in its own
animation frame. At 100% only the visible frames paint and they stay vector, so the pan cost
is unchanged within noise; the edit there repaints one frame instead of 24 calls.
Memory counts retained canvases outside Konva's own stage/layer canvases and outside
`paintScene` (whose opacity layers are transient), minus the 45,000-pixel Konva measuring
canvas; the JS heap was not separately measured.

**Pixels at rest** (layer canvas 1876×1536, same camera): at 100% the after build is
byte-identical to before (vector both). At Fit 9,129 of 2,881,536 pixels (0.32%) differ:
99.6% of those by 1–3 of 255 (compositing a transparent bitmap instead of painting onto the
layer rounds differently), 31 by 9–16, isolated along card shadow/edge seams. A standalone
Chromium check of the same rounded-rect/text paint drawn directly vs through a detached
canvas blit showed only ≤2/255 differences; the larger seam values were not traced further.

## Commands run (2026-11-10)

| Command | Result |
| --- | --- |
| `cargo build --offline -p opencanvas-server --bin opencanvas-server` | pass (debug) |
| `npm --prefix web run typecheck` | pass |
| `npm --prefix web test` | pass, 100 tests (adds `frame-paint.test.mjs`, 6 tests; updated `stale-page-scenes`, `all-frames`) |
| `npm --prefix web run build` | pass; Vite's existing >500 kB chunk warning |
| `node web/tests/all-frames-repaint.mjs --root <main worktree> --label before` | pass, no page errors |
| `npm --prefix web run test:repaint -- --label after --compare before` | pass, no page errors (final code; table uses this run) |
| `npm --prefix web run test:all-frames` | unit part 4/4 pass; browser smoke **fails** at `ready(page, 1)` (`3 !== 1`) — identical on pristine `main` |
| `npm --prefix web run test:continuity` | **fails** waiting for `Scene r7` — identical on pristine `main` |
| `npm --prefix web run test:zoom` | **fails** waiting for `Scene r7` — identical on pristine `main` |
| `git diff --check` | pass |

The three browser smokes predate All as the default view (2026-11-06): they expect a
single-frame first screen. As a diagnostic only, temporary copies that pre-seed a
single-frame selection in local storage (removed afterwards, not committed) were run on
both trees: `test:zoom` passed on this branch (AC1–AC5, AC7); `test:continuity` failed at
the same later step (line 196, `Scene pending`) on this branch and on `main`;
`test:all-frames` passed the All board, board hit selection, frame-local inspector bounds,
copy link, revision r8 with preserved camera, reload, page switching, resize and Fit on this
branch, and failed on both trees at the same later font-download count assertion (line 177,
`2 !== 1`).

## Limitations

- At high zoom a frame larger than the canvas is never cached; its visible part repaints as
  vectors on each draw (as before, culled by `paintTree`). Tiling would be the next step.
- The first draw after a zoom settles re-rasterizes every visible frame in one layer draw,
  about the cost of one former vector frame. Fit/zoom buttons are a scale change too: one
  vector draw, then the raster draw 180 ms later.
- The shared Konva layer still redraws whole: labels, frame rects with Konva shadow blur and
  every blit are drawn on each draw. Off-screen frames are skipped by the bitmap path only.
- Headless Chromium only; no Safari/Firefox run, no physical-device timing. Wheel/keyboard
  input in the harness is synthetic.
- Publication coalescing does not reduce React commits for a one-frame edit (see notes).
