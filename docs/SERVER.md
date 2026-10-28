# Local backend

The Rust process owns one data-root lock and serves the official `rmcp` 3.4.0 Streamable HTTP endpoint at `/mcp`, viewer HTTP API at `/api/v1`, and built viewer files from `web/dist`. It listens on `127.0.0.1`; port defaults to 3847. Creating the server does not create a project. `create_project` is the operation that creates `<data-root>/<slug>/db.sql` and `assets/`.

## Build and launch

From the repository root, install the locked Node dependencies and Chromium once, then build the three local artifacts:

```sh
npm ci
PLAYWRIGHT_BROWSERS_PATH="$PWD/.runtime/browsers" npx playwright install chromium
npm run build:renderer
cargo build --offline -p opencanvas-layout --bin layout-step
npm --prefix web ci
npm --prefix web run build
cargo build --offline -p opencanvas-server
```

`--offline` assumes Cargo dependencies were fetched already; use `cargo fetch` when setting up a new machine. Rust, Node and Chromium are intentional dependencies. The worker is a persistent NDJSON Node child and launches one headless Chromium. The Rust layout-step executable handles layout; the shared renderer handles text measurement and pixels.

For an isolated project root:

```sh
OPENCANVAS_TEST_ROOT="$(mktemp -d)"
target/debug/opencanvas-server --data-root "$OPENCANVAS_TEST_ROOT" --workspace "$PWD" --port 3847
```

Visit `http://127.0.0.1:3847`. `--port 0` selects a random loopback port and prints the actual URL. `--data-root` also accepts `OPENCANVAS_DATA_ROOT`; the CLI flag wins. With neither option the production default is `~/.opencanvas`. Tests always use temporary roots and random ports. Stop the process with Ctrl-C; a second process using the same data root receives a lock error. Keep the selected root when restarting to retain project IDs, revisions, nodes and resources.

The backend serves the built viewer. For frontend development, `npm --prefix web run dev` proxies `/api` to port 3847. No frontend call mutates design data.

## Connect an MCP client

Configure a Streamable HTTP server URL, for example the following client-neutral JSON shape (the surrounding configuration key varies between clients):

```json
{"type":"http","url":"http://127.0.0.1:3847/mcp"}
```

Clients initialize the MCP session, send `notifications/initialized`, retain the returned `Mcp-Session-Id`, and use `tools/list` / `tools/call`. The SDK transport supports JSON and SSE results; SSE starts with an empty priming event which is not a JSONRPC result. Concurrent JSONRPC requests must use distinct IDs. The server advertises 13 tools with explicit schemas and validates those schemas at runtime. Domain failures are tool results with `isError:true` and `structuredContent.error`; protocol failures remain MCP JSONRPC errors.

Start with `create_project`, then `apply_batch`, and read `get_outline` before requesting a scoped `get_subtree` or `get_design_context`. Text requires explicit font/style properties as described by storage validation; the bundled font ID is `font_bundled`. Mutations require `expectedRevision`, a stable `idempotencyKey`, and ordered operations. Retry exactly the original body after a lost response. A changed body with the same key receives `IDEMPOTENCY_KEY_REUSED`; a stale revision with a new key receives `REVISION_CONFLICT` and `currentRevision`.

`render_frame` accepts either `{projectId,revision,frameId,viewport,...}` or `{projectId,jobId}` for polling. Render responses include a job ID and status. Ready PNG metadata includes HTTP URI, MIME, pixel dimensions and actual Node/Chromium versions. No project filesystem paths are returned. `inspect_layout` reads bounds and line lists from a job/scene key; it does not recompute layout or provide a deeper rule explanation.

## Consistency and limits

