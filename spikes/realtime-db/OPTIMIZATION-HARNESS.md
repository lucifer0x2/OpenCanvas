# Task15 optimization harness — 2026-11-02

Measurements and checks below describe the dated historical run. Generated raw
logs, JSON/JSONL, manifests and archives are no longer versioned in the current
tree; see [output policy and reruns](evidence/README.md).

This is isolated PostgreSQL fixture work. Product SQLite storage, real services and
user data remain outside this lane. Original results are summarized in the
[Task15 report](../../docs/TASK15-REPORT.md).

## Frozen original source and baseline

Before any optimization source mutation, copied all source bytes in `core/`,
`postgres/`, `harness/`, `infra/`, including embedded public test evidence, to ignored
a temporary `.runtime/` baseline directory. Excluded only generated `target/`,
`node_modules/`, `.runtime/` and root evidence. The 46-file source fingerprint is
`a26d059225c86212e490bac395516b7d6c0281ec603c2a939362c234deba7d24`, identical to
the previously accepted source. The historical capture recorded each source hash;
that temporary baseline is not supplied in the current tree.
The captured runner reads Git HEAD from its ancestor repository; source hashes and
binary hashes, rather than dirty checkout HEAD alone, identify the actual program.

The historical baseline selected `PHASES=rustContract,contractAndMatrix` in that
frozen capture. To exercise those phases against current source from repo root:

```sh
PHASES=rustContract,contractAndMatrix node spikes/realtime-db/harness/run-all.mjs
```

This creates fresh ignored outputs; it does not reproduce the old algorithm.

No matrix/count/build overrides were set. Result: exit0, Rust DB contracts10/10,
full 100/5000×1/4/8 matrix,12 iterations/writer,312 successful edits. Sources stayed
unchanged and owned gateway processes/container/anonymous volume were cleaned.
This deliberately selected baseline scope excludes history/recovery/migration;
the optimized final acceptance must run those phases separately in its full runner.

| Nodes | Writers | Conflicts | Operation p95ms | Operation p99ms | POST p99ms | Max retries |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 100 | 1 | 0 | 35.8 | 35.8 | 6.6 | 0 |
| 100 | 4 | 78 | 184.0 | 315.8 | 30.7 | 28 |
| 100 | 8 | 409 | 628.3 | 966.9 | 23.3 | 75 |
| 5000 | 1 | 0 | 130.3 | 130.3 | 60.2 | 0 |
| 5000 | 4 | 72 | 1595.0 | 4036.9 | 104.4 | 36 |
| 5000 | 8 | 335 | 7492.8 | 13253.2 | 223.4 | 84 |

Operation latency includes revision snapshot reads and every retry. These numbers
are a fresh baseline, not replacements for earlier accepted measurements. The first
capture run also finished all measured phases and cleanup, but the old runner exited1
after saving its pass report because its omitted root evidence parent did not exist
for `latest.json`. That failed invocation was not used as the clean comparison input.
Only the ignored evidence parent was created before the clean rerun, without changing
captured source. No benchmark ran concurrently with other lane builds or benchmarks.

## Client modes and metrics

`legacy` retains the original full snapshot revision read, immediate conflict retry,
new key after definitive conflict, and original retry bound. `optimized` reads
`GET /projects/:id/revision`, then applies the same three-node assignment fixture.
Both modes have identical node/writer/iteration counts, a continuous full snapshot
reader, and a second-gateway SSE stream. Writers start together and proceed
independently: no turn scheduler or serialized fairness mechanism hides contention.

Optimized explicit `REVISION_CONFLICT` responses use bounded equal-jitter exponential
backoff:2ms initial ceiling,64ms cap,1000 conflicts maximum. At least half the
ceiling is slept. A network failure has an uncertain outcome; up to3 retries retain
the exact payload and key. Collision, validation and HTTP server responses do not
silently rebase. The known fixture assignments are safely rebuilt after definite
revision conflict; this helper does not generalize to arbitrary production intent.

Each mode records raw POST attempt latency, complete operation latency including
revision reads/sleeps, attempt/conflict/success counts, throughput, maximum retries,
per-writer counts and latency/retry fairness. Optimized success records also contain
network retries, requested sleep duration and revision read cost. Actual complete
operation latency includes scheduler delays as well. Full snapshot reader frequency
remains the original continuous loop; observed read counts naturally vary with runtime.
Node values/untouched geometry, revision/commit ranges and SSE ordering are asserted.
Empty fixture traffic and empty snapshot reads fail.

The default new runner executes both complete six-cell matrices sequentially in one
fresh owned runtime with fresh projects, then durability/recovery/history/backup and
index evidence. Index fixtures run last in a separate temporary database. Raw file
names carry client mode. Existing evidence directories and raw files are never overwritten.
The original baseline uses its original binary/client; optimized legacy and optimized
client use the same new binary. All binaries are Cargo dev profile, PostgreSQL16.9
image/digest/resource limits remain pinned, and host gateways remain uncapped.

`compare.mjs` reads baseline raw successes to recover original max retries/fairness,
and refuses mismatched workload/resources/toolchains, overlapping run times, changed
source fingerprints, incomplete matrices or empty traffic. Source/binary provenance
and configuration are embedded in the comparison. Fixture timing remains sensitive
to host scheduling/cache; no product SLA or isolated SQL latency is claimed.

## Validation status

Focused deterministic retry tests5/5 pass: jitter bounds, definitive conflict rebase,
lost response preserving exact body/key, non-conflict error rejection and finite retry
bounds. Comparison guard tests2/2 pass, including missing modes, empty traffic,
resource changes, overlapping runs and source mutation. All12 Node tests pass,
including the original parser/empty-stream3 and index evidence guard2 tests.

Integrated targeted command:

```sh
PHASES=rustContract,indexEvidence node spikes/realtime-db/harness/run-all.mjs
```

Run `pg-1793597486325-94df21f9` exited0: startup, actual DB contract12/12 and
indexEvidence pass; the remaining phases were intentionally skipped. Source hashes
before/after match and owned cleanup passed. This is targeted integration evidence,
not a completed optimized matrix/recovery/history acceptance run. Root independently
runs the complete default suite and comparison after this lane's source freeze.
Changed harness modules remain below250 lines; no production modules were edited.
