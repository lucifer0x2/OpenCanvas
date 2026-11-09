# Progressive viewer scenes — 2026-11-06

All is the default for new/invalid selection and project/page navigation; valid saved
single-frame choices and deep links are preserved. Authored frame dimensions reserve
the complete board before scene work. Static skeletons have no animation. One WASM
frame computes at a time and publishes after its resources finish, with compact accessible
`x/y` progress. Cooperative paint turns and background-tab admission suspension keep
the queue from monopolizing input. Cancellation stops future admissions and late
scene/resource/cache publications. Inspector and hit testing use the exact current revision.

Production WASM views share IndexedDB `opencanvas.browser-scenes` between All and
single-frame loading. Keys include schema, actual source/WASM content fingerprint,
engine, scene/painter versions, project/revision/frame, viewport, default theme/state.
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

Bounds: 64 scenes, 32 MiB total, 8 MiB per scene. Old project revisions are evicted on
successful writes, followed by oldest stored entries. An index key cursor reads eviction
metadata without cloning cached payloads. Opening and each transaction have 350 ms
deadlines; unavailable/corrupt/blocked/quota storage degrades to computation. Successful
resource-ready scenes schedule writes at idle (or after paint), without awaiting disk.

Geometry packing is keyed by authored sizes, and ready frame React components and paint
callbacks are memoized. Konva still redraws the shared vector layer on board updates;
there is no new full-size raster per frame, and overflow/shadows retain their behavior.
Cold computation still does the full first-pass scene work. This change provides earlier
partial visibility and subsequent reuse; it makes no measured CPU/latency claim.

Verification performed: `npm --prefix web run typecheck`,
`npm --prefix web run build` (includes TypeScript), and `git diff --check`.
The production build retains Vite's large-bundle warning. Existing selection regression
expectations were adjusted to All. No browser, runtime, IndexedDB execution, database,
stress/benchmark or test suite was run; the user will self-test the isolated viewer.
No processes, ports, database, production deployment, commits or pushes were changed.
