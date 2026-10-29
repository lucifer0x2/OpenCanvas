# Server operations and consistency

Companion to [Local backend](SERVER.md). Interaction variants are specified in
[Interaction states](INTERACTION-STATES.md).

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
- Named design themes come from token declarations; unknown themes return `UNSUPPORTED_THEME`. Interaction states are default, hover, pressed, focus and disabled; see [state semantics](INTERACTION-STATES.md). Literal, typed token references, aliases and default/themes envelopes are resolved by storage. Authored node `props` remain intact in context; `resolvedProps`, token/style `resolvedValue` carry concrete-theme values separately.
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
No percentage/breakpoint/column-wrap or changes to existing dashboard data (stack align/justify came later with P08a).

## P08a stack alignment and per-side padding

`props.layout` of a row/column container also takes `align` (cross axis:
`start` default, `center`, `end`, `stretch`) and `justify` (main axis: `start`
default, `center`, `end`, `space-between`), mapped 1:1 to CSS `align-items` and
`justify-content`. `padding` is a number or a 1–4 item list with CSS shorthand
meaning; each item is 0..100000 or a dimension token. `stretch` sizes children
whose cross size is unset; numeric and `hug` sizes are kept (`hug` = CSS
`fit-content`), which also makes a hug column of equal-width items expressible.
With `space-between`, `gap` is the minimum spacing; alignment overflow is unsafe
like CSS. Free containers reject `align`/`justify`; free children still use x/y
from the parent's outer edge.

Errors stay `VALIDATION_FAILED` with a precise `field`: `layout.padding`,
`layout.align`, `layout.justify`, `layout.mode` (a missing mode explains that
`update_node` replaces the whole layout object) and `layout.<key>` for unknown
layout keys, with `node.suggestion` for CSS/Figma spellings (`alignItems` →
`align`, `flex-start` → `start`, `SPACE_BETWEEN` → `space-between`, `paddingLeft`
→ `padding`). Authored lists and token refs read back verbatim; `resolvedProps`
hold numbers. `tools/list`, scene schema and renderer are unchanged; existing
layouts serialize and lay out exactly as before. Upgrade backend and layout-step
together: an older layout binary cannot deserialize a padding list.

## Declared animations: `props.animations`

Layer nodes (not pages) may author `animations`: up to 8 `{name, on?, duration, delay?, easing?, iterations?, direction?, fillMode?, keyframes}` with 1..16 WAAPI-shaped keyframes of `opacity`, `translateX`, `translateY`, `scale`, `rotate` and `fill`. Times are milliseconds; `fillMode` defaults to `both`; missing 0/1 keyframes take the authored (resting) props. Errors are `VALIDATION_FAILED` with the full path (`props.animations[0].keyframes[1].offset`), `nodeId`, `nodeKind`, and `allowedKeys`/`suggestion` for unknown keys. Number tokens resolve in timing and numeric keyframe values, color tokens and fill styles in `fill`; `resolvedProps.animations` holds theme literals. `get_design_context` adds `animated: [{ref:"<nodeId>:<name>", on}]` over the whole scope (same on every page), absent when the frame has none. Static renders ignore animations (resting state). Full key table: `skill://opencanvas/props.md`. `tools/list` is unchanged (props schema is a free object).
