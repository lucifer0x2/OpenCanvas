# Task25 — incremental validation and page-scoped access

Status: slices 1–3 implemented with targeted checks 2026-11-13 on
`feat/incremental-validation`, from bootstrap `7cc36e0` / Task24 baseline
`81f883cf0b8e7d864ce30585d8db14094bdb5425`. Slices 4–6 remain open.
No deployment or production-limit change. The slice commit is the commit adding
this report; later slices must record their own HEAD and evidence.

Handoff requested by the user after source commit `a98012d`. The combined default
test command was stopped during compilation (exit 143), before any test result;
it has not passed or failed acceptance. The exact Task24 matrix has not started.
See [handoff checkpoint](checkpoints/2026-11-13-task25-handoff.md) for the next steps.

## Slice 1 behavior and invariant

`Candidate::validate_after(before, operations, changed)` requires a **fully valid
before snapshot**, and a candidate obtained by applying that exact batch, with
all mutation/cascade/pruning IDs included in `changed`. Stored revisions satisfy
this invariant through the existing validated write boundaries. External raw SQL
mutation is outside that contract; this slice adds no whole-snapshot digest or
per-revision proof cache. Bundle validation/import and untrusted pure read/resolve
paths retain `Candidate::validate()`, the full reference validator.

Affected IDs include changed nodes and descendant subtrees, former/destination
parents and their remaining subtrees for structural operations, and component
users reached through **both** before/after master ancestry. A componentId usage
index reaches instances of masters whose descendants changed, then transitive
nested instances and their virtual descendants. Changes to inherited parent
layout still validate descendants in their placed context and every theme.

Token/style/font/asset changes, scoped theme writes, page writes, explicit note
operations and cascaded note changes fall back to full validation. The operation
classifier is exhaustive. Changed node references are resolved against complete
resource maps. Notes, navigation, tokens/aliases, animation triggers, component
cycle/depth and expansion guards remain global. Expansion is still project-wide;
only affected authored and virtual properties are checked incrementally.

Validation now borrows `Resolver { candidate, nodes, theme }`: no per-theme
Candidate clones and no cloned resource maps for expanded validation. The full
path preserves the original phase/error order; instance validation keeps the
expander's authored-ID/depth-first order rather than sorting virtual IDs. Apply
checks and operation attribution are unchanged. Debug storage builds compare the
incremental and full final-candidate results, including the complete serialized
error (code, field, opIndex, message and node context), for every storage fixture
that reaches that boundary. Release commits execute only the selected path.

## API for the next slices

`validate::NodeIndex<'a>` borrows one immutable node set and exposes node, children,
component users and component ancestors. Rebuild it after mutation. The current
validation call constructs before/after indexes; revision sharing belongs to
slice 3, not this slice.

`ValidationWork` owns `affected_ids`, optional expanded node map and
`ValidationStats` (authored, affected, expanded, affected-virtual counts and
full-fallback flag). Slice 2 now carries this artifact into scoped work to reuse
candidate expansion. Full fallback returns no expansion; its expanded count is
zero/unknown, and its affected count is the whole authored candidate. Do not
interpret that zero as an empty scene.

## Correctness and actual checks

All commands used `-j2`, tests at most two threads. Logs are temporary, excluded
from version control. The initial storage run had two loopback-bind permission
failures; the same full command passed with loopback escalation.

| Check | Result |
| --- | --- |
| Default scene-input suite | 19 passed, 0 failed |
| Default storage suite, all final-candidate fixtures differential | 161 passed, 0 failed, 27 existing ignored PostgreSQL/disposable tests |
| Targeted instances/boundaries/schemes/wrap/scoped/link cascade | 20 passed, 1 ignored (included in full storage count) |
| New explicit differential fixtures | 7 passed (included in scene count) |
| Release 1/15/30 synthetic evidence/parity | 1 passed |
| Clippy scene-input/storage `--lib -- -D warnings` | passed |
| Task24 server benchmark target, feature build/check only | passed |
| fmt scene-input/storage/server packages; diff whitespace | passed |
| Global fmt | pre-existing differences in unchanged render/path_scan.rs, shadow.rs, stroke.rs; those files remain unchanged |
| Coordinator web validation | 117 unit + 10 HTTP; React shell + 22 browser checks + Layers smoke + font workflow passed on temporary fixtures |

