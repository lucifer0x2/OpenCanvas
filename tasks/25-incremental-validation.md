# Task25 — Incremental validation and page-scoped access, unchanged storage layout

**Status:** packaged 2026-11-13; product implementation not started. Requested by
the user after [Task24](../docs/TASK24-REPORT.md). Workspace:
`/Users/luis/Workspace/OpenCanvas`; branch `feat/incremental-validation`, from
Task24 baseline `81f883cf0b8e7d864ce30585d8db14094bdb5425`.

## Why and boundaries

Step 1 must make projects with 100–200k authored nodes practical without changing
SQLite/PostgreSQL storage layout. Task24 measured PostgreSQL 30× (201,330 authored,
599,400 expanded) strict commits at 12.6 s, scoped commits at 34.1 s and one-frame
context at 30.4 s. Whole-project validation, repeated dependency expansion and
whole-project reads dominate; increasing a constant alone does not meet the goal.

This task supersedes the implementation order in
[LARGE-PROJECT-PROPOSAL](../docs/LARGE-PROJECT-PROPOSAL.md): derive indexes in memory,
validate the affected set, then scope reads and fetching. No page-column migration,
new database engine, cross-project library, render-cap increase or runtime rollout.
Keep strict/scoped concurrency, atomic rollback, revision/history/idempotency,
resource integrity, privacy and existing HTTP/MCP contracts.

Read `AGENTS.md`, `PLAN.md`, Task24 task/report and the proposal before starting.
Inspect the existing modules and callers before choosing new files. Shared pure
logic belongs in `scene-input`; persistence/concurrency in `storage`; request and
response handling in `server`; fetching/UI state in `web`. Small explicit APIs,
one responsibility per module; ideal 100–200 lines, evaluate splitting above 250,
avoid handwritten files above 350. Report remaining exceptions with reasons.

## Safe environment

- Never write to native 3857/3858, PostgreSQL 2703, `~/.opencanvas` or preview3877.
  No deploy, restart, push, reset, stash or discarding other work. No AI attribution
  in commits or deliverables.
- Only the Task24 benchmark may GET native3857 to fetch its source snapshot/assets.
  All mutations, seeded fixtures and browser tests use temporary SQLite roots and
  a disposable PostgreSQL on a random loopback port. Never use native credentials.
- Keep private snapshots/assets, generated JSON/JSONL/logs/manifests/archives and
  benchmark output in OS temp or ignored paths. Version source, tests, conclusions
  and rerun commands only. Delete private response bodies as the wrapper does.
- Main session coordinates and verifies. Give each implementation slice to a new
  worker with `fork_turns="none"`; workers do not spawn workers. Do not reuse a
  completed worker. Each slice has tests, a report update and its own commit.

## Sequence and authorization

Implement slices **1 → 2 → 3 first**, separately tested and committed. Before
starting **4 or 5**, assess their actual file/API/UI scope and dependencies. If
either becomes large, stop after a clean checkpoint, report the proposed split
and wait for the user before beginning that larger work. Do not silently expand
the task. Slice 6 follows the correctness/performance evidence from prior slices.
Caps may remain benchmark-feature-only during 1–3; do not claim default-build
100–200k support before the production caps and bundle checks in slice 6 land.

### 1. Validate the affected set, preserve a full reference path

Scope: `crates/scene-input/src/validate/` and focused new graph/affected-set modules;
the `crates/storage/src/batch.rs` validation seam and tests as needed.

- Establish the invariant that the before snapshot is fully valid. Keep the old
  full validator as a test/reference path and as the conservative fallback.
- Build affected = changed IDs + affected descendant subtrees + old/new parents
  for reparenting + instances using a changed master subtree. Use a componentId
  usage index: changing a descendant of a master must reach its instances and
  dependent nested instances, including overrides and virtual descendants.
- Include token/style references and inherited/layout/parent relationships needed
  to validate affected nodes correctly. Keep checks that are inherently global
  global; never omit reference, cycle, navigation, animation or expansion guards.
