# Live production acceptance

Run from the repository root with installed npm/Cargo/Chromium dependencies:

```sh
npm run test:e2e
```

The script builds the shared renderer, Rust layout/server and production viewer. Cargo runs offline; perform the initial dependency installation/build first. To reuse built artifacts, run `node spikes/e2e/run.mjs`. Loopback listening and headless Chromium must be allowed by the execution environment.

The harness uses the actual production binary and built viewer. It creates a fresh OS temporary data root and random loopback port, never writes to `~/.opencanvas`, calls no AI provider and closes its browser/MCP/server processes in `finally`. It deletes its temporary design after the run. A restart reuses the same port so browser local storage and resource URLs remain comparable.

## Protocol and assertions

1. Initialize the actual `/mcp` Streamable HTTP session, send `notifications/initialized`, negotiate protocol/session headers, list tools and call the advertised tools. Parse JSON or SSE response bodies, skipping empty SSE priming events.
2. Confirm an empty project list and reject traversal in a slug. Create one project, import a generated two-color PNG, then atomically create page/frame/text/rectangle/image and token/style references. Explicitly use `font_bundled`; resource metadata and text schema remain part of the public contract.
3. Discard a successful batch result, retry its identical body and key, and assert revision 2/replayed. A changed body with that key fails. An invalid batch creates a node before its bad reference operation; the complete batch rolls back and no revision or node remains.
4. Read outline/subtree/context/tokens/styles/assets/fonts at revision 2. Check authored references separately from resolved values, resource IDs/hash and the temporary `db.sql`/`assets` structure. Match `inspect_layout` bounds and exact text line lists to the ready render job.
5. Open the actual built viewer through the backend. Observe its received scene and the real offscreen canvas that Konva draws. Compare the entire scene, lines, provenance and exact decoded RGBA bytes with the strict worker PNG. Pixel tolerance is **zero** on the recorded Chromium runtime; no screenshot substitution or alternate renderer is used.
6. Select a layer, zoom, pan by keyboard and fit. Update the literal brand token through MCP; the browser reaches authored/scene revision 3 through production SSE without manual refresh. Its scene key and resolved color change. Repeat scene/pixel equality at 390×844 and verify increased text wrapping. Capture a 390 px shell with collapsed panels and no horizontal overflow.
7. Terminate the backend process group, restart with the same data root/port, and assert identical authored snapshot and asset bytes. A new MCP session renders revision 3 with the same scene key. Browser return and reload retain the selected project/page/frame/node. Compare pixels again and assert every observed viewer `/api/` request used GET with no browser page errors.

`browser.mjs` only observes the existing `drawImage` call by copying the real source canvases to PNG data URLs, keyed by size. It preserves the original draw call. A canvas that has itself received a canvas copy is a derived target — a Konva layer, or a zoom-out preview level — and is skipped. The viewer also rasterizes the scene at its current display scale, so the comparison selects the 1:1 canvas by the job's viewport size; that canvas is the shared renderer's unscaled output. This checks the shared image before viewport pan/zoom, shadows and selection overlays; those shell decorations are deliberately outside exported-design pixel equality.

## Recorded result — 2026-10-28

`node spikes/e2e/run.mjs` passed with Chromium 156.0.8078.4, Node v24.21.0 and MCP protocol 2025-11-25. The later `npm run test:e2e` run includes stronger typed semantic readback and inspect assertions; its current machine-readable result is authoritative.

Artifacts under [`artifacts/e2e`](../../artifacts/e2e/) contain:

- `evidence.json`: run timestamps, browser/server/protocol, successful checks, revisions, scene keys, exact pixel counts, PNG hash/dimensions and viewer request methods.
- `readback.json` and `scene.json`: actual MCP semantic results and ready revision-3 render job.
- `frame.png`: actual backend PNG, 640×480.
- `desktop.png`, `token-update.png`, `mobile.png`, `restarted.png`: actual viewer screenshots.

The verified design compares 1,228,800 RGBA channels at 640×480 for revisions 2/3 and restart, and 1,316,640 at 390×844, with zero differences. The design contains five authored nodes and one imported image. Resource readiness must be ready and strict PNG must have no error.

This acceptance fixture does not replace storage crash/fault-injection tests, SSE retention-gap/race tests, missing-resource/worker-failure tests, or migration/bundle checks. It does not test arbitrary historical revision rendering or cross-browser typography. The discarded response models the caller losing an already-committed result; it does not inject network loss at every durable-commit boundary.

## Module boundaries

`protocol.mjs` owns raw MCP and bounded waiting; `server.mjs` owns temporary backend process lifecycle; `design.mjs` owns the authored fixture and exact render-job polling; `readback.mjs` owns semantic/inspect checks; `browser.mjs` owns viewer observations/interactions/pixel equality; `restart.mjs` owns persistence verification; `run.mjs` orchestrates the single acceptance case and writes evidence. All hand-written harness modules are below 250 lines; the largest is `run.mjs` at 206 lines after formatting at this checkpoint.
