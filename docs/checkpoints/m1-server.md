# M1/M2/M3 server checkpoint — 2026-10-28

Server implementation and targeted runtime verification are complete within the scope below. No commit, push, reset, checkout or write to `~/.opencanvas` was performed. The shared workspace is `/Users/luis/Workspace/OpenCanvas`, branch `main`, with changes from several workers; preserve the whole dirty tree. Parent owns overall acceptance and documentation.

## Implemented surface

- Official `rmcp=3.4.0` Streamable HTTP `/mcp`: real initialization/session lifecycle, 13 advertised tools, typed input schemas enforced by a small runtime validator for precisely the emitted JSON Schema vocabulary. HTTP and MCP both cap request bodies at 15 MiB; imports cap decoded bytes at 10 MiB. Tool errors are structured `isError` results; HTTP query/body failures use `{error}`.
- Loopback-only CLI listener, default 3847, random port with `--port 0`, explicit `--data-root` / `OPENCANVAS_DATA_ROOT`, production `web/dist` serving. Host/Origin guards restrict mutations; loopback worker origins are allowed only for read-only asset/font routes. No arbitrary client paths or URLs.
- SQLite work via `spawn_blocking`; commits are durable before response/broadcast. Atomic batches, revision conflicts and idempotency replay are storage-backed. Replay does not broadcast another authored commit. Automatic frame compute is bounded and independent of commit success.
- Authored snapshot HTTP API, verified asset/font bytes, bounded render jobs and PNG API. Immutable scoped frame snapshots detach their authored page parent in worker input. Exact revision R remains on job/scene/provenance when R+1 commits; new requests for unavailable authored revisions fail explicitly.
- SSE subscribes before reading snapshot/log. Broadcast is a wakeup channel; commit delivery always comes from the ordered durable log. Replay chunks cap at 1,000, heartbeat 10 seconds, broadcast buffer 128. Invalid/ahead/wrong-project cursors, explicit durable-log gaps and lag reset using actual current revision. Current retention is lifetime/no expiry; there is no log pruning or GC implementation.
- Named-theme resolution uses the concrete storage APIs. Context preserves authored `props` and adds `resolvedProps`; tokens/styles retain authored values plus `resolvedValue`. Cursor pagination covers all nodes/resources with no silent budget truncation, binds revision/tool/project/scope/query and rejects stale/mismatched cursors. Tree pages cap at 200 entries, 64 KiB, depth 8; resource arrays cap at 100. Depth limits descent relative to each continuation page's first node. An indivisible oversize property returns `CONTEXT_ITEM_TOO_LARGE`.
- Frame render cap 2,048 nodes, scaled pixels 16 million, scale 4, queue 8 plus one active, retained jobs 32, worker response 48 MiB. One persistent Node/Chromium process, outer 45-second job deadline and worker internal limits. Worker crash/timeout/protocol/strict-export failure restarts the child. A new enqueue retries failed cache entries while retaining old job readback. Queue/worker timeout failures are retryable.
- Startup cache fingerprint includes Node version, installed Playwright, actual browser/headless-shell executable metadata and pipeline files. Runtime replacement requires backend restart; cache is process-local. Successful PNG provenance contains actual Chromium and Node versions. Job scene key matches scene `sourceSceneKey` before strict export. PNG bytes must have a valid signature/IHDR dimensions matching metadata and the pixel budget before an image URI is published.
- Missing images can produce a non-strict diagnostic scene but never strict PNG success. Missing fonts do not fabricate measurement. `inspect_layout` reads exact job/scene bounds/lines/status, with correct filtered-scope truncation; deeper rule explanations remain outside this version.

## Files and module boundaries

Owned `crates/server/**`, `docs/SERVER.md` and this checkpoint. Added `rusqlite=0.41.0` only as a dev dependency (already the storage dependency) for deliberate temporary-log corruption tests; Cargo.lock package dependency metadata changed accordingly. No storage/layout/viewer/worker sources were modified.

