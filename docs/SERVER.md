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

`--host` takes `127.0.0.1` (default) or `0.0.0.0`, which listens on every interface for a container whose port is published on host loopback only; Host/Origin checks are unchanged, and worker callbacks and `imageUrl` still use `http://127.0.0.1:<port>`, so no other address is accepted. See "Docker" in `README.md`.

The backend serves the built viewer. For frontend development, `npm --prefix web run dev` proxies `/api` to port 3847. No frontend call mutates design data.

## Connect an MCP client

Configure a Streamable HTTP server URL, for example the following client-neutral JSON shape (the surrounding configuration key varies between clients):

```json
{"type":"http","url":"http://127.0.0.1:3847/mcp"}
```

Clients initialize the MCP session, send `notifications/initialized`, retain the returned `Mcp-Session-Id`, and use `tools/list` / `tools/call`. The SDK transport supports JSON and SSE results; SSE starts with an empty priming event which is not a JSONRPC result. Concurrent JSONRPC requests must use distinct IDs. The server advertises its tools with explicit schemas and validates those schemas at runtime. The tool count grows between releases, so `tests/tool_names` and `docker/smoke.mjs` check `tools/list` by name, never by count. Every published `inputSchema` is a plain top-level `"type":"object"` with no top-level `oneOf`/`anyOf`/`allOf`/`not`: MCP requires the object type and the Anthropic API rejects top-level combinators, so one such schema makes clients like Claude Code drop the whole tool list (`tests/tool_schemas.rs` guards this). Domain failures are tool results with `isError:true` and `structuredContent.error`; protocol failures remain MCP JSONRPC errors.

Protocol 2026-07-28 is also supported (`server/discover` lists every version rmcp knows). It has no `initialize` and no session: each POST carries the `MCP-Protocol-Version: 2026-07-28` and `Mcp-Method` headers (plus `Mcp-Name` with the tool name or resource URI for `tools/call` and `resources/read`) and `params._meta` with `io.modelcontextprotocol/protocolVersion` and `io.modelcontextprotocol/clientCapabilities`. That revision requires SEP-2549 cache hints on list and read results, and clients such as Claude Code reject the whole catalog without them. `tools/list`, `resources/list`, `resources/templates/list` (always empty) and `resources/read` therefore return `ttlMs: 300000` and `cacheScope: "private"` (`src/cache_hints.rs`): the catalog is fixed for the process lifetime and the server answers one local user, so a client may reuse it for five minutes. The hints are sent on every protocol version, because older revisions accept extra result fields; rmcp still strips `resultType` for them (`tests/protocol_2026.rs`).

Every session pays for the whole `tools/list` before its first call. `tests/tools_budget.rs` is the one size budget for it (total and per tool, both printed with `--nocapture`) and also checks that `tools/list` publishes exactly the catalog (`opencanvas_server::catalog_tool_names()`). A change that adds or grows a tool records the bytes before and after; feature tests measure only what their feature adds.

Start with `create_project`, then `apply_batch`, and read `get_outline` before requesting a scoped `get_subtree` or `get_design_context`. Use `find_nodes` to locate a node by name, text, kind, ID or parent in one call; `get_outline {nodeId:<page>, depth:1}` or `find_nodes {parentId:<page>, kinds:["frame"]}` lists a page's root frames in one call. Text requires explicit font/style properties as described by storage validation; the bundled font ID is `font_bundled`. Mutations require `expectedRevision`, a stable `idempotencyKey`, and ordered operations. Retry exactly the original body after a lost response. A changed body with the same key receives `IDEMPOTENCY_KEY_REUSED`; a stale revision with a new key receives `REVISION_CONFLICT` and `currentRevision`, plus `changedSince` (see "Changes since a revision" below).

`render_frame` accepts either `{projectId,revision,frameId,viewport,...}` or `{projectId,jobId,waitMs?}` for polling. Its published `inputSchema` is the flat union of both shapes with only `projectId` required; the server still validates the strict two-mode form, so mixing fields of both modes, `waitMs` without `jobId`, neither mode or an unknown field returns `INVALID_REQUEST`. Render responses include a job ID and status.

Scene is opt-in (`src/render_response.rs`): the MCP response is the job record without `scene`, the computed node/line/font/asset tree that is tens to hundreds of KB for a real frame. It keeps `jobId`, `status`, `projectId`, `revision`, `frameId`, `sceneKey`, and once the job has left `pending`, `provenance`, `diagnostics`, `readiness`, `imageUrl`/`image` (PNG dimensions) or `error`. Its size grows with fonts, assets and diagnostics, not with node count (a 300-node frame: about 3.6 KB for the whole `tools/call` result, against 88 KB with the scene; `tests/render_response.rs`). Read bounds and text lines with `inspect_layout` for that `sceneKey`, which pages and selects fields. Alternatively, pass `includeScene: true` (boolean, accepted in both modes, default `false`) to receive the full record exactly as before. `includeScene` is the only field added to the published schema. The job table, `get_design_context` and `inspect_layout` are unaffected, and the HTTP `/api/v1/jobs/:id` route that the viewer reads still returns the full record.

