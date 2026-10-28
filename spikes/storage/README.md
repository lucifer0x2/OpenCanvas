# Storage spike — predeclared protocol

Protocol recorded before the first benchmark run on 2026-10-28.
Hardware recorded before run: Apple M5, Mac17,3, 10 physical CPU cores,
24 GiB RAM, macOS 26.4 (25E246), local temporary filesystem.
Run `cargo run --release -p opencanvas-storage --example storage_spike`.
The program uses only a temporary data root, removed on completion.

Small fixture: 100 authored nodes. Large fixture: 5,000 authored nodes.
Each has one page, two fixed frames, rectangle/text leaves, Vietnamese text,
literal token and primitive style. All batches use the real candidate validator,
row persistence, commit log and idempotency transaction.

Latency budgets, chosen before measurement: single-client p95 <= 100 ms small /
500 ms large per 10-op create, 10-op property update, and 10-op reparent batch;
8-client successful batch p95 <= 2,000 ms; queue wait p95 <= 1,500 ms.
These are local interactive response targets, not guarantees for other hardware.
Read snapshot p95 <= 500 ms large. Each contention configuration runs 1, 4, and
8 writer clients plus a snapshot reader. Clients deliberately use an optimistic
revision and count conflicts, then retry after readback. No automatic merge.

Measure wall latency, queue wait, candidate read/validate/serialize, SQL row write,
commit/fsync separately. Report logical request bytes and actual DB/WAL bytes;
filesystem bytes are allocation/growth, not physical device writes. macOS `time -l`
provides whole-process CPU/peak RSS. Reader correctness asserts coherent snapshots.
Hold a read transaction while writing to demonstrate WAL checkpoint pinning, then
end it and checkpoint to demonstrate progress. No forced per-batch checkpoint.

Hardware and actual results will be appended after execution; unmeasured metrics
will be labeled explicitly. The bundled runtime must report SQLite >= 3.51.3.

## Measured result — 2026-10-28

Decision: retain SQLite WAL/FULL for MVP. All declared latency budgets passed on
this machine. Runtime reported **SQLite 3.54.1** (rusqlite 0.41.0, bundled
libsqlite3-sys 0.39.1 in Cargo.lock); no system SQLite dependency.

Commands actually run:

```sh
cargo test -p opencanvas-storage --features fault-injection -- --test-threads=2
cargo build --release -p opencanvas-storage --example storage_spike
/usr/bin/time -l target/release/examples/storage_spike > spikes/storage/results.json 2> spikes/storage/resource-usage.txt
```

Tests: **17 passed, 0 failed**, eight suites including documentation/unit harnesses,
2.17 seconds on the final targeted run, including ordered mutual-style-alias and
legacy alias-cycle rejection without recursion or partial commit. The first benchmark completed all assertions
and emitted valid `results.json`, but the sandbox denied `time`'s `kern.clockrate`
sysctl, so its wrapper exited 1 after printing CPU times. A second identical binary
run with resource-accounting permission exited 0; its JSON is
`results-resource-run.json` and resource accounting is `resource-usage.txt`.
Primary latency tables below use the first run; the second is retained, not
silently substituted. Each run uses newly created temporary projects, then removes
them. No home-directory projects or demo data were created.

Hardware: Apple M5, Mac17,3, 10 physical cores, 24 GiB RAM; macOS 26.4 (25E246).
Release optimization, local temporary filesystem, no compilation in measured
process time. Whole-process resource run: **13.61 s wall, 19.47 s user CPU,
0.96 s system CPU, 91,144,192 bytes peak RSS (86.9 MiB)** and 44,237,256-byte
peak memory footprint as reported by macOS time. CPU includes fixture generation,
cleanup batches, conflicts, readers, raw-engine baseline and both fixture sizes;
per-operation CPU/RSS attribution was not measured. Zero block-output operations
from macOS time is not a physical-write measurement and is not interpreted as one.

Each sequential case has 30 measured batches of 10 operations. Create is followed
by an untimed 10-op subtree-delete cleanup, keeping authored node count at 100 or
5,000. The large fixture is a project-scale storage workload, not a promise to
render a 2,500-leaf frame under the renderer's separate 2,048-node scene cap.

