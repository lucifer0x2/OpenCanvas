# Task25 — incremental validation and page-scoped access

Status: slice 1 implemented and checked 2026-11-13 on
`feat/incremental-validation`, from bootstrap `7cc36e0` / Task24 baseline
`81f883cf0b8e7d864ce30585d8db14094bdb5425`. Slices 2–6 remain open.
No deployment or production-limit change. The slice commit is the commit adding
this report; later slices must record their own HEAD and evidence.

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
full-fallback flag). The storage seam currently consumes only the statistics;
slice 2 can carry this artifact into scoped/history work to reuse candidate
expansion. Full fallback currently returns no expansion; its expanded count is
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

| Real-source metric | Task24 PG 30× median | After slice 1 |
| --- | --- | --- |
| Strict small edit | 12,639 ms | not measured yet |
| Scoped small edit | 34,121.9 ms | not measured yet |
| One-frame context | about 30,400 ms | unchanged read path; not measured yet |

Run the unchanged baseline matrix after slices 1–3, retaining at least five
samples and full median/p95/RSS/phase/count comparisons:

```sh
TASK24_COPIES_LIST="1 15 30" bash scripts/task24-bench.sh
```

Remaining: scoped guards/history still repeat full dependency work (slice 2),
revision-matched derived ownership/cache (3), page closure reads (4), compression
and page fetching (5), production caps/bundle alignment (6). No full server suite,
new disposable PostgreSQL matrix or exact-source Task24 cells were run in this
slice. Existing ignored PostgreSQL tests remain unclaimed. No native runtime,
production PostgreSQL or preview mutation; no push/deploy.

## Modules and sizes

New validation modules: `validate/index.rs` 80 lines (borrowed graph indexes),
`validate/incremental.rs` 177 (classifier/closure/pipeline/stats). Context remains
in `scope.rs` 101; full/apply pipeline `candidate.rs` 231; instance checks 224;
storage batch seam 208. New tests are 215/67/143 lines; Task24 parity helper 93,
commit metrics 86, seed 174. All modified/new handwritten code files are below
250 lines; no size exception required. Public APIs stay limited to one borrowed
index and the candidate validation work/result types.