Bounded wait on a poll: `waitMs` (integer 0–10,000, default 0) is accepted only with `jobId`; on the enqueue form, or out of range or not an integer, the call returns `INVALID_REQUEST`. Absent or 0 returns the job record immediately, byte for byte as before. Otherwise the call returns as soon as the job leaves `pending` (`ready` or `failed`), or the record as it is when `waitMs` elapses (still `pending`; poll again). The response has the same shape either way; there is no extra field telling a timed-out wait apart. The project check and `JOB_NOT_FOUND` happen before waiting, and a record evicted during the wait returns `JOB_NOT_FOUND`. Waiting parks on an in-process completion signal: it holds no storage handle, read transaction, blocking thread or job-table lock, so reads and commits proceed meanwhile. A server shutdown ends every wait at once with the current record. Use `waitMs` instead of tight polling, including on the job IDs that `get_design_context` returns; the context read itself still never waits. The HTTP `/api/v1/jobs/:id` route is unchanged and does not wait. Ready PNG metadata includes HTTP URI, MIME, pixel dimensions and actual Node/Chromium versions. No project filesystem paths are returned. `inspect_layout` reads bounds and line lists from a job/scene key; it does not recompute layout or provide a deeper rule explanation. Its paging and field selection are described in the next section.

## Inspecting a render job: `inspect_layout`

```json
{"projectId":"p_example","jobId":"j_…","sceneKey":"…","fields":["bounds"],"limit":100}
```

| Field | Meaning |
| --- | --- |
| `projectId`, `jobId`, `sceneKey` | Required; the job must belong to the project and carry that scene key, otherwise `INVALID_REQUEST`. |
| `nodeIds` | Optional, ≤200 IDs; only those scene nodes are returned (unknown IDs are ignored). |
| `fields` | 1–2 of `bounds`, `lines`; default both. Order and duplicates do not matter. Every node always has `id`. |
| `limit` | 1–200 nodes per page, default 200. |
| `budget` | 1,024–65,536 bytes of `structuredContent` per page. **Omitted means no byte cap**, which is the v1 behaviour. A single node larger than the budget returns `CONTEXT_ITEM_TOO_LARGE`. |
| `cursor` | `nextCursor` of the previous page; repeat every other argument unchanged. |

Response: `schemaVersion, projectId, revision, sceneKey, status, nodes[{id, bounds?, lines?}],
diagnostics, truncated`, plus `nextCursor` **only when `truncated` is true**. Nodes are in
scene order; following `nextCursor` returns every selected node exactly once.

- A call without `fields`, `limit`, `budget` and `cursor` returns the same bytes as before
  Task07B, with one fix: a scene with more than 200 selected nodes used to return
  `truncated:true` and drop the rest; it now also returns `nextCursor`.
- The cursor binds the tool, project, job, scene key, `nodeIds` (as given), the normalized
  `fields` set, `limit` and `budget`; any change returns `INVALID_CURSOR`. It does not bind a
  revision: the job's scene is immutable, so commits after the render do not invalidate it.
  If the job is evicted (32 retained records), the read fails with `JOB_NOT_FOUND` and
  must restart from a new render.
- `diagnostics` is repeated on every page. `fields:["bounds"]` drops the text line lists;
  on the stacks sample it is ~25% smaller per viewport (`TASK07-REPORT.md`, 07B).

## Exporting node images: `export_nodes`

