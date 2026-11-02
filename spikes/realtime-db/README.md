# PostgreSQL realtime spike

Research fixture only: this does not replace OpenCanvas SQLite storage or start the real
3847/3848 services. The user selected PostgreSQL for further work. The older
`spacetimedb/` files and evidence at the evidence root are historical, not this run's results.

From the worktree root, with Docker running, Rust dependencies cached and the pinned
image available:

```sh
node --test spikes/realtime-db/harness/*.test.mjs
node spikes/realtime-db/harness/run-all.mjs
```

The runner builds default v1 and `compat-v2` binaries using Cargo `--offline --locked`,
creates a fresh PostgreSQL 16.9 container pinned to the digest in `infra/runtime.mjs`,
and starts two independent Rust HTTP gateway processes. PostgreSQL gets 2 CPU/2 GiB;
host gateways have no imposed CPU/RSS limit. This is an intentionally pinned test
version, not a claim about the latest release. Docker sockets and loopback binds need
an execution context permitted to access them.

Each invocation creates `.runtime/pg-<unique>/` with mode700 and synthetic credential
files mode600. Credentials are never printed or copied into evidence. Container data
uses a private anonymous Docker volume. Cleanup verifies the exact run ownership label,
stops only owned gateway child processes, and removes only that container and its
anonymous volume. Inherited runtimes/containers are untouched. Credentials, private logs
and backup dump remain in ignored `.runtime/` for local debugging; no user data is used.

Results go to `evidence/pg-<unique>/report.json`, `contract.json`, topology/provenance,
attempt and success JSONL, and the real-DB Rust contract test log. `evidence/latest.json`
points to the last completed invocation, including failures. A failed phase exits nonzero
and retains preceding evidence. Only a complete fresh default run covers the whole matrix.
`NODE_COUNTS`, `WRITERS`, `ITERATIONS`, `HISTORY_COMMITS` can reduce an investigation run;
`PHASES=singleDatabaseOutage,historyGrowth` selects investigation phases;
`SKIP_BUILD=1` reuses binaries and `CONTRACT_TESTS=0` skips Rust tests. These override
settings are not equivalent to full acceptance evidence. Use no concurrent benchmark runs.

The default sequence checks:

1. Real-DB contract tests: project row locks, SQL fault rollback, canonical replay,
   dry-run, consistent snapshots, listener reconnect, polling repair and cursor gaps.
2. Cross-process semantics and 100/5,000-node fixtures with 1/4/8 writers, 12 successful
   edits each, sequentially in `legacy` then `optimized` client modes on fresh projects.
   Legacy keeps full snapshot revision reads and immediate conflict retry. Optimized
   uses the revision-only endpoint and bounded independent jitter/backoff. The continuous
   snapshot reader and SSE remain active in both modes. Raw conflict attempts and
   complete operation latency include revision reads, retries and sleeps; writer counters,
   untouched geometry, revision/commit ranges, snapshots and SSE ordering are asserted.
3. Gateway death with durable pending jobs, actual fixture worker SIGKILL, lease retry,
   fencing, immutable job inputs/artifacts, resource hashes and bounded shared upload IDs.
   Output is opaque fixture bytes; Chromium/native rendering is not ported.
4. Continuous writes, reads and reconnecting SSE during gateway restarts and separately
   a stop/start of the sole database. Outage/errors, retry p95/p99, overlapping request
   counts, SSE reconnect gaps, duplicate and missing commits are recorded. A sole database
   stop/start is an availability outage, not HA or a rolling database upgrade.
5. A 5,000-node fixture plus 1,000 additional commits measures DB/history growth, summed
   process VmRSS (shared pages may count twice), Docker memory, gateway restart, and
   database restart with snapshot and durable replay verification. No production SLA follows.
6. Additive description migration, mixed v1/v2 gateway clients, actual v2-to-v1 binary
   rollback, and `pg_dump`/`pg_restore` into a different DB with snapshot and byte hashes.