- Token/style/font/theme resource writes, page structure changes and note writes
  use full validation initially. Unsupported/uncertain operation mixes also fall
  back to full validation; make the classifier explicit and test it.
- Remove per-theme whole-Candidate clones: pass the theme as a parameter through
  validation/resolution. Reuse expansion where semantics permit it.
- Preserve accept/reject, error code **and field**, including operation-index
  attribution and existing error precedence. Compare incremental against full
  reference for all existing validation tests and Task24 fixtures, not only valid
  text updates. Cover deletes, reparent, master-child edits, nested instances,
  overrides, themes, refs, late invalid operations and rollback.

Task24 seeds rows directly and historically relies on the first strict commit to
fully validate them. Once strict commits use the incremental path, explicitly
full-validate the fixture before timing. Raw benchmark seeding cannot establish
the valid-before invariant or serve alone as incremental correctness evidence.
Use already-valid before snapshots for differential operation tests.

AC: full-reference differential tests pass for both success and failure; the
default validation suites retain behavior; affected work for a small edit does
not scale with unrelated pages. Record changed-set/expansion counts and timings.

### 2. Derive scoped guards and history from indexes

Scope: `crates/storage/src/scoped*`, `batch.rs`, PostgreSQL batch/history seams,
shared indexes and focused concurrency/history tests.

- Compute node→page once; derive guarded-page dependency closures through indexes
  for masters/instances, token/style/font references and navigation targets.
- Reuse validation's expansion/index work rather than recomputing full current
  and candidate closures. Include both old and new footprints when dependencies
  move, disappear or are pruned.
- Build history envelopes from changed IDs plus required removals/dependencies,
  preserving the existing persisted envelope contract and conservative fallback.
- Keep existing scoped semantics: unrelated pages may commit from an older base;
  overlapping/dependent changes conflict; strict/global operations, guard coverage,
  history-gap/size limits and replay behavior stay correct on SQLite and PG.

AC: targeted scoped/dependency/history/idempotency/atomicity tests pass on both
backends, with negative cases for shared masters, resource edits and reparenting.
Report guard/envelope parity and the removal of repeated full expansion.

### 3. Share derived indexes alongside each revision's snapshot

Scope: snapshot/index ownership in scene-input/storage and PostgreSQL Moka/cache
integration; server read entry points only as needed to consume the shared value.

- Keep immutable derived indexes per revision alongside the snapshot; share them
  between commit and read callers. Do not rebuild indexes for every request or
  expose a large mutable cross-layer API.
- Tie indexes to the same project identity/generation/revision as their snapshot.
  Publish new snapshot+indexes atomically after durable commit. Failed commits,
  retries, restore, eviction and concurrent old/new readers must not mix revisions.
- Respect current owned snapshot/public APIs where required, using small internal
  shared accessors. No persistent storage-layout or migration change.

AC: cache/revision isolation and stale-read tests pass; strict/scoped commit and
read paths demonstrably reuse matching indexes. Commit 3 and checkpoint before
assessing slices 4–5.

### 4. Read only the requested page closure

Scope: shared page closure/expansion API and server MCP context, outline, subtree,
render and inspect read paths; find pagination and contract tests.

- `get_design_context`, page `get_outline`, `get_subtree`, `render_frame` and
  `inspect_layout` use the page/dependency closure, including external masters and
  all resources needed for resolution/rendering. Skip whole-project validation on
  reads only because committed snapshots have already passed the write invariant.
- `find_nodes` iterates pages and stops at its existing limit/cursor boundary;
  retain order, stable revision/cursor checks, authored/virtual matching and all
  existing filtering behavior.
- Preserve existing responses byte-for-byte for the same fixture/revision/input,
  including ordering, provenance, errors and diagnostics. Add comparisons against
  the full reference read path, including cross-page masters/navigation and themes.

AC: context/outline/subtree/find/render/inspect parity and relevant server/scene
tests pass, with one-page read cost bounded by its closure rather than project size.