`export_nodes {projectId, nodeIds (1–8), settings?, theme?, revision?, waitMs?}` returns image files of nodes (not a backup bundle; see `docs/BACKUP.md` for those). Each node is one export job in the requested queue tier: its root frame (the page's child that contains it) is computed once at authored width/height for `theme`, scale 1, then the node's subtree alone is rasterized once per setting. The crop is the node's layout bounds (children overflowing it are cut, like Figma `use_absolute_bounds`), the background is transparent, and the node is drawn without ancestors: its own opacity applies, ancestor clip and opacity do not.

- Settings come from the call's `settings` (applied to every node), else the node's `exportSettings` prop, else one PNG at 1x. A setting is `{format?:"png", scale?:0.01–4 | width?:1–16384 | height?:1–16384, suffix?}`, at most one of the three sizes. A target width/height derives the scale from the node bounds; outside 0.01–4 the item fails with `EXPORT_SCALE_RANGE`. The file name is the node `name` (else its ID, with `/ \ : * ? " < > |` and control characters replaced by `_`, cut to 100 bytes) + suffix + `.png`; the suffix defaults to `@<scale>x` when scale is not 1, else empty. An invalid `settings` argument fails the whole call with `VALIDATION_FAILED` and `field` such as `settings[1].scale`, by the same rules as the prop (`skill://opencanvas/props.md`).
- `exportSettings` is a node prop (every kind except page): an array of 0–8 settings. `[]` clears it (`update_node` merges keys, so a key cannot be removed). Two settings yielding the same file name, unknown keys (`constraint` → use scale/width/height, `type` → `format`) and formats other than `png` are rejected with `field` `exportSettings[i]…`.
- The call waits up to `waitMs` (default and max 10,000) for all jobs, then returns `{projectId, revision, exports:[{nodeId, jobId, status, files?, error?}]}`. A ready item lists `files[]` `{url, fileName, format, mimeType, scale, width, height, bytes}`; no scene and no base64. A pending item: repeat the identical call, which reuses the same `jobId` while the record is retained, or poll `render_frame {jobId, waitMs}`.
- Per-item errors leave the other nodes running: `NODE_NOT_FOUND`, `EXPORT_UNSUPPORTED` (page, or a node outside a root frame), `QUEUE_FULL` (retryable, `retryAfterMs`), and at run time `EMPTY_BOUNDS` (zero width or height), `PIXEL_BUDGET` (16M pixels per file, 48 MiB per job), `EXPORT_SCALE_RANGE`, `RESOURCE_NOT_READY`. Whole-call errors: `INVALID_REQUEST`, `VALIDATION_FAILED` (settings), `REVISION_UNAVAILABLE` (`revision` other than head), `PROJECT_NOT_FOUND`.
- `GET /api/v1/jobs/{job}/files/{index}` serves a file with its MIME type and `Content-Disposition: attachment; filename="<ascii>"; filename*=UTF-8''<encoded>`; `FILE_NOT_FOUND` (404) for a wrong index or a render job, `IMAGE_NOT_READY` (409) while pending or failed, `JOB_NOT_FOUND` (404) once evicted. `GET /api/v1/jobs/{job}` returns the export record (`kind:"export"`, `nodeId`, `files`, `provenance`, `readiness`; no `scene`, no `imageUrl`). Export records share the 32 retained job records with renders and prerenders, so download right away; a later identical call recreates evicted files. No filesystem path is returned or accepted.

## MCP skill resources and context diagnostics

The initialization capabilities advertise both tools and resources. `resources/list`
returns exactly three embedded Markdown documents (`text/markdown`), available without
project creation or filesystem access through `resources/read`:

- `skill://opencanvas/implement-frame.md`: context-to-code checklist, stack/sizing
  mapping, text baseline cautions, and when to state assumptions or ask the user.
- `skill://opencanvas/author-frame.md`: stack-first `apply_batch` authoring,
  fill/hug/min/max choices, and verification at two viewports.
- `skill://opencanvas/props.md`: every authored `props` key by node kind, with
  value ranges, required keys and common wrong names. It is rendered at startup
  (`src/props_doc.rs`) from the storage validator's property table
  (`crates/storage/src/props_catalog.rs`), so it cannot drift from validation.

### Property validation errors

The validator checks every authored (and style-resolved) key against that table
before checking values. An unknown key, or a key on the wrong kind (text metrics
outside `text`, `assetId`/`fit`/`crop` outside `image`, `layout` outside
page/frame/group), returns `VALIDATION_FAILED` with these added `error` fields:

```json
{"code":"VALIDATION_FAILED","message":"Unsupported authored property 'cornerRadius' on rectangle node; did you mean 'radius'?",
 "retryable":false,"opIndex":3,"field":"props.cornerRadius","nodeId":"card","nodeKind":"rectangle",
 "allowedKeys":["name","styleId","x","y","width","height","minWidth","maxWidth","minHeight","maxHeight","fill","opacity","clip","radius"],
 "suggestion":"radius"}
```

`suggestion` (a known alias such as `color`→`fill`, a case-insensitive match, or a
key within two edits) is omitted when nothing is close or the key is merely on the
wrong kind. Value errors keep their existing `field` (for example `opacity`,
`layout.wrap`, `fontId`) and gain only `nodeId`/`nodeKind`. All fields are
additive; `code`, `message`, `retryable`, `opIndex` and `currentRevision` keep
their meaning.

Unknown resource URIs return MCP resource-not-found (`-32002`) for protocol
2025-11-25; rmcp maps this to invalid params (`-32602`) for 2026-07-28 and newer.
Resources are static, built into the server binary; no subscriptions or templates are advertised.
Existing tools remain available to clients that only use tools.

`get_design_context.diagnostics` uses one code with two evidence cases:

| Code | Meaning and fields |
| --- | --- |
| `NO_REFLOW_INFORMATION` | Every node in the scoped frame uses `free`, explicitly or by default. `frameId` identifies the scope; `authoredSize` preserves the authored width/height, independent of the requested viewport; `message` explains that reflow information is absent and asks the agent to state assumptions or ask the user. |
| `NO_REFLOW_INFORMATION`, `reason: "UNCHANGED_BOUNDS"` | A successful multi-viewport comparison found identical descendant bounds in a scope containing stacks. The message reports this observation without claiming that the scope is all-free. `frameId` and `authoredSize` have the same meaning. |

The diagnostic examines the full frame scope at the read revision before depth,
property filters or pagination and repeats on continuation pages. A row/column
anywhere inside that scope suppresses the authored all-free case; stacks elsewhere in the project do not.
An empty diagnostics array does not certify responsive behavior or absence of
overflow. Legacy context reads without `viewports` do not compute bounds. For nested frames whose authored dimensions use fill/hug
or are omitted, `authoredSize` preserves those values (omitted dimensions are null).

## Multi-viewport context

`get_design_context` accepts optional `viewports: [{width,height}, ...]`: 1–4
positive viewports, each dimension at most 100,000 logical pixels, with a total
area at most 16 million pixels. Do not combine `viewport` and `viewports`.
The old single `viewport` remains an echo on an authored-only read; omitting
both retains the old behavior. No layout or breakpoint authoring schema changes.

```json
{"projectId":"p_example","frameId":"frame","viewports":[{"width":1040,"height":720},{"width":800,"height":720}],"properties":["name"]}
```

This read enqueues the existing immutable `SceneRequest` jobs at scale 1 and
default interaction state, or reads their process-local cache. It does not wait
for worker/Chromium completion. In addition to the existing context fields:

- `computed.status`: `pending`, `ready` or `failed` (failure takes precedence).
- `computed.coordinateSpace: "frame-local"`; `computed.viewports` preserves request
  order. Each item has `viewport`, `status`, `jobId`, `sceneKey` and optional `error`.
  Queue rejection has null job/scene IDs and retryable `QUEUE_FULL` with `retryAfterMs` and `hint`.
- Each returned node has `computedBounds: [Bounds|null, ...]` in that same order.
  `Bounds` is `{x,y,width,height}` in logical pixels. A pending or failed viewport
  contributes null, even when a non-strict scene exists with a strict PNG error.
- `reflow` is null until at least two viewports succeed with complete finite
  bounds. Then it is true if any descendant's x/y/width/height differs exactly
  from the first viewport; otherwise false. The resized root is excluded from
  comparison but its bounds are returned. `computed.comparisonScope` is
  `all-descendants-excluding-root`, and `comparedNodes` gives that scope's count.

Reflow uses the **whole frame scope**, before depth/property filters or pagination.
False emits `NO_REFLOW_INFORMATION`: the authored all-free diagnostic when
applicable, otherwise `reason: "UNCHANGED_BOUNDS"`. True suppresses that diagnostic
for the measured comparison, including unusual all-free frames with fill sizing.
Pending/failure never becomes false; an authored all-free diagnostic may still be
present independently of job readiness. No result certifies other viewport sizes,
absence of overflow or breakpoint behavior.

While pending, poll the returned job IDs using `render_frame` (with `waitMs`) or HTTP, then repeat
the identical context request **without cursor** until ready. Start pagination at
that point. Node bounds travel with each node as one indivisible page item, even
when property filters omit all authored geometry. All pages include `truncated`
and `nextCursor`; continuation binds revision, theme and the ordered viewports,
along with the other query parameters. Small byte budgets may return the usual
`CONTEXT_ITEM_TOO_LARGE` error for metadata alone or one complete node plus bounds;
no successful response silently drops bounds or viewport entries. Invalid count,
dimensions or total area returns the usual `INVALID_REQUEST` error envelope.

Failed jobs remain visible to context reads until eviction. Correct the resource
or worker problem, explicitly retry the failed viewport with `render_frame` at the
same revision/theme/scale 1, then repeat context; it picks the newest matching job.
Queue rejection can be retried directly by repeating context. If cache eviction
causes a continuation to become pending, restart from the first ready page.

Cold cost is intentionally visible: reuse currently performs both layout/text
measurement and strict PNG rendering per uncached viewport, on the one worker.
There is no second layout engine or bounds-only worker path. Cache hits reuse
the exact render/inspect scene key. The existing 45-second **active-job** deadline,
30-second worker operation/resource deadlines, one-active/eight-queued limit and
32 retained records still apply; queue waiting is outside the active deadline.
The context read itself never polls or holds a read transaction across that wait.
Measured local examples and exact commands are in `TASK03-REPORT.md`.

## Compact context: `compact:true`

`get_design_context` accepts an optional `compact` boolean (only this tool; the
other tree reads reject it as `INVALID_REQUEST`). Absent or `false` returns the
previous response byte for byte, cursors included (`false` is normalized to absent
before the scope hash). With `compact:true` every page adds two declarations:

- `resolvedPropsMode: "differing"`: each node's `resolvedProps` keeps only keys
  whose resolved value differs from the authored one, plus keys a style adds. The
  full resolved object is the authored `props` **without `styleId`**, overlaid with
  these keys; `{}` means everything resolves to its authored value. The `properties`
  filter applies first, as before.
- `resources: "referenced"`: `tokens`, `styles`, `fonts` and `assets` hold only
  items the frame's nodes reference, with the same item shape (`resolvedValue`
  included). References are `{tokenId}` and `{styleId}` values, top-level
  `styleId`, `fontId` and `assetId` in node props; then the fonts and tokens of
  those styles; then `{aliasId}` targets of those tokens in the default **and every
  theme branch**, to a fixed point. The set is collected over the whole frame scope,
  before property filters and pagination, so every page draws from the same set.

