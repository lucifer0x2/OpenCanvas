# Task25 — incremental validation and page-scoped access

Status: slices 1–3 implemented and coordinator-verified 2026-11-13 on
`feat/incremental-validation`, from bootstrap `7cc36e0` / Task24 baseline
`81f883cf0b8e7d864ce30585d8db14094bdb5425`. Slices 4–6 remain open.
Default gates and the exact Task24 matrix are recorded below; latency targets
remain unmet. No deployment or production-limit change. Later slices must record
their own HEAD and evidence.

Resumed from docs checkpoint `b9652e947d7bd03455068f0bcf2e2e4292fce8f7`, product
source `a98012d33b14738c115414dd029df9240bc017bc`. The earlier interrupted run
(exit 143) is superseded by the coordinator verification below. Slices 4–6
and production caps remain pending; see the original
[handoff checkpoint](checkpoints/2026-11-13-task25-handoff.md).

## Coordinator default gates after slice 3 — 2026-11-13

The exact combined command completed all targets with `--no-fail-fast`: scene
20 passed, storage 173 passed / 30 ignored, server 269 passed / 6 ignored /
1 failed. Exit 101 was solely `native_layout_parity` preflight rejecting a stale
local `renderer/dist/index.js`; it had not compared scenes. Rebuilding that ignored
artifact exposed a second fixture precondition: missing ignored Inter font bytes.
The font and license were restored from the official commit pinned in
`spikes/native-render-parity/README.md`; SHA-256 matched
`4989b125924991b90d05b2d16e0e388c48f7d5bb8b30539bbf9c755278d0ccaf`.
The same parity target then passed 1/1 in 33.15 s. No product source changed.

Thus all 463 enabled tests passed across the combined run and focused rerun;
36 tests remained ignored by default. This is **not** a claim that the original
combined command exited successfully, or that ignored PostgreSQL tests ran in
this gate. Their earlier disposable-PG evidence remains stage-specific below.
Clippy covered all three libraries and the server binary with warnings denied.
Package fmt and raw diff whitespace passed. Global fmt still exits 1 only for
unchanged `render/src/path_scan.rs`, `shadow.rs`, `stroke.rs`; they were preserved.

```sh
cargo test --offline --no-fail-fast -p opencanvas-scene-input -p opencanvas-storage -p opencanvas-server -j2 -- --test-threads=2
npm run build:renderer
mkdir -p spikes/native-render-parity/out/fonts
curl -fL https://raw.githubusercontent.com/rsms/inter/e3a3d4c57d5ecc01453a575621882a384c1995a3/docs/font-files/InterVariable.ttf -o spikes/native-render-parity/out/fonts/inter.ttf
curl -fL https://raw.githubusercontent.com/rsms/inter/e3a3d4c57d5ecc01453a575621882a384c1995a3/LICENSE.txt -o spikes/native-render-parity/out/fonts/inter-license.txt
shasum -a 256 spikes/native-render-parity/out/fonts/inter.ttf
cargo test --offline -p opencanvas-server -j2 --test native_layout_parity -- --test-threads=2
cargo clippy --offline -p opencanvas-scene-input -p opencanvas-storage -p opencanvas-server -j2 --lib --bin opencanvas-server -- -D warnings
cargo fmt --package opencanvas-scene-input --package opencanvas-storage --package opencanvas-server --check
cargo fmt --all --check
git diff --check
```

Temporary logs: `/tmp/opencanvas-task25-default-gates-resume.log`,
`/tmp/opencanvas-task25-native-layout-rerun.log` (missing fixture),
`/tmp/opencanvas-task25-native-layout-ready.log` (pass),
`/tmp/opencanvas-task25-clippy.log`, `/tmp/opencanvas-task25-global-fmt.log`.
Tests used temporary storage/listeners; no native/preview mutation or deployment.

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

## Task24 rerun after slices 1–3 — 2026-11-13

**Correctness matrix completed; latency acceptance not met.** PostgreSQL 30×
strict improved 4.73× and scoped 9.67×, but remain above ~1.5 s. Context misses
≤1 s. The no-1×-regression criterion is unproven: several read/HTTP paths are
materially slower than baseline. Production limits remain unchanged.

The exact command exited 0: both engines × 1/15/30 own-master and 15× shared,
one warmup then five measured samples. All eight fixtures passed explicit full
validation and ten untimed full/incremental operation comparisons before timing
(80 comparisons total). The coordinator checked 508 numeric summaries, each
with n=5 and five retained values. No heavy test/build overlapped measurements.