The explicit fixtures independently apply both modes from full-valid before
snapshots. Coverage includes success and rejection across 1/15/30-page scales,
master descendants, nested instances, overrides, inherited layout, refs,
delete/reparent/reorder, old/new master footprints, theme/resource/page/note
fallback, late invalid operations, precedence, link pruning and note cascade.
Existing storage tests additionally cover atomic rollback, history, scoped
dependencies, expansion limits, themes and overrides through the differential
seam. Pure read/bundle corruption fixtures continue using the full validator.

The Task24 seeder now full-validates its exact scaled fixture **before timing**
and runs untimed full/incremental differential operation cases (text update,
invalid resource, late invalid op, deletion, reparent, themes, token/style,
override). This replaces its historical dependence on the first measured strict
commit. Existing metric names, sample counts and measurement order stay intact;
commit output adds a separate `validationCounts` object with explicit units.
The Task24 fixture helper compiles; real-source cells will run after slice 3.

```sh
cargo test --offline -p opencanvas-scene-input -j2 -- --test-threads=2
cargo test --offline -p opencanvas-storage -j2 -- --test-threads=2
cargo clippy --offline -p opencanvas-scene-input -p opencanvas-storage -j2 --lib -- -D warnings
cargo check --offline -p opencanvas-server --features large-project-bench --test large_project_benchmark -j2
cargo test --offline --release -p opencanvas-scene-input --features large-project-bench -j2 --test incremental_evidence -- --ignored --nocapture --test-threads=1
cargo fmt --package opencanvas-scene-input --package opencanvas-storage --package opencanvas-server --check
git diff --check
```

## Slice 1 synthetic validation evidence

Release-only fixture: one warmup, five measured samples per mode/scale. Pure
validation only, excludes candidate cloning/apply/storage/history/SQL/cache.
Shared resources/themes and two-level nested components/overrides are present;
most nodes are unrelated rectangles. Counts deliberately differ from the real
Task24 source (especially its much larger instance expansion). These results
are not Task24 strict/scoped commit acceptance and do not establish a production
node limit. Full reference here includes the borrowed-context improvement.

| Copies | Authored / expanded | Affected authored / virtual | Full median / p95 ms | Incremental median / p95 ms |
| --- | --- | --- | --- | --- |
| 1 | 6,712 / 6,715 | 1 / 0 | 27.848 / 31.652 | 7.425 / 9.166 |
| 15 | 100,610 / 100,641 | 1 / 0 | 412.724 / 476.565 | 126.294 / 136.163 |
| 30 | 201,215 / 201,276 | 1 / 0 | 1,024.152 / 1,395.071 | 296.699 / 461.720 |

At 30× the median global/index phases still include before index 37.378 ms,
after index 38.290 ms, notes/navigation 27.552 ms, animation triggers 4.299 ms,
and instance validation/expansion 189.793 ms (its nested expansion phase is
74.900 ms; do not add it again). Affected closure is 0.080 ms and the two authored
theme-property checks total 0.015 ms. Phase medians need not sum to total median.
Affected property work stays at one authored node regardless of unrelated pages;
the remaining project-wide phases are explicit future optimization work.

The same 1/15/30 large fixtures also passed differential valid text updates,
late parent-layout rejection, invalid override rejection and master-descendant
updates. Changed master descendants reached 3/31/61 virtual nodes respectively.

## Task24 comparison and remaining work

| Real-source metric | Task24 PG 30× median | After slices 1–2 |
| --- | --- | --- |
| Strict small edit | 12,639 ms | not measured yet |
| Scoped small edit | 34,121.9 ms | not measured yet |
| One-frame context | about 30,400 ms | unchanged read path; not measured yet |

Run the unchanged baseline matrix after slices 1–3, retaining at least five
samples and full median/p95/RSS/phase/count comparisons:

```sh
TASK24_COPIES_LIST="1 15 30" bash scripts/task24-bench.sh
```