Nodes, their order, `computedBounds`, `diagnostics`, `computed` and `reflow` are
unchanged: diagnostics and reflow are still computed over the full scope. Paging
works as for the default read; a compact cursor is not valid for a non-compact read.
The key set mirrors storage validation (`validate/refs.rs`, `properties.rs`,
`node.rs`, `tokens.rs`), whose helpers are crate-private;
`crates/server/tests/compact_context.rs` pins it against storage's resolver: the
compact resources alone resolve the frame to identical values for every theme, and
dropping any one of them fails validation. Measured savings are in
`TASK07-REPORT.md` (07C).

## Finding nodes: `find_nodes`

Read-only search in one revision, for when the agent knows a name, visible copy,
kind or ID. Use `get_outline` when it needs structure. It does not filter the other
reads: outline, subtree and context keep their schemas and responses.

```json
{"projectId":"p_example","frameId":"welcome-dashboard-stacks","textContains":"Tổng quan","properties":["text","fill"]}
```

| Field | Meaning |
| --- | --- |
| `projectId` | Required. |
| `nodeId` / `frameId` | Optional, mutually exclusive scope root (default: whole project). `frameId` must be a frame. |
| `kinds` | 1–6 of `page`, `frame`, `group`, `rectangle`, `text`, `image`; OR within the list. |
| `nameContains` | 1–128 characters, against `props.name`; nodes without a string name never match. |
| `textContains` | 1–256 characters, against `props.text` of text nodes only. |
| `ids` | 1–200 exact node IDs, for batch reads by ID. |
| `parentId` | One node ID; matches its direct children only (not deeper descendants). `parentId:<pageId>` with `kinds:["frame"]` lists the page's root frames. An unknown ID matches nothing. |
| `properties` | ≤20 authored prop keys to attach to every match, same column filter as `get_subtree`; omitted means no `props`. |
| `limit`, `budget`, `cursor`, `revision` | 1–200 matches per page (default 50), 1,024–65,536 bytes (default 65,536), and the usual cursor/revision. No `depth`, `theme` or `viewport`. |

