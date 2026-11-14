# Large projects: page-scoped data and shared libraries — proposal

**Status:** proposal, 2026-11-12. No code changed. Branch `feat/project-node-limit`.
Goal from the user: one project must hold hundreds of thousands of nodes. Today a
project stops at 20,000 nodes.

## 1. Where the limit lives today

| Cap | Source | Counts |
| --- | --- | --- |
| 20,000 authored / project | `crates/scene-input/src/validate/candidate.rs:47` | Stored node rows |
| 20,000 expanded / project (`INSTANCE_LIMIT`) | `crates/scene-input/src/instances.rs:57` | Authored nodes + every virtual instance child, whole project |
| 2,048 / frame, 16 Mpx | `crates/layout/src/lib.rs:33` | One render input |
| Bundle preflight | `crates/storage/src/bundle_history.rs:8` | nodes/styles ≤ 20,000, tokens ≤ 4,096 (but `bundle_validation.rs:58` compares tokens with `MAX_NODES`) |

The constant is not the real problem. Every layer works on the **whole project
snapshot**:

- **Commit:** `validated_candidate` (`crates/storage/src/batch.rs:157`) builds a
  `Candidate` from the full snapshot, applies operations and calls `validate()`.
  `validate()` clones the candidate per theme, and `validate_instances` expands
  every instance in the project. With `pageIds` (scoped concurrency) only the
  *guard* is page-scoped; load and validation are still project-wide.
- **Storage:** SQLite `nodes` and PostgreSQL `entities` have no page column; a
  read loads all rows. The Moka cache holds one full typed snapshot per project.
- **Reads:** `GET /projects/{p}/snapshot` returns every node. MCP `find_nodes`,
  `get_outline`/tree, viewports, export and scoped dependency checks call
  `instances::expand` on the whole project.
- **Viewer:** the React app fetches the full snapshot even though it shows one
  page; SSE `commit` events carry `changedIds` but the client refetches everything.

Raising the constant to 500k keeps all of that O(project) on every commit, every
read and every browser load. Cost grows with themes × expanded nodes.

## 2. Assessment of the two directions

### Direction B — page-scoped reads and writes (recommended first)

Fits how agents work: one page per edit, project-wide search only. It is the
change that actually removes O(project) work, and it builds on scoped concurrency,
which already derives each page's dependency closure (masters, tokens, styles,
fonts, navigation targets — `crates/storage/src/scoped/dependencies.rs`).

Target limits after B:

- **Per page:** expanded ≤ 20,000 (keeps validation/render bounded, same as today).
- **Per project:** authored ≤ 1,000,000 (bounded by storage/backup, not memory).
  Page count capped (e.g. 2,000).
- Per frame render cap 2,048 is unchanged and independent.

### Direction A — shared library project

Useful, but **does not by itself raise the per-project ceiling**: instances in a
consumer still expand their virtual children, so expanded counts stay the same.
It mainly gives reuse across projects (design system, fonts, assets) and smaller
consumer projects. Costs:

- Render, ETag, cache identity and render-job provenance must include the
  library's revision; otherwise a library edit silently changes consumers.
- Concurrency guards, backup/restore bundles (must stay self-contained), public
  viewer privacy (a public consumer must not expose a private library) and
  deletion of a used component all become cross-project.

Recommendation: do A after B, with **pinned versions** — the consumer records
`libraryProjectId@revision`; an explicit "update library" commit moves the pin.
Under B, a library master only counts toward the pages that use it.

## 3. Plan for Direction B

### Phase 0 — measure (no product change)

Synthetic multi-page project generator (e.g. 50 pages × 4k authored, instances
reaching 200k expanded), temporary data root, SQLite and PostgreSQL. Measure
cold/warm snapshot load, commit validate, scoped commit, `find_nodes`, snapshot
JSON size and browser parse/render. Confirms where time goes before changing
storage. Constants raised only inside the benchmark build.

### Phase 1 — page-scoped reads (no storage format change)

- `GET /projects/{p}/manifest`: revision, pages (id, name, order, root frame ids),
  tokens, styles, fonts, assets, component index (componentId → page), notes count.
- `GET /projects/{p}/pages/{page}`: the page subtree plus its dependency closure
  (master subtrees from other pages, navigation target stubs). ETag = the last
  revision that touched that closure (derivable from commit footprints already
  recorded by scoped history).
- SSE `commit` adds `changedPageIds`; the viewer refetches only affected pages.
- Viewer: SWR key per page; manifest drives the page list; URL navigation already
  selects one page.
- MCP `get_design_context`, `get_subtree`, `get_outline(page)`, `render_frame`,
  `inspect_layout` expand only the page closure. `find_nodes` stays project-wide
  but iterates page by page with its existing limit/cursor instead of expanding
  everything at once.
- Full `/snapshot` stays for export/backup and old clients, with a size cap.

### Phase 2 — page-partitioned storage and validation

- Add a denormalized root `page_id` to node rows (SQLite `nodes`, PostgreSQL
  `entities` kind `node`) with an index; migration backfills it.
- Side indexes maintained on commit: instance usage (page → componentIds) and
  navigation links (page → target pages).
- Scoped commit loads only guarded pages + closure, validates that partial
  candidate, and checks project-wide invariants through the database: ID
  uniqueness via primary key, references to other pages via the indexes,
  deleting a master via the usage index.
- Moka caches per-page entries plus the manifest instead of one project blob.
- Strict (unscoped) commits remain for page create/delete/reorder and
  token/style/resource writes; they touch the manifest-level data, not all nodes.
- Caps move: per-page expanded 20k, per-project authored 1M.
- Backup/export streams page by page.

### Phase 3 — shared library (Direction A)

As in section 2, after Phase 2 is stable.

## 4. Risks and open questions

- Token/style edits affect every page; they stay strict and must invalidate all
  page ETags (manifest revision in the key).
- A master page used by many pages becomes a hot dependency; consider a
  convention of dedicated component pages.
- Current scoped history caps (1,000 intervening commits / 8 MiB) may need a
  per-page index once projects are large and busy.
- Old MCP clients that rely on full snapshots keep working only below the size cap.
- Each phase needs its own task file in `tasks/` with AC before assignment.