~~~sh
TASK24_COPIES_LIST="1 15 30" bash scripts/task24-bench.sh
node spikes/large-project/summarize.mjs <output-dir>
~~~

Checkout b9652e947d7bd03455068f0bcf2e2e4292fce8f7, product source
a98012d33b14738c115414dd029df9240bc017bc; only progress documents were dirty.
Baseline is the Task24 source committed as 81f883c (its report records ee894c0
plus then-uncommitted Task24 changes). No product source changed during timing.
Release, Rust/Cargo 1.99.1, Node v24.21.0, Apple M5 / 10 cores / 24 GiB /
Darwin arm64; PG 16.15 through OrbStack, container 2 CPUs / 2 GiB, pool 4 and
benchmark timeouts 600 s. Baseline used Docker Desktop. One laptop run per cell,
not a matched interleaved baseline experiment: differences below ~20% are not
causal evidence. Source counts match the baseline workload shape.

Both engines share these counts; 119 tokens, 89 styles, 42 fonts, 45 hash-checked
assets and one named theme stay shared. Sizes/admission match baseline at reported
precision. Gzip figures are offline level-6 estimates, **not** HTTP compression.

| Scale | Authored / expanded | Pages / instances / notes | Objects MiB (gzip) | Rows MiB (gzip) | Payload admitted |
| --- | --- | --- | --- | --- | --- |
| 1× own | 6,711 / 19,980 | 19 / 1,708 / 487 | 1.6 (0.2) | 1.3 (0.2) | yes |
| 15× own | 100,665 / 299,700 | 285 / 25,620 / 7,305 | 24.1 (1) | 20.1 (1) | no |
| 30× own | 201,330 / 599,400 | 570 / 51,240 / 14,610 | 48.1 (1.9) | 40.3 (1.8) | no |
| 15× shared | 65,805 / 243,910 | 285 / 19,586 / 6,535 | 15.2 (0.7) | 12.6 (0.6) | yes |

### Before / after: commit and context

All latency pairs are **median / p95 milliseconds**; p95 is nearest rank over
five samples (the maximum). Baseline retains Task24 reporting precision.

| Scale | Engine | Strict before | Strict after | Scoped before | Scoped after | Context before | Context after |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1× own | postgres | 377.2 / 389.8 | 128.1 / 150.8 | 619.2 / 658.6 | 197.8 / 255.5 | 765 / 787.2 | 1,786.2 / 2,164.2 |
| 1× own | sqlite | 404.8 / 422 | 131.5 / 141.9 | 735.9 / 766.6 | 159.3 / 172.3 | 788.8 / 876.2 | 1,313.2 / 1,590.5 |
| 15× own | postgres | 5,912.9 / 5,944 | 1,085.9 / 1,370.7 | 11,782.7 / 13,677.6 | 1,617.4 / 1,716.1 | 13,368.1 / 13,552.9 | 12,586.2 / 15,362.7 |
| 15× own | sqlite | 6,533.2 / 6,782.1 | 1,560.9 / 1,655.6 | 15,025.1 / 24,125.3 | 1,518.9 / 1,536.6 | 14,132.2 / 21,383 | 18,110.3 / 21,460.5 |
| 30× own | postgres | 12,639 / 13,294.1 | 2,673.4 / 2,858.8 | 34,121.9 / 55,458.4 | 3,527.9 / 4,514.8 | 30,426.1 / 34,675.8 | 33,981.2 / 38,668.9 |
| 30× own | sqlite | 12,723.7 / 13,348.2 | 4,776.2 / 5,326.6 | 38,871.9 / 40,419.7 | 2,626.3 / 3,907.4 | 42,608.6 / 46,357.8 | 38,098.2 / 44,011.2 |
| 15× shared | postgres | 6,654.9 / 9,519.6 | 961.6 / 1,391.4 | 8,360.4 / 18,265.3 | 719.6 / 784.9 | 12,463.4 / 15,387.9 | 8,918.2 / 9,109.6 |
| 15× shared | sqlite | 4,624.3 / 4,654.6 | 1,578.8 / 2,371 | 8,998.4 / 10,291.8 | 1,983.6 / 2,615.8 | 12,290.1 / 22,192.1 | 21,258.3 / 21,923.1 |