Remaining: revision-matched derived ownership/cache (3), page closure reads (4),
compression and page fetching (5), production caps/bundle alignment (6). Slice 2
removes repeated per-page expansion, but builds revision-local indexes per
operation until slice 3. No full server suite or exact-source Task24 cells were
run in slices 1–2. Disposable PostgreSQL checks for slice 2 are recorded below.
No native runtime, production PostgreSQL or preview mutation; no push/deploy.

## Slice 1 modules and sizes

New validation modules: `validate/index.rs` 80 lines (borrowed graph indexes),
`validate/incremental.rs` 177 (classifier/closure/pipeline/stats). Context remains
in `scope.rs` 101; full/apply pipeline `candidate.rs` 231; instance checks 224;
storage batch seam 208. New tests are 215/67/143 lines; Task24 parity helper 93,
commit metrics 86, seed 174. All modified/new handwritten code files are below
250 lines; no size exception required. Public APIs stay limited to one borrowed
index and the candidate validation work/result types.

## Slice 2: indexed guards and sparse history

Based on slice 1 `6c4a156`; the slice 2 commit is the commit adding this section.
`PageIndex` derives authored node→page, page→nodes and ordered root lists with
one traversal. `BatchScope` shares before/after topology through `Arc`, computes
the effective footprint once and reuses it for guard coverage and durable
history. JSON byte comparison remains in place, including `-0.0`. Changed IDs
include apply cascades, deletion and incoming-link pruning; old/new page keys
are retained for moves. Resource import callers keep the conservative full-diff
`envelope(before, candidate, ...)` entry point. The persisted envelope, seal,
hash, coverage validation, public operation arrays and restore scrubbing stay
unchanged.

`DependencyIndex` owns expanded nodes, expanded page membership and forward
adjacency for page/token/style/font references. It scans authored and expanded
properties once, retaining references hidden by overrides. Closure visits only
guarded/transitively referenced component or navigation pages; it does not scan
all expanded nodes or walk every ancestor again per page. All token branches,
styles, selected font families and backing assets retain the old dependency
keys. Font selection uses one `FontReferences` context against the existing
expansion and every token theme; snapshot callers clone only resource maps,
candidate callers borrow them. No per-page Candidate clone or font re-expansion.

For a stale scoped batch with an incremental instance expansion, guards now
perform **one before expansion and zero extra candidate/font expansions**.
The candidate map is moved from `ValidationWork`. Full-validation fallback or
instance-free validation may return `expanded=None`; that case explicitly
expands the candidate once. At the current revision dependency intersection is
skipped after page IDs, operation restrictions, source/destination, effective
footprint coverage and page-structure checks. Revision/history-gap, size limits,
replay and strict/global behavior remain unchanged. Navigation compares indexed
roots on affected pages while preserving the global wildcard for page/flow
changes, root reorder/deletion and first-root creation; safe root append keeps
the old relative-order prefix rule.

## Slice 2 parity and actual checks

Four new storage tests compare indexed closure keys with the retained old
closure, sparse footprints with frozen pre-index history/navigation algorithms,
and complete sealed envelopes byte-for-byte. Cases cover aliases/themes, hidden
authored refs, nested masters/instances, navigation targets, signed zero/no-op,
old/new page reparenting, deleted IDs, pruned links, notes/cascades, page/root
append/delete/reorder and old/new font-family/backing-asset metadata. The
current-revision shortcut is compared with the stale path over **32**
operation/page-guard pairs, including full serialized error parity. Both absent
expansion cases are exercised. A separate scene test compares both borrowed
font contexts with the former full-Candidate/theme-loop helper, including
font-family sibling selection and placed style overrides.

Existing SQLite and disposable PostgreSQL contracts add ABA, replay/reconnect,
rollback, shared resources/master conflicts, new dependencies, font byte
replacement, legacy/corrupt/gapped history, restore and bundle scrubbing.
Initial storage lib checks hit two sandbox listener-bind failures; the exact
targeted command passed with temporary loopback listeners allowed. A filtered
probe ran zero integration cases; only the corrected unfiltered run is counted.

