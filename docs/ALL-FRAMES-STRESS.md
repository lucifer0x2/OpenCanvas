# All-frame stress — 2026-11-06

Reproducible harness: [spikes/all-frames-stress/README.md](../spikes/all-frames-stress/README.md).
Source-only additions on `feat/viewer-wasm-scene`; no commit, push, deployment,
native service restart, live database write or `~/.opencanvas` change.

## Fixture and environment

Four temporary SQLite projects each have one page, nine root frames and exactly
**10,000 renderable nodes** (nine frames, 1,999 text nodes and 7,992 rectangles).
The page container makes **10,001 snapshot nodes**. Frame zero has 1,112 computed
nodes; the other eight have 1,111 each. Every frame is 800 pixels wide. Heights:

| Profile | Frame heights |
| --- | --- |
| short | 9 × 600 |
| medium | 9 × 1800 |
| long | 9 × 6000 |
| mixed | 600, 1200, 2400, 4800, 9600, 1800, 6000, 900, 3600 |

Coordinates fill each frame's height with a deterministic 32-column grid; all
content stays inside its frame. Text uses the pinned bundled Noto Sans font,
8 px text/10 px line height. There are no images or network fonts. This dense
grid isolates node count and page height; it does not represent complex effects,
components, images or rich typography. Separate projects respect the current
20,000-node/project cap.

Machine: Apple M5, 10 cores, 24 GiB RAM, macOS Darwin 25.4.0 arm64, Node 24.21.0.
The native executable was rebuilt from the worktree with locked/offline Cargo,
two build jobs, **Debug/unoptimized** profile. WASM uses the existing generated
release artifact in `web/public/scene-wasm`. No Release-native comparison or SLA
was established.

## HTTP and actual WASM results

All four projects were authored through real MCP calls in 21 atomic batches
of at most 500 operations. All snapshots had revision 21 and 10,001 nodes;
conditional GET returned HTTP 304 for every ETag. Actual wasm-bindgen
`SceneComputer` computed all nine scenes for every profile, with exact node
counts, revisions and authored viewport dimensions checked. One bundled font was
registered per page computer.

| Profile | Seed time | Snapshot bytes | Latest GET | ETag GET | First compute, sum of 9 frames | Warm compute, sum of 9 frames (3 runs) |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| short | 15.34 s | 2,039,161 | 259 ms | 1.7 ms | 1.460 s | 1.424 / 1.402 / 1.407 s |
| medium | not retained after interrupted first attempt | 1,876,167 | 256 ms | 3.9 ms | 1.375 s | 1.376 / 1.368 / 1.366 s |
| long | 18.69 s | 1,883,343 | 292 ms | 5.4 ms | 1.898 s | 1.882 / 1.915 / 1.916 s |
| mixed | 28.05 s | 1,886,017 | 406 ms | 2.7 ms | 3.465 s | 3.404 / 3.466 / 3.543 s |

Node first/warm compute includes Rust snapshot parse/resolution/layout/text/scene
JSON and JS scene parse. Snapshot serialization, required-font discovery, font
download/decode, module initialization and browser messaging/painting are outside
those compute columns. Discovery summed 1.271 / 1.202 / 1.647 / 3.266 seconds.
WASM module initialization was measured separately. The complete page harness
(nine discoveries plus 36 computations and font loading) took 7.12 / 6.83 /
9.43 / 17.48 seconds.

Long/mixed Node measurements overlapped normal asynchronous native commit
prerenders during authoring. Their higher timings cannot be attributed to page
height alone. Browser measurement starts after restarting only the temporary
backend to clear that backlog.

## Native API and limits

The initial short-page `/scenes` run completed all nine actual scene jobs at
revision 21, summing exactly 10,000 nodes with no reported errors. Batches of
4/4/1 took **53.78 / 52.21 / 21.82 seconds**, total **127.81 seconds**. Sampled
native RSS after batches peaked at **145.8 MiB**. This includes real strict PNG
rendering, queue waiting and the post-seed background prerender backlog. Initial
HTTP polling was 25 ms; the final harness uses the viewer's 500 ms interval.
These native numbers are not comparable to the WASM compute-only columns.

An initial medium batch still had a pending job at the old 120-second harness
window. The harness stopped and preserved the temporary authored fixtures.
This is an observed end-to-end delay, not proof of a deadlock: queued jobs have
their own execution deadline after starting. The final harness observes up to
240 seconds per batch and refuses further admission while a job remains pending.
Native remaining profiles run after browser measurements, in fresh temporary
processes to exclude seed prerenders.

Because a human-operated browser was unavailable while the parent session was
interrupted, the standalone native continuation ran against independent SQLite
`.backup` clones of the real authored fixtures. **Medium completed nine scenes /
10,000 computed nodes at revision 21 in 417.95 seconds**. Its 4/4/1 batches took
139.54 / 188.07 / 90.34 seconds. Frames 0–2 produced PNGs; frames 3–8 retained
their complete computed scenes with `JOB_TIMEOUT` and no PNG. A `ready` scene
with a render error is not a successful strict PNG. Sampled RSS after batches
peaked at 46.1 MiB; this is not a continuous peak measurement.

The native deadline includes CPU-task draining: it is not a hard wall-clock kill
of a paint operation. The observed 90-second final batch must not be presented as
a job finishing within its configured 45-second execution deadline.

A separate real project with 10,000 nodes in one frame was successfully authored.
Its scene request returned **HTTP 400**, `VALIDATION_FAILED`, message
`Frame exceeds 2048 render nodes`. The limit was not raised. A page with 10,000
nodes across nine frames and one frame containing 10,000 nodes have different
current support limits.

## Browser and remaining work

The test-only Vite preview mounts actual product `main.tsx`, with persisted All
selected before mounting. It observes real Worker messages and uses existing DOM
controls. It records cold All with default Layers open, warm All with Layers open,
warm All with Layers closed, actual board dimensions/labels/Fit, DOM counts,
long tasks, resources, exposed JS heap and RAF gaps during synthetic wheel and
keyboard input. It posts raw results to `artifacts/all-frames-stress/browser.json`.
No scene or job response is mocked.

Browser execution and remaining native page runs are pending at this report
checkpoint. The earlier Chromium shell-launch failure is avoided by navigating
the preview in native Chrome through CUA. A prepared preview is not a browser
pass. Product fixes are intentionally outside this stress task.

## Verification and artifacts

- Fresh targeted native binary build passed.
- Syntax checks passed for the harness modules.
- Deterministic fixture validation passed for all four profiles: exactly 10,000
  renderable nodes, nine frames, 1,999 text nodes, every leaf inside frame bounds.
- Actual HTTP snapshot/ETag and actual Node WASM checks passed for all four pages.
- Preview HTML and instrumentation module both returned HTTP 200; served script
  passed `node --check`. This verifies harness serving, not browser execution.
- Hand-written modules remain below 250 lines. Browser observation, DOM probing,
  fixture authoring, backup cloning, process lifecycle, native measurement, WASM
  measurement, preview serving and orchestration have separate responsibilities.

Generated snapshots/JSON/logs remain under ignored `artifacts/all-frames-stress/`.
The report and harness sources are the reviewable artifacts; no generated raw
evidence is added to version control. See the harness README for run/resume and
cleanup commands.