PG 30× misses strict/scoped budgets by 1.78× / 2.35× and context by 33.98×.
PG context 30× is +11.7% versus baseline, within the stated noise band; no
improvement claim. SQLite context 15× is +28.1%, shared 15× +73.0%; context
1× is +66.5% SQLite / +133.5% PG. These observed regressions are retained.
Cold-load/encode/find/outline also drift, including paths outside this change.
No matched baseline rerun established source causality versus host/allocator/
virtualization effects. Do not dismiss these as proven noise or claim no regression.

### After: strict commit phases

Median / p95 ms. Instances/expansion are nested within validation; do not sum
them again. Phase medians need not sum to the total median.

| Scale | Engine | Load | Candidate build | Validation | Themes | Instances | Expansion | Guard | History | Persist | Cache fill |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1× own | postgres | 28.6 / 29.2 | 6.4 / 8.1 | 55.9 / 69.1 | 0.045 / 0.061 | 51.8 / 63.8 | 37 / 47.5 | – | 0.3 / 0.4 | 4.6 / 6.2 | 5.5 / 12.6 |
| 1× own | sqlite | 12.4 / 12.7 | 6.7 / 7.5 | 63.3 / 68.9 | 0.039 / 0.045 | 58.9 / 64.3 | 41.6 / 46.2 | – | 0.3 / 0.4 | 19.9 / 22.1 | – |
| 15× own | postgres | 197.2 / 247.1 | 59.1 / 73.7 | 513.9 / 613.6 | 0.026 / 0.029 | 476.8 / 570.9 | 326.6 / 398.1 | – | 3.6 / 3.9 | 10.7 / 12.7 | 62.4 / 77.5 |
| 15× own | sqlite | 163 / 170.6 | 79.8 / 103.8 | 756 / 852.6 | 0.044 / 0.074 | 707.3 / 797.7 | 472.4 / 548.7 | – | 4.6 / 6.1 | 206.3 / 235.1 | – |
| 30× own | postgres | 407.2 / 542.8 | 143.5 / 163.2 | 1,223.7 / 1,324.6 | 0.031 / 0.039 | 1,138.7 / 1,239.6 | 680.3 / 864.6 | – | 8.2 / 10.1 | 11 / 15.8 | 104.4 / 187.7 |
| 30× own | sqlite | 331.8 / 384.4 | 159.7 / 210.7 | 2,013.4 / 2,990 | 0.04 / 0.072 | 1,907.3 / 2,818.8 | 1,272.2 / 2,021 | – | 11.1 / 13.5 | 861.4 / 1,102.3 | – |
| 15× shared | postgres | 191.1 / 248.4 | 45.4 / 63.5 | 444.1 / 563.9 | 0.03 / 0.04 | 414.7 / 525.3 | 258 / 322.1 | – | 3.5 / 4.4 | 7.9 / 19.8 | 45.9 / 67.6 |
| 15× shared | sqlite | 113.4 / 268.9 | 58.3 / 140.1 | 837.7 / 1,248.8 | 0.052 / 0.079 | 779.8 / 1,183.6 | 503.4 / 827.6 | – | 6 / 7.8 | 211.4 / 392.4 | – |

### After: scoped commit phases

Median / p95 ms. Instances/expansion are nested within validation; do not sum
them again. Phase medians need not sum to the total median.