| Final check | Actual result |
| --- | --- |
| Targeted storage lib/scoped/bundle/notes/assets | 46 passed, 1 existing ignored PG test |
| Scene lib + incremental integration | 13 passed (includes new legacy-font parity test) |
| SQLite scoped MCP/file contract | 2 passed, 1 existing ignored PG test |
| Disposable PG storage scoped/domain/transactions/resources/fidelity | 11 passed |
| Disposable PG scoped MCP | 1 passed |
| Clippy scene-input/storage touched libs, warnings denied | passed |
| Package fmt + raw diff whitespace | passed |

All tests used `-j2`, at most two test threads; PostgreSQL used one test thread
and a fresh container on a random loopback port, cleaned up by the wrapper.
Generated logs remain in OS temp. Rerun commands:

```sh
cargo test --offline -p opencanvas-storage -j2 --lib --test scoped_concurrency --test scoped_navigation --test scoped_history --test scoped_bundle --test bundle_roundtrip --test bundle_database --test bundle_archive --test notes --test assets -- --test-threads=2
cargo test --offline -p opencanvas-scene-input -j2 --lib --test incremental -- --test-threads=2
cargo test --offline -p opencanvas-server -j2 --test scoped_concurrency -- --test-threads=2
bash scripts/task19-check.sh bash -c 'cargo test --offline -p opencanvas-storage -j2 --test scoped_concurrency --test postgres_domain --test postgres_transactions --test postgres_resources --test postgres_fidelity -- --ignored --test-threads=1 && cargo test --offline -p opencanvas-server -j2 --test scoped_concurrency -- --ignored --test-threads=1'
cargo clippy --offline -p opencanvas-scene-input -p opencanvas-storage -j2 --lib -- -D warnings
cargo fmt --package opencanvas-scene-input --package opencanvas-storage --check
git diff --check
```

## Slice 3 handoff and slice 2 module sizes

`batch::validated_candidate` returns `(Candidate, Vec<String>, ValidationWork)`.
`BatchScope` owns shared before/after `Arc<PageIndex>` plus the already-computed
footprint. `DependencyIndex::with_snapshot_tree(snapshot, Arc<PageIndex>)` reuses
topology; `from_candidate(candidate, expanded, Arc<PageIndex>)` consumes the
validation expansion. `closure(pages, &FontReferences)` is read-only. Adjacency
is forward page/resource→references, not inverse componentId→instance users;
the existing `NodeIndex` from slice 1 still has borrowed component-user indexing.
Internal module visibility can be widened when slice 3 consumes these types.

No revision cache is introduced here. Index construction and reference scanning
are still project-wide once per operation; before expansion and the remaining
global validation phases still scale with project size. Slice 3 must bind shared
indexes to project/generation/namespace/revision and budget any retained expanded
map, including snapshots too large for current JSON cache admission. Added
phase/count keys: `scoped.expand.before`, `scoped.expand.fallback`,
`scoped.expansion_reused`, `scoped.closure_nodes`, `scoped.closure_keys`,
`history.compared_ids`. No performance target or production node limit is
claimed before the post-slice-3 Task24 measurements.

New production modules: `scoped/index.rs` 100 lines, `dependency_index.rs` 188,
`font_references.rs` 97. Existing touched modules: scoped 193, history 193,
navigation 46, SQLite batch 211, PostgreSQL batch 193, scene refs 193. New
parity/reference test modules are 197/61/100/78 lines; the 131-line old closure
is now test-only. All touched handwritten code files remain below 250 lines;
no size exception. This was the slice 2 checkpoint; slice 3 follows below.

## Slice 3 — immutable revision indexes shared by commits and reads

Implemented on top of slice 2 `b1622cb` and coordinator log archive `1bd675a`.
The slice commit is the commit adding this section. No storage layout, migration,
production cap, HTTP representation, viewer or runtime deployment changed.

`validate::NodeTopology` owns authored children and componentId→instance IDs.
`NodeIndex::with_topology` borrows node payloads while sharing this adjacency;
`Candidate::validate_after_indexed` accepts matching before/after topologies.
The existing `validate_after`, owned snapshot and `edit_context` APIs remain.
The node lookup views are still assembled per validation call; this slice avoids
rebuilding the owned topology, page map and reference indexes for the old revision.

