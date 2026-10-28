# OpenCanvas

OpenCanvas lets an agent create and inspect local designs through MCP while a read-only browser viewer displays the result. A Rust backend owns SQLite, revisions, validation and rendering jobs. The viewer and PNG exporter share one Canvas2D renderer and precomputed text lines.

The browser offers project/page/frame selection, layers, inspection, pan, zoom, fit and viewport presets. Design changes come from MCP. Starting the server creates no demo project.

## Local setup

The verified development environment is Rust/Cargo 1.99.1, Node 24.21.0, npm 11.19.0 and Playwright Chromium 156.0.8078.4. Dependencies are pinned in the lockfiles. From this repository:

```sh
npm ci
npm --prefix web ci
export PLAYWRIGHT_BROWSERS_PATH="$PWD/.runtime/browsers"
npx playwright install chromium
npm run build:renderer
cargo build -p opencanvas-layout -p opencanvas-server
npm --prefix web run build
```

The first installation needs registry/network access. With dependencies cached, the Cargo build also works with `--offline`. The production build currently reports a JavaScript chunk-size warning of about 575 kB.

Start the production server from the repository root:

```sh
export PLAYWRIGHT_BROWSERS_PATH="$PWD/.runtime/browsers"
./target/debug/opencanvas-server --workspace "$PWD"
```

Open [the local viewer](http://127.0.0.1:3847). The backend serves `web/dist`, launches the Node/Chromium rendering worker when needed and listens only on loopback. Stop it with Ctrl-C: open event streams are told to end, and a second Ctrl-C (or three seconds) leaves immediately. Keep the repository's built renderer, layout binary, worker and browser installation available while running.

The default data root is `~/.opencanvas`; `OPENCANVAS_DATA_ROOT` or `--data-root DIR` overrides it. `--port NUMBER` changes the port, and `--port 0` chooses a free port printed at startup. Each project uses `<data-root>/<slug>/db.sql` (SQLite binary) and `assets/`. Only `create_project` creates a project. Use a separate temporary data root for experiments and tests.

For frontend development, keep the backend running and run `npm --prefix web run dev`. Its `/api` proxy defaults to port 3847; set `OPENCANVAS_API_URL` for another backend URL. See [viewer behavior](docs/VIEWER.md) and the [HTTP contract](docs/CONTRACT.md).

## Connect an agent

Configure an MCP client with **Streamable HTTP** and URL `http://127.0.0.1:3847/mcp` (adjust the port if needed). This is an HTTP endpoint, not a stdio command. A minimal connection entry is:

```json
{ "url": "http://127.0.0.1:3847/mcp" }
```

The surrounding configuration format depends on the client. The client initializes a session and retains the negotiated MCP session/protocol headers. The checked transport negotiates protocol `2025-11-25`.

Use `list_projects`/`create_project`, then `get_outline` before scoped `get_subtree` or `get_design_context`. Resource metadata is available through `list_tokens`, `list_styles`, `list_assets` and `list_fonts`. `import_asset` accepts bounded base64 bytes, never filesystem paths. `apply_batch` takes a `projectId`, `expectedRevision`, stable `idempotencyKey` and ordered operations. Reuse the exact body/key after a lost response; read back after a revision conflict.

Create a page, then a frame, then its descendants. Text must declare a valid font (the bundled font is `font_bundled`), size and line height, or obtain them from a valid text style. `render_frame` returns a job; poll it by `jobId`, inspect its exact revision/provenance and obtain the PNG through `imageUrl`. `inspect_layout` reads the bounds and text lines for that job's scene key. Available tools expose their schemas through `tools/list`.

To implement an existing design in code rather than author one, read `get_outline` for the frame,
then `get_design_context` for that `frameId`. Nodes carry authored `props` beside `resolvedProps`:
the authored side holds the token and style references the design was built from, the resolved
side what they evaluate to for one theme, so code written against the reference survives a theme
change. Build the structure the design declares — a container's layout `mode` is `row`, `column`
or `free`, with `padding` and `gap` — instead of deriving it from measured bounds; those bounds
and the wrapped text lines belong to one viewport and are there to check the result. The design
covers theme `default` and state `default` and states nothing about interaction, animation or
data. The connect-time MCP instructions repeat this, so a client sees it without reading this
file.

## Verification

```sh
npm run test:e2e
```

This builds the required artifacts using cached Cargo dependencies, starts the production Rust server on a random loopback port with a temporary data root, and drives MCP plus Chromium. It terminates the test processes and removes the temporary project afterward. A sandbox must permit loopback listeners and Chromium. The shorter `node spikes/e2e/run.mjs` reuses existing builds.

The live acceptance run verifies MCP creation/import, SQLite structure, typed authored/resolved readback, exact scene and PNG provenance, viewer/PNG pixel equality, token updates through production SSE, narrow viewport wrapping, invalid-batch rollback, idempotent retry, and persistence through backend restart and browser reload. It also checks that viewer API traffic uses GET only. See [the E2E protocol and limitations](spikes/e2e/README.md), [machine-readable evidence](artifacts/e2e/evidence.json), [PNG](artifacts/e2e/frame.png) and [actual viewer screenshot](artifacts/e2e/desktop.png).

Targeted module checks remain separate:

```sh
npm run check:renderer
npm run test:pipeline
npm --prefix web run typecheck
npm --prefix web test
cargo test -p opencanvas-storage --offline --features fault-injection -- --test-threads=2
cargo test -p opencanvas-server --offline -- --test-threads=2
```

The live E2E covers one small project on the recorded Chromium runtime; it does not establish cross-platform pixel identity or every failure/recovery case. Consult [PLAN.md](PLAN.md), [storage benchmark](spikes/storage/README.md), [resolution scope](docs/RESOLUTION.md) and [backup/restore](docs/BACKUP.md) for requirements, limits and module evidence. Milestone acceptance is tracked separately in [docs/ACCEPTANCE.md](docs/ACCEPTANCE.md).
