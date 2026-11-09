# All-frame stress harness

Run from the isolated `OpenCanvas-viewer-wasm-scene` worktree. Requires existing
Node dependencies and generated `web/public/scene-wasm` bindings. No live database,
native launchd service, original checkout or fixed application port is used.

```sh
CARGO_TARGET_DIR=/Users/luis/Workspace/OpenCanvas/target cargo build --locked --offline -j 2 -p opencanvas-server --bin opencanvas-server
node spikes/all-frames-stress/run.mjs
```

`STRESS_SERVER_BIN=/absolute/path/to/debug/opencanvas-server` overrides the executable.
The runner uses a new OS temporary SQLite data root and ephemeral loopback ports.
Four projects each contain one page, nine 800-pixel-wide frames, and exactly
10,000 renderable nodes including the frames; page containers add one snapshot
node. Short/medium/long heights are 600/1800/6000. Mixed heights are
600/1200/2400/4800/9600/1800/6000/900/3600. A fifth project probes 10,000 nodes in
one frame, expected to hit the explicit 2048-node render limit.

Authoring uses real MCP `create_project` and 500-operation `apply_batch` calls.
Native scene jobs use the actual `/scenes` API in batches of four and wait for
every job to terminate before admitting another batch. Native page timing includes
the pipeline's strict PNG work. The observation window is 240 seconds per batch
polling phase, sampled every 500 ms; pending work stops further page admission.
This window is a harness bound, not an SLA. Each native profile starts with a fresh
process and independent SQLite `.backup` clone of the actual authored fixture,
so previous jobs and commit prerenders do not contaminate later profiles. Each
data root has one backend owner; the preview keeps its original temporary root.

Node uses the actual wasm-bindgen `SceneComputer`, original HTTP snapshots and
bundled font bytes. Each frame has font discovery, one first computation and three
warm computations. Node timings exclude browser messaging, Konva, Layers and paint.

When `PREVIEW_URL` appears, open it in a real browser. A Vite test-only script
mounts the unchanged product `main.tsx`, sets persisted All before mounting,
observes actual Worker results, and operates existing Frame/panel controls.
It automatically visits all four profiles, measuring cold All, warm All with
Layers open, warm All with Layers closed, layout/Fit/labels, DOM counts, long tasks,
requests, exposed heap and synthetic wheel/keyboard RAF gaps. Wheel/keyboard input
is synthetic DOM input, not a physical-device latency measurement. Node/native
work pauses during the browser phase. Native remaining checks start after browser
results are posted. No browser launch or CDP is required.

If a human-operated browser is unavailable, `node spikes/all-frames-stress/native-only.mjs`
continues remaining native profiles into `artifacts/all-frames-stress/native.json`
using independent backup clones. Run this while the preview is idle; stop that
process before browser measurement to avoid CPU contention.

Raw snapshots, metrics, browser results and logs live under ignored
`artifacts/all-frames-stress/`. `measurements.json` is checkpointed per profile.
If interrupted, `node spikes/all-frames-stress/run.mjs --resume` reuses the exact
temporary fixtures recorded there and preserves the prior attempt. Resume expects
fully seeded fixtures; a partially seeded project must be investigated explicitly.

Stop with Ctrl-C (or SIGTERM to the `harnessPid` in `session.json`). Only the
harness's temporary processes are stopped. Temporary SQLite data remains for
review; remove that specific `dataRoot` from `session.json` when no longer needed.
Do not use this harness against live ports or a user data root.