| Scale | Engine | Load | Candidate build | Validation | Themes | Instances | Expansion | Guard | History | Persist | Cache fill |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1× own | postgres | 36.1 / 64.5 | 12.6 / 15.5 | 83.1 / 98.1 | 0.046 / 0.058 | 76 / 90.8 | 53.5 / 67.3 | 1.2 / 1.6 | 0.3 / 0.6 | 4.7 / 9.7 | 14.8 / 24.9 |
| 1× own | sqlite | 12.4 / 15.9 | 6.3 / 15 | 80.4 / 82.3 | 0.043 / 0.079 | 75.2 / 76.3 | 55 / 55.9 | 0.2 / 0.8 | 0.3 / 0.4 | 22.6 / 29.7 | – |
| 15× own | postgres | 275.8 / 353.7 | 111.6 / 121.7 | 714.7 / 807.2 | 0.042 / 0.044 | 641.1 / 757.6 | 452.5 / 533.7 | 2.5 / 5 | 4.4 / 6 | 9.6 / 10.3 | 80.9 / 104.7 |
| 15× own | sqlite | 144.9 / 169.7 | 76.1 / 79 | 749.9 / 803.4 | 0.037 / 0.043 | 699.7 / 752.2 | 479.8 / 520.4 | 0.7 / 1 | 4.5 / 5.2 | 190.4 / 202.1 | – |
| 30× own | postgres | 564.8 / 1,002.7 | 202.8 / 269.7 | 1,376.3 / 2,385.7 | 0.036 / 0.051 | 1,285.2 / 2,283.4 | 946 / 1,851.7 | 2.7 / 10.9 | 12.4 / 15.7 | 10.5 / 11.9 | 149.5 / 457.3 |
| 30× own | sqlite | 229 / 290.1 | 97.3 / 123.2 | 1,369.5 / 1,421.7 | 0.029 / 0.033 | 1,293.2 / 1,339.1 | 760.2 / 975.6 | 1.3 / 1.9 | 11.4 / 12.7 | 336.3 / 1,320.5 | – |
| 15× shared | postgres | 151.4 / 176.2 | 34.5 / 36.5 | 331.6 / 358.4 | 0.024 / 0.026 | 308.3 / 334.3 | 203.3 / 220.1 | 1.4 / 1.6 | 2.4 / 2.5 | 7 / 12.9 | 40.9 / 42.2 |
| 15× shared | sqlite | 143 / 253.8 | 78 / 126.3 | 908.3 / 1,492.4 | 0.051 / 0.075 | 846.5 / 1,409.3 | 583.5 / 1,020 | 1.9 / 2.1 | 6.9 / 9.7 | 225.4 / 419.2 | – |

### Additional 30× phase detail and work counts

| Phase (ms) | postgres strict | postgres scoped | sqlite strict | sqlite scoped |
| --- | --- | --- | --- | --- |
| candidate.apply | 0.043 / 0.059 | 0.045 / 0.096 | 0.043 / 0.054 | 0.03 / 0.034 |
| validate.index.before | 16.145 / 19.094 | 19.389 / 64.628 | 19.066 / 28.612 | 11.617 / 14.171 |
| validate.index.after | 4.774 / 5.858 | 6.04 / 12.351 | 5.74 / 9.213 | 4.632 / 5.096 |
| validate.affected | 0.025 / 0.05 | 0.039 / 0.129 | 0.053 / 0.091 | 0.016 / 0.045 |
| validate.tokens | 0.084 / 0.099 | 0.096 / 0.151 | 0.125 / 0.484 | 0.081 / 0.092 |
| validate.notes_navigation | 40.124 / 48.309 | 45.005 / 110.815 | 63.462 / 103.083 | 38.016 / 49.104 |
| validate.animation_triggers | 6.025 / 8.372 | 7.029 / 24.623 | 6.004 / 17.23 | 5.157 / 6.15 |
| persist.delta | 115.303 / 177.832 | 155.174 / 252.955 | – | – |

PG 30× strict validation 1,223.662 ms includes instances 1,138.680 ms and
expansion 680.304 ms. Load is 407.158 ms; SQL persist is 10.999 ms, plus a
separate delta-preparation phase of 115.303 ms. Scoped validation is 1,376.250 ms,
load 564.825 ms, guard 2.719 ms and history 12.447 ms. Baseline PG scoped guard
was 18,361.6 ms; strict history was 1,142.3 ms. Sparse properties/history no
longer dominate. Full expansion, global checks, row loads, candidate construction
and reference-index work remain. Uninstrumented remainder includes index/reference
scans, allocation/drop and bookkeeping; do not label it SQL or zero-cost work.

| Scale | Authored / expanded per commit | Affected authored / virtual | History IDs compared | Old-index hits / topology reuse | Full fallback | Dependency builds |
| --- | --- | --- | --- | --- | --- | --- |
| 1× own | 6,711 / 19,980 | 1 / 0 | 1 | 1 / 1 | 0 | not built |
| 15× own | 100,665 / 299,700 | 1 / 0 | 1 | 1 / 1 | 0 | not built |
| 30× own | 201,330 / 599,400 | 1 / 0 | 1 | 1 / 1 | 0 | not built |
| 15× shared | 65,805 / 243,910 | 1 / 0 | 1 | 1 / 1 | 0 | not built |

