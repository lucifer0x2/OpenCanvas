# Integration contract v1

`PLAN.md` remains the source of requirements. This contract aligns independently owned modules; changes require updating consumers and evidence.

## HTTP viewer surface

Base `/api/v1`, localhost. Success returns the described JSON value; failures return `{ "error": { "code", "message", "retryable", "currentRevision"? } }` with an appropriate HTTP status. No filesystem paths.

- `GET /projects` → `{ projects: ProjectRef[], nextCursor?: string|null }`. ProjectRef: `{ projectId, slug, name, revision, schemaVersion? }`.
- `POST /projects` with `{slug,name}` → ProjectRef. The browser viewer does not call mutation routes.
- `GET /projects/:projectId/snapshot` → storage Snapshot with `schemaVersion`, `projectId`, `slug`, `name`, `revision`, `nodes`, `tokens`, `styles`, `assets`, `fonts`, `retention`, `truncated:false`. Nodes: `{id,parentId,sortKey,kind,props}`; `sortKey` is a number (integer sibling index), not a string. Authored data is not replaced by computed data.
- `GET /projects/:projectId/events?cursor=<projectId>:<revision>` → SSE. Durable `commit` event data `{projectId,revision,changedIds}`, SSE id `<projectId>:<revision>`. `reset_required` data `{projectId,currentRevision,reason}`. `scene_ready` includes `{projectId,revision,sceneKey,jobId}`. Heartbeats are comments. Last-Event-ID also accepted. Register before snapshot/read durable log to avoid a gap; replay after cursor.
- `GET /projects/:projectId/scene?frameId=...&revision=...&width=...&height=...&theme=default&state=default` → RenderJob below. This is a read request for a computed representation and may enqueue bounded work. Scene revision must exactly match requested revision. Old unavailable snapshot returns `REVISION_UNAVAILABLE`.
- `GET /jobs/:jobId` → RenderJob.
- `GET /jobs/:jobId/image` → PNG when ready, otherwise structured readiness error.
- `GET /projects/:projectId/assets/:assetId` → verified project asset bytes and correct MIME type.
- Bundled font endpoint `/api/v1/fonts/font_bundled` serves the exact licensed fixture font.

RenderJob: `{ jobId, status: "pending"|"ready"|"failed", projectId, revision, frameId, sceneKey?, scene?, provenance?, readiness?, diagnostics?, error?, imageUrl? }`. Ready `scene` conforms to `renderer/types.ts`; its provenance source revision is authoritative. Failure never returns success for strict PNG. Scene can be present with missing-resource diagnostics for labeled non-strict viewer placeholders, while strict export fails.

`sceneKey` equals `scene.provenance.sourceSceneKey`. Successful PNG metadata also
includes MIME, dimensions, scale and actual Node/Chromium versions. Cache keys
bind immutable input, revision, concrete design theme/state, scale and a startup
pipeline/runtime fingerprint. Restart after replacing runtime or pipeline files.
Failed jobs remain inspectable by ID; a new enqueue can retry the same request.
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
Each page is capped at 200 tree entries, depth 8 and 64 KiB, with at most 100
items per resource kind. Continuation may start at a deeper tree node; depth is
relative to that page's first entry. Resource-only continuation pages are valid.

Design context keeps authored `props` and adds `resolvedProps`, plus authored
token/style values and `resolvedValue` for the requested concrete design theme.
The node `props` field is never computed geometry. Theme aliases and fallback
rules are specified in `RESOLUTION.md`. Interaction state supports `default`.
The viewer currently requests the default design theme; its light UI theme is
independent. Snapshot database `schemaVersion:2` is separate from scene/API
representation schema 1.

Commit/idempotency retention is currently lifetime, with no pruning or GC.
SSE detects malformed/foreign/ahead cursors, replay gaps and broadcast overflow
and requests an authoritative snapshot reset. Backup/import/restore are Rust
storage APIs documented in `BACKUP.md`; no corresponding MCP tool is advertised.
