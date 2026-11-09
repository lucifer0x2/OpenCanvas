# Task22 — Design-session token efficiency (server side)

**Status:** ready to assign. Not started. User requested on 2026-11-09 all four
server-side items below.

## Why

Six AXIS Courses design sessions (lead, U09a, U09b, U10, U10-fix1, U-m1fix;
transcripts on the user's machine, 2026-11-09) used **288 M cache-read tokens**,
10.6 M cache-write and 1.1 M output. Cache reads were about 60 % of cost:
each model call re-read 150–420 K tokens of context (U10 peak 682 K, U09a peak
638 K). Tool payloads are small per call, but they stay in context and are paid
for on every later call. Measured contributors:

| Source | Measured | Root cause |
| --- | --- | --- |
| `import_asset` arguments | 1.18 MB over 71 calls (16.6 KB/call), plus ~0.5 MB of `cat font.woff2.b64` | Tool only accepts inline base64, so fonts/images pass through the model twice; U09a alone 785 KB |
| PNG reads after `render_frame` | 93 full-size images, 400–600 KB PNG each, same file re-read up to 6× | Only full-frame PNG at viewport × scale; no size cap or node crop in context |
| `skill://opencanvas/props.md` | ~16 KB per read, read 12× in U10 alone (each subagent) | One monolithic document for every kind |
| `find_nodes` | 5 KB/call average; worst 50 KB, of which ancestor `path` = 34 KB (63 %) | Each match repeats up to 8 ancestors with names |
| `get_subtree` | 3.8 KB/call average; worst 34 KB, `props` = 22 KB (60 %) | No property projection or depth limit; agents raise `budget` to 65536 |

Fixed cost (instructions 10 KB, `tools/list` 29.5 KB) is cached and is not the problem;
keep it within `tools_budget`.

## Scope

Workspace `/Users/luis/Workspace/OpenCanvas`, branch `feat/page-concurrency`.
Do not touch the untracked `docs/GRAPHITE-RENDERER-RESEARCH.md`. One fresh worker per item
(they touch disjoint modules); items are independent and may land in any order. No
deploy, no native-service restart, no DB manipulation, no push. Keep modules focused,
preferably below 250 lines; split rather than grow `schemas.rs`.

### 22a — `import_asset` from the project inbox

- Add `path` as an alternative to `base64`: exactly one of the two (strict `oneOf` in
  validation, flat union in the published schema, following `schemas/render.rs`).
- Resolve `path` exactly like `apply_batch_file`: inbox-relative or absolute inside
  `<data-root>/<slug>/inbox`, traversal and symlinks rejected. Reuse
  `crates/storage/src/batch_file/inbox.rs` instead of duplicating the descriptor walk.
  Make it shared if needed. Same 10 MiB decoded limit.
- `mediaType` stays required. Reject bytes whose signature contradicts it
  (PNG/JPEG/WebP/TTF/OTF/WOFF2).
- Idempotency and the stored asset hash use decoded bytes. The same key with the same bytes
  replays whether it arrived as `path` or `base64`. Changed bytes under the same key conflict
  as today.
- Must work on every storage backend (SQLite, PostgreSQL). Inbox stays on the filesystem.
- Update the tool description, `instructions.rs` and `skills/author-frame.md`: save assets
  into the inbox and pass `path`, and never print or paste base64 into the conversation.
  Keep `base64` for clients without filesystem access.

### 22b — Bounded, focused renders

- `render_frame` enqueue gains `maxEdge` (integer, e.g. 256–4096): the output's long edge
  is capped by downscaling after `scale`. The response reports the actual pixel size.
- `render_frame` enqueue gains `nodeId`: render the frame normally (ancestor background, clip
  and theme apply), then crop to that node's laid-out bounds, with optional `padding`.
  `export_nodes` stays the tool for isolated, transparent node exports. The descriptions must
  say which tool to use when.
- Both options are part of the immutable job key. Defaults keep today's output
  byte-identical.
- Guidance (render description and skills): verify edits with a node crop and `maxEdge`
  around 1024, take a full-frame render only for a final check, and do not re-read an
  unchanged image.

### 22c — Per-kind property docs

- Keep `skill://opencanvas/props.md` as a short index: kinds, key lists via `kind_line`,
  and links.
- Publish per-kind resources, for example `skill://opencanvas/props/text.md`: the keys
  table rows and notes relevant to that kind.
- Publish animations, instances and interaction states as their own resources.
- Generate everything from the same validator tables (`props_doc.rs` and its submodules),
  so the published keys still equal the accepted keys. Extend the existing
  `every_kind_line_and_key_is_published` test to cover the split.
- Point validation-error guidance and the skills at the per-kind resource. Target: each
  per-kind resource ≤ 4 KB. Record actual sizes.

### 22d — Leaner `find_nodes` / `get_subtree`

- `find_nodes`: stop repeating ancestor objects per match. Return per-match ancestor IDs and
  one top-level `ancestors` map (id → name, kind, parentId) shared by the page, or make the
  full path opt-in. Pick one, document it, and keep `pathTruncated` semantics.
- `get_subtree`: add `properties` (projection, same contract as `find_nodes.properties`) and
  `depth` (relative to `nodeId`). Pagination, `budget` and `truncated` work as before.
- This changes the response shape: update `skills/implement-frame.md`, instructions and all
  in-repo consumers and tests in the same change.
- Target: on the two worst measured shapes (76 text matches with
  `properties:[text, styleId]`; a 135-node subtree), at least 40 % fewer bytes. Show it with
  a fixture test that prints the byte counts before and after.

## Acceptance

- Focused tests per item: schema validation (accepted and rejected shapes), inbox path
  security (traversal, symlink, missing file, oversize), idempotent replay across
  `path`/`base64`, render job-key and crop/`maxEdge` pixel-size checks, props-doc
  completeness, and response-shape tests for reads.
- Run `cargo test -p opencanvas-server --test tools_budget -- --nocapture` before and after.
  Record bytes in the report, and stay within `TOTAL_MAX` (32 KiB) and `PER_TOOL_MAX`
  without raising them.
- Existing `structured_batch`, `scoped_concurrency`, `tool_schemas`, `skill_layer` and read
  tests pass. The pre-existing failures listed in `docs/NATIVE-COMBINED-CANDIDATE.md` remain
  out of scope.
- Default outputs of untouched modes stay byte-identical (render PNG without new options,
  `import_asset` with `base64`).
- Report `docs/TASK22-REPORT.md`: measured byte sizes before and after for each item,
  checks actually run, and remaining limitations. The root session reviews the diff before
  any commit. Deployment is a separate, explicit user decision (see `AGENTS.md`).

## Out of scope

- Client or orchestration changes in AXIS Courses: per-screen fresh sessions, the lead
  reading less, scripted batch generation. Track these separately.
- Changing the MCP `text` + `structuredContent` duplication. Investigate only, with
  evidence of how Claude Code and Codex feed each into the model, and note the result in
  the report.