At least one of `kinds`, `nameContains`, `textContains`, `ids`, `parentId` is required, otherwise
`INVALID_REQUEST`; criteria are ANDed. Matching lowercases both sides with
`str::to_lowercase` and tests substring containment. There is **no accent folding and
no Unicode normalization** (NFC/NFD): "Tong quan" does not find "Tổng quan", and text
stored in a different normalization form than the query does not match.

Response:

```text
schemaVersion, projectId, revision, scope (root ID or null)
criteria      the requested criteria; ids are echoed as idCount
scannedNodes  nodes in scope, totalMatches  matches in scope (both before paging)
matches[]     {id, kind, name, parentId, depth, path:[{id,kind,name}], pathTruncated?,
               matchedOn:["name"|"text"|"kind"|"id"|"parent"], textPreview?, props?}
truncated, nextCursor, limits {nodes, bytes}
```

- Matches are in preorder, the outline's order. `depth` counts from the scope root
  (a project-scope page is 0). `path` runs from the scope root to the parent, keeping
  the 8 nearest ancestors; `pathTruncated: true` appears only when it was cut.
- `textPreview` (only with `textContains`) is at most 120 characters (not bytes) of
  `props.text`, positioned around the first match.
- Three outcomes are distinct: no match is `totalMatches: 0`, `matches: []`,
  `truncated: false` (not an error); a complete answer is `truncated: false` with
  `nextCursor: null`; more pages is `truncated: true` with `nextCursor`. A single match
  larger than `budget` returns `CONTEXT_ITEM_TOO_LARGE`.