These counts hold for every measured sample, mode and engine. Index hit/reuse
are events per commit, not nodes (the generic harness currently labels them
unit:"nodes"). Each small edit still expands the entire project. Current-revision
scoped commits never request dependency construction. This matrix does **not**
measure cold stale guards, warm closure traversal, edit-context latency or cache
eviction/multi-project contention. Their correctness evidence is separate.


### After: snapshots and HTTP

Median / p95 ms. Payload admission still rejects 15×/30× own-master snapshots;
metadata caching does not make oversized HTTP payloads cacheable. In combined
GET cells below, the first pair is cold and the second is repeat.

| Scale | Engine | Storage cold | Moka warm | Encode objects | GET objects cold ; repeat | GET rows cold ; repeat |
| --- | --- | --- | --- | --- | --- | --- |
| 1× own | postgres | 16.8 / 20.4 | 3.6 / 4.6 | 11.7 / 12.1 | 18.6 / 20.5 ; 2 / 2.1 | 32.7 / 49.1 ; 2.3 / 2.6 |
| 1× own | sqlite | 16.9 / 22.8 | – | 25.5 / 30.1 | 32.3 / 33.7 ; 2.6 / 2.7 | 32.5 / 36 ; 2.8 / 2.9 |
| 15× own | postgres | 271.8 / 311.9 | 207.9 / 220.1 | 150.2 / 172.9 | 435.1 / 450.1 ; 446.9 / 488.1 | 515.4 / 742.6 ; 527.8 / 1,033.2 |
| 15× own | sqlite | 121.2 / 133.6 | – | 172.8 / 178.5 | 398.1 / 463.4 ; 390.6 / 402 | 691.2 / 814.6 ; 615.7 / 1,027.2 |
| 30× own | postgres | 542.8 / 621.4 | 383.9 / 405.5 | 263.6 / 305.6 | 748.7 / 1,209.5 ; 812 / 996.9 | 774.3 / 829.3 ; 777 / 1,174.3 |
| 30× own | sqlite | 265.8 / 316.5 | – | 238 / 250 | 704.5 / 831.1 ; 685.9 / 853.8 | 798.4 / 825.5 ; 788.4 / 865.1 |
| 15× shared | postgres | 148 / 155.7 | 25.2 / 33.4 | 94.4 / 98.1 | 123.3 / 130.3 ; 5.4 / 7 | 121.2 / 127.8 ; 7.2 / 7.6 |
| 15× shared | sqlite | 90.4 / 101.5 | – | 134 / 137.4 | 201.3 / 264.8 ; 6.3 / 7.7 | 178.4 / 180.7 ; 6.3 / 6.8 |

### After: expansion, find and outline

Median / p95 ms. Every MCP read returned a result without tool errors. Context
and outline answer sizes remain constant across scales; internal work grows.
Find retains totalMatches/scannedNodes and existing cursor semantics; page-at-a-time
traversal is not implemented.

| Scale | Engine | Expansion | Find name many | Find name none | Find text | Outline page |
| --- | --- | --- | --- | --- | --- | --- |
| 1× own | postgres | 25 / 27.9 | 132.2 / 202 | 54.1 / 57.3 | 64.8 / 67.1 | 76.4 / 78.7 |
| 1× own | sqlite | 30.1 / 35.2 | 143.7 / 166.2 | 84.6 / 97.9 | 121.1 / 150.9 | 107.7 / 112.1 |
| 15× own | postgres | 782.2 / 1,574.1 | 1,626.6 / 3,426.2 | 1,021.4 / 1,111.2 | 1,467.8 / 2,389.7 | 1,522.4 / 2,366.9 |
| 15× own | sqlite | 664.3 / 979.6 | 1,940 / 3,891.6 | 1,783.4 / 2,546.3 | 1,936.5 / 2,362.6 | 949.7 / 1,238.6 |
| 30× own | postgres | 770.8 / 966.4 | 2,258.9 / 3,469.8 | 1,550.4 / 1,782.7 | 1,895.8 / 2,039 | 1,510.9 / 1,757.7 |
| 30× own | sqlite | 1,089.8 / 1,250.8 | 2,910 / 7,919 | 1,634.6 / 1,806.2 | 1,691.5 / 1,921.5 | 1,274.5 / 1,336.6 |
| 15× shared | postgres | 290.2 / 322.5 | 812.1 / 862.2 | 375.7 / 411.6 | 434.5 / 449.3 | 305 / 316.2 |
| 15× shared | sqlite | 321 / 345.2 | 1,134.2 / 2,182 | 543.3 / 720.3 | 1,069.6 / 1,692.1 | 790.4 / 1,256.3 |