- HTTP snapshot is authored data at one revision; its database schema version is currently 2. Worker input, computed scene and MCP read envelopes use scene/API schema 1. These version numbers describe different formats.
- Scene input is an owned immutable snapshot, with the scoped frame detached from its authored page for layout. Its source revision and scene key remain fixed if a later mutation commits. Old completed jobs remain readable by job ID until eviction; a new request for an authored revision no longer available returns `REVISION_UNAVAILABLE`.
- Scene cache identity includes project, revision, frame, viewport, theme, state, scale and a startup fingerprint of pipeline artifacts, actual Node version, installed Playwright and the browser executable identity. The cache is process-local. Restart the backend after changing Node, Chromium, layout binary or renderer/worker artifacts. PNG provenance records actual runtime versions, not guessed pins.
- Named design themes come from token declarations; unknown themes return `UNSUPPORTED_THEME`. Only the default interaction state is supported. Literal, typed token references, aliases and default/themes envelopes are resolved by storage. Authored node `props` remain intact in context; `resolvedProps`, token/style `resolvedValue` carry concrete-theme values separately.
- Tree/context pages contain at most 200 entries and 64 KiB. `depth` is 0–8 and limits descent relative to the first node of each continuation page. `nextCursor` continues a stable preorder traversal, including deeper nodes, then resource metadata, without dropping budget-trimmed entries. Each page includes scope, revision, limits and `truncated`; resource arrays are capped at 100 items per page. Resource-list tools cap the whole page at 100 items. One indivisible item exceeding the requested budget returns `CONTEXT_ITEM_TOO_LARGE`; use a property filter or larger budget.
- Cursors bind project, revision, tool, scope, filters, theme and page parameters. Repeat the same parameters with `cursor`. A concurrent mutation or changed query rejects the cursor with `INVALID_CURSOR`; restart from the outline. An explicit unavailable read revision returns `REVISION_UNAVAILABLE`.
- Mutation batches cap at 500 operations; assets cap at 10 MiB decoded and 13,981,016 base64 characters. The transport body cap is 15 MiB, including the nested MCP service. Asset import accepts bounded bytes and media metadata, never arbitrary paths or URLs.
- Rendering caps at 2,048 scoped nodes, 16 million scaled pixels, scale at most 4, one active job plus eight queued jobs, 32 retained job records and 48 MiB per worker response. Backend jobs have a 45-second deadline; the shared worker also enforces its own 30-second operation and resource deadlines. `QUEUE_FULL` is retryable. Failed jobs and ready scenes with a strict-PNG error do not become permanent cache hits: retry creates a new job while old failure readback is retained.
- A non-strict viewer scene may contain a labeled missing-image diagnostic. Strict PNG must then fail and has no image URI; `/jobs/:id/image` returns `IMAGE_NOT_READY`. Missing fonts cannot produce fabricated text measurements. Worker exit, invalid protocol or timeout fails the representation and restarts the process for subsequent jobs.
- Host is limited to loopback names and mutation origins to the configured backend origin. Only read-only asset/font routes allow random `http://127.0.0.1:<port>` origins for the headless worker. Binary routes verify storage assets and use `nosniff`.
- SSE subscribes before reading the snapshot/log, then replays durable ordered commits using `projectId:revision`. Wrong, malformed or ahead cursors, durable-log gaps and a lagged 128-event broadcast buffer cause `reset_required` with actual current revision. Heartbeats are sent every ten seconds. `scene_ready` carries source revision and scene key independently of authored commits. Current commit/idempotency retention is the project lifetime, with no expiration or background GC. Gap tests deliberately remove a temporary log row; they do not claim log-pruning support.

## Verification and backup scope

The server integration tests use raw MCP JSONRPC clients against the official `rmcp` transport and the real Axum HTTP/SSE router. `render.rs` launches actual Chromium through the worker and HTTP manifests. `failures.rs` uses a deterministic protocol process to test crash/timeout/restart and queue behavior independently of browser timing. These fixtures write only temporary projects:

```sh
cargo test --offline -p opencanvas-server --tests -- --test-threads=1
cargo check --offline -p opencanvas-server
cargo fmt -p opencanvas-server --check
```

A restricted sandbox may need permission to bind loopback for the integration command. See `docs/checkpoints/m1-server.md` for commands and measured results. Browser viewer/restart E2E is a separate acceptance seam; backend tests alone do not claim that browser flow.

Versioned backup/export/import/restore are public **storage APIs**, documented in `docs/BACKUP.md`. This server does not add backup MCP tools or client filesystem access. Live database main-file copies are not backups; use the storage bundle implementation which captures a consistent database and matching resource manifest/binaries.
