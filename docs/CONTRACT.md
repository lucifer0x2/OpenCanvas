# Integration contract v1

`PLAN.md` remains the source of requirements. This contract aligns independently owned modules; changes require updating consumers and evidence.

## HTTP viewer surface

The default listener remains loopback-only. An optional separate public viewer
listener exposes the read/export surface with the same shared state and SSE,
while excluding MCP, POST project creation and POST font import for every Host.
It accepts only GET/HEAD and POST to the exact login and exports paths. Encoded/ambiguous paths
and other mutation methods are denied. `GET /api/v1/viewer` returns
`{readOnly:false}` locally and `{readOnly:true}` publicly so font UI can hide
imports while retaining downloads. The server enforces this independently of UI.
Public scene/job/export/font responses project generated resource URLs onto the
configured HTTPS origin; cached jobs, worker callbacks, local responses and
authored strings keep their original values. See [public sharing](SERVER.md#public-viewer-through-tailscale-funnel)
for configuration and the scope of shared access.

`GET /api/v1/viewer/auth` returns `{authenticated,required}`. Local access
and public access with the password disabled report `{authenticated:true,required:false}`.
Public `POST /api/v1/viewer/auth` takes `{password}` with the configured
Origin. Success issues the session cookie and reports
`{authenticated:true,required:true}`. An incorrect password returns 401 and
throttling returns 429. With password protection enabled, public data routes,
downloads, export, and event streams require a valid session cookie.
Public project reads are further restricted to projects with
`privacy_status: "public"`. Private projects are omitted before pagination.
Direct URLs, resources and jobs owned by private projects return 404.

Font inventory/import and exact-byte download routes are specified in
[Project fonts](FONTS.md). MCP `list_fonts` preserves pagination and adds
`mediaType`, `byteSize`, `source` and `downloadUrl`. Font `family` remains authored
metadata; shared rendering uses an internal hash/weight/style CSS family.

Base `/api/v1`, localhost. Success returns the described JSON value; failures return `{ "error": { "code", "message", "retryable", "currentRevision"? } }` with an appropriate HTTP status. No filesystem paths.

- `GET /projects` → `{ projects: ProjectRef[], nextCursor?: string|null }`. ProjectRef: `{ projectId, slug, name, revision, schemaVersion?, privacy_status: "public"|"private" }`.
- `GET /projects/:projectId` → ProjectRef. The URL accepts a project ID or slug.
- `POST /projects` with `{slug,name}` → ProjectRef with `privacy_status: "private"`. Local only.
- `PATCH /projects/:projectId` with `{privacy_status}` → updated ProjectRef. Local only. The authored revision stays the same. The equivalent MCP tool is `update_project` with `{projectId,privacy_status}`.
- `GET /projects/:projectId/snapshot` → HTTP snapshot with `schemaVersion`, `projectId`, `slug`, `name`, `revision`, `privacy_status`, `nodes`, `tokens`, `styles`, `assets`, `fonts`, `notes`, `retention`, `truncated:false`. Notes: `{id,nodeId,text,author?,resolved?,replies?}` (see `docs/SERVER.md` "Notes"). Nodes: `{id,parentId,sortKey,kind,props}`; `sortKey` is a number (integer sibling index), not a string. Authored data is not replaced by computed data.
  - `?nodeFormat=rows` opts into `nodeFormat:"rows-v1"` with `nodeColumns:["id","parentId","sortKey","kind","props","extra"]`. Each node becomes an array in that order. `parentId` remains nullable. The trailing `extra` object, when present, contains additional top-level node fields. All other snapshot fields use the default representation.
  - Both formats return a strong, format-specific `ETag` on GET and HEAD. Visibility changes the ETag even if the authored revision stays the same. Cache control is `private, no-cache` locally and `no-store` on the public listener. A matching `If-None-Match` returns a bodyless `304 Not Modified` after authoritative project lookup and access checks. IDs and slugs for the same project select the same representation. See [HTTP snapshot cache](MOKA-SNAPSHOT-CACHE.md#http-snapshot-response-cache).
- `GET /projects/:projectId/events?cursor=<projectId>:<revision>` → SSE. Durable `commit` event data `{projectId,revision,changedIds}`, SSE id `<projectId>:<revision>`. `reset_required` data `{projectId,currentRevision,reason}`. `scene_ready` includes `{projectId,revision,sceneKey,jobId}`. Heartbeats are comments. Last-Event-ID also accepted. Register before snapshot/read durable log to avoid a gap; replay after cursor.
  The same stream carries **non-durable workspace hints** without an SSE `id`: `project_created` `{projectId,slug,name,revision}` after a successful `create_project` (MCP or `POST /projects`), and `project_activity` `{projectId,revision}` when **another** project commits. Clients refetch `GET /projects` on reconnect or `reset_required`.
- `GET /events` → SSE workspace hints and heartbeat comments every 10 s. No cursor, replay or query parameters (`INVALID_REQUEST` otherwise). A broadcast overflow sends `reset_required` `{reason:"buffer_overflow"}` and ends the stream.
- Project and workspace SSE send `project_visibility_changed` with `{}` and
  no cursor ID when the visible list may change. The public listener suppresses
  private project hints and ends streams whose project becomes private.
- `GET /projects/:projectId/scene?frameId=...&revision=...&width=...&height=...&theme=default&state=default` → RenderJob below. This is a read request for a computed representation and may enqueue bounded work. Scene revision must exactly match requested revision. Old unavailable snapshot returns `REVISION_UNAVAILABLE`.
- `POST /projects/:projectId/exports` → `{projectId,revision,exports:[{nodeId,jobId,status,files?,error?}]}`. Body is the `export_nodes` MCP arguments without `projectId`: `nodeIds` (1–8), optional `settings` (1–8), `theme`, `revision`, `waitMs` (0–10000). The exact MCP runtime schema, cross-field validation, pixel/queue budgets and pipeline apply. `projectId` in the body and unknown fields are rejected. Same Host/Origin guard as other routes. This requests computed files and never changes the authored revision. Poll pending jobs with `GET /jobs/:jobId`, then download the returned file URL.
- `GET /jobs/:jobId` → RenderJob.
- `GET /jobs/:jobId/image` → PNG when ready, otherwise structured readiness error.
- `GET /jobs/:jobId/files/:index` → one file of a ready export job (`export_nodes`) with its MIME type and an attachment `Content-Disposition`; `FILE_NOT_FOUND` 404 for a wrong index or a render job, `IMAGE_NOT_READY` 409 while pending/failed.
- `GET /projects/:projectId/assets/:assetId` → verified project asset bytes and correct MIME type.
- Bundled font endpoint `/api/v1/fonts/font_bundled` serves the exact licensed fixture font.

RenderJob: `{ jobId, status: "pending"|"ready"|"failed", projectId, revision, frameId, sceneKey?, scene?, provenance?, readiness?, diagnostics?, error?, imageUrl? }`. Ready `scene` conforms to `renderer/types.ts`; scene nodes may carry `stroke {color[4], width[4], align}` (sides [top, right, bottom, left], present only when a side is wider than 0; `scene-2`/`web-2`); its provenance source revision is authoritative. Failure never returns success for strict PNG. Scene can be present with missing-resource diagnostics for labeled non-strict viewer placeholders, while strict export fails.

ExportJob (same routes and job table): `{ jobId, kind: "export", nodeId, status, projectId, revision, frameId (the root frame), sceneKey, provenance?, readiness?, files?: [{ url, fileName, format, mimeType, scale, width, height, bytes }], error? }`; never `scene` or `imageUrl`. A failed export (for example `EMPTY_BOUNDS`, `PIXEL_BUDGET`, `EXPORT_SCALE_RANGE`) has `status: "failed"`. Details: `docs/SERVER.md`, "Exporting node images".

`sceneKey` equals `scene.provenance.sourceSceneKey`. Successful PNG metadata also
includes MIME, dimensions, scale and runtime identity. Default Chromium jobs keep
their actual `browserVersion`/`runtimeVersion`. Opt-in native jobs instead carry
`engine: "native"`, `engineVersion: "opencanvas-render-0.1.0/tiny-skia-0.12.0"`
and `runtimeVersion: "rust-native"`, with no invented `browserVersion`.
Native and Chromium pipeline fingerprints/cache identities are separate. Cache keys
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

### Responsive layout

Child `width` and `height` accept literal decimal strings from `"0%"` to
`"100%"`. The corresponding parent axis must be numeric or follow a fill or
percentage chain to a numeric size. The selected frame uses the render viewport
as its root size, except that a `height:"hug"` root (row/column layout only)
takes its content height; the scene viewport reports it and children cannot
fill or take a percentage of it. Percentages use the parent's content box in a stack and its
padding box under free positioning. Pixel min/max bounds still apply.

Containers may set `layout.breakpoints` to up to eight `{maxWidth, layout}`
objects with unique, strictly increasing integer thresholds from 1 to 100000.
The first rule with `viewport.width <= maxWidth` supplies the complete layout.
Above all thresholds, the base layout applies. Every rule is validated in all
themes and follows the same wrap guards. Render, preview and `inspect_layout`
share the computed scene for the same revision, viewport, theme and state.

Viewer addresses use `/?project=<projectId>&page=<pageId>&frame=<frameId>[&node=<nodeId>]`.
All query values are percent-encoded stable IDs, including virtual IDs with `/`.
The three required scope keys open without a selected node; `frame=:all:` denotes
all frames on the page. The optional `node` selects an element and is omitted on
deselection. Partial addresses are completed after authoritative reads. The URL
owns selection; localStorage does not participate. Browser Back/Forward restores
scope without reload. Explicit navigation pushes history; defaults/fallback and
revision cleanup replace it, preserving unrelated query/hash and `history.state`.
Canonical copied links omit unrelated query/hash, viewport and cached job IDs.
All addresses load the latest revision. Removed or moved
nodes use the viewer's visible missing-selection notice and scope fallback. AI
clients URL-decode the query once, confirm `find_nodes {projectId,ids:[nodeId]}`,
then read `get_design_context {projectId,frameId}` and verify ancestry. Use
`get_subtree {projectId,nodeId}` for a smaller authored read. An AI
must report missing/moved IDs rather than silently substituting a different node.

## MCP/domain

MCP Streamable HTTP `/mcp`, versioned typed schemas and bounded results. Public storage/layout types remain authoritative for exact Rust signatures. Domain operations are ordered: `create_node`, `update_node`, `reparent_node`, `reorder_node`, `delete_subtree`, `upsert_token`, `upsert_style`. Resource imports use bounded bytes, never arbitrary filesystem paths. See worker reports for validation and limits.

`apply_batch` / `apply_batch_file` optionally accept `pageIds` (1–100 distinct
existing pages). In scoped mode `expectedRevision` is the revision of the actual
authored read: the backend derives page and transitive shared-resource guards
and permits unrelated newer commits. Without `pageIds`, strict global revision
matching is unchanged. Scoped success adds `editGuard {projectId,expectedRevision,
pageIds}`; `get_edit_context {projectId,pageIds}` returns guard metadata and up to
100 dependency keys from one snapshot, without design data or internal paths.
Never replace an older design read's revision with a newer metadata-only guard.
Project revision remains the identity for snapshots/SSE/jobs/cache. Unsupported
scoped operations fail explicitly; shared writes, notes, page/global navigation
structure and first-root creation use strict mode after rereading. Complete
coverage, persistence, conflict recovery and limits: [page concurrency](PAGE-CONCURRENCY.md).

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
independent. Snapshot database `schemaVersion:4` is separate from scene/API
representation schema 1.

Components are authored `frame` nodes with `props.component:{name}`. An authored
`instance` is a leaf with `componentId`, optional sizing/position/name/visibility,
and flat descendant-ID `overrides` (`text`, `fill`, `visible`, `styleId`, `assetId`,
`name` only). Storage expands these references before resolution/layout; the engine
and renderer receive ordinary frames and stable virtual children
`<instanceId>/<masterChildId>`. Virtual children are read-only. Snapshot and bundle
data always retain authored references, never the expanded tree.

`get_design_context` defaults to `instances:"reference"`: scoped instances remain
leaves, `components` contains the transitive referenced master definitions once on
the first page, and optional instance `computed.children` maps virtual IDs to bounds
and lines. `instances:"expand"` returns the virtual tree and its computed bounds.
The mode is part of cursor identity. Compact resources include dependencies of both
master definitions and placed overrides. Outline/subtree expose reference leaves
with `componentId` and expanded `childCount`; virtual IDs can be read directly.
`find_nodes` retains authored instance kinds and searches effective child text.
Cycles, nesting beyond 8 levels and an expanded project above 20,000 nodes fail
before commit. `COMPONENT_IN_USE` returns `count` and at most 20 `instanceIds`;
virtual mutations return `INSTANCE_CHILD_READONLY`. Component subtrees cannot use
links, flow starts or animations. Master updates invalidate scenes at the new revision.
Expansion retains master-root and descendant interaction states. The instance
override whitelist does not include states; opt-in `resolvedStates` also applies
to reference component definitions. See [Components and instances](COMPONENTS.md).

### Hover animation sources

An authored `props.animations` entry may carry a literal `triggerNodeId` on
`on:"hover"`. It identifies an authored group or frame ancestor in the
same root frame. The keyframe and timing belong to the animated node.
Without a source ID the animated node supplies its own hover region.
`get_design_context.animated` includes an explicit source ID alongside
`ref` and `on`. See [Declared animations](SERVER-OPERATIONS.md#declared-animations-propsanimations)
for summary budgets and complete discovery.

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
`CONTEXT_ITEM_TOO_LARGE` errors apply, including to fixed metadata, except that
measured overflow advisories (`TEXT_CONTENT_OUTSIDE_BOX` items and their per-job
`TEXT_OVERFLOW_SUMMARY`) never cause it on their own: they get at most a quarter
of the byte budget and are compacted in fixed steps (drop per-job provenance, drop
item evidence, drop items round-robin, one read-wide summary), identically on every
page of a read, with `compaction {step, droppedNodes, retrieve}` and updated
`reportedNodes`/`omittedNodes` on the summary. Items reference the job by
`sourceSceneKey`; the full provenance travels once per job on its summary. Viewport count
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