### Memory and viewer

RSS is process peak / after-seed observation, not a sampled median/p95. Peak
includes harness fixture construction and now untimed full/parity validation;
allocator retention can raise the seed floor. This is not a server-only memory
bound or matched attribution to the metadata cache.

| Scale | Engine | Peak MiB before → after | After-seed MiB before → after |
| --- | --- | --- | --- |
| 1× own | postgres | 420.7 → 357.6 | 115 → 153 |
| 1× own | sqlite | 398.3 → 320.1 | 106 → 149.2 |
| 15× own | postgres | 4,420.5 → 3,578.6 | 1,132 → 1,998.3 |
| 15× own | sqlite | 4,560.7 → 3,535.2 | 1,102 → 1,782.5 |
| 30× own | postgres | 5,899.6 → 5,491.8 | 2,531 → 3,545.9 |
| 30× own | sqlite | 5,875.2 → 4,515.1 | 2,164 → 3,532.8 |
| 15× shared | postgres | 3,644.1 → 3,556.9 | 980 → 1,234.8 |
| 15× shared | sqlite | 3,596.7 → 2,723.8 | 964 → 1,206.2 |

PG 30× peak is 5,491.8 MiB versus 5,899.6 baseline; SQLite 4,515.1 versus
5,875.2. Both seed floors increased substantially. These figures do not justify
1M authored in production. Viewer below is Node/V8 parse/index only, not Chromium
heap, canvas/render or All-frames timing.

| Scale | Parse before → after ms | Index before → after ms | Retained heap MiB before → after |
| --- | --- | --- | --- |
| 1× own | 8.9 / 10.2 → 8.6 / 9.9 | 2.1 / 3.3 → 2 / 2.6 | 4.1 → 4.1 |
| 15× own | 94.3 / 102 → 89.7 / 95 | 41 / 51.1 → 41.8 / 50.8 | 59.2 → 59.2 |
| 30× own | 203.6 / 220.1 → 213.3 / 231.6 | 59.1 / 59.3 → 64 / 72 | 118.6 → 118.6 |
| 15× shared | 56.9 / 58.8 → 86.1 / 89.8 | 16.8 / 28 → 42.9 / 56.5 | 37.9 → 37.9 |

### Evidence retention and remaining scope

OS temporary output, unversioned:
/var/folders/pw/yxvtwnf517qchxqynh2w7pz80000gn/T/opencanvas-task24.YjaoQN.
Wrapper log: /tmp/opencanvas-task25-matrix.log. Numeric JSON, time logs, metadata
and sample arrays remain there. Private source/assets and saved response bodies
were removed by the wrapper. Docker inspection confirmed no disposable Task19
container remained. No native/preview writes, restart, push or deployment.

Slices 4–5 are large and unstarted. See [proposed independent handoffs](checkpoints/2026-11-13-task25-next-slices.md).
Further commit optimization is also required for the write latency target;
read/HTTP/viewer work alone does not promise to remove measured write costs.
Slice 6 production caps/bundle alignment follows later evidence.

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
The coordinator gate/matrix results are recorded above. Final Rust size audit
against Task24 baseline found no modified file above 250 lines; largest 237.
Slices 4–6 remain pending; stop before starting the larger work.

## Slice 4a — scoped expansion and resolution (2026-11-13)

The user approved slice 4 after checkpoint `c0ac08c`. This part introduces the
pure scene-input boundary and matching storage accessor; server MCP/render/find
integration belongs to separate 4b/4c/4d commits. No storage layout, response,
production cap, render-node or pixel budget changes.

`read_scope::ScopedSnapshot` provides indexed authored lookup/root iteration,
`expand(ReadScope::Page|Target, definitions)`, closure-only `resolve`, resource-only
`resolved_tokens`, shallow navigation metadata and indexed `notes_for_nodes`.
Expansion includes placed ancestor chains and transitive external/nested masters.
With `definitions=true`, authored definitions are also expanded/resolved for MCP
reference metadata; otherwise masters are lookup dependencies only. `ScopedNodes`
keeps authored output, dependency references and expanded nodes distinct. Source
vector order, stable virtual IDs, placed themes, overrides, font selection,
visibility/opacity and serialized error fields match the full reference fixtures.
`ResolvedNodes::from_nodes` and scoped `make_input_with_scale` reuse existing
frame preparation, exact 2,048-node limit and viewport/scale checks.

