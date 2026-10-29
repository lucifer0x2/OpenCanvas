# Integration contract v1

`PLAN.md` remains the source of requirements. This contract aligns independently owned modules; changes require updating consumers and evidence.

## HTTP viewer surface

Base `/api/v1`, localhost. Success returns the described JSON value; failures return `{ "error": { "code", "message", "retryable", "currentRevision"? } }` with an appropriate HTTP status. No filesystem paths.

- `GET /projects` → `{ projects: ProjectRef[], nextCursor?: string|null }`. ProjectRef: `{ projectId, slug, name, revision, schemaVersion? }`.
- `POST /projects` with `{slug,name}` → ProjectRef. The browser viewer does not call mutation routes.
- `GET /projects/:projectId/snapshot` → storage Snapshot with `schemaVersion`, `projectId`, `slug`, `name`, `revision`, `nodes`, `tokens`, `styles`, `assets`, `fonts`, `notes`, `retention`, `truncated:false`. Notes: `{id,nodeId,text,author?,resolved?,replies?}` (see `docs/SERVER.md` "Notes"). Nodes: `{id,parentId,sortKey,kind,props}`; `sortKey` is a number (integer sibling index), not a string. Authored data is not replaced by computed data.
- `GET /projects/:projectId/events?cursor=<projectId>:<revision>` → SSE. Durable `commit` event data `{projectId,revision,changedIds}`, SSE id `<projectId>:<revision>`. `reset_required` data `{projectId,currentRevision,reason}`. `scene_ready` includes `{projectId,revision,sceneKey,jobId}`. Heartbeats are comments. Last-Event-ID also accepted. Register before snapshot/read durable log to avoid a gap; replay after cursor.
  The same stream also carries two **non-durable workspace hints** without an SSE `id`, so they never move the commit cursor: `project_created` `{projectId,slug,name,revision}` after a successful `create_project` (MCP or `POST /projects`), and `project_activity` `{projectId,revision}` when **another** project commits. Neither reads the durable log; only a commit of the streamed project replays it. Hints are not replayed after a reconnect or `reset_required`: clients refetch `GET /projects`, which is the source of truth, on every (re)open.
- `GET /events` → SSE workspace stream for a client that has no project to follow yet (an empty workspace). It carries only `project_created` and `project_activity` with the data above, heartbeat comments every 10 s, and ends on server shutdown. No cursor, no replay, no query parameters (`INVALID_REQUEST` otherwise); a broadcast overflow sends `reset_required` `{reason:"buffer_overflow"}` and ends the stream. Same loopback Host/Origin guard as every route.
- `GET /projects/:projectId/scene?frameId=...&revision=...&width=...&height=...&theme=default&state=default` → RenderJob below. This is a read request for a computed representation and may enqueue bounded work. Scene revision must exactly match requested revision. Old unavailable snapshot returns `REVISION_UNAVAILABLE`.
- `GET /jobs/:jobId` → RenderJob.
- `GET /jobs/:jobId/image` → PNG when ready, otherwise structured readiness error.
- `GET /jobs/:jobId/files/:index` → one file of a ready export job (`export_nodes`) with its MIME type and an attachment `Content-Disposition`; `FILE_NOT_FOUND` 404 for a wrong index or a render job, `IMAGE_NOT_READY` 409 while pending/failed.
- `GET /projects/:projectId/assets/:assetId` → verified project asset bytes and correct MIME type.
- Bundled font endpoint `/api/v1/fonts/font_bundled` serves the exact licensed fixture font.

RenderJob: `{ jobId, status: "pending"|"ready"|"failed", projectId, revision, frameId, sceneKey?, scene?, provenance?, readiness?, diagnostics?, error?, imageUrl? }`. Ready `scene` conforms to `renderer/types.ts`; scene nodes may carry `stroke {color[4], width[4], align}` (sides [top, right, bottom, left], present only when a side is wider than 0; `scene-2`/`web-2`); its provenance source revision is authoritative. Failure never returns success for strict PNG. Scene can be present with missing-resource diagnostics for labeled non-strict viewer placeholders, while strict export fails.

ExportJob (same routes and job table): `{ jobId, kind: "export", nodeId, status, projectId, revision, frameId (the root frame), sceneKey, provenance?, readiness?, files?: [{ url, fileName, format, mimeType, scale, width, height, bytes }], error? }`; never `scene` or `imageUrl`. A failed export (for example `EMPTY_BOUNDS`, `PIXEL_BUDGET`, `EXPORT_SCALE_RANGE`) has `status: "failed"`. Details: `docs/SERVER.md`, "Exporting node images".

`sceneKey` equals `scene.provenance.sourceSceneKey`. Successful PNG metadata also
includes MIME, dimensions, scale and actual Node/Chromium versions. Cache keys
bind immutable input, revision, concrete design theme/state, scale and a startup
pipeline/runtime fingerprint. Restart after replacing runtime or pipeline files.
Failed jobs remain inspectable by ID; a new enqueue can retry the same request.
A queued commit prerender that a newer one replaces is dropped without a record;
retention evicts unrequested prerenders before requested jobs. `QUEUE_FULL`
carries `retryAfterMs` (estimated time for the jobs ahead) and `hint`; prerenders
never cause it for requested renders.
Old completed jobs never overwrite another job or the viewer's chosen revision.

## Browser integration

Frontend package in `web/`, built to `web/dist`. Its dev server proxies `/api` to backend, default backend `http://127.0.0.1:3847`. Production backend serves built viewer and SPA entry.

