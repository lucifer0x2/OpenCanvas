# Progressive viewer scenes — 2026-11-06

All is the default for new/invalid selection and project/page navigation; valid saved
single-frame choices and deep links are preserved. Authored frame dimensions reserve
the complete board before scene work. Static skeletons have no animation. One WASM
frame computes at a time and publishes after its resources finish, with compact accessible
`x/y` progress. Cooperative paint turns and background-tab admission suspension keep
the queue from monopolizing input. Cancellation stops future admissions and late
scene/resource/cache publications. Inspector and hit testing use the exact current revision.

Production WASM views share IndexedDB `opencanvas.browser-scenes` between All and
single-frame loading. Keys (schema 2) include schema, actual source/WASM content fingerprint,
engine, scene/painter versions, project, frame content digest, frame, viewport, default
theme/state; not the project revision. The digest is SHA-256 of the frame's exact layout
input without its revision, computed in WASM by `frame_keys` (`crates/scene-wasm/src/content_key.rs`
over `scene-input` `frame_content_bases`): subtree, resolved tokens/aliases/themes/styles,
expanded component instances and overrides, used font faces and asset hashes, viewport/
theme/state, plus note-derived overflow intent. One key pass per page and revision reuses
the worker's one snapshot transfer. Equal digests at a newer revision are served from a
96-entry memory tier, then disk, restamped to the accepted revision; computed scenes are
never restamped and must match the request. A module without `frame_keys` computes uncached.
Vite hashes dirty frontend/renderer/Rust scene/text sources and deployed WASM bytes;
Git HEAD alone is insufficient. Dev/HMR disables persistence to avoid a stale startup hash.
The cache never replaces the accepted HTTP/SSE snapshot or stores authored snapshots,
tokens, credentials, FontFace/images, runtime maps or closures. It stores only scene JSON.

Reads verify SHA-256 integrity, identity, pipeline, geometry and snapshot manifests.
Resource transport URLs are replaced with controlled current-project paths; fonts/assets
reload through the existing hash-checked resource loaders. Missing/failed resources are
not persisted. Retry bypasses and invalidates each matching browser cache entry. Explicit
server mode bypasses browser persistence. A WASM fallback starts at the uncompleted suffix;
server batches remain at most four and await terminal outcomes before advancing.
Unknown job transport/timeout errors stop further admission and late callbacks.

Bounds: 256 scenes, 64 MiB total, 8 MiB per scene. Entries of another schema or build
are evicted on successful writes, followed by oldest stored entries; older revisions stay
valid because keys are content digests. An index key cursor reads eviction
metadata without cloning cached payloads. Opening and each transaction have 350 ms
deadlines; unavailable/corrupt/blocked/quota storage degrades to computation. Successful
resource-ready scenes schedule writes at idle (or after paint), without awaiting disk.

Geometry packing is keyed by authored sizes, and ready frame React components and paint
callbacks are memoized. Konva still redraws the one shared layer on every board update,
but since 2026-11-10 each ready frame of the All board paints from its own device-pixel
bitmap at the resting zoom (`web/src/paint/`): a redraw caused by another frame, selection,
labels or pan blits bitmaps and runs `paintScene` only for the frame whose scene changed.
Zoom gestures, frames larger than the canvas and unbounded content paint vectors; off-screen
frames paint nothing. One page load publishes at most once per animation frame. Overflow,
shadows and the frame label/selection/skeleton overlays keep their behavior. Design,
limits and before/after measurements: [VIEWER-FRAME-PAINT.md](VIEWER-FRAME-PAINT.md).
Cold computation still does the full first-pass scene work.

Revisions are stale-while-revalidate (2026-11-09): after a first render, a new revision of
the same page/frame/preset keeps every frame's last scene on screen (`scene/staleFrames.ts`).
Frames whose content key is unchanged become current immediately, keeping their loaded
resources and the identical scene object (the accepted revision is carried on the frame
as `FrameSceneView.revision`, not restamped into `provenance`); changed frames swap in
individually; frames added show a skeleton, removed ones disappear. Progress is one quiet line that fades in only after 600 ms. A failed frame or page
keeps its last scene and reports through the compact status with Retry. Placeholders appear
only on first load, a new scope, a never-rendered frame or Retry. Stale scenes are not hit
tested or inspected as current until revalidated.

Verification performed: `npm --prefix web run typecheck`,
`npm --prefix web run build` (includes TypeScript), and `git diff --check`.
The production build retains Vite's large-bundle warning. Existing selection regression
expectations were adjusted to All. No browser, runtime, IndexedDB execution, database,
stress/benchmark or test suite was run; the user will self-test the isolated viewer.
No processes, ports, database, production deployment, commits or pushes were changed.