Trust is explicit: ordinary `ScopedSnapshot::validate` fully validates the entire
snapshot, and existing standalone resolve/input APIs are unchanged. The separate
`from_validated` constructor documents its complete validated-revision/index
preconditions. Only builtin SQLite/PostgreSQL adapters construct storage's private
committed authority binding. `SharedSnapshot::trusted_scoped` returns `Some` only
while both public snapshot/index Arcs still match that original pair; replacing
either Arc or using a custom backend loses authority. `.scoped()` then fully
validates; read layers needing legacy unvalidated behavior can use `None` to retain
their old path. Supplying a cache identity from a custom backend grants no trust.

The revision metadata cache lazily builds `ScopeIndex` from the actual authoritative
snapshot vector, never Candidate's BTreeMap order. Warm reads reuse it; a new
candidate/revision starts with an empty slot. The index retains IDs/positions,
children, navigation sources and note positions, without node/note payloads.
Admission reserves conservative node/link metadata plus 2 MiB for note positions
before materialization; default 5,000 notes with 128-byte anchors fit within
1,160,000 proxy bytes. An index exceeding its reservation is returned without
retention. The 256 MiB metadata cache and existing payload-cache limits are
unchanged. Cold snapshot SQL/loading and one index build still scale with the
project; trusted scoped reads do not full-validate or clone a whole Candidate.

Validated component subtrees cannot contain links/flowStart/animations, and
instance/override schemas cannot add links. Negative tests establish this guard,
so shallow authored incoming navigation metadata also serves expanded contexts;
inventing inherited virtual navigation would violate the existing invariant.
Cross-page targets, source screen ancestry and source order are retained.

Actual focused checks (temporary roots/disposable PostgreSQL only):

| Check | Result |
| --- | --- |
| Pure scope/scene/navigation parity, invalid/untrusted cases and render boundaries | 12 passed |
| SQLite warm/commit/cold order, old readers, Arc swaps and custom identity fallback | 2 passed, 1 PG ignored |
| Lazy metadata reuse and 5,000-note reservation unit test | 1 passed |
| Fresh disposable PostgreSQL scope identity/order/old-reader/Arc swap parity | 1 passed |
| Scene-input/storage library clippy, package fmt and raw diff whitespace | passed |

The bounded-work fixture grows from one page to 25 pages with 50 extra authored
nodes per page. The unchanged target's closure/expanded/resolved counts remain
**8 / 11 / 11** in both cases; leaf-target counts also match. Counts exclude full
reference validation and cold index construction. These are deterministic work
counts, not release latency medians or the final context acceptance measurement.
Virtual target preparation currently expands its outer authored instance closure,
then filters the requested virtual subtree for scene input. A wide outer instance
can therefore expand sibling branches; it never requires the entire containing
page/project. Resource arrays are intentionally still available in full.

```sh
cargo test --offline -p opencanvas-scene-input -j2 --test scoped --test scoped_input --test scoped_navigation -- --test-threads=2
cargo test --offline -p opencanvas-storage -j2 --test scoped_reads --test scoped_backend -- --test-threads=2
cargo test --offline -p opencanvas-storage -j2 --lib lazy_scope_index -- --test-threads=2
bash scripts/task19-check.sh cargo test --offline -p opencanvas-storage -j2 --test scoped_reads -- --ignored --test-threads=1
rtk bypass cargo test --offline -p opencanvas-scene-input -j2 --test scoped scoped_expansion_and_resolution_work -- --test-threads=1 --nocapture
cargo clippy --offline -p opencanvas-scene-input -p opencanvas-storage -j2 --lib -- -D warnings
cargo fmt --package opencanvas-scene-input --package opencanvas-storage --check
git diff --check
```

The first new navigation fixture correctly failed because it put links in a
component subtree; it became a negative invariant test. The first SQLite cold
fixture attempted a second owner of one live data root and hit `DATA_ROOT_LOCKED`;
dropping the first owner before reopening fixed the intended cold-read test.
Clippy's map-key iteration warning was fixed and the gate rerun. New modules are
18–117 lines; modified revision-index module 230, its tests 141, largest existing
touched adapter 237. The largest new test is 212 lines; no touched Rust file
exceeds 250. No live service/data access, deploy, restart or push occurred.