Storage owns `RevisionIndexes`: topology, node→page/page→members/root topology,
inverse authored reference usage (tokens/styles, nested overrides and the other
existing dependency kinds), and a lazy dependency graph. Sparse property updates
reuse the old topology and page-map Arcs. Structural/component identity edits
build the candidate topology once. Candidate reference usage is built once, then
the same candidate index Arc is used by validation, scoped coverage/history and
the post-commit cache. Scoped dependency construction consumes the inverse usage
index to derive authored page adjacency instead of rescanning authored props.

`StorageBackend::shared_snapshot` and `Storage::shared_snapshot` return one
`SharedSnapshot { snapshot: Arc<Snapshot>, indexes: Arc<RevisionIndexes> }`.
SQLite and PostgreSQL attach matching cached indexes. The MCP `get_edit_context`
consumer uses this pair on a blocking worker, reusing its lazy dependency graph.
Other full scene/read APIs retain their previous validation/expansion behavior;
page-scoped reads belong to slice 4. A custom backend's default implementation
builds fresh indexes per read and never caches an identity it cannot establish.

Identity and publication:

- The cache key is the complete `SnapshotCacheIdentity`: canonical project,
  schema, revision, incarnation and readiness. PostgreSQL incarnation includes
  the durable namespace and project generation from its authoritative SQL read
  or locked transaction. SQLite brackets its read with database-file stamps and
  includes current resource readiness without changing persisted resource rows.
- SQLite's pinned project additionally checks device/inode before using its
  writer. Replacement at the same ID/revision rejects the old direct handle;
  the adapter forgets it and reopens the new database. This prevents old writer
  rows from being labelled with the replacement file's identity.
- Failed/previewed candidates are never published under the next revision.
  Replays return before candidate construction. New indexes publish only after
  SQLite persistence/PostgreSQL transaction commit, with no fallible I/O in the
  fill. Snapshot and index pairing always uses the identity of that snapshot's
  read; no separately probed newer revision is attached to old data.
- Full immutable keys keep late old fills separate from newer revisions. Old
  readers retain their Arcs across commit, expiry and eviction. Resource imports
  advance revision normally and build the corresponding indexes lazily on the
  next consumer; resource/page/note validation keeps its full fallback. Restore,
  recreated files/projects, namespace rotation and restart cannot reuse a prior
  generation's indexes. Raw SQL corruption within an unchanged identity remains
  outside the validated-write invariant; no full-project digest was added.

### Cache and cold-work limits

The derived-index cache is **separate** from the typed/HTTP snapshot caches.
It uses Moka, a 1-hour insertion TTL and **256 MiB weighted capacity** per SQLite
data-root/PostgreSQL backend (clones share it). The existing snapshot 16 MiB
admission and 64 MiB payload capacity remain unchanged. Index keys and weights
never serialize/hash snapshot JSON. Independent backend construction starts an
independent empty metadata cache.

Each index weight accounts for topology/page/reference ID strings and container
allowances, plus a reserved **4 MiB** for lazy dependency metadata. The reservation
is charged before materialization; a dependency graph above 4 MiB is returned to
the current caller but not retained. Full expanded node payloads are **never**
retained in the dependency cache: construction scans authored/expanded references
and computes font-ID sets by page, then drops expansion and resolution context.
This is a conservative allocation proxy, not a hard RSS limit: Moka bookkeeping,
temporary builds, expanded candidates, SQL snapshots and reader-held Arcs remain
additional live allocations; eviction does not revoke those readers.

Strict and current-revision scoped commits do not materialize dependency graphs.
A cold stale guard or first `get_edit_context` still performs a project-wide
expansion and font resolution across all pages once for that revision. Warm
consumers traverse cached page adjacency and font-ID sets. This cold cost is
explicitly retained here; bounding it by requested closure requires the page
expansion/read work in slice 4. This slice does not claim the final Task24 latency
targets or default-build support for 200k nodes.

### Slice 3 actual checks