7. Natural index plans and query/FK correctness on a separate disposable populated DB,
   after all measured phases. See `OPTIMIZATION-INDEXES.md`.

`CLIENT_MODES=legacy` or `CLIENT_MODES=optimized` selects one client for investigation;
the default runs both complete six-cell matrices (624 successful edits in total).
Raw filenames include mode and output directories must be fresh. To compare the captured
original server/client baseline with server-only and server+client optimization:

```sh
node spikes/realtime-db/harness/compare.mjs \
  spikes/realtime-db/evidence/optimization-baseline-clean-1793597163652/report.json \
  spikes/realtime-db/evidence/NEW-RUN/report.json \
  spikes/realtime-db/evidence/NEW-RUN/comparison.json
```

Comparison refuses mismatched resource/workload settings, missing modes, empty traffic,
overlapping run times or source changes. Baseline raw evidence remains unchanged.
The client retry helper is only a fixture policy for known independent assignments;
it does not merge arbitrary authoring intent. Uncertain POST outcomes retry the exact
same body/key; only a definite revision conflict allows a fresh revision/key.
See `OPTIMIZATION-HARNESS.md` for capture provenance and validation scope.

Supported commands are the strict synthetic create/update/reparent/delete node subset.
This HTTP JSON gateway is not MCP session implementation. Fixture bytea resource/staging
storage proves shared durability only for bounded small payloads. Production object storage,
domain port, auth/ingress, lifecycle/GC, HA/failover, real Kubernetes, DB major upgrade and
SQLite import/cutover remain separate work. See `HARNESS-REPORT.md` for per-AC evidence.

## Task17 opt-in Redis snapshot reads

The PostgreSQL spike can cache encoded `GET /projects/:id` responses in actual Redis.
Provide `PG_URL` and `REDIS_URL` through a private environment file, then start the
spike binary as usual. Without `REDIS_URL`, snapshots read PostgreSQL directly.
Startup applies the additive generation/validation migration before accepting requests;
`compat-v2` still requires the existing `infra/migration-v2.sql` description migration.
This changes the research fixture only; OpenCanvas SQLite and ports3847/3848 are unchanged.

| Environment variable | Default | Bound |
| --- | --- | --- |
| `REDIS_CACHE_TTL_SECONDS` |3600seconds |1–3600seconds |
| `REDIS_CACHE_MAX_BYTES` |8MiB JSON response |1byte–16MiB |
| `REDIS_CACHE_TIMEOUT_MS` |100ms per Redis operation, including connect |1–2000ms |

Each cached GET first probes authoritative project generation/revision in PostgreSQL.
Keys include a DB configuration namespace, project identity, generation, revision and
v1/v2 response format; v2 also hashes description metadata that may change outside
the revision counter. Cold reads obtain metadata and nodes in one `REPEATABLE READ`
transaction and populate the revision actually read. A concurrent commit after a hot
read's probe may return the snapshot observed at that probe, as a consistent read.
No mutation waits for Redis or populates a candidate before PostgreSQL COMMIT.

Missing, evicted, malformed, oversized or unavailable entries fall back to PostgreSQL.
A failing authoritative probe returns a database error even if Redis contains an entry.
Connections and commands have finite timeouts, GET atomically checks object length in
Redis before returning bytes, and up to16Redis operations run concurrently per gateway.
Configure Redis server `maxmemory`/eviction separately; the owned test fixture uses
64MiB/`allkeys-lru`. Redis is disposable derived storage and needs no persistence here.

The byte envelope binds the key and response with SHA-256 so ordinary corruption and
values copied to a wrong key are rejected without decoding/reserializing the JSON on
every hit. This is integrity checking, not protection against a malicious party holding
Redis write credentials. Restore to the same DB identity must rotate project generation
or the namespace before enabling cache; the spike's restore fixture uses a different DB.
`GET /health` reports fixed `cacheMetrics` counters with no URLs, keys or credentials.
See [Redis HOT read implementation and contracts](REDIS-HOT-READS.md) and
[Task17](../../tasks/17-postgres-delta-redis-hot-reads.md).
