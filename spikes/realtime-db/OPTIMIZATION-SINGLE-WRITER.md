# Single-writer phase diagnosis — 2026-11-02

Measurements and checks below describe the dated historical run. Generated raw
logs, JSON/JSONL, manifests and archives are no longer versioned in the current
tree; see [output policy and reruns](evidence/README.md).

The measured 5,000-node workload spends **81.38–82.87%** of server elapsed time
loading, decoding and applying the complete project for a three-node edit.
Set-based persistence accounts for 2.68–2.81%; durable COMMIT accounts for
6.54–7.13%. This supports reducing the full-project access/validation path before
changing indexes or optimizing the small changed-row write further.

This change adds profiling only. It does not optimize algorithms, introduce a
cache, change the schema, migrate the runtime or claim a measured speedup.
The design proposal is in
[`DATABASE-WRITE-DESIGN-RESEARCH.md`](../../docs/DATABASE-WRITE-DESIGN-RESEARCH.md).

## Instrumentation and boundaries

`OC_SPIKE_PROFILE_BATCH=1` enables one JSON stderr record per batch in the isolated
PostgreSQL gateway. The default remains disabled, with no timestamps or logs.
The record contains an in-process sequence, outcome, operation/node counts and
elapsed microseconds. It contains no IDs, idempotency keys, props, SQL or URLs;
the HTTP response and storage contracts are unchanged. The environment flag is
read once per process, so enabling/disabling it requires a gateway restart.

| Field | Measured boundary |
| --- | --- |
| `phasesUs.hash` | Request validation, canonical serialization and hash |
| `pool`, `begin`, `lock`, `replay` | Awaited connection/transaction/lock/replay phases |
| `nodeQuery` | Awaited full-project query, including PostgreSQL work, transfer and scheduling |
| `nodeDecode` | Row/string/JSONB materialization and collection into all nodes |
| `apply` | Existing complete core application, including validation and sorting |
| `persist` | Response encoding, changed-node partition/encoding and awaited bulk write |
| `commit` | Awaited durable transaction COMMIT |
| `coreUs` | Clone, execute, candidate validation and sort, nested inside `apply` |

Times are wall-clock observations rather than isolated SQL or CPU timings.
Do not add `coreUs` to `apply`. Aggregate shares below divide summed phase times
by summed server totals; they do not add independent percentiles. `totalUs`
starts inside batch execution and includes unprofiled gaps and candidate cleanup;
HTTP parsing/response transmission, revision GET and client costs are outside it.
Failed/dry-run/replay requests have partial phase sets; diagnosis uses successful
measured writes only. Core `apply` still delegates to the same implementation;
the opt-in `apply_profiled` only records its existing subphases.

## Two fresh baseline repetitions

Both runs used the identical frozen instrumented source and dev binary
(`cargo dev`, unoptimized with debug info), PostgreSQL 16.9, two gateways,
one writer, a continuous snapshot reader and SSE. Each cell seeded its fixture,
warmed up 20 writes and measured 100 three-node updates. Fixtures are shallow:
one root page and text children; the edits change three `name` values. These
results do not measure deep trees, structural edits or production release builds.
No build or other database workload ran during either timed repetition.

The historical repetitions were recorded as r1 and r2. Client latency, workload
details and repetition comparison are documented in
[`SINGLE-WRITER-BENCHMARK.md`](../../docs/SINGLE-WRITER-BENCHMARK.md).

| 5,000-node elapsed time, ms | r1 p50 | r2 p50 |
| --- | ---: | ---: |
| Server batch total | 35.323 | 35.007 |
| Full node query | 9.834 | 9.307 |
| Full node decode | 8.578 | 8.811 |
| Core apply | 10.253 | 10.354 |
| ↳ Candidate validation, inside apply | 8.966 | 9.065 |
| ↳ Full clone, inside apply | 1.092 | 1.132 |
| ↳ Execute three operations, inside apply | 0.073 | 0.074 |
| ↳ Sort, inside apply | 0.047 | 0.047 |
| Persist, including bulk write | 0.876 | 0.846 |
| Durable COMMIT | 1.421 | 1.477 |

Full query + decode + apply accounted for 82.87%/81.38% of elapsed server time.
Candidate validation alone accounted for 24.57%/24.56%, clone 3.23%/3.50%,
operation execution 0.20%/0.20% and sorting 0.14%/0.13%. Removing only the clone
or sort would leave most measured cost. A delta-first path with a conservative
validation proof/fallback is a design candidate, not a measured implementation.

Both repetitions passed all harness consistency checks: exact revisions and
events, cross-gateway replay, unchanged nodes and matching sentinel snapshots.
Owned containers, volumes, gateway processes and credentials were removed.

## Verification and source scope

Executed successfully for this profiling-only change:

- Core offline tests: 7/7, including profiled/unprofiled result/error parity and
  input immutability.
- Offline strict Clippy for core and PostgreSQL, all targets/features, `-D warnings`.
- `cargo fmt --check` for both isolated crates.
- Two baseline runs with exactly 100 measured successful profiles in each cell;
  source/binary hashes stayed fixed throughout each run.

Root independently verified the retained real-DB contract target after the timing
window: **19/19** (18 DB scenarios + one blob/hash unit) using a fresh isolated
PostgreSQL runtime and one test thread; cleanup completed. The command was:

```sh
cargo test --offline --locked \
  --manifest-path spikes/realtime-db/postgres/Cargo.toml \
  --test contract -- --test-threads=1
```

No multiwriter matrix or full recovery/backup cycle was rerun in this
profiling-only turn. Existing durable/replay/snapshot contract checks remain.

Modified Rust line counts: core `lib.rs` 136, `profile.rs` 21, `tests.rs` 147;
PostgreSQL `batch.rs` 101, `db.rs` 178, `profile.rs` 87, `main.rs` 87 and contract
registry 48. Each module has one profiling/application/read responsibility and
remains below 200 lines. There are no new dependencies. The original
uninstrumented source was preserved before adding timings; the instrumented
baseline and workspace source matched exactly before the runs.
