# M0 layout / renderer decision

Accepted on 2026-10-28 for the bounded MVP row/column layout contract. This spike is reusable implementation, not an image mockup. Taffy 0.15.1 performs real layout with a synchronous custom leaf callback. Browser Canvas2D supplies text metrics, shaping and one wrapping implementation. Authored data is never rewritten with bounds or lines.

## Module boundaries

- `crates/layout/src/model.rs`: authored input and computed-scene DTOs. `validation.rs`: supported geometry/tree and budgets. `tree.rs`: authored-to-Taffy tree. `engine.rs`: cache-only callback and scene projection. `util.rs`: deterministic hashes and measure keys. `main.rs`: NDJSON adapter.
- `renderer/types.ts`: web contract. `resources.ts`: verified font/image loading. `measurement.ts`: the only wrapping implementation. `drawing.ts`: scene painting without measurement. `index.ts`: public exports.
- `worker/runtime.mjs`: Chromium plus allowlisted loopback bundle server. `pipeline.mjs`: one compute/render job, measurement batching, bounded cache, deadlines/cancellation. `cli.mjs`: NDJSON process adapter.
- `spikes/pipeline/`: authored fixtures plus geometry, lifecycle and pipeline assertions. No user project data is created.

Files are conventionally formatted. At handoff, maximum handwritten file size is 230 lines (`pipeline.test.mjs`), worker pipeline 226, Rust model 225 and engine 183; none exceeds 250. Short DTO/export/entry files intentionally remain small.

## Pinned runtime and font

| Component | Tested pin |
| --- | --- |
| Rust Taffy | `=0.15.1`, Flexbox and tree only |
| Playwright | `1.64.1`, lockfile pinned |
| Chromium | Chrome for Testing `156.0.8078.4`, Playwright build `1248` |
| Node | `v24.21.0` on macOS arm64 |
| TypeScript / esbuild | `7.0.2` / `0.28.2` |
| Scene / renderer | `scene-1` / `web-1` |
| Font | Bundled Noto Sans, 400 normal; SHA-256 `bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d` |

The unmodified font binary, copyright, SIL OFL 1.1 license and source manifest are under `renderer/fonts/`. Family used at runtime is `OpenCanvas Noto Sans`, not a system-font lookup. Fetch bytes are SHA-256 checked, loaded through `FontFace`, added to `document.fonts`, awaited and checked before measurement. Images are hash-checked and `decode()` is awaited. Resource fetch/decode/loading has a 10 second deadline. Strict failure includes structured codes and resource IDs, never local paths. A missing project font has no implicit fallback; computation fails explicitly. Non-strict viewer computation supports missing-asset placeholders and diagnostics.

## Exact integration contract

Rust exports `compute(&LayoutInput, &MeasureCache) -> Result<LayoutStep, LayoutError>`. `LayoutStep` is `{status:"pending",requests:[...]}` or `{status:"ready",scene:{...}}`. The public serializable types are re-exported by `opencanvas-layout`; the equivalent browser types and functions are exported by `renderer/index.ts`.

Input has `schemaVersion:1`, `projectId`, `revision`, `frameId`, `viewport:{width,height}`, resolved `theme`/`state` (default `default`), authored `nodes`, `fonts`, and `assets`. Nodes accept storage's extra `sortKey`; backend must project a snapshot into stable sibling paint order before computation. Frame scope includes its descendants; page/other roots are not drawn. The selected root is fixed to the requested viewport.

Font entries are `{id,family,hash,url,weight,style}`. Asset entries are `{id,hash,url,mediaType}`. URLs must point to controlled HTTP routes: the spike serves `/renderer/fonts/NotoSans.ttf`; backend may serve identical font bytes at `/api/v1/fonts/font_bundled`. Backend assets use absolute loopback URLs because the worker's bundle server has a different origin. Asset responses need CORS authorization for that worker origin (or equivalent worker routing); `data:` fixture URLs also work. No arbitrary local path is accepted by the worker protocol.

Computed scene contains separate flat pre-order `nodes`, frame-local absolute `bounds`, precomputed local text `lines` (each `{text,x,y,width}`), font/asset manifests, diagnostics and provenance. `parentId` supports ancestor clipping and opacity without another tree layout. Provenance contains project/revision/frame, concrete viewport/theme/state, pipeline versions, deterministic source scene key and manifest hashes. PNG adds scale, Chromium/Node versions and resource readiness.

```js
import { PipelineWorker } from '../../worker/pipeline.mjs';
const worker = await PipelineWorker.start();
try {
  const result = await worker.compute(input, {strict:false}); // viewer missing-image diagnostics
  const png = await worker.render(result.scene);              // strict by default
} finally { await worker.close(); }
```

`node worker/cli.mjs` uses one JSON object per input/output line. Requests: `{id,op:"compute",input,strict?}`, `{id,op:"render",scene,scale?,strict?}` or `{id,op:"compute_render",input,scale?}`. Success includes `status:"ready"`; failure includes `status:"failed",code,message`. Render returns PNG `base64`, MIME type, dimensions and provenance, which backend exposes as a bounded resource. CLI remains sequential; backend may terminate its process to cancel. Programmatic `cancel()` rejects the active job and closes the page; restart that worker before another job.

