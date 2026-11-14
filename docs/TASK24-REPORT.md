# Task24 — Large-project baseline benchmark (Phase 0)

**Status:** measured 2026-11-12 on branch `feat/project-node-limit` (HEAD
`ee894c0` plus the uncommitted Task24 changes). Measurement only: the default build
and its behaviour are unchanged. Handoff: [tasks/24](../tasks/24-large-project-benchmark.md);
context: [LARGE-PROJECT-PROPOSAL](LARGE-PROJECT-PROPOSAL.md).

## Rerun

```sh
bash scripts/task24-bench.sh
```

The wrapper prints an OS temporary output directory. It reads the source design
from the loopback native server with HTTP GET only (one snapshot GET and one GET
per asset; font files are assets), builds the release test target with two Cargo
jobs, runs every SQLite cell in a temporary data root, then every PostgreSQL cell in
one fresh container started by `scripts/task19-check.sh` (2 CPUs, 2 GiB, random
loopback port, removed on exit), then the Node viewer step and `summary.md`. The
fetched design and the saved response bodies are deleted on exit; only counts,
sizes and timings remain (`cell-*.json`, `viewer-*.json`, `time-*.txt`, logs,
`metadata.txt`). Nothing in the output directory is versioned.

Knobs: `TASK24_ENGINES` (default `sqlite postgres`), `TASK24_COPIES_LIST` (default
`1 5 15 30`), `TASK24_SHARED_COPIES` (default `15`; empty skips the variant),
`TASK24_WARMUP`/`TASK24_SAMPLES` (default 1/5, at least 5), `TASK24_SOURCE_PROJECT`
(default `axis-training`). `node spikes/large-project/summarize.mjs <dir>` re-prints
the tables of an output directory. A full run takes about 45 minutes.

| File | Role |
| --- | --- |
| `crates/scene-input/src/bench_phase.rs` | Feature-gated phase timer (`time`, `Span`, `take`); a plain call otherwise |
| `crates/{scene-input,storage,server}/Cargo.toml` | Feature `large-project-bench`; server test target with `required-features` |
| `crates/scene-input/src/model.rs`, `model/notes.rs` | `MAX_NODES`/`MAX_NOTES` raised only under the feature |
| `crates/scene-input/src/validate/{candidate,instances}.rs`, `crates/storage/src/{batch.rs,postgres/batch.rs,postgres/adapter.rs}` | Phase wrappers only |
| `crates/server/tests/large_project_benchmark.rs` + `large_project_benchmark/` | Harness: `source`, `scale`, `seed`, `snapshots`, `mcp`, `commits`, `stats`, `cell` |
| `spikes/large-project/viewer-cost.mjs`, `summarize.mjs` | Node viewer cost; Markdown tables |
| `scripts/task24-bench.sh` | Wrapper |

## Method