### 5. Compress snapshots and fetch/cache pages in the viewer

Scope: HTTP page/manifest and snapshot routes, compression/cache/ETag handling,
SSE commit payloads, React/SWR hooks and relevant viewer tests.

- Negotiate gzip/br for full snapshots; preserve default object and rows JSON
  semantics. Decompressed bytes match the existing representation; vary caches
  and ETags correctly by representation/encoding and honor conditional GET.
- Add page GET with its dependency closure and ETag; expose page/manifest metadata
  as needed. Keep full `/snapshot` for compatible clients and export/backup.
- Add `changedPageIds` to SSE commits, including pages affected through shared
  masters/resources; keep existing fields and replay/reset semantics.
- SWR fetches the current page, with per-page cache entries. Preserve URL selection,
  Back/Forward, project switching, current-page refresh, frame/node visibility,
  stale-response guards, realtime updates, font workflow and All frames behavior.
  Do not refetch every page after an unrelated-page commit.

AC: HTTP encoding/ETag/closure tests plus relevant web unit/HTTP and React browser
tests pass on temporary backends. Existing HTTP/MCP responses remain compatible;
document additive page/SSE contracts and cache invalidation.

### 6. Move production limits together and document supported scale

Scope: scene-input node/instance/note limits, storage bundle validation/preflight,
boundary tests and public contract/storage/README documentation.

- Enforce expanded ≤20,000 **per page**. Bound authored nodes at approximately
  1,000,000/project only with measured memory/latency evidence; record the chosen
  exact limit and justification, including dependency/master accounting.
- Scale notes deliberately alongside project/page limits; cover exact boundaries
  and rollback. Do not leave the old 5,000-note cap blocking otherwise valid pages.
- Align bundle import/export/preflight with the new limits. Fix the tokens≤4096
  mismatch where bundle validation currently compares token count with MAX_NODES;
  keep token/style limits independent from authored/expanded node limits.
- Keep the 2,048/frame renderer and pixel budgets unchanged. Document operational
  limits, full-read/export costs, no storage migration and remaining bottlenecks in
  `docs/CONTRACT.md`, relevant storage docs and `README.md`.

AC: default-build cap/bundle/note tests verify limits across SQLite/PG, no benchmark
feature is needed for the supported production scale, and backup/restore remains
self-contained and compatible within the documented bounds.

## Final verification and deliverables

Run targeted tests at each slice with limited parallelism. At completion run the
default-feature scene-input, storage and server suites, clippy for touched crates,
fmt/diff checks, and relevant web unit/HTTP tests, typecheck and React/browser tests
if web changes land. Record actual commands, failures and known pre-existing issues;
do not replace a failing suite with an unqualified pass claim.

```sh
TASK24_COPIES_LIST="1 15 30" bash scripts/task24-bench.sh
cargo test --offline -p opencanvas-scene-input -j2 -- --test-threads=2
cargo test --offline -p opencanvas-storage -j2 -- --test-threads=2
cargo test --offline -p opencanvas-server -j2 -- --test-threads=2
```

Benchmark AC: PostgreSQL 30× strict **and** scoped small-edit median approximately
≤1.5 s; one-frame context ≤1 s; no 1× regression under comparable conditions.
Retain ≥5 samples and baseline phase breakdowns; compare medians/p95/RSS, authored
and expanded counts, bytes, cache behavior and affected-set sizes. Separate noise
from a demonstrated regression. If a target is missed, report the remaining phase
cost instead of raising timeouts or claiming acceptance. Measurements at 200k do
not alone justify a 1M authored production limit.

Write `docs/TASK25-REPORT.md` with before/after Task24 tables, commands and HEADs,
correctness/parity evidence, changed modules and sizes, limits and anything not
verified. Append concise assignment/result/blocker/checkpoint entries to
`docs/agents/LOG.md`. Each slice ends with a clean commit, hash/file list, checks
and remaining scope. No push or deploy. Bootstrap ends with this handoff only;
slice 1 must be assigned independently.