- Cursors follow the same rule as the other reads: they bind project, revision, tool
  and every query field; a concurrent mutation or changed query returns
  `INVALID_CURSOR`, and an explicit unavailable revision `REVISION_UNAVAILABLE`.
- Search by position or bounds is not supported. It would need a computed scene.
  Narrow by name/text/kind, then read bounds with `get_design_context` `viewports`
  or `inspect_layout` `nodeIds`. The search is a linear scan of the snapshot, with no
  full-text index; measurements are in `TASK06-REPORT.md`.

## Changes since a revision: `get_changes`

Read-only: what the durable commit log recorded after a revision, so an agent can
catch up or rebase without re-reading the design. It does not change the revision
model, idempotency or the SSE cursor.

```json
{"projectId":"p_example","sinceRevision":12,"limit":100,"includeOperations":false,"budget":65536}
```

| Field | Meaning |
| --- | --- |
| `projectId`, `sinceRevision` | Required. `sinceRevision` is an integer ≥ 0; commits strictly after it are returned. |
| `limit` | 1–100 commits per page, default 100. |
| `includeOperations` | Default false. True adds each commit's logged `operations` (asset imports omit the internal `relativeKey`). |
| `budget` | 1,024–65,536 bytes of `structuredContent`, default 65,536. |

```text
schemaVersion, projectId, fromRevision (= sinceRevision), currentRevision
commits[]   {revision, changedIds, operations?}   in revision order, whole commits only
changed     {nodes, tokens, styles, assets, fonts, notes, removed}   union of this page's changedIds
truncated, nextRevision, limits {commits, bytes}
```

- `changed` groups each ID by what it is in the snapshot at `currentRevision`, in first-seen
  order; an ID that no longer exists is in `removed`. Storage reports the IDs an operation
  touched (an updated, created or deleted node and every deleted descendant, siblings whose
  order changed, upserted tokens/styles, imported assets/fonts), not their parents.
- The page and `currentRevision` come from one snapshot; commits that land after it are left
  for the next call. `nextRevision` is the last revision on the page (or `sinceRevision` when
  there are none): `truncated: false` means it equals `currentRevision` and you are caught up;
  `truncated: true` means call again with `sinceRevision: nextRevision`.
- Errors use existing codes: `sinceRevision` above the current revision, or a range whose log
  rows are missing (a bundle import clears the log), returns `RESET_REQUIRED` with
  `currentRevision`, meaning re-read the snapshot. A single commit larger than `budget` returns
  `CONTEXT_ITEM_TOO_LARGE` (omit `includeOperations` or raise `budget`). Out-of-range or unknown
  fields return `INVALID_REQUEST`; an unknown project `PROJECT_NOT_FOUND`.

**`REVISION_CONFLICT.changedSince`.** When `apply_batch` or `import_asset` loses, the error
object keeps `code`, `message`, `retryable`, HTTP status and `currentRevision` exactly as
before and gains:

```json
"changedSince":{"fromRevision":3,"toRevision":5,"changedIds":["card","title"],"truncated":false}
```

`fromRevision` is the request's `expectedRevision`, `toRevision` the `currentRevision`;
`changedIds` is the distinct IDs of those commits in commit order, at most 200, read from at most
100 commits. `truncated: true` means the list is incomplete (more IDs or commits, or missing log
rows); fall back to `get_changes` or a readback. It is omitted when `expectedRevision` is not
below `currentRevision`, or when the log cannot be read (the plain conflict is returned). Agent
rule: if `truncated` is false and no changed ID is a node you edit or one of its ancestors,
resend with `expectedRevision: toRevision` and a **new** `idempotencyKey` (the body changed);
otherwise re-read those IDs with `find_nodes {ids}`. Storage validates every batch again, so a
wrong rebase fails with `VALIDATION_FAILED` rather than writing bad data.

## Notes

Design intent the props cannot carry (behavior, constraints, effects the renderer lacks) is a
**note** anchored to a node. Notes are part of the revisioned document: they are written by
`apply_batch` operations, so idempotent replay, `expectedRevision`, `changedSince`,
`get_changes`, the SSE `commit` event and bundles cover them unchanged. Stored JSON equals the
JSON read back; defaults are omitted:

```json
{"id":"note-cta","nodeId":"cta","text":"CTA sticks to the bottom while scrolling.","author":"designer",
 "resolved":true,"replies":[{"text":"Done with position: sticky.","author":"coder"}]}
```

