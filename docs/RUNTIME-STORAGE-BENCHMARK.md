# Task19b — Runtime storage and snapshot GET, one sequential writer

Runs A and B measured the Redis/bitcode implementation of Task19.
[Run C](#results--release-run-c-with-moka-2026-11-03) measured the
[PostgreSQL snapshot cache](MOKA-SNAPSHOT-CACHE.md) before the HTTP response cache.
[Run D](#results--release-run-d-with-http-response-cache-2026-11-03) measures
the HTTP response cache and both node formats.

This measures the Task19 product adapters, not the simplified scalar database
spike. SQLite is still the default CLI engine. No production engine, dependency,
service or user data is changed by this benchmark.

## Run command

Use the same command for the current SQLite, PostgreSQL and PostgreSQL+Moka
fixtures. Runs A and B used it on the historical source recorded below.

```sh
bash scripts/task19-bench.sh
```

The wrapper prints an OS temporary output directory containing `samples.json`,
`metadata.txt` and `run.log`. Samples and service credentials are not versioned.
It builds only the release integration-test target, with two Cargo jobs, before
provisioning fresh services through `task19-check.sh`. The existing runner owns
the random loopback ports, private credentials and EXIT cleanup. SQLite uses a
temporary data root; HTTP uses a real ephemeral listener and AppState/router.
Ports3847/3848 and `~/.opencanvas` are not used.

PostgreSQL is capped at 2 CPUs and 2 GiB with a pool of four connections. The
metadata records the image digest, limits, Docker host, toolchain, HEAD/branch
and matching before/after source fingerprints. Runs A and B also used Redis
with 1 CPU, 256 MiB, a 64 MiB `allkeys-lru` limit, 100 ms network budget, 300 s
TTL and 16 MiB object limit.

## Current HTTP workload and timers

Each backend is measured at the same final revision for the default object
representation and the opt-in `?nodeFormat=rows` representation. The runner
uses one real listener and one reqwest client per cell, pre-fills the serialized
HTTP cache for both formats, then verifies the reconstructed rows and the
default response against the authored snapshot. Exact JSON object comparisons
distinguish absent fields from explicit nulls. Each format has a separate ETag,
and sending the other format's ETag returns the selected format with status 200.

The HTTP groups run in order from default 200 through default 304, rows 200
and rows 304, with 20 warmups and 100 samples each. The 200 groups record time
through complete body receipt, JSON parse time and total time through parse. The
304 groups record time from request through the empty response body and verify
the echoed ETag. Raw `samples.json` schema 3 stores the groups under `http.full`
and `http.rows`, with each 304 group under `notModified`. The byte counts are
response payload bytes. Token counts and the cost to reconstruct objects on a
rows client are outside the timers.

The backend's typed Moka snapshot cache and the server's serialized HTTP cache
are separate. `storageHotProof` checks the former on direct PG+Moka reads.
HTTP `hotProof` checks both PostgreSQL cells, with one authoritative identity
probe per request and zero SQL entity reads across the 120 calls per warmed
group. Thus, a PostgreSQL-only cell can still hit the HTTP cache. The warm HTTP
200 group in this harness includes that cache, which changes the measured path
from runs A–C.

## Historical workload and timer boundaries

The fixture has exactly100 or5,000 authored nodes: one page, one fixed frame,
rectangles and one simple bundled-font text node for every tenth element index.
All engines receive the same operation sequence in batches of at most500ops.
The project name is identical, engine-specific slugs have the same byte length
and generated IDs have the same width. Full snapshots must match after replacing
only projectId and slug. Canonical snapshot hashes and actual JSON byte counts
are recorded per cell; font metadata remains included.

There is one sequential writer, no concurrent read load and no MCP writer or
render job. Each of20warmup and100measured calls changes x on exactly three
rectangle nodes, uses a unique idempotency key and advances revision once.
Request construction and harness assertions are outside the timer. The timer
encloses `ProjectHandle::apply_batch` through `Storage`, including product
validation, serialization, driver/pool work and successful durable commit.

After all writes, reads remain at a fixed final revision. A setup snapshot
fills the optional Redis cache before the HOT cells. Twenty warmup and100sample
calls are measured separately for each of:

- `storageSnapshot`: complete blocking `ProjectHandle::snapshot` call. Direct
  groups run in one spawn_blocking task; scheduling that group is excluded.
- `http.receiveBody`: request start through receipt of the complete body from a
  real HTTP GET snapshot, including existing server spawn_blocking boundaries,
  authoritative probe/read, serialization and loopback transport. A single
  reqwest async client is reused, with default connection reuse and30s timeout.
- `http.parseJson`: parsing that complete body into `serde_json::Value`.
- `http.total`: request start through completion of JSON parsing, directly
  timed on every request. It is not a sum of percentile summaries.

Every read is compared with the entire expected snapshot outside its timer.
HTTP also verifies its `truncated:false` envelope field and consistent body
length. Revision/history, atomic three-node changes and exact original replay
after later revisions are checked. Replay must leave the full history and
snapshot unchanged. Any error/assertion aborts the run; six completed cells are
required for a successful run. Completed sample groups report failures0.

For each Redis HOT group, actual isolated Redis `cmdstat_get` must increase
exactly120, `cmdstat_set` must increase0, and all returned snapshots must match.
These counters include warmup. GET is invoked inside the cache's Lua script;
the INFO control commands are outside timings. No direct GET is used by the
harness. STRLEN and MEMORY USAGE compare the current binary envelope with the
same snapshot's JSON representation. They do not measure compression of an
unrelated fixture. Initial cold/miss setup is excluded; no cold latency is
claimed.

Quantiles use nearest rank: sorted[ceil(p*N)-1]. Each distribution has100samples;
the reported p99 is an observation from this run, not a service SLA. Runs are
reported separately, with all samples retained and no outlier removal.

## Results — release run A, 2026-11-03

The complete run passed: six cells, each with100measured samples after20warmup,
zero failures, test duration41.99s. Source fingerprints matched before/after:
`f48113d489c6ce5114b7727792132c4b21b1cebb9cb02594ca899c01a502106d`.
Raw evidence is the temporary directory
`/var/folders/pw/yxvtwnf517qchxqynh2w7pz80000gn/T/opencanvas-task19-bench.uFBWbN/`.
This path is disposable; use the command above to generate new evidence.

Each table cell is **p50 / p95 / p99 in milliseconds**. PG+Redis means confirmed
HOT reads, not cache-only reads: each still probes PostgreSQL authority.

| Nodes | Backend | Commit | Storage snapshot | HTTP total including parse |
| ---: | --- | ---: | ---: | ---: |
|100|SQLite|6.022 /7.146 /8.030|0.332 /0.575 /0.643|0.434 /0.465 /0.480|
|100|PostgreSQL|7.024 /9.927 /15.329|0.757 /1.045 /1.139|1.232 /3.498 /5.227|
|100|PG+Redis|3.273 /4.074 /15.696|0.704 /0.778 /0.875|1.424 /1.687 /1.796|
|5,000|SQLite|18.989 /20.179 /21.650|2.879 /3.186 /3.673|11.495 /14.418 /14.631|
|5,000|PostgreSQL|30.591 /34.104 /36.559|8.169 /14.073 /14.694|30.451 /39.175 /40.830|
|5,000|PG+Redis|47.564 /70.037 /70.799|16.997 /21.503 /24.661|27.759 /30.828 /31.764|

The independently timed HTTP phases below explain which boundary is reported;
their percentile values must not be added to obtain total percentiles.

| Nodes | Backend | Request through complete body | Client JSON parse |
| ---: | --- | ---: | ---: |
|100|SQLite|0.374 /0.405 /0.418|0.059 /0.062 /0.064|
|100|PostgreSQL|1.167 /3.422 /5.170|0.058 /0.068 /0.079|
|100|PG+Redis|1.358 /1.631 /1.737|0.062 /0.075 /0.076|
|5,000|SQLite|8.719 /10.783 /10.921|2.803 /3.631 /3.700|
|5,000|PostgreSQL|24.872 /32.083 /33.890|5.828 /7.471 /8.031|
|5,000|PG+Redis|23.530 /26.637 /27.106|4.238 /4.802 /5.097|

For100nodes the seed revision is1 and final revision121; for5,000nodes the seed
revision is10 and final revision130. Canonical snapshot SHA256 agrees across all
three engines within each fixture:100nodes
`0a6e4dcdc3cc13ebe7daa8792973bb84e8b2ed5f3fffe1410d5a9a85fb67cb6a`;
5,000nodes `64cccacf8394322f5f8d50003de57b774bf1c21a3dbcda9f66724e37b38c49bd`.

| Nodes | Snapshot JSON bytes, each engine | HTTP body bytes, each engine | Redis STRLEN | MEMORY USAGE | STRLEN/JSON | Memory/JSON |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
|100|17,307|17,325|16,526|20,712|0.955|1.197|
|5,000|861,742|861,760|799,810|917,736|0.928|1.065|

Thus the current bitcode envelope is only4.5%/7.2% smaller than the same snapshot
JSON for these fixtures; Redis's reported key/value allocation is19.7%/6.5%
larger than JSON bytes. MEMORY USAGE is per-key accounting, not total Redis RSS.
The100node direct/HTTP HOT GET deltas were120/120 and SET deltas0/0. The5,000node
direct/HTTP HOT deltas were also120/120 and0/0. Entire snapshot comparisons passed
for all480HOT reads including warmup, so these are not silent-miss measurements.

SQLite was fastest in all snapshot/HTTP p50 cells of run A. For5,000nodes the
confirmed HOT storage read regressed from8.169ms PG to16.997ms PG+Redis, while
HTTP total p50 moved from30.451ms to27.759ms. At100nodes HOT storage p50 moved
from0.757ms to0.704ms, while HTTP total regressed from1.232ms to1.424ms. These
complete-call observations do not establish a general cache speedup. The large
differences between PG write groups despite Redis-free writes, and variation in
client parse time for equal-sized bodies, show why run order/noise and an
independent repeat matter. No unmeasured phase is declared the bottleneck.

## Build and environment of run A

- Release optimized integration test, default Cargo release profile,2build jobs;
  Rust1.99.1 (`58cf8f9c3`, LLVM23.1.1), Cargo1.99.1, native Darwin arm64.
- HEAD `438254936823cb4ef8639d85c1404302c608f7f7`, dirty branch
  `claude/design-properties`; the source fingerprint above identifies the
  measured uncommitted source, rather than HEAD alone.
- SQLite runtime3.54.1. Docker host Linux aarch64,10CPUs and12,600,156,160bytes RAM.
  Per-container limits are the caps described above, not dedicated CPU cores.
- PostgreSQL16.15, `postgres:16-alpine`, digest
  `sha256:0cefaabd882e42bcb2a2f3dc56d3ce4770560ba62fd7f197f1a2e008fd5731ab`.
- Redis7.4.12, `redis:7-alpine`, digest
  `sha256:8da19c7399dca900c6822a1cf30f9d2285585e2c4b457ff4095b0f4500054d3f`.

Targeted release compile, strict release Clippy for this test, rustfmt check,
Bash syntax check and diff whitespace check passed. No full correctness suite or
frontend build was rerun. The wrapper exited0, and the Docker task19 inventory
was empty after cleanup. New harness modules are at most142lines each; the
wrapper is57lines, with separate fixture, direct calls, HTTP, Redis proof and
statistics responsibilities. No size exception is required.

## Results — release run B, 2026-11-03

The second run used the same source fingerprint, image digests, resource
limits, fixture order and release profile. All six cells completed in 41.61 s,
each with 20 warmup calls and 100 measured calls per group, zero failures.
Raw samples, metadata and the successful test log are in the temporary directory
`/var/folders/pw/yxvtwnf517qchxqynh2w7pz80000gn/T/opencanvas-task19-bench.UDn4nc/`.

Each cell shows **p50 / p95 / p99 in milliseconds**.

| Nodes | Backend | Commit | Storage snapshot | HTTP total including parse |
| ---: | --- | ---: | ---: | ---: |
| 100 | SQLite | 5.003 / 7.018 / 7.173 | 0.329 / 0.468 / 0.533 | 0.465 / 0.489 / 0.524 |
| 100 | PostgreSQL | 3.900 / 5.078 / 5.641 | 0.725 / 0.770 / 0.774 | 0.934 / 1.122 / 1.191 |
| 100 | PG+Redis | 3.015 / 3.188 / 3.254 | 0.699 / 0.749 / 0.754 | 0.988 / 1.064 / 1.190 |
| 5,000 | SQLite | 18.909 / 19.191 / 23.007 | 2.807 / 2.947 / 2.981 | 12.299 / 13.195 / 13.287 |
| 5,000 | PostgreSQL | 31.458 / 37.334 / 39.676 | 6.133 / 8.841 / 9.648 | 34.904 / 40.686 / 45.376 |
| 5,000 | PG+Redis | 54.507 / 66.636 / 80.838 | 14.697 / 17.366 / 18.059 | 34.166 / 39.954 / 41.997 |

| Nodes | Backend | Request through complete body | Client JSON parse |
| ---: | --- | ---: | ---: |
| 100 | SQLite | 0.404 / 0.429 / 0.461 | 0.060 / 0.063 / 0.069 |
| 100 | PostgreSQL | 0.879 / 1.064 / 1.133 | 0.054 / 0.062 / 0.065 |
| 100 | PG+Redis | 0.933 / 1.004 / 1.136 | 0.056 / 0.060 / 0.063 |
| 5,000 | SQLite | 9.159 / 9.882 / 9.938 | 3.136 / 3.382 / 3.406 |
| 5,000 | PostgreSQL | 27.771 / 32.640 / 34.976 | 6.964 / 8.773 / 11.370 |
| 5,000 | PG+Redis | 28.771 / 34.594 / 36.258 | 5.282 / 5.957 / 6.409 |

Independent verification recomputed every percentile from both raw files and
matched the measured source fingerprint to the current tree. Canonical snapshot
hashes, revision ranges and representation sizes matched across runs. In each
run the four Redis HOT groups each recorded 120 GETs and zero SETs, with exact
snapshot comparisons after each call. The Task19 Docker inventory was empty.

Across both runs SQLite had the lowest snapshot and HTTP median at both fixture
sizes. At 5,000 nodes, Redis HOT storage snapshots took 2.08× and 2.40× the
PostgreSQL-only median. HTTP total medians fell by 2.69 ms in run A and 0.74 ms
in run B. In run B, time through receipt of the body actually rose by 1.00 ms,
while client JSON parse times fell. The end-to-end difference does not
establish a server-side improvement. At 100 nodes, Redis reduced the direct
snapshot median slightly and increased the HTTP total median in both runs.

## Interpretation limits

The fixed order is100nodes then5,000nodes; within each size SQLite, PostgreSQL,
then PostgreSQL+Redis. The PG cells share one fresh database, so earlier projects
and database/index cache warmth remain present. The native SQLite path and
Docker VM PostgreSQL/Redis path have different transport and operating-system
boundaries. Durability is SQLite synchronous FULL and PG synchronous_commit on;
these timings are backend-call latency, not physical I/O measurement.

PG+Redis commits never call Redis. The fixed order prevents isolating the cause
of the differences between the two PG write groups. The current PG adapter
loads and validates the full candidate before persisting deltas. Redis hits
still probe PG authority, create a fresh Redis connection, decode owned data and
bridge through an intermediate JSON DOM. This benchmark measures complete calls;
it does not attribute a bottleneck to individual uninstrumented phases.

There is no contention, multi-writer, browser rendering, durability-fault,
deployment or long-run growth claim. The older debug scalar-spike21ms number is
not comparable to this release product workload. Engine optimization and any
decision to switch the CLI remain separate work.

## Results — release run C with Moka, 2026-11-03

This run used `bash scripts/task19-bench.sh` at the fingerprint below. The
runner uses one disposable PostgreSQL 16.15 container (2 CPUs, 2 GiB) and
enables `pg_stat_statements` for all PostgreSQL cells. Moka runs in the same
process as the adapter. The release build, fixture shape, write batch size,
timer boundaries and order follow runs A and B, replacing PG+Redis with
PG+Moka. The additional PostgreSQL instrumentation and normal run variation
limit direct comparison across runs. This run took 33.61 s and completed
six cells with 20 warmups and 100 samples per group, zero failures.

Source fingerprint before and after:
`04fe937eb1cd0a7319ee4ddfc71eb40a8008ca031dedd3dde1e583ac25ab957a`.
PostgreSQL image digest:
`sha256:0cefaabd882e42bcb2a2f3dc56d3ce4770560ba62fd7f197f1a2e008fd5731ab`.
Raw evidence is in the temporary directory
`/var/folders/pw/yxvtwnf517qchxqynh2w7pz80000gn/T/opencanvas-task19-bench.8IaFbC/`.
The Task19 Docker container and volume inventories were empty after cleanup.

Each cell shows **p50 / p95 / p99 in milliseconds**.

| Nodes | Backend | Commit | Storage snapshot | HTTP total including parse |
| ---: | --- | ---: | ---: | ---: |
| 100 | SQLite | 5.188 / 7.037 / 7.126 | 0.270 / 0.363 / 0.460 | 0.470 / 0.577 / 0.698 |
| 100 | PostgreSQL | 2.849 / 3.649 / 3.984 | 0.933 / 2.025 / 2.437 | 2.084 / 2.560 / 2.801 |
| 100 | PG+Moka | 4.228 / 5.338 / 5.788 | 0.594 / 3.629 / 14.046 | 1.008 / 1.438 / 1.946 |
| 5,000 | SQLite | 31.230 / 45.330 / 58.564 | 3.918 / 4.381 / 4.475 | 14.466 / 22.679 / 35.337 |
| 5,000 | PostgreSQL | 40.168 / 92.374 / 122.493 | 6.173 / 9.640 / 9.783 | 19.997 / 22.971 / 23.716 |
| 5,000 | PG+Moka | 32.541 / 35.833 / 36.619 | 0.995 / 1.390 / 1.464 | 14.309 / 16.959 / 17.983 |

| Nodes | Backend | Request through complete body | Client JSON parse |
| ---: | --- | ---: | ---: |
| 100 | SQLite | 0.408 / 0.512 / 0.625 | 0.060 / 0.082 / 0.107 |
| 100 | PostgreSQL | 2.003 / 2.482 / 2.734 | 0.074 / 0.086 / 0.089 |
| 100 | PG+Moka | 0.911 / 1.342 / 1.778 | 0.094 / 0.105 / 0.138 |
| 5,000 | SQLite | 10.684 / 17.025 / 28.838 | 3.571 / 5.105 / 7.384 |
| 5,000 | PostgreSQL | 17.102 / 19.843 / 20.699 | 2.960 / 3.169 / 3.229 |
| 5,000 | PG+Moka | 9.950 / 12.131 / 13.464 | 4.269 / 4.972 / 5.220 |

All four Moka HOT groups recorded 120 PostgreSQL identity probes and zero SQL
entity reads, with snapshot comparisons for every call. Independent verification
recomputed percentiles and matched canonical snapshot hashes, revision ranges and
response sizes to run B. Moka's cache weight uses the serialized JSON size
(17,307 and 861,742 bytes for the two fixtures) as a proxy. Heap/RSS and cold
fill latency were outside the timed groups.

At 5,000 nodes the Moka hit median was 0.995 ms versus 6.173 ms for the
uncached PostgreSQL snapshot. The HTTP median moved from 19.997 to 14.309 ms.
At 100 nodes the direct median fell while its observed p99 increased. This
single run does not establish tail behavior. Both PostgreSQL write groups use
the same commit path.

## Results — release run D with HTTP response cache, 2026-11-03

Run D completed all six cells in 55.32 s. Each timing group had 20 warmup
and 100 measured calls. The server used the HTTP Moka cache in all three
backends, with a separate opt-in PostgreSQL snapshot cache in the PG+Moka
cell. Both formats were prewarmed before timing. The exact JSON round-trip,
format-separated ETags and empty 304 bodies passed. Independent verification
recomputed percentiles from raw samples. Each PostgreSQL HTTP group made
120 identity probes and zero entity reads, including warmup.

| Nodes | Object body bytes | Row body bytes | Reduction |
| ---: | ---: | ---: | ---: |
| 100 | 17,325 | 13,313 | 23.2% |
| 5,000 | 861,760 | 656,848 | 23.8% |

Each cell shows **p50 / p95 / p99 in milliseconds**. GET 200 includes JSON
parsing. Row-to-object parity checks run outside the timer. GET 304 is timed
through receipt of the response and has zero body bytes.

| Nodes | Backend | Object 200 | Rows 200 | Object 304 | Rows 304 |
| ---: | --- | ---: | ---: | ---: | ---: |
| 100 | SQLite | 0.669 / 1.275 / 1.402 | 0.856 / 1.230 / 1.379 | 0.495 / 0.887 / 1.040 | 0.837 / 1.304 / 1.516 |
| 100 | PostgreSQL | 2.947 / 6.022 / 7.544 | 2.447 / 8.526 / 9.113 | 2.012 / 3.656 / 8.607 | 1.619 / 2.595 / 3.493 |
| 100 | PG+Moka | 0.648 / 0.884 / 1.070 | 0.505 / 0.586 / 0.599 | 0.527 / 0.643 / 0.807 | 0.384 / 0.447 / 0.553 |
| 5,000 | SQLite | 12.925 / 32.375 / 102.053 | 9.942 / 28.287 / 46.837 | 1.210 / 13.303 / 23.906 | 1.074 / 4.372 / 9.631 |
| 5,000 | PostgreSQL | 7.430 / 9.544 / 10.634 | 5.438 / 6.184 / 6.875 | 0.779 / 1.581 / 2.017 | 0.667 / 1.160 / 1.390 |
| 5,000 | PG+Moka | 8.358 / 12.088 / 12.789 | 5.671 / 6.343 / 6.989 | 0.808 / 1.429 / 2.078 | 0.660 / 0.964 / 1.205 |

Run D uses fixed group order and shows substantial timing variation. Its
body byte counts are deterministic for these fixtures. The 200 timings
compare two representations with the response cache already warm. Runs
A–C used earlier code and a different HTTP workload.

The before and after source fingerprint was
`00048c0ee22d67dbdeb5df26f878477ad22439918626d13ce8ece61561deb081`.
The wrapper exited 0. Raw samples and metadata are in temporary directory
`/var/folders/pw/yxvtwnf517qchxqynh2w7pz80000gn/T/opencanvas-task19-bench.aKqlxV/`.
The Task19 Docker container and volume inventories were empty after cleanup.

## Results — release run E after the PostgreSQL review fixes, 2026-11-03

Same wrapper, workload and machine as run D. The "before" column is a run made
immediately before the changes. The changes: per-worker prepared statements,
cache probe and miss read in one pool checkout, write-through cache fill after
commit, `Arc` snapshots on cache hits, stored-text diff with one CTE write
statement per commit, and dry runs on a read-only snapshot without the row lock.
Each cell shows the **p50 in milliseconds** from one run each; tails varied
between runs and are not compared.

| Nodes | Backend | Commit before | Commit after | Snapshot before | Snapshot after |
| ---: | --- | ---: | ---: | ---: | ---: |
| 100 | PostgreSQL | 2.687 | 2.307 | 0.712 | 0.478 |
| 100 | PG+Moka | 2.401 | 1.877 | 0.252 | 0.154 |
| 5,000 | SQLite | 18.414 | 18.959 | 2.690 | 2.757 |
| 5,000 | PostgreSQL | 30.246 | 25.383 | 7.005 | 5.182 |
| 5,000 | PG+Moka | 30.631 | 27.714 | 1.330 | 1.052 |

The worker profile of one 5,000-node PostgreSQL commit attributes about 10 ms
to shared `Candidate::validate` over the whole project (also paid by SQLite) and
6–8 ms to transferring the entity rows, which grew after the connection sat idle
between commits on Docker Desktop loopback. Incremental validation is the next
lever and lies outside the PostgreSQL adapter.
