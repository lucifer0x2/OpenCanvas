# PostgreSQL bulk SQL — isolated benchmark and protocol evidence

Measurements and checks below describe the dated historical run. Generated raw
logs, JSON/JSONL, manifests and archives are no longer versioned in the current
tree; see [output policy and reruns](evidence/README.md).

The new server uses six top-level SQL executions and six protocol synchronization
cycles per valid write, independent of 1/3/500 changed rows. Timing results are
mixed: 500-row writes improve clearly in the separate scaling scenario, while
the original 5,000-node one/four-writer matrix cells regress in this run.

This report compares the existing optimized client against the same client after
server-side bulk SQL changes. It does not reuse an old run as the baseline. The
matrix retains 100/5,000 nodes, 1/4/8 writers and 12 edits per writer, three
changed nodes per edit, continuous full-snapshot reads and SSE verification.

## Baseline

Fresh run: `evidence/query-baseline-1793601110/report.json`.
All six cells passed, 312/312 edits committed, reader traffic was nonempty and
SSE/commit revision ranges matched. The source was frozen before the backend
changes in `.runtime/query-baseline.kRJExD`; it remained unchanged during the run.
Source fingerprint:
`a792088d464857c9c551608463a7df857014a84f25503d8f85c45efcd3e19aed`.
The raw report records each source file and both dev binary hashes before/after.

Both baseline binaries were built once with `cargo build --offline --locked`
(v1 and `--features compat-v2`). Measurements used `SKIP_BUILD=1`; no backend
build/test ran concurrently. PostgreSQL was pinned to 16.9 and the recorded image
digest, with 2 CPUs/2 GiB; gateways were independent uncapped host processes.
Each comparison side uses its own newly initialized database. The inherited
spike database and user services were not used.

| Nodes | Writers | POST p50 / p95 / p99 ms | Whole operation p50 / p95 / p99 ms | Edits/s | Conflicts | Max retries |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 100 | 1 | 5.98 / 44.21 / 44.21 | 6.87 / 50.48 / 50.48 | 64.96 | 0 | 0 |
| 100 | 4 | 5.10 / 7.60 / 30.92 | 5.79 / 80.46 / 230.62 | 131.74 | 40 | 8 |
| 100 | 8 | 5.08 / 30.57 / 33.83 | 6.04 / 197.80 / 600.70 | 142.75 | 120 | 13 |
| 5,000 | 1 | 34.64 / 68.93 / 68.93 | 35.25 / 69.76 / 69.76 | 21.54 | 0 | 0 |
| 5,000 | 4 | 35.40 / 63.32 / 66.62 | 38.01 / 604.20 / 1,215.51 | 21.93 | 53 | 17 |
| 5,000 | 8 | 37.63 / 66.04 / 115.10 | 47.82 / 1,663.03 / 3,353.47 | 20.59 | 278 | 37 |

The historical attempts/successes JSONL recorded every POST and whole operation, including
writer identity, retries, revision reads and requested backoff. Each writer
completed 12 edits; the report retains per-writer latency/retry distributions.
This demonstrates completion for this finite run, not a general fairness bound.
The one-writer cells contain only 12 samples and eight-writer cells contain 96;
p99 therefore lies close to the slowest sample. No SLA is inferred.

Independent `cleanup-verification.json`
confirmed no owned container or gateway process remained after the run.

## Optimized matrix

The root ran the frozen optimized source in
`evidence/pg-1793608137042-b80957ec/report.json`:
all nine phases passed, including the same six-cell, 312-edit matrix. The
`comparison-bulk.json`
validator checked sequential runs, a fresh DB on each side, identical client
mode/counts/profile/resources/toolchain, frozen source/binaries, complete commits,
nonempty readers and all writers completing their 12 edits.

| Nodes | Writers | POST p50 / p95 / p99 ms | Whole operation p50 / p95 / p99 ms | Edits/s | Conflicts | Max retries |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 100 | 1 | 3.97 / 6.09 / 6.09 | 4.76 / 40.23 / 40.23 | 110.67 | 0 | 0 |
| 100 | 4 | 3.37 / 26.33 / 34.48 | 4.00 / 124.12 / 206.76 | 169.80 | 23 | 7 |
| 100 | 8 | 3.07 / 7.76 / 50.41 | 4.04 / 164.07 / 338.47 | 208.34 | 124 | 10 |
| 5,000 | 1 | 41.31 / 90.04 / 90.04 | 42.11 / 115.04 / 115.04 | 16.85 | 0 | 0 |
| 5,000 | 4 | 41.40 / 73.87 / 95.24 | 46.62 / 846.02 / 1,518.43 | 18.54 | 56 | 18 |
| 5,000 | 8 | 38.31 / 66.69 / 76.78 | 51.39 / 1,502.43 / 2,609.90 | 21.30 | 295 | 31 |

The 5,000/8 whole-operation p99 improved 3,353.47 → 2,609.90 ms, but median
47.82 → 51.39 ms and conflicts 278 → 295 increased. The 5,000/1 p99 regressed
69.76 → 115.04 ms and 5,000/4 regressed 1,215.51 → 1,518.43 ms. Fewer commands
therefore do not establish a blanket latency improvement. Existing full candidate
loading/validation remains; this run does not profile its individual phases.
All writers completed, yet the optimized 5,000/8 per-writer p99 ranged from
157.22 to 2,609.90 ms. No starvation or latency bound is claimed.

## Opt-in protocol proof