| Operation | Rule (failures are `VALIDATION_FAILED` with `field` and `opIndex`) |
| --- | --- |
| `create_note {id,nodeId,text,author?}` | `id` new and not a node ID (`id`); `nodeId` an existing page, frame or descendant (`nodeId`). |
| `reply_note {id,text,author?}` | Appends to `replies`; the note must exist (`id`); at most 100 replies (`replies`: create a new note). |
| `update_note {id,text?,resolved?,nodeId?}` | At least one field (`update_note`); `resolved:false` reopens; `nodeId` re-anchors. |
| `delete_note {id}` | The note must exist (`id`). |

- `text` is 1–2000 characters, `author` (self-declared, unauthenticated) 1–64; at most 5000
  notes per project. `create_node` rejects an ID held by a note.
- `delete_subtree` also deletes the notes anchored to every deleted node; their IDs are in
  `changedIds` and in `get_changes` `removed`. History stays readable with
  `get_changes includeOperations`.
- `changedIds` of a note operation is the note ID; `get_changes` groups live ones as `notes`.
- A commit whose operations are all note operations enqueues no prerender (its `commit` event
  is still sent). It still advances the revision, so older scene keys and cursors go stale.
- `get_design_context` takes `notes: "open"` (default), `"all"` (adds resolved notes, which
  carry `resolved:true`) or `"none"`. It lists, after `nodes`, the notes anchored to the
  frame's ancestors (outermost first), then to the frame and its descendants in preorder,
  each node's notes by ID. They share the 100-items-per-field page cap and the byte budget.
  With no note in scope the `notes` key is absent and the response is byte-identical to before.
- `GET /snapshot` carries every note in `notes`. Viewer display (P02b) and a project-wide
  `list_notes` (P02c) are later phases.

## Consistency and limits

- HTTP snapshot is authored data at one revision; its database schema version is currently 3. Worker input, computed scene and MCP read envelopes use scene/API schema 1. These version numbers describe different formats.
- Scene input is an owned immutable snapshot, with the scoped frame detached from its authored page for layout. Its source revision and scene key remain fixed if a later mutation commits. Old completed jobs remain readable by job ID until eviction; a new request for an authored revision no longer available returns `REVISION_UNAVAILABLE`.
- Scene cache identity includes project, revision, frame, viewport, theme, state, scale and a startup fingerprint of pipeline artifacts, actual Node version, installed Playwright and the browser executable identity. The cache is process-local. Restart the backend after changing Node, Chromium, layout binary or renderer/worker artifacts. PNG provenance records actual runtime versions, not guessed pins.
- Named design themes come from token declarations; unknown themes return `UNSUPPORTED_THEME`. Only the default interaction state is supported. Literal, typed token references, aliases and default/themes envelopes are resolved by storage. Authored node `props` remain intact in context; `resolvedProps`, token/style `resolvedValue` carry concrete-theme values separately.
- Tree/context pages contain at most 200 entries and 64 KiB. `depth` (0–8) is a filter counted from the scope root, which is depth 0: with no `nodeId`/`frameId` pages are 0 and their root frames 1, so `get_outline {depth:1}` lists every page and root frame, and `{nodeId:<page>, depth:1}` one page's root frames. Nodes deeper than `depth` are left out of every page (a continuation never descends further); a listed node whose children were left out carries `omittedChildren` (its direct child count), so read it again as the scope root to go deeper. Without `depth` the read covers the whole scope and `limits.depth` is `null`. `depth` does not narrow `get_design_context` diagnostics, reflow or `compact` resources, which cover the whole frame. `nextCursor` continues a stable preorder traversal of the listed nodes, then resource metadata, without dropping budget-trimmed entries. Each page includes scope, revision, limits and `truncated`; resource arrays are capped at 100 items per page. Resource-list tools cap the whole page at 100 items. One indivisible item exceeding the requested budget returns `CONTEXT_ITEM_TOO_LARGE`; use a property filter or larger budget.
- Cursors bind project, revision, tool, scope, filters, theme and page parameters. Repeat the same parameters with `cursor`. A concurrent mutation or changed query rejects the cursor with `INVALID_CURSOR`; restart from the outline. An explicit unavailable read revision returns `REVISION_UNAVAILABLE`.
- Mutation batches cap at 500 operations; assets cap at 10 MiB decoded and 13,981,016 base64 characters. The transport body cap is 15 MiB, including the nested MCP service. Asset import accepts bounded bytes and media metadata, never arbitrary paths or URLs.
- Rendering caps at 2,048 scoped nodes, 16 million scaled pixels, scale at most 4, one active job plus eight queued requested renders and eight queued commit prerenders, 32 retained job records and 48 MiB per worker response. When the record table is full, the oldest finished prerender that no requested render reused goes first, then the oldest finished record of any kind, so a run of commits does not evict a ready `render_frame` job. Backend jobs have a 45-second deadline; the shared worker also enforces its own 30-second operation and resource deadlines. `QUEUE_FULL` is retryable and carries `retryAfterMs` (a moving average of recent successful job durations times the jobs that would still run first: the queued requested renders plus the running one; 100–10,000) and a `hint`; HTTP routes also send `Retry-After` in whole seconds. Failed jobs and ready scenes with a strict-PNG error do not become permanent cache hits: retry creates a new job while old failure readback is retained.
- After a non-replayed commit the backend prerenders, at authored size, default theme and scale 1, the frames that contain the commit's changed nodes: for each changed node the outermost enclosing frame (the one the viewer lists), at most four, in storage order. When no changed ID resolves to a frame (token/style-only changes, deletions, page edits) it prerenders the first four frames in storage order, as before. Prerender jobs are never awaited and run on the same worker, but in a lower queue tier: requested renders (`render_frame`, `get_design_context` viewports, HTTP scene) always run first and never get `QUEUE_FULL` because of prerenders. A newer prerender of the same frame supersedes one still queued, and a full prerender tier drops its oldest; a dropped prerender never ran and nobody holds its job ID, so its record is removed at once and takes no retained slot. A running job is never interrupted. A requested render whose scene key matches a queued prerender reuses that job and moves it into the requested tier, so it can no longer be superseded.
- The backend keeps every project it has opened open for its lifetime (one SQLite writer each; after 128 pinned IDs and slugs the set is released and reopened on demand), so reads by ID or slug do not scan the data root. Storage path checks still run on every open. A project directory removed or replaced outside the backend while it runs is unsupported; reads then report `PROJECT_NOT_FOUND` for the old identity rather than another project's data, but a write sent by the old ID before any read can still land in the removed database.
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