**Source.** The `axis-training` project, read through `GET .../snapshot` and
`GET .../assets/{asset}` on `127.0.0.1:3857` (45 assets, 1.5 MB, all present and
hash-checked; 41 of 42 fonts are asset-backed). 1×: 6,711 authored nodes, 19,980
expanded (99.9 % of today's 20,000 cap), 19 pages, 1,708 instances, 310 component
masters, 119 tokens, 89 styles, 487 notes and one named theme, so validation runs
two theme passes. The optional `axis-courses` source was not run. This report names
no design content.

**Scaling.** `scale.rs` appends copies 1..k−1 of every page. Every cloned ID gets
the suffix `-xKK`; parent IDs, `componentId`, override keys, every other prop string
equal to a cloned node ID (links, `flowStart`, animation and state targets) and note
targets follow it; `name`/`text` values are never rewritten. Tokens, styles, fonts
and assets stay shared. Scales: 1×, 5× (33.6k authored), 15× (100.7k), 30× (201.3k).
The **shared-master** variant (15×) clones pages without master subtrees: cloned
instances keep pointing at the source masters, so authored grows by 4,221 nodes per
copy instead of 6,711, while expanded grows almost as fast.

**Fixture loading.** The project is created through the product API in temporary
storage only (a `tempfile` SQLite root; the disposable container), then its entity
rows and asset bytes are written directly at revision 1, because seeding 200k nodes
through 400 whole-project validations would take longer than the benchmark. The
stored node ID set is compared with the fixture before timing, and the first strict
commit validates the complete seeded project; every cell passed it.

**Cap override.** Feature `large-project-bench` (scene-input → storage → server)
raises `MAX_NODES` to 2,000,000 and `MAX_NOTES` to 1,000,000 and enables the phase
timer. No default feature enables it; the test target has `required-features` and
asserts that the cap is raised. `MAX_NOTES` had to move too: 15× has 7,305 notes,
and the 5,000-note cap failed every validating call, including `get_design_context`.

**Phases.** `bench_phase::time`/`Span` wrap these product phases. Without the
feature `time` is a plain call and `Span` an empty struct.

| Phase | Covers |
| --- | --- |
| `load` | SQLite `read_snapshot` / PostgreSQL `entities_for_write`, inside the writer |
| `candidate.from_snapshot`, `candidate.apply` | Candidate build; operations and link pruning |
| `candidate.validate` | `Candidate::validate()`. Inside it: `validate.theme` (clone + content check, summed over both themes) and `validate.instances` (graph, `instances::expand`, override checks and per-theme property checks of every virtual node) |
| `scoped.check` | History query + `scoped::check` (dependency closures of current and candidate) |
| `history.envelope` | `scoped::envelope`: footprint diff over the whole project, hash |
| `persist` | SQLite `Project::persist` incl. commit; PostgreSQL `transaction::persist` + commit |
| `cache.fill` | PostgreSQL post-commit Moka write-through |

**Cells and timers.** One process per cell (engine × variant × scale), so
`/usr/bin/time -l` reports its peak RSS. That figure includes the harness, which
builds the scaled fixture before measuring; "RSS after seed" shows that floor. Each
item has 1 warm-up then 5 samples; tables show **median / p95 ms** (nearest rank).

1. *Storage cold load*: `Storage::snapshot` on SQLite and on a second PostgreSQL
   backend without a cache. *Moka warm*: the production-shaped PostgreSQL backend
   (Moka on) after one fill, including its identity probe and owned copy. *Encode
   objects*: `to_value` + `truncated` + `to_vec`, as the object representation does.
2. *GET*: a real listener and router per sample. *Cold* is the first GET on a fresh
   `AppState` (empty response cache, new connection); *repeat* is a second GET on
   the same state. Gzip size is computed by the harness (level 6); the snapshot route
   itself does not compress.
3. and 4. *Commits*: one `update_node` text change on a text node outside masters on
   the source's second page, through `ProjectHandle::apply_batch` (strict) and
   `apply_scoped_batch` with `pageIds` = that page (scoped). MCP, SSE and render
   enqueue are outside this boundary. Each sample advances the revision.
5. *Reads*: `instances::expand` of the whole project (direct call), then over one
   MCP session: `find_nodes` `nameContains:"a"` (many matches, first page returned),
   an absent `nameContains` (zero matches, full scan), `textContains:"e"`;
   `get_outline` of the target page; `get_design_context` of its root frame without
   viewports (no render job).
6. *Viewer*: `viewer-cost.mjs` in Node on the saved object body: `JSON.parse`, then
   the viewer's own `snapshotNodeIndex` with a newer revision each run, and the heap
   retained by both after GC.

PostgreSQL uses the production pool size (4) and lock timeout; the statement and
operation timeouts are raised from 15 s/30 s to 600 s so the largest scales report
a time instead of `STORAGE_UNAVAILABLE`. SQLite and PostgreSQL cells come from two
invocations of the same wrapper on the same source tree: SQLite from the first (its
PostgreSQL cells failed on a harness bug, fixed by seeding off the async runtime),
PostgreSQL from `TASK24_ENGINES=postgres`.

**Environment.** Apple M5, 10 cores, 24 GiB, Darwin 25.4 arm64; rustc/cargo 1.99.1
(LLVM 23.1.1), release profile, 2 build jobs; Node v24.21.0; PostgreSQL 16.15
(`postgres:16-alpine`) in Docker Desktop (linux aarch64), 2 CPUs / 2 GiB, reached
through the Docker loopback port. One run per cell, fixed order (scales ascending,
then the shared variant), no concurrent load. Single runs on a laptop: treat
differences under ~20 % between engines as noise; the scaling trends are the result.

## Results

### Counts and sizes

| Scale | Engine | Authored | Expanded | Pages | Instances | Snapshot JSON MiB | Objects body MiB (gzip) | Rows body MiB (gzip) | Cache admitted |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1× own | postgres | 6,711 | 19,980 | 19 | 1,708 | 1.6 | 1.6 (0.2) | 1.3 (0.2) | yes |
| 1× own | sqlite | 6,711 | 19,980 | 19 | 1,708 | 1.6 | 1.6 (0.2) | 1.3 (0.2) | yes |
| 5× own | postgres | 33,555 | 99,900 | 95 | 8,540 | 8 | 8 (0.4) | 6.7 (0.4) | yes |
| 5× own | sqlite | 33,555 | 99,900 | 95 | 8,540 | 8 | 8 (0.4) | 6.7 (0.4) | yes |
| 15× own | postgres | 100,665 | 299,700 | 285 | 25,620 | 24.1 | 24.1 (1) | 20.1 (1) | no |
| 15× own | sqlite | 100,665 | 299,700 | 285 | 25,620 | 24.1 | 24.1 (1) | 20.1 (1) | no |
| 30× own | postgres | 201,330 | 599,400 | 570 | 51,240 | 48.1 | 48.1 (1.9) | 40.3 (1.8) | no |
| 30× own | sqlite | 201,330 | 599,400 | 570 | 51,240 | 48.1 | 48.1 (1.9) | 40.3 (1.8) | no |
| 15× shared | postgres | 65,805 | 243,910 | 285 | 19,586 | 15.2 | 15.2 (0.7) | 12.6 (0.6) | yes |
| 15× shared | sqlite | 65,805 | 243,910 | 285 | 19,586 | 15.2 | 15.2 (0.7) | 12.6 (0.6) | yes |

### Snapshot load and HTTP GET (median / p95 ms)

| Scale | Engine | Storage cold load | Moka warm | Encode objects | GET objects cold | GET objects repeat | GET rows cold | GET rows repeat |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1× own | postgres | 9.2 / 9.8 | 1.9 / 2.6 | 6.2 / 6.3 | 11.3 / 11.7 | 0.9 / 1.1 | 13 / 15.4 | 1 / 1.1 |
| 1× own | sqlite | 6.7 / 7.6 | – | 6.2 / 6.4 | 20.7 / 22.3 | 1.8 / 1.9 | 26.5 / 29.2 | 2.1 / 2.3 |
| 5× own | postgres | 54 / 56.5 | 14.1 / 20.2 | 35.4 / 35.8 | 60.7 / 63.7 | 2.2 / 2.3 | 64.8 / 69.9 | 2 / 2.2 |
| 5× own | sqlite | 39.9 / 40.4 | – | 41 / 43.3 | 113.4 / 118.7 | 3.4 / 3.9 | 111.6 / 113.9 | 3.1 / 3.2 |
| 15× own | postgres | 193.6 / 208.5 | 232.6 / 246.7 | 126.2 / 128.3 | 433.8 / 443.6 | 427 / 432.1 | 388.6 / 416.7 | 389.3 / 412.7 |
| 15× own | sqlite | 147.6 / 182.3 | – | 138.4 / 157.3 | 505.4 / 635.5 | 490 / 571.4 | 480.8 / 673.8 | 530.4 / 557.5 |
| 30× own | postgres | 356.2 / 421.9 | 362.9 / 410.5 | 214.2 / 227.1 | 731.2 / 762.4 | 719 / 746.2 | 848.4 / 868 | 835.3 / 857.1 |
| 30× own | sqlite | 225.5 / 256.8 | – | 232 / 265 | 691.8 / 722.3 | 669.8 / 734.7 | 678.2 / 894 | 694.8 / 758.8 |
| 15× shared | postgres | 142.3 / 191.1 | 24.9 / 38.7 | 95 / 124.2 | 203.2 / 269.7 | 5.8 / 7.5 | 133.1 / 160.4 | 3.2 / 3.5 |
| 15× shared | sqlite | 75.4 / 80.8 | – | 77.8 / 96.6 | 252.5 / 419.6 | 5.2 / 8.7 | 301.2 / 388.5 | 4.8 / 6.6 |

### Commit, strict (median / p95 ms)

| Scale | Engine | Total | Load | from_snapshot | validate() | of which themes | of which instances (expand) | History envelope | Persist | Cache fill |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1× own | postgres | 377.2 / 389.8 | 12.4 / 13.6 | 3.3 / 3.5 | 332 / 346.3 | 81.9 / 86.1 | 247.2 / 263.1 (20) | 15.4 / 15.9 | 2.7 / 4.2 | 3.9 / 4.2 |
| 1× own | sqlite | 404.8 / 422 | 7.4 / 7.4 | 3.5 / 3.7 | 363.6 / 379.7 | 92 / 96.1 | 270.5 / 282 (22) | 16.6 / 17.9 | 11.7 / 12.6 | – |
| 5× own | postgres | 1,885.5 / 1,904.9 | 59.3 / 65.3 | 18 / 20.2 | 1,662.1 / 1,690 | 406.5 / 421.1 | 1,246.1 / 1,262.5 (99.6) | 89.8 / 94.8 | 5.7 / 6.7 | 20 / 20.8 |
| 5× own | sqlite | 2,055.8 / 2,274.7 | 35.5 / 40.6 | 17 / 18.9 | 1,824 / 2,035.4 | 431.3 / 507 | 1,385.7 / 1,525.1 (109.6) | 98.8 / 111.1 | 47.4 / 50.3 | – |
| 15× own | postgres | 5,912.9 / 5,944 | 171.1 / 176.2 | 56.5 / 60.7 | 5,148.9 / 5,170.1 | 1,248.4 / 1,367.2 | 3,794.9 / 3,943.7 (310.5) | 393 / 410.9 | 6.9 / 17.2 | 53.2 / 56.8 |
| 15× own | sqlite | 6,533.2 / 6,782.1 | 126.1 / 127.8 | 52 / 53.8 | 5,697.1 / 5,962.4 | 1,362.1 / 1,452 | 4,249.7 / 4,487.7 (328.6) | 454.8 / 470.5 | 133.5 / 134.4 | – |
| 30× own | postgres | 12,639 / 13,294.1 | 463.8 / 503.8 | 123.4 / 125.9 | 10,538.3 / 11,168.4 | 2,469.1 / 2,682.1 | 8,042.7 / 8,442.8 (754.3) | 1,142.3 / 1,260.6 | 10.7 / 12 | 105.6 / 107 |
| 30× own | sqlite | 12,723.7 / 13,348.2 | 258.6 / 288.8 | 103.7 / 126.3 | 10,780.3 / 11,157 | 2,404.8 / 2,451.1 | 8,332 / 8,652.7 (691.6) | 1,240.7 / 1,375.4 | 260.9 / 299.8 | – |
| 15× shared | postgres | 6,654.9 / 9,519.6 | 202.5 / 315.8 | 43.9 / 61.3 | 5,673.3 / 8,579 | 970.8 / 1,821.2 | 4,581.9 / 6,722 (259.7) | 362.7 / 457 | 6.1 / 8.3 | 57.6 / 113.9 |
| 15× shared | sqlite | 4,624.3 / 4,654.6 | 67.6 / 75.8 | 30.4 / 32.1 | 4,144.1 / 4,195.3 | 801.9 / 877.8 | 3,328.5 / 3,372.9 (228.3) | 243.8 / 261 | 80.7 / 85.8 | – |

### Commit, scoped (median / p95 ms)

| Scale | Engine | Total | Load | from_snapshot | validate() | of which themes | of which instances (expand) | Scoped check | Persist | Cache fill |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1× own | postgres | 619.2 / 658.6 | 11.5 / 12.2 | 3.1 / 3.2 | 309.5 / 323.4 | 77.9 / 79.4 | 230.6 / 242.7 (18.7) | 271 / 292.7 | 2.5 / 5.4 | 3.8 / 4 |
| 1× own | sqlite | 735.9 / 766.6 | 7.2 / 7.6 | 3.5 / 3.6 | 363.6 / 383 | 90.6 / 95 | 275 / 288.5 (22.6) | 328.5 / 340.8 | 12.3 / 12.8 | – |
| 5× own | postgres | 3,371 / 3,728 | 63.7 / 76.9 | 19.6 / 20.9 | 1,672 / 1,878.3 | 433.3 / 461.8 | 1,251.1 / 1,408.5 (105.2) | 1,461.6 / 1,613.3 | 5.5 / 6.4 | 21.8 / 22.9 |
| 5× own | sqlite | 4,791.2 / 6,072.6 | 49.4 / 52.8 | 22.5 / 24.8 | 2,515.1 / 3,276.2 | 584 / 650.8 | 1,898.8 / 2,615.4 (140.7) | 1,981 / 2,502.5 | 58.6 / 63.7 | – |
| 15× own | postgres | 11,782.7 / 13,677.6 | 228.5 / 271.2 | 60.9 / 69.5 | 5,296.7 / 6,044.1 | 1,307.6 / 1,755.4 | 3,973.4 / 4,551.3 (325.2) | 5,439.9 / 6,665.4 | 7.9 / 13.2 | 56.7 / 63.7 |
| 15× own | sqlite | 15,025.1 / 24,125.3 | 144 / 232.5 | 60.7 / 108.4 | 6,446 / 11,206.8 | 1,610.8 / 2,750.1 | 5,100.3 / 8,979.5 (354.4) | 5,863.3 / 11,547.5 | 160.6 / 215.9 | – |
| 30× own | postgres | 34,121.9 / 55,458.4 | 593.8 / 1,019.4 | 142.6 / 338.5 | 11,770.1 / 24,307.3 | 2,833.2 / 4,965.4 | 8,862.2 / 19,244.9 (832.3) | 18,361.6 / 28,157.3 | 10 / 14.2 | 154.1 / 385.8 |
| 30× own | sqlite | 38,871.9 / 40,419.7 | 279.2 / 316.2 | 120.2 / 264.4 | 16,073.3 / 16,978.1 | 3,901.2 / 4,205.8 | 12,320 / 13,396.9 (904.9) | 18,977.1 / 21,337.2 | 423.7 / 479.7 | – |
| 15× shared | postgres | 8,360.4 / 18,265.3 | 186.3 / 452.4 | 37.7 / 76.1 | 4,050.3 / 8,871.5 | 771.5 / 1,505.4 | 3,264.6 / 7,339.5 (222.1) | 3,832.9 / 8,307.2 | 10.3 / 15 | 101.5 / 112.6 |
| 15× shared | sqlite | 8,998.4 / 10,291.8 | 76.8 / 101.1 | 30.6 / 37.3 | 4,400.6 / 5,365.4 | 802.6 / 980.6 | 3,592.8 / 4,462.8 (226.2) | 3,845.5 / 4,348.2 | 81.1 / 106.8 | – |

### Reads (median / p95 ms)

| Scale | Engine | expand() whole project | find_nodes name, many | find_nodes name, none | find_nodes text | get_outline page | get_design_context frame |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1× own | postgres | 17.3 / 19.2 | 74 / 77.5 | 40.3 / 41.1 | 42.3 / 43.2 | 31 / 33.2 | 765 / 787.2 |
| 1× own | sqlite | 20.8 / 24.1 | 82 / 86.1 | 45.4 / 47.6 | 51.6 / 52.1 | 39 / 39.5 | 788.8 / 876.2 |
| 5× own | postgres | 110.5 / 125.1 | 331.5 / 375.1 | 176.6 / 182.1 | 184 / 192.9 | 138.4 / 140 | 3,952.3 / 4,086.1 |
| 5× own | sqlite | 116.1 / 142.8 | 390.8 / 416.3 | 214.5 / 217.9 | 234.5 / 239.3 | 192.4 / 200.3 | 4,836.2 / 4,925 |
| 15× own | postgres | 341.4 / 375.5 | 1,188.7 / 1,239.5 | 709.3 / 746.3 | 839 / 865.8 | 611.5 / 702.2 | 13,368.1 / 13,552.9 |
| 15× own | sqlite | 690.7 / 739 | 2,029.6 / 2,207.5 | 648.1 / 949.9 | 757.8 / 761.1 | 974.1 / 1,136.2 | 14,132.2 / 21,383 |
| 30× own | postgres | 622.1 / 804.2 | 2,423.9 / 2,497.4 | 1,416.1 / 1,517.7 | 1,536.9 / 1,555.2 | 1,233 / 1,280.8 | 30,426.1 / 34,675.8 |
| 30× own | sqlite | 840.6 / 890.5 | 2,774.6 / 3,074.7 | 1,454.7 / 1,541.2 | 2,170.5 / 2,777.5 | 1,824 / 2,529.8 | 42,608.6 / 46,357.8 |
| 15× shared | postgres | 525.7 / 538.9 | 1,129.3 / 1,440 | 581 / 749.2 | 635 / 726.3 | 400.9 / 585.2 | 12,463.4 / 15,387.9 |
| 15× shared | sqlite | 315.6 / 413.8 | 1,008.1 / 1,669.5 | 550.7 / 677.5 | 800.7 / 946.2 | 480 / 619.6 | 12,290.1 / 22,192.1 |

### Viewer cost in Node and peak RSS

| Scale | Engine | JSON.parse ms | snapshotNodeIndex ms | Retained heap MiB | Peak RSS MiB | RSS after seed MiB |
| --- | --- | --- | --- | --- | --- | --- |
| 1× own | postgres | – | – | – | 420.7 | 115 |
| 1× own | sqlite | 8.9 / 10.2 | 2.1 / 3.3 | 4.1 | 398.3 | 106 |
| 5× own | postgres | – | – | – | 2,587.8 | 452 |
| 5× own | sqlite | 34.8 / 37.9 | 9.2 / 13.7 | 20.3 | 1,991.5 | 437 |
| 15× own | postgres | – | – | – | 4,420.5 | 1,132 |
| 15× own | sqlite | 94.3 / 102 | 41 / 51.1 | 59.2 | 4,560.7 | 1,102 |
| 30× own | postgres | – | – | – | 5,899.6 | 2,531 |
| 30× own | sqlite | 203.6 / 220.1 | 59.1 / 59.3 | 118.6 | 5,875.2 | 2,164 |
| 15× shared | postgres | – | – | – | 3,644.1 | 980 |
| 15× shared | sqlite | 56.9 / 58.8 | 16.8 / 28 | 37.9 | 3,596.7 | 964 |

Every MCP read returned a result (no tool errors) at every scale once the note cap
was raised. The outline and design-context responses were the same size at every
scale (72 KB and 92 KB): the work grows with the project, the answer does not.

## Which costs grow fastest (1× → 30×, nodes ×30)

| Cost | SQLite | PostgreSQL | Growth |
| --- | ---: | ---: | --- |
| Scoped commit | 736 → 38,872 ms (×53) | 619 → 34,122 ms (×55) | Super-linear; `scoped.check` alone ×58 |
| `get_design_context`, one frame | 789 → 42,609 ms (×54) | 765 → 30,426 ms (×40) | Super-linear |
| `get_outline`, one page | 39 → 1,824 ms (×47) | 31 → 1,233 ms (×40) | Super-linear |
| Strict commit | 405 → 12,724 ms (×31) | 377 → 12,639 ms (×34) | Linear in expanded nodes |
| `find_nodes`, full scan | 45 → 1,455 ms (×32) | 40 → 1,416 ms (×35) | Linear |
| Storage cold load | 6.7 → 226 ms (×34) | 9.2 → 356 ms (×39) | Linear |
| GET /snapshot (cold) | 21 → 692 ms (×33) | 11 → 731 ms (×65*) | Linear; *PG 1× was a Moka hit |
| Body / viewer parse + index | 1.6 → 48 MiB; 11 → 263 ms | same bytes | Linear in authored nodes |
| Peak RSS (process) | 0.4 → 5.9 GB | 0.4 → 5.9 GB | Linear |

### Top three cost drivers

1. **Whole-project validation on every commit.** `validate()` is 83–90 % of a
   strict commit at every scale and both engines (10.5–10.8 s at 30×). Of that,
   `validate_instances` is 74–81 %: it re-checks the properties of every virtual
   node under every theme (599k expanded × 2 themes at 30×) and walks override
   ancestry. `instances::expand` itself is only ~6 % of validate (0.7 s at 30×).
   The per-theme full clone + content check is the next 22–25 %. The shared-master
   variant points at the **expanded** count: with 35 % fewer authored nodes but only
   19 % fewer expanded nodes, the SQLite strict commit fell 29 % (the PostgreSQL
   shared cell was too noisy to compare: median 6.7 s, p95 9.5 s).
2. **The scoped guard costs another full pass.** `scoped.check` derives the current
   and candidate dependency closures over the whole expanded project (font
   resolution under every theme included): 0.3 s at 1×, 18–19 s at 30×. A scoped
   commit is 1.3–3.1× slower than a strict one and passes the 30 s production
   PostgreSQL operation timeout at 30× (median 34 s). Page-scoped concurrency today
   buys correctness for parallel agents, not speed.
3. **Page- and frame-scoped reads do whole-project work.** Every MCP read clones the
   full snapshot (`AppState::snapshot`), and tree/find reads expand the whole
   project. `get_design_context` for one frame additionally runs a full
   `Candidate::validate()` and resolves the props of every expanded node in the
   project (`resolved_expanded_nodes`) before selecting the frame: 30–43 s at 30×
   for an unchanged 92 KB answer. Its cost tracks expanded nodes (shared 15×: 12.3 s
   vs 13.4–14.1 s own 15×).

Other findings:

- **16 MiB cache admission limit.** Moka and the HTTP response cache both refuse
  entries above 16 MiB of JSON, which this design reaches at ≈ 65–70k authored nodes
  (15× shared, 65.8k: 15.2 MiB, admitted; 15× own: 24.1 MiB, refused). Above it every
  GET re-encodes (repeat GET 2–6 ms → 0.4–0.8 s), and "Moka warm" PostgreSQL reads
  cost a probe plus a full SQL load (233 ms at 15×, slower than the uncached backend).
- **History envelope** (`scoped::envelope`, a footprint diff over the whole project)
  takes 1.1–1.2 s per commit at 30×, about 10 % of a strict commit.
- **SQLite persist** rebuilds `Candidate::from_snapshot(before)` and diffs every row
  (261 ms at 30×); PostgreSQL persists the row delta in 3–11 ms at every scale.
  The in-writer `load` is 259 ms (SQLite) and 464 ms (PostgreSQL) at 30×.
- **Viewer.** The browser downloads the uncompressed object body: 48 MiB at 30×
  (1.9 MiB if gzipped), then spends 204 ms in `JSON.parse` and 59 ms building the
  node index in Node, retaining 119 MiB of heap, for one visible page.
- **Memory.** Peak RSS reaches 5.9 GB per process at 30×, and 2.2–2.5 GB of it is
  already resident after the harness built and seeded the fixture. Validation holds
  several whole-project copies at once (candidate, per-theme clone, placed expanded
  candidate).

## Implications

**Phase 1 — page-scoped reads.** The largest read win is `get_design_context`:
validate once at commit time (the stored revision is already valid) and resolve only
the frame's closure instead of `resolved_expanded_nodes` over the project; that
removes 30–43 s per call at 200k. `get_outline(page)`, `get_subtree` and
`find_nodes` must expand only the page or closure being read and stop cloning the
whole snapshot per read (share the `Arc` the backends already hold). A per-page
`GET` makes the viewer cost independent of project size: at 30× the average page is
~86 KB of JSON versus 48 MiB. Per-page cache entries are required anyway, because a
whole-project entry stops fitting the 16 MiB limits at ~65k authored nodes. Add
response compression for the remaining full snapshot (×25 smaller here).

**Phase 2 — page-partitioned storage and validation.** Raising the cap without
partitioned validation gives 12.6–12.7 s strict and 34–39 s scoped commits at 200k
authored nodes. Phase 2 has to:

- validate only the changed pages and their closure, with the virtual-node property
  checks restricted to instances whose master or overrides changed (the 74–81 %
  share above), and per-theme checks on the changed nodes instead of a full clone;
- derive scoped guards from stored per-page dependency and instance-usage indexes
  instead of recomputing both closures over the whole expanded project;
- compute the history footprint from the changed IDs, load only guarded pages, and
  write SQLite rows from the changed set instead of diffing the whole project;
- move the per-project caps together: nodes (authored and expanded), notes
  (5,000 today) and the bundle preflight, while keeping a per-page expanded bound.

Even with those, keep the 600k expanded nodes of a 200k-authored project out of any
single request: here expanded = 3× authored, so a per-page expanded cap of 20,000
is ~6.7k authored nodes per page.

## Skipped or limited

- **Chromium heap** was not measured; the viewer cost is Node (V8) parse, index and
  retained heap, without rendering. Konva/canvas rendering cost is not included.
- **`axis-courses`** (second source) was not run.
- **Commits through MCP** (`apply_batch` with events and render enqueue) and
  `get_design_context` with `viewports` (render jobs) are outside the measured
  boundary.
- **PostgreSQL timeouts** were raised to 600 s; with the production 30 s operation
  timeout the 30× scoped commit (median 34 s, run inside the pool worker) would fail
  instead of reporting a time. Read-side validation runs in the server, outside it.
- **One run** per cell on a laptop with Docker Desktop; p95 spreads at 15×/30×
  (for example SQLite scoped 15×: 15.0 s median, 24.1 s p95) show memory pressure
  and noise. PostgreSQL image digest and SQLite library version were not recorded.
- **Peak RSS** includes the harness (source copy and scaled fixture); it is an upper
  bound for a server process holding one such project.
