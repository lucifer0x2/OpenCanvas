# Historical Task19 Redis/bitcode snapshot cache

This records the original implementation measured in the
[Task19b baseline](RUNTIME-STORAGE-BENCHMARK.md). For the current runtime API,
limits and test commands, see [Moka snapshot cache](MOKA-SNAPSHOT-CACHE.md).

Task19 adds `SnapshotCache` to runtime storage, with **bitcode `=0.6.9`** chosen
by the user. It is disabled unless a `PostgresBackend` is explicitly given a
cache. SQLite remains the default composition root. This does not switch the
running service, migrate data or cache asset/font file bytes.

```rust,ignore
let backend = PostgresBackend::connect(postgres_url)?;
let config = RedisCacheConfig::new(redis_url, backend.cache_namespace());
let cache = SnapshotCache::new(config)?;
let storage = Storage::new(backend.with_cache(cache)?);
```

The namespace comes from a durable PostgreSQL schema registry. A cache with a
different namespace returns `INVALID_CACHE_CONFIG`. The Redis endpoint must be a literal IPv4
or IPv6 address using `redis://`; AUTH and database selection are supported.
Hostname resolution, TLS and Redis Cluster are outside this adapter's current
configuration contract. Endpoint parsing and command encoding use `redis`;
bounded RESP2 transport controls the socket deadline and reply allocation.

## Authority and ordering

Every current snapshot read first obtains canonical project identity,
generation and revision from PostgreSQL. PostgreSQL failure returns a storage
error, even when Redis contains a warm entry. A valid hit avoids entity reads;
a miss reads metadata and entities in one consistent PostgreSQL transaction.
The fill uses that transaction's actual generation/revision after releasing its
locks. Mutation/commit paths never call Redis.

Each project has one latest-value key:

```text
opencanvas:snapshot:bitcode069:v1:<SHA256(database namespace)>:<SHA256(project id)>
```

The value contains a fixed envelope and a typed binary payload. Its identity
includes wire/codec version, domain schema, namespace, canonical project,
PostgreSQL generation epoch and revision. Generation is database-assigned and
monotonically increasing. A Lua fill compares fixed-width big-endian
`(generation, revision)` bytes, so delayed fills cannot overwrite a newer head
and full `u64` revision values do not pass through Lua's floating-point numbers.
Neither revision nor generation creates an additional key. Recreated projects
replace the prior generation immediately on a current fill.

The SHA256 payload checksum detects corruption and truncation. It is **not
authentication** against a party that can write arbitrary Redis values. Redis
access is trusted infrastructure; PostgreSQL remains authoritative.

## Binary representation and limits

The full `Snapshot` is represented: nodes and their arbitrary properties,
tokens, styles, asset/font metadata, notes/replies and retention metadata.
Components, instances and themes are represented by their existing domain
properties. Asset/font bytes remain in PostgreSQL resource storage.

The serializer converts the domain structure to typed tokens, preserving null,
bool, signed `i64`, unsigned `u64`, `f64`, strings, arrays and objects. Tokens use
bitcode's native checked `Encode`/`Decode` API. There is no embedded JSON text.
The current implementation uses an intermediate owned `serde_json::Value` tree
to bridge the existing domain models; this is an allocation tradeoff, and no
speed or size improvement has been established by benchmark.

Bitcode decodes fixed arrays of 64 primitive tokens per frame. No recursive
type, `Vec` or `String` is supplied to the bitcode decoder. String spans refer
to an explicit UTF-8 table. Counts and frame lengths are checked before each
decode, and spans must consume the string table exactly once in order. This
prevents compact forged vector lengths or overlapping spans from requesting
unbounded owned allocations. Reconstruction rejects malformed types, duplicate
object keys, invalid UTF-8, nonfinite floats, missing/trailing data, more than
1,000,000 tokens or nesting deeper than 64. Deeper valid database values remain
readable from PostgreSQL; they are simply ineligible for this cache.

`StorageBackend::snapshot` returns owned `Snapshot`/`serde_json::Value` models.
Cache hits decode and allocate those models. This implementation does not use
zero-copy archives or change the HTTP/MCP JSON contract. Bitcode does not promise
a stable format across major versions; wire version/magic and key prefix must
change with incompatible DTO or codec changes. Old keys expire naturally.

Defaults are a 100 ms total network budget per get/fill, 300 second TTL and
16 MiB maximum envelope plus payload. Configuration rejects zero/over-five-
second network budgets, TTL outside 1 ms–1 hour and object limits outside
1 KiB–64 MiB. An atomic Lua `STRLEN` guard precedes `GET`; a separate bounded
bulk reader rejects excessive RESP lengths before allocation. The total socket
deadline includes connect, AUTH, SELECT, command write and reply read, including
slow trickling peers. Serialization/materialization CPU time is separate from
the network budget and bounded by the object/token/depth limits.

One nonwaiting capacity slot is shared by cache clones. Contention immediately
misses/skips, so there is no queue behind a slow connection. Each operation uses
a fresh connection, trading connection overhead for simple bounded failure
handling. Redis errors, malformed/oversized values and timeouts are misses;
misses continue through PostgreSQL. Cache fills are best effort. Existing keys
expire, including keys for removed projects; key count follows projects, not
revision count.

## Verification

Run codec, envelope, configuration/contention and bounded-transport unit tests:

```sh
cargo test -p opencanvas-storage --lib redis_binary -- --test-threads=2
cargo clippy -p opencanvas-storage --lib -- -D warnings
```

The targeted unit suite passed **6 tests** and strict library Clippy passed on
2026-11-03. Its temporary loopback
listener requires an execution environment that permits ephemeral socket binds.
It checks all snapshot fields, `u64::MAX`, `i64::MIN`, f64 bit fidelity including
negative zero, binary rather than JSON text, identity/schema/checksum mismatch,
every truncation, frame/count/UTF-8/depth/span guards, no-wait contention and a
total deadline against a deliberately trickling local peer. Real Redis/PG
contracts and actual commands/results are tracked in
[`TASK19-REPORT.md`](TASK19-REPORT.md); this document does not claim those tests
passed before that report records their execution.

Primary format/API references:
[bitcode 0.6.9 documentation](https://docs.rs/bitcode/0.6.9/bitcode/),
[bitcode upstream](https://github.com/SoftbearStudios/bitcode),
[Redis EVAL](https://redis.io/docs/latest/commands/eval/).