## Mutation responses and image fit

`apply_batch` and `import_asset` answer with the same top-level commit fields:
`projectId`, `committedRevision`, `changedIds`, `renderStatus` and `replayed`.
`import_asset` adds `asset` (the stored metadata) and still repeats the commit
fields under `result`, the earlier nested shape, so existing clients keep working.
Read `committedRevision` at the top level for the next `expectedRevision`
(`tests/image_fit.rs`).

Image `fit` has one vocabulary, `IMAGE_FITS` in `crates/storage/src/image_fit.rs`:
`fit` stretches the image to its bounds (CSS `object-fit: fill`), `contain` (the
default) shows the whole image inside them, and `cover` fills them and crops.
Authored props, `resolvedProps` and scene nodes all carry the authored name;
resolution no longer renames `fit` to `fill`. Any other value returns
`VALIDATION_FAILED` with `field: "fit"`, and the message points CSS/Figma names at
their match (`fill`/`stretch` → `fit`, `none`/`scale-down` → `contain`). Stored
designs never held another value, because the validator always rejected it. The
layout engine accepts the same list (a storage test compares the two constants)
and still reads `fill`, the former scene name, as `fit`; the browser renderer
(`renderer/image-fit.ts`, `npm run test:renderer`) also draws `fill` like `fit`
for scenes cached before the change.

## Task04 row-wrap contract

`apply_batch` supports literal boolean `props.layout.wrap`, default false. True
requires a row container with definite positive width (numeric or finite fill
ancestor chain); every direct width-fill child requires explicit positive
`minWidth`, including resolved dimension refs. Storage checks all themes and the
final candidate, so token/theme/update/reparent changes cannot bypass the guard.
Non-boolean wrap, unknown layout fields and min>max on either axis reject with
`VALIDATION_FAILED` and field details; failed batches reserve no revision/key.

No new endpoint or schema version: subtree/context preserve authored wrap,
resolvedProps preserve its literal value, and `viewports` computedBounds/inspect
expose rows through existing flat bounds. `reflow:true` still compares all
descendants, not a claim that any dashboard fits mobile. Render and viewer use
the unchanged scene/line-list/shared-render contract. See `docs/RESOLUTION.md`
for layout semantics and `docs/TASK04-REPORT.md` for acceptance.

Upgrade backend and layout binary together, rebuild layout-step then restart the
backend for the startup pipeline fingerprint to change. Real3847 was not
restarted by Task04 and still serves its older loaded code. Props JSON is additive,
with no DB migration or snapshot/API/scene version bump. An old layout binary can
ignore unknown fields; mixed-version/downgrade and hot replacement are unsupported.
No percentage/breakpoint/column-wrap/align API or changes to existing dashboard data.
