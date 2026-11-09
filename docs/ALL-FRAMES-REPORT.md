# Viewer All frames — 2026-11-06

Implemented on `feat/viewer-wasm-scene` in the separate
`OpenCanvas-viewer-wasm-scene` worktree. Changes are uncommitted. The existing
native app, launchd configuration, live database and original dirty checkout
were not deployed, restarted or modified by this task.

## Behavior

- Frame offers **All**, scoped to the selected page's root frames. Selection
  survives snapshot reconciliation/reload; switching pages preserves All.
- One shared canvas lays out actual frame dimensions in authored order. It
  evaluates shelf widths against the pane aspect, centers short rows and leaves
  space for labels. Three frames favor a horizontal row on a landscape pane;
  four or nine can use two or three rows. Counts are not hard-coded column rules.
- Labels remain 12 CSS pixels at Fit and normal zoom, with ellipsis inside the
  displayed frame width. Extreme zoom-out can hide colliding labels; full names
  remain in Layers. Packing stays stable during pan/zoom. Resize reflows the
  board while preserving the camera; Fit explicitly frames the new arrangement.
- Scenes and inspector bounds remain frame-local. Pointer hit testing applies
  board offsets without modifying authored data. Copy link addresses the real
  frame/node; a failed frame does not hide current computed bounds of a healthy
  selected frame.
- WASM computes sequentially through the existing shared worker and revision/font
  caches. Board resources share one map and load at most four manifests at once.
  All does not allocate an unused full-resolution parity canvas per frame.

## API and failures

`GET /api/v1/projects/{project}/scenes?pageId=…&revision=…&offset=0&limit=4`
returns `{projectId, revision, pageId, jobs, nextOffset, total}`. Optional paired
`width`/`height` selects a common viewport; otherwise each frame uses authored
numeric dimensions, falling back to 800×600. See [server contract](SERVER.md).

Each batch reads one immutable snapshot, validates all inputs and reserves queue
and record capacity before admitting work. Pagination reaches every root frame;
the maximum batch is four. Auth/privacy and public resource URL projection use
the same protections as the single-scene endpoint.

The server path waits for every job in a batch to reach a confirmed terminal
state before admitting the next batch. A confirmed frame failure keeps its slot
and offers Retry. Timeout, transport or identity errors stop further admission
because the previous jobs might still be running. Stale revisions and malformed
batch responses fail explicitly. WASM failure uses this bounded server path.

## Verification

Root independently ran:

```sh
npm --prefix web run typecheck
npm --prefix web test
npm --prefix web run build
CARGO_TARGET_DIR=/Users/luis/Workspace/OpenCanvas/target cargo test --locked --offline -j 2 -p opencanvas-server --test scenes -- --test-threads=1
CARGO_TARGET_DIR=/Users/luis/Workspace/OpenCanvas/target cargo clippy --locked --offline -j 2 -p opencanvas-server --lib -- -D warnings
git diff --check
```

- Web: **78 passed**, typecheck/build passed. Vite reports its bundle-size warning.
- HTTP integration: **3 passed**, covering eleven-frame pagination, dimensions,
  empty/single pages, invalid queries, public auth/privacy and resource downloads.
- API worker additionally ran scene units 3, jobs units 24, final retention units
  4, native compute 3, native export 1 and queue integration 4, all passing.
- Independent read-only review ran 18 focused tests and confirmed fixes for ID
  sentinel collision, ASCII tie ordering, timeout admission, inspector failures,
  inactive-hook Retry cancellation and board-specific zoom minimum.
- Impeccable mechanical scan of changed UI components returned no findings.
- Root used native Chrome through CUA to observe three frames in one row, four
  in 2×2, nine in 3×3, labels/gaps and frame-local inspector coordinates. The final
  label-size correction is covered by pure tests/source review; its final
  screenshot was not captured after the GUI session timed out.

Full Playwright smoke and the existing Chromium prerender test could not run:
Chromium exits before page execution with macOS MachPort error 141/SIGTRAP.
The new reproducible smoke is `npm --prefix web run test:all-frames` and must be
rerun where browser launch works. It is not reported as passed.

Modified/new source and test files have at most 236 lines after formatting. Large
existing reference documents are documentation, not newly expanded code modules.
No commit, push or production deployment was performed.