All mutation fixtures used temporary roots or fresh disposable PostgreSQL 16
containers with random loopback ports and automatic cleanup. No native service,
native database, preview or user data root was accessed.

| Targeted check | Result |
| --- | --- |
| Revision cache unit + actual SQLite reuse/identity tests | 7 passed |
| Metadata-only 200,020 authored admission probe, above 16 MiB payload | 1 passed |
| SQLite revision/scoped/history/bundle/navigation/backend integration | 12 passed, 3 PG cases ignored |
| Scoped closure/guard/history parity against the slice 2 references | 4 passed |
| Scene incremental/full differential fixtures | 7 passed |
| Fresh PG revision identity/reuse/scoped/cache/namespace/fidelity | 10 passed |
| SQLite HTTP/MCP/structured/custom-backend compatibility | 9 passed, 1 PG case ignored |
| Fresh PG actual MCP scoped/get_edit_context compatibility | 1 passed |
| Clippy scene-input/storage/server libraries, `-D warnings` | passed |
| Three-package fmt, raw diff whitespace, AGENTS budget | passed |

Pointer assertions prove repeated authoritative reads share one index Arc;
actual sparse commits reuse that read's topology/page Arcs; lazy dependency
builders are not called again on warm reads; reference/component edits advance
their usage indexes while old readers remain unchanged. Tests cover preview,
failure and replay publication, same ID/revision with another file/generation,
namespace/schema/readiness isolation, out-of-order fills, expiry/eviction/restart,
custom backends without identity and a 17 MiB single-node payload. The separate
200k metadata probe exercises admission without serializing that payload or
changing production caps; it is not a validation or commit benchmark.
Its single debug-build sample retained a 40,570,678-byte index weight (including
the 4,194,304-byte dependency reservation): cold build 831.344 ms, warm lookup
9.083 µs. These are metadata proxy/admission observations, not RSS measurements,
release medians or evidence for the final latency targets.

The initial file-replacement fixture put a displaced project inside the data
root under a different valid slug and correctly hit `PROJECT_INVALID`; moving
that fixture outside the root made the intended same-ID replacement check pass.

```sh
cargo test --offline -p opencanvas-storage -j2 --lib revision_indexes -- --test-threads=2
rtk bypass cargo test --offline -p opencanvas-storage -j2 --lib two_hundred_thousand_nodes -- --ignored --nocapture --test-threads=1
cargo test --offline -p opencanvas-storage -j2 --lib scoped:: -- --test-threads=2
cargo test --offline -p opencanvas-scene-input -j2 --test incremental -- --test-threads=2
cargo test --offline -p opencanvas-storage -j2 --test revision_indexes --test scoped_concurrency --test scoped_history --test scoped_bundle --test scoped_navigation --test backend_contract -- --test-threads=2
bash scripts/task19-check.sh cargo test --offline -p opencanvas-storage -j2 --test revision_indexes --test scoped_concurrency --test postgres_cache --test postgres_namespace --test postgres_fidelity -- --ignored --test-threads=1
cargo test --offline -p opencanvas-server -j2 --test snapshot_http --test scoped_concurrency --test structured_batch --test storage_backend -- --test-threads=2
bash scripts/task19-check.sh cargo test --offline -p opencanvas-server -j2 --test scoped_concurrency -- --ignored --test-threads=1
cargo clippy --offline -p opencanvas-scene-input -p opencanvas-storage -p opencanvas-server -j2 --lib -- -D warnings
cargo fmt --package opencanvas-scene-input --package opencanvas-storage --package opencanvas-server --check
rtk bypass git diff --check
```

New/reworked modules remain below 250 lines: revision indexes ~220,
dependency adjacency 222, SQLite batch 190 plus validation 76, PG batch 235,
PG adapter 237, page topology 129 and scene topology 64. The three unit-test
modules and integration fixture are each below 200 lines. No file-size exception.
Extra benchmark phase counts expose `indexes.topology_builds`,
`indexes.topology_reused`, `indexes.cache_hits` and `indexes.dependency_builds`.
The coordinator will run the exact Task24 1/15/30 matrix and default crate gates
after this commit. Slices 4–6 remain pending; stop before starting them.