- `http.rs`, `mcp.rs`: HTTP and MCP transport adapters.
- `schemas.rs`, `schemas/{operations,validation}.rs`: published bounded schemas and runtime schema enforcement.
- `domain.rs`: mutation/import orchestration and durable notifications.
- `reads.rs`, `reads/{tree,pagination}.rs`: authored/resolved reads and revision/scope-bound continuation.
- `jobs.rs`, `jobs/{input,png}.rs`: immutable queue/cache, controlled manifests and PNG validation.
- `worker.rs`, `fingerprint.rs`: process protocol/lifecycle and installed runtime cache identity.
- `events.rs`, `state.rs`, `error.rs`, `main.rs`: replay stream, shared state/DB seam, envelopes and CLI.
- Tests: `transport.rs`, `context.rs`, `render.rs`, `failures.rs`, `support/mod.rs`.

All handwritten server sources and tests are below 250 lines. Largest source: `jobs.rs` 216 lines; largest test: `transport.rs` 213 lines at final count. No modular-size exception is needed.

## Actual commands/results from this worker

| Command | Actual result |
| --- | --- |
| `cargo check --offline -p opencanvas-server` initial | Found partial-move error in old reads.rs; fixed during read split |
| `cargo check --offline -p opencanvas-server` after implementation | PASS, including latest PNG guard |
| `cargo test --offline -p opencanvas-server --test transport -- --test-threads=1` sandbox | Bind denied `EPERM`; this did not indicate a backend failure |
| Same targeted tests with approved loopback escalation | Initial harness errors corrected: skip SDK empty SSE priming events and use distinct concurrent JSONRPC IDs |
| `cargo test --offline -p opencanvas-server --test transport -- --test-threads=1` escalated final | **3 passed**, 0.97 s; actual MCP lifecycle/list/call, atomic rollback, valid changed-payload key reuse, concurrent same-base conflict, lost-response replay, HTTP snapshot/guards/errors, durable SSE replay/live wakeup/reset and deliberate SQL log gap reset |
| Context suite as part of `cargo test --offline -p opencanvas-server --tests -- --test-threads=1` escalated | **1 passed**; preorder/depth continuation, stale/scope cursor rejection, explicit authored/resolved named theme, complete resource continuation; the same run initially exposed and later-fixed render root-parent bug |
| `cargo test --offline -p opencanvas-server --test render -- --test-threads=1` escalated | **2 passed**, 1.88 s after detaching frame parent; actual Node/Chromium worker through HTTP manifests, PNG dimensions/provenance/exact R after R+1/theme keys, missing image diagnostic + strict failure + identical-request recovery |
| `cargo test --offline -p opencanvas-server --test failures --test transport -- --test-threads=1` escalated | Failure orchestration **3 passed**, 1.66 s; transport harness concurrent-ID bug fixed afterwards |
| `cargo test --offline -p opencanvas-server --test failures --test render -- --test-threads=1` escalated final | **5 passed**, 3.47 s after PNG signature/dimension validation; fake process only for deterministic crash/timeout/restart/queue seams, separate real Chromium tests remain in render.rs |
| `cargo fmt -p opencanvas-server --check` | PASS |
| `cargo clippy --offline -p opencanvas-server --lib --bin opencanvas-server -- -D warnings` | Stopped at three dependency-scope storage lints (bundle integer expression/modulo and bundle_validation type complexity); storage ownership respected |
| `cargo clippy --offline --no-deps -p opencanvas-server --lib --bin opencanvas-server -- -D warnings` final | **PASS**, server warnings corrected |

Nine targeted server integration tests have passed across these separate runs. Parent will run the final full server suite independently; do not turn the separate per-suite evidence above into a fabricated single-run result.

## Integration handoff and remaining scope

The separate live E2E worker reported `npm run test:e2e` and subsequent `node spikes/e2e/run.mjs` passing production MCP → SQLite → HTTP/SSE → viewer → PNG → process restart, including exact RGBA comparisons at desktop/mobile sizes, read-only viewer requests and semantic readback. Artifacts are under `artifacts/e2e/`; parent independently viewed screenshots and owns acceptance of those results. This server worker did not drive the browser UI.

Read `docs/SERVER.md` for build/launch/MCP commands and all supported caps. Backup/export/import/restore remain public storage APIs, documented in `docs/BACKUP.md`; there are no extra server backup tools. Deep layout explanation/diff, arbitrary interaction states, historical authored snapshot storage, automatic log expiration/GC and automatic runtime hot replacement are unsupported explicitly, without claiming these as implemented.

No server/listener/browser is left running by this worker. Test child processes use kill-on-drop and their own runtimes; completed command sessions are terminal. No user project was created outside temporary roots.
