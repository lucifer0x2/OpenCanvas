# Task17 Redis HOT snapshot reads

Measurements and checks below describe the dated historical run. Generated raw
logs, JSON/JSONL, manifests and archives are no longer versioned in the current
tree; see [output policy and reruns](evidence/README.md).

Implementation scope: `spikes/realtime-db/postgres`; default SQLite runtime is unchanged.
Redis is optional, disposable derived storage. PostgreSQL owns project identity,
revision, entities, idempotency and durable commit. No batch mutation imports or invokes
the cache modules, and startup does not connect to Redis.

## Read contract

`GET /projects/:id` with Redis enabled performs an indexed PostgreSQL project-row probe
before lookup. Keys contain `oc:snapshot:v1`/`v2`, SHA-256 of DB connection configuration,
hex-encoded project/generation, authoritative revision, and the response metadata hash.
Two gateways using the same configuration share entries; different configurations may
miss safely. Recreated projects get a fresh database-generated UUID generation, including
when the new project reaches the same revision as a prior project with that ID.
The v2 description hash covers updates that do not advance the revision counter.

A hit returns encoded JSON bytes without node SELECT, node decoding, DOM JSON parsing
or reserialization. It represents the consistent head observed at the authoritative
probe; a concurrent commit after that probe may be observed by the next GET instead.
A miss starts a read-only `REPEATABLE READ` transaction, reads identity/description and
nodes from one snapshot, and fills the key for that actual snapshot, even if its revision
differs from the initial probe. The DB lease from the probe is released before cache IO.
No cache mutex spans SQL. Notification delivery and invalidation are unnecessary.

Each Redis value has a magic/version header and SHA-256 of key plus JSON bytes.
Corruption, truncated entries and copied wrong-key values become misses. Only the
gateway's encoder writes valid entries. The digest is not a MAC and does not defend
against a malicious writer with Redis credentials; Redis is not an authorization boundary.
Restore into the same DB configuration must rotate project generations or namespace
before cache enablement. The existing dump/restore fixture uses a separate DB namespace;
there is no new same-identity restore API in this task.

## Failure and resource limits

The exact Redis Rust client is locked to `redis-rs1.8.1`. Its multiplexed connection is
reused, while connect/response timeouts and an outer whole-operation timeout bound lock
waits, connection setup and command response. Error cleanup uses `try_lock`, so fallback
does not queue indefinitely after its deadline. Broken connections are discarded and
later reads reconnect after Redis restart. There are16nonwaiting operation slots.
Client API reference: [redis-rs connection configuration](https://docs.rs/redis/1.8.1/redis/struct.AsyncConnectionConfig.html).

Default operation timeout is100ms, capped at2000ms. A failed lookup and failed cache
fill may each consume one operation budget; PostgreSQL fallback time is additional.
Writes consume zero Redis operation budgets. Default JSON object cap is8MiB, hard cap
16MiB, plus36bytes envelope. A single Redis Lua script checks `STRLEN` and fetches an
entry atomically, preventing a large stored object from being transferred to the client.
Oversized generated responses are served from PostgreSQL and are not cached.
TTL defaults to1hour and is capped at1hour. Server total memory/eviction policy is external;
the owned Redis7.4.2 test fixture configures64MiB/`allkeys-lru` with persistence disabled.

Malformed URLs emit only `redis_cache_config_invalid` and disable cache. Redis operation
errors expose only fixed counters. Redis unavailable, cache deletion, eviction, timeout,
malformation and wrong-key binding all fall back to PostgreSQL. An authoritative DB
probe error is returned; a cached success is not used to mask that error.

`/health.cacheMetrics` contains `probes`, `nodeQueries`, `hits`, `misses`, `fallbacks`,
`malformed`, `stores`, `oversize`, `disabled`, `dbErrors`. These are monotonic process
counters for the snapshot read path, not all SQL issued by the gateway. Oversized Redis
entries are counted as misses; oversized generated bodies increment `oversize`.

## Targeted verification

Use an owned temporary PG/Redis fixture and a private exported environment; never point
these contracts at user data. Socket access must be permitted by the execution context.
`compat-v2` additionally requires the existing additive
`infra/migration-v2.sql` to be applied to that fixture first; its `description`
column is separate from the delta metadata initialized at gateway startup.
The verified owned fixture had this v2 migration installed.

```sh
cargo test --manifest-path spikes/realtime-db/postgres/Cargo.toml --locked \
  --test contract -- --test-threads=1
cargo test --manifest-path spikes/realtime-db/postgres/Cargo.toml --locked \
  --features compat-v2 --test contract -- --test-threads=1
cargo clippy --manifest-path spikes/realtime-db/postgres/Cargo.toml --locked \
  --all-targets -- -D warnings
cargo clippy --manifest-path spikes/realtime-db/postgres/Cargo.toml --locked \
  --all-targets --features compat-v2 -- -D warnings
cargo fmt --manifest-path spikes/realtime-db/postgres/Cargo.toml -- --check
```

The eight v2 cache contracts cover cold/hot/TTL, an exclusive node-table lock proving
the hot path avoids nodes plus an uncached positive control, malformed/truncated/wrong-key
entries, deletion and size caps,40concurrent silent-Redis failures with a finite deadline,
paired metadata/node consistency across a commit, commit between initial probe and cold
fill, two independent cache clients observing later revisions without invalidation,
same-ID/same-revision recreation, description changes without revision, and failed PG
COMMIT leaving the cached authoritative snapshot unchanged. Default v1 excludes only
the v2 description-specific test. Cross-process restart/outage/eviction, DB outage and
timed workloads belong to the separate Task17 lifecycle/evidence harness.

Current integrated results after the ordered-error NUL-ID regression: default v1
**33passed**, compat-v2 **34passed**. Strict offline/locked all-target Clippy for
both features and core/PostgreSQL formatting passed; counts and rerun commands are
retained in the [Task17 report](../../docs/TASK17-REPORT.md).
Initial invocations without exported fixture variables or without sandbox socket access
were setup failures, corrected before the successful real-service runs.

## Module boundaries and sizes

`hot_snapshot.rs` handles DB identity and consistent reads; `hot_reads.rs` orchestrates
cache lookup/fill; `redis_cache.rs` handles Redis transport/entry integrity/limits;
`cache_metrics.rs` exposes fixed counters. `http.rs` selects encoded snapshot responses,
`transport.rs` writes bytes, and `compatibility.rs` shares consistent cold snapshot logic.
All changed implementation modules remain below200lines. Cache contract modules are
170and216lines, below the250line review threshold; their assertions are grouped by
transport/resource behavior and revision/identity races.