Viewer imports `loadResources(fonts,assets,{strict:false})` and `drawScene(canvas,scene,resources,{strict:false})` once at 1:1 as the parity canvas, then paints the scene with the same `paintScene` from a Konva.Shape `sceneFunc` under the stage zoom, with Konva selection/highlight rectangles over computed bounds. This Canvas2D drawing decision permits byte-identical worker/viewer rasterization without Konva internals or Konva.Text wrapping. Konva supplies the UI stage, transforms and interaction overlays. It does not compute design layout.

## Batched cache and convergence

1. Rust builds the Taffy tree, disables integer rounding, and invokes `compute_leaf_layout` from Taffy's synchronous callback.
2. Callback looks up a key made from renderer version, exact text, font ID/hash/family/weight/style, font size, explicit line height and width quantized to 1/64 logical pixel. Missing entries are recorded in a map. Internal zero values only discover constraints; a pass with any miss cannot publish a scene.
3. Node sends the entire miss batch in one `page.evaluate`, not one RPC per text leaf. Canvas wraps and returns width/height and the final line list for every key.
4. Rust rebuilds and reruns from the enriched cache. Final projection also requires a measurement for each final text width. At most six passes may run; non-convergence is an explicit failure.

No SQLite transaction or writer queue belongs in this compute path. Cache is process-local to one Chromium instance, never shared between runtime versions; more than 10,000 prior entries causes reset before a job. It can be discarded safely. Future persisted caches must include Chromium/OS pipeline identity. Resource manifests are checked again for rendering. A scene/resource renderer/hash mismatch rejects drawing.

## Supported rules and budgets

- Row/column, scalar padding/gap; fixed/fill/hug, min/max; free children with parent-local x/y. Fill inside an unresolved hug axis is rejected. No percentages, CSS Grid, baseline layout, rich text or transforms/rotation.
- Root frame fixed to viewport; at most 2,048 input nodes, tree depth 128, text 64 KiB per node and 1 MiB across input. Logical dimensions and absolute position magnitude at most 100,000.
- Wrap collapses horizontal whitespace, preserves explicit LF, breaks long words at Vietnamese grapheme boundaries using `Intl.Segmenter`. Baseline = `fontSize + (lineHeight-fontSize)/2`; line y advances by explicit lineHeight. Height = line count times lineHeight. Coordinates/widths snap to 1/64 pixel; drawing never recomputes widths or line breaks.
- Parent clip propagates through frame-local absolute bounds; frames clip by default, groups only when requested. Overflow otherwise paints. Rectangle radius, per-node inherited opacity, contain/cover/stretch images and normalized source crop are supported. Public `fit` and internal `fill` both mean stretch. Crop `{x,y,width,height}` lies in [0,1] with positive area inside the source. Opacity multiplies through ancestors; isolated group compositing is not implemented.
- Render max 16,000,000 physical pixels, scale >0 and <=4. One active job per worker, 30 second whole-job deadline, 5 second Rust subprocess deadline, six layout passes. Timeout/cancel closes the page and requires restart. Chromium/process failure only fails this compute job.
- Pixel comparison tolerance is zero between two canvases on this pinned runtime; no cross-OS/browser/font pixel-perfect claim. Browser/version difference requires invalidation or a documented tolerance before using cross-runtime snapshots.

## Reproduce and measured results

```sh
npm ci
PLAYWRIGHT_BROWSERS_PATH="$PWD/.runtime/browsers" npx playwright install chromium
cargo test -p opencanvas-layout
npm run check:renderer
npm run test:pipeline
```

The sandbox initially rejected loopback binding with `listen EPERM`; tests were rerun with explicit sandbox escalation for a loopback-only server. Real Chromium ran successfully; no fake or skipped PASS was substituted. Generated evidence is under `spikes/pipeline/output/` (ignored, recreated by tests): `evidence.json`, `wide.scene.json` and `fixture.png`.

First real run, Node24.21.0/Chromium156.0.8078.4: 6/6 assertions passed. At viewport420, the title occupied width380, two lines, hug height64; at viewport240, width200, four lines, hug height128. Wide compute used two layout passes, one batch RPC and three measurements; narrow used two passes, one batch and two measurements. Repeating wide used one pass and zero measurement RPC. Export and independent viewer canvas PNG bytes were identical with `measureText` disabled during drawing. A real image decode delayed by180ms was awaited (observed194ms in recorded run). Missing font, font hash mismatch, missing image and strict missing-font PNG all failed readiness. Geometry sampled exact RGBA values proving sibling paint order, nested clip, global frame-local positioning and crop.

Subsequent scope added non-strict missing-image viewer output, per-resource decode timeout, concurrency and explicit cancellation. The lifecycle fixture caught a page/browser close race; the fix awaits page cancellation before closing Chromium, then its targeted 2/2 rerun passed. Final command results are recorded in `RESULTS.md`; do not infer full backend or M1/M2 acceptance from this M0 spike.

Primary APIs inspected: [Taffy 0.15.1](https://docs.rs/taffy/0.15.1/taffy/), its downloaded `compute_layout_with_measure` source, [Playwright BrowserType](https://playwright.dev/docs/api/class-browsertype), and [Google Fonts Noto Sans](https://github.com/google/fonts/tree/main/ofl/notosans).