Viewer imports shared renderer source from `renderer/`, calls resource loading + `drawScene` on an offscreen canvas, and mounts it in Konva for pan/zoom/selection overlays. No frontend layout engine or text rewrap. Source manifest URLs are controlled HTTP URLs. Scene rendering uses strict=false for labeled resource placeholders; status exposes diagnostics.

Layers ordered by `sortKey` within parent. Page/frame selection derives from authored nodes; frame scene includes only descendants. Local selection/pan/zoom/viewport do not mutate design or revision.

## MCP/domain

MCP Streamable HTTP `/mcp`, versioned typed schemas and bounded results. Public storage/layout types remain authoritative for exact Rust signatures. Domain operations are ordered: `create_node`, `update_node`, `reparent_node`, `reorder_node`, `delete_subtree`, `upsert_token`, `upsert_style`. Resource imports use bounded bytes, never arbitrary filesystem paths. See worker reports for validation and limits.

Tool arguments are checked against the published bounded schema at runtime.
Tree/context/resource reads return one authored revision, `truncated` and
`nextCursor`. Cursors bind project, revision, tool and query scope/filters/bounds;
changing scope or advancing revision requires restarting pagination. A response
budget never silently discards a complete item. An item exceeding the byte
budget returns `CONTEXT_ITEM_TOO_LARGE`; callers can use a property filter.
Each page is capped at 200 tree entries and 64 KiB, with at most 100 items per
resource kind. `depth` (0–8) filters nodes by distance from the scope root (depth 0);
deeper nodes are never listed, and a node whose children were cut carries
`omittedChildren`. Without `depth` the whole scope is listed (`limits.depth: null`).
Resource-only continuation pages are valid.

Design context keeps authored `props` and adds `resolvedProps`, plus authored
token/style values and `resolvedValue` for the requested concrete design theme.
The node `props` field is never computed geometry. Theme aliases and fallback
rules are specified in `RESOLUTION.md`. Interaction state supports default, hover,
pressed, focus and disabled, with paint-only flat overrides after layout; see
[Interaction states](INTERACTION-STATES.md) for context, rendering and diagnostics.
Navigation links (`props.links`, `flowStart`) are authored data; `get_design_context.navigation` summarizes them.
The viewer currently requests the default design theme; its light UI theme is
independent. Snapshot database `schemaVersion:2` is separate from scene/API
representation schema 1.

### Multi-viewport context (additive)

Only `get_design_context` adds optional `viewports: [{width,height}, ...]`.
Accept 1–4 entries, positive dimensions ≤100,000 and summed area ≤16M logical
pixels. It is mutually exclusive with legacy `viewport`. Single `viewport` and
no-viewport calls preserve the authored-only response and viewport echo.

Multi-viewport context reuses immutable render jobs at scale 1/default state and
the requested concrete theme, without awaiting Chromium. The additive fields are:

```json
{
  "computed": {
    "status": "ready",
    "coordinateSpace": "frame-local",
    "comparisonScope": "all-descendants-excluding-root",
    "comparedNodes": 1,
    "viewports": [
      {"viewport":{"width":320,"height":200},"status":"ready","jobId":"job_example1","sceneKey":"scene1"},
      {"viewport":{"width":160,"height":200},"status":"ready","jobId":"job_example2","sceneKey":"scene2"}
    ]
  },
  "reflow": true,
  "nodes": [
    {"id":"child","computedBounds":[{"x":0,"y":0,"width":320,"height":28},{"x":0,"y":0,"width":160,"height":56}]}
  ],
  "truncated": false,
  "nextCursor": null
}
```

This example omits unchanged context and authored/resolved node fields.
`computedBounds` has one slot per requested viewport. Pending/failed slots are
null. Each viewport item may include a structured `error`; a queue rejection has
null job/scene IDs. Overall status is failed if any viewport fails, otherwise
pending if any is pending, otherwise ready. Strict PNG errors count as failed,
even if a non-strict scene exists. Missing/incomplete bounds cannot produce a
successful comparison. `reflow` remains null until ≥2 viewports succeed.

Reflow compares exact x/y/width/height across the entire descendant scope at one
revision, excluding the root resized by the viewport. Depth, properties, limit
and byte-budget filters never reduce this comparison. False returns the existing
`NO_REFLOW_INFORMATION` diagnostic; scopes with stacks use reason
`UNCHANGED_BOUNDS` and describe observed equality, not all-free authoring.
`authoredSize` always preserves authored dimensions. True suppresses that code
for the successful measured comparison. No breakpoint semantics are inferred.

Poll jobs then repeat the same read from page one until ready before consuming
continuations. Complete node bounds are indivisible pagination items; the normal
64 KiB/200-entry/depth-8 limits, explicit truncated/nextCursor and
`CONTEXT_ITEM_TOO_LARGE` errors apply, including to fixed metadata. Viewport count
and total pixel violations return `INVALID_REQUEST`, with no partial page.
Cursors bind ordered viewports and theme/query/revision. If jobs are evicted and
a continuation becomes pending, restart the ready read from page one.

Context retains cached job errors for readback; use explicit `render_frame`
retry with the same revision/viewport/theme to replace a failure, then repeat
context. Queue rejection can be retried directly. Existing scene cache,
render pixel/node limits, queue and active-job/worker deadlines apply. Cold
requests currently pay for PNG generation as well as layout/measurement.

Commit/idempotency retention is currently lifetime, with no pruning or GC.
SSE detects malformed/foreign/ahead cursors, replay gaps and broadcast overflow
and requests an authoritative snapshot reset. Backup/import/restore are Rust
storage APIs documented in `BACKUP.md`; no corresponding MCP tool is advertised.