| Nodes | Operation | Total p50 / p95 ms | Read+JSON parse p95 ms | Candidate validation p95 ms | SQL+row serialization p95 ms | Commit p95 ms |
| ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 100 | create | 5.828 / 6.827 | 0.445 | 0.959 | 0.386 | 4.701 |
| 100 | update | 4.911 / 5.002 | 0.247 | 0.377 | 0.236 | 4.208 |
| 100 | reparent | 4.861 / 5.238 | 0.263 | 0.486 | 0.256 | 4.089 |
| 5,000 | create | 16.492 / 17.776 | 2.550 | 7.325 | 2.092 | 5.268 |
| 5,000 | update | 13.631 / 13.842 | 2.531 | 4.931 | 1.956 | 3.940 |
| 5,000 | reparent | 15.638 / 16.728 | 2.534 | 7.083 | 2.005 | 4.583 |

The stages are independently timed; their p95 values must not be added as though
the same sample produced every percentile. Total also includes path checks,
canonical request serialization/hash and unclassified Rust bookkeeping.

Each contention client commits 20 ten-node update batches while a viewer reads
every ~5 ms plus snapshot/serialization cost. Conflicts read back and retry with a
new expected revision; no mutation is silently merged. Latency and queue columns
are for **successful attempts only**. Conflict counts are full run totals. Time
spent in failed attempts, readback, starvation/fairness and the logical request's
retry-to-success latency are **not included** in those percentiles and were not
separately measured. The writer mutex is serial, not a fair FIFO scheduler.

| Nodes | Clients | Successes | Conflicts | Batch p50 / p95 ms | Queue p95 ms | Viewer snapshot p95 ms | Viewer JSON serialization p95 ms |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 100 | 1 | 20 | 0 | 4.452 / 5.221 | 0.000 | 1.301 | 0.066 |
| 100 | 4 | 80 | 120 | 4.593 / 5.802 | 0.000 | 1.350 | 0.089 |
| 100 | 8 | 160 | 780 | 4.370 / 6.137 | 1.155 | 1.384 | 0.047 |
| 5,000 | 1 | 20 | 0 | 15.962 / 21.974 | 0.000 | 5.716 | 1.049 |
| 5,000 | 4 | 80 | 120 | 19.074 / 25.374 | 8.587 | 5.391 | 0.931 |
| 5,000 | 8 | 160 | 560 | 27.836 / 45.421 | 27.633 | 5.705 | 1.035 |

Raw minimal-engine baseline (10 rows, ~128-byte values, 30 batches, same
WAL/FULL/fullfsync) had commit p50/p95 **3.952/4.892 ms** small and
**3.969/4.248 ms** large; SQL-only write p95 0.084/0.018 ms. This isolates a lower
bound on engine overhead; it is not equivalent to a validated project mutation.
The project read/clone/validation/diff cost remains visible above and grows with
the whole materialized project, even when only 10 nodes change. No document-blob
replacement or alternate storage engine was introduced to hide that cost.

Measured canonical request bytes for sequential create/update/reparent:
45,365/27,070/24,851 small; 45,370/27,071/24,860 large. Contention request bytes
for successful 1/4/8-client batches: 18,330/73,320/146,640 at either fixture size.
These exclude retries, untimed cleanup and seeding. Filesystem apparent sizes
after workload: small main 684,032 B + WAL 4,663,872 B + SHM 32,768 B; large main
2,883,584 B + WAL 4,647,392 B + SHM 32,768 B. Commit/idempotency history contributes
to main-file growth. Physical-device bytes written and write amplification were
not measured; JSON/request bytes and WAL allocation are the available proxies.

Checkpoint experiment pins a real read transaction, commits 20 updates, then
releases the reader. Small checkpoint made progress only to 918/1,132 WAL pages
while pinned, then 1,132/1,132 after release. Large: **728/1,128 -> 1,128/1,128**.
The reader still observed its original revision throughout all 20 commits.
WAL apparent size need not shrink after PASSIVE checkpoint; pages become reusable.
Policy is 1,000-page autocheckpoint, 16-MiB journal-size target, and explicit
PASSIVE checkpoint. The size target is not a hard cap while readers pin history;
do not keep read transactions across SSE/render/network work.

Correctness asserted during the measured run: snapshots have monotonic revisions
and all ten marker nodes come from the same commit; fixtures retain exact node
counts, writer conflict handling succeeds, pinned reader stays consistent, and
checkpoint completes after reader release. Targeted tests additionally establish
no partial invalid batch, no double revision on replay, key reuse rejection,
ordered reparent/reorder/delete, cycle/type/ref/layout rejection, restart durability,
root lock, slug/symlink containment, safe errors and asset hash verification.
Child-process recovery covers abrupt exit before/after commit, lost-response retry,
and asset publish/metadata boundaries. This is process failure testing, not a
power-cut test. Neither raw process RSS nor p95 from 20–160 samples establishes a
production saturation limit. Revisit whole-project validation and writer fairness
before claiming support for substantially larger/higher-contention projects.