[`harness/query-counts.mjs`](harness/query-counts.mjs) loads either the immutable
baseline source/binary or the current source/binary through `SPIKE_SOURCE`. The
same external observer is used on both sides. Each proof creates a fresh owned
PostgreSQL container and one gateway, seeds separate 501-node projects, and
measures updates and leaf deletions changing 1, 3 and 500 rows plus one mixed
500-row batch. No reader/SSE client runs during this diagnostic; matrix timings
never use the observer.

[`infra/pg-wire-probe.mjs`](infra/pg-wire-probe.mjs) transparently forwards
plaintext PostgreSQL v3 frames over loopback and records aggregate message tags
and fixed statement categories. Authentication payloads, Bind values, row bytes,
raw SQL and credentials are never emitted or saved. Unknown message framing,
invalid lengths and unsupported TLS startup fail closed. Parser tests cover
fragmentation/coalescing, startup versus authentication framing and rejection.

- SQL executions are frontend `Execute` plus `SimpleQuery`; BEGIN/COMMIT count.
- Protocol synchronization cycles are frontend `Sync` plus `SimpleQuery`, checked
  against backend `ReadyForQuery`. They are completion barriers, not TCP packets.
- `Parse`, `Bind`, `Close` and all backend tags remain separately visible. Thus
  reduced SQL commands and avoided prepare/execute synchronization are distinct.
- Diagnostic output records source/binary provenance before/after, observer file
  hashes and owned-resource cleanup. It makes no latency claim about the proxy.

Both proof runs passed all seven cases:
`baseline`,
`optimized`.

| Changed rows | Operations tested | SQL executions before → after | Protocol sync cycles before → after |
| ---: | --- | ---: | ---: |
| 1 | Update; delete | 10 → 6 | 26 → 6 |
| 3 | Update; delete | 12 → 6 | 32 → 6 |
| 500 | Update; delete; mixed | 509 → 6 | 1,523 → 6 |

The optimized path had four Parse/Bind/Execute/Sync messages plus two simple
BEGIN/COMMIT commands in every case. Baseline Close messages were 8/10/507;
Close+Sync may be pipelined asynchronously with later requests. Accordingly,
the observed sync counts are not physical TCP roundtrips or individually awaited
critical-path roundtrips. The persistence CTE contains multiple DML subcommands
but executes as one top-level command.

Category labels are auxiliary: the frozen observer labels a DELETE template as
`candidate-read` because it contains `FROM nodes`, and the persistence CTE as
`notify` because it contains `pg_notify`. Actual Execute/Sync/ReadyForQuery tag
counts are independent of those labels. Source was kept frozen across the matrix
and proof rather than changing the observer after measurements.

## Separate bulk-width timing scenario

After the mixed matrix result, one additional paired scenario ran sequentially
with the same binaries, resources and client helper hashes. It used a fresh DB
per side and a fresh 5,000-node project for each width, 12 updates per width,
one writer and no reader/SSE/proxy/retry. Every update changed all requested rows.
Whole-operation timing includes the lightweight revision GET, payload creation,
POST and parsed response; fixture seeding/final verification are excluded.

The historical external width-scaling driver was hashed in both reports and excluded
from measured application source fingerprints. That generated driver and its outputs
are not supplied in the current tree; the results below remain historical.

| Changed rows | POST before p50 / p95 / p99 ms | POST after p50 / p95 / p99 ms | Whole operation before p50 / p95 / p99 ms | Whole operation after p50 / p95 / p99 ms |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 34.14 / 37.88 / 37.88 | 37.84 / 40.76 / 40.76 | 35.21 / 38.68 / 38.68 | 38.62 / 42.04 / 42.04 |
| 3 | 34.95 / 38.26 / 38.26 | 37.49 / 41.15 / 41.15 | 35.73 / 39.48 / 39.48 | 38.17 / 41.95 / 41.95 |
| 500 | 155.60 / 225.78 / 225.78 | 65.75 / 71.68 / 71.68 | 156.36 / 226.68 / 226.68 | 66.64 / 72.84 / 72.84 |

For 500 changed rows, POST median improved about 2.37× and p99 about 3.15×.
The 1/3-row cases remain slower after the change. This scenario helps establish
bulk-width scaling, not a replacement for the original contention matrix. Each
cell has 12 samples; p95/p99 equal the maximum. There was exactly one run per
side per scenario, with no best-of reruns.

`bulk-diagnostics-comparison.json`
was generated by a historical diagnostic validator, which checked
equal driver/client hashes and resources, sequential fresh DBs, frozen binaries,
all protocol completion counts and the scaling quantiles against raw JSONL.

## Repeat commands

From the repository root, run the current matrix, then separate protocol diagnostics
with unique new output directories:

```sh
EVIDENCE_DIR="$PWD/spikes/realtime-db/evidence/query-current-NEW" \
  CLIENT_MODES=optimized NODE_COUNTS=100,5000 WRITERS=1,4,8 ITERATIONS=12 \
  node spikes/realtime-db/harness/run-all.mjs
EVIDENCE_DIR="$PWD/spikes/realtime-db/evidence/query-counts-current-NEW" \
  node spikes/realtime-db/harness/query-counts.mjs
```

These commands measure current source, including later Task17 changes. Recreating the
historical paired comparison requires a separately preserved pre-bulk source/binary;
the old ignored `.runtime/` path is not supplied by a checkout. The generated scaling
driver can be inspected in prior Git commit `71e2142`; it is not a current runner.

The full optimized runner also performs contract/recovery/history/migration/
backup/index phases. Setting `PHASES=contractAndMatrix CONTRACT_TESTS=0` limits
either side to matrix validation; it must not be described as full regression.
All runners refuse to overwrite existing evidence. The six added Node tests
passed; they cover framing/privacy and comparison guards. All diagnostic modules
are below 100 lines and each has one responsibility.
