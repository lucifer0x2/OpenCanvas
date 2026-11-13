# PostgreSQL snapshot cache

The server enables an in-process cache using Moka 0.12.15's
`moka::sync::Cache` when launched with `DATABASE_PROVIDER=postgresql`.
PostgreSQL supplies the authoritative project state. An embedded server can
attach the cache through the storage adapter:

```rust,ignore
use opencanvas_storage::{PostgresBackend, SnapshotCache, Storage};

let backend = PostgresBackend::connect(postgres_url)?;
let cache = SnapshotCache::new(backend.cache_namespace());
let storage = Storage::new(backend.with_cache(cache)?);
```

Each cache is bound to the database namespace from the durable SQL schema
registry. Injection rejects a different namespace with `INVALID_CACHE_CONFIG`.
Clones of one cache share entries in the same process. Each independently
constructed cache starts empty, including after a process restart.

## Reads and ordering

PostgreSQL resolves the project ID or slug and returns the canonical ID,
namespace, generation and revision. The cached snapshot is eligible only
when all of these agree. An unavailable PostgreSQL database returns its storage
error even if a previous snapshot is cached. A hit skips SQL entity and binary
table reads. The probe and a miss run in one pool checkout: the probe is a
single prepared lookup, and a miss loads the snapshot in a repeatable-read
transaction on the same connection, filling the cache after it commits.

Writes are write-through. After a batch or asset import commits, the adapter
rebuilds the new snapshot from the rows as loaded plus the exact payload text it
just wrote, applies the same ordering and asset/font readiness as a cold read,
and fills the cache under the committed revision on the caller thread. Replays,
dry runs, rejected batches and writes whose commit outcome is unknown fill
nothing. Concurrent misses are not coalesced: waiting inside a pool worker could
exhaust the pool.

Each canonical project ID can hold one entry per cache. Moka compares fills
under its per-key lock. A higher database generation wins, followed by a higher
revision within the same generation. Duplicate or older fills leave the resident
entry and its deadline in place. If eviction removes the resident entry, the
next lookup still probes PostgreSQL before accepting any subsequent fill.
Recreating a project at the same ID and revision changes its generation. After
the database namespace rotates, a previously connected backend reads directly
from SQL until it reconnects and uses a cache bound to the new namespace.

## Expiration and capacity

An entry expires **3,600 seconds after insertion**. Reads leave its expiration
unchanged. A newer accepted snapshot starts its own 3,600-second period.
Capacity eviction can remove entries earlier.

The cache stores typed snapshots behind `Arc` and hands out the `Arc`.
`snapshot_with_cache_identity` returns it unchanged, so the HTTP route
serializes without copying; `snapshot()` clones on the caller thread to honor
the owned `Snapshot` return type, never while holding a pool worker. The value preserves
the domain's numeric types and floating-point bits, including negative zero.
Asset and font metadata can be cached as part of the snapshot. Their file
bytes continue to come from resource storage.

A fill counts serialized JSON bytes using a streaming sink that aborts at
**16 MiB**. Moka applies a **64 MiB** total weight limit per cache. These
counts are a proxy for the relative size of retained snapshots. The in-memory
types, cache bookkeeping, concurrent fills and response clones have their own
allocation costs. Oversized snapshots remain readable directly from PostgreSQL.
The count runs on every fill attempt, including write-through after a commit
(about 1 ms at 5,000 nodes). Hot reads do no serialization in the cache.

## Verification

```sh
cargo test --offline -p opencanvas-storage --lib hot_cache -- --test-threads=2
bash scripts/task19-check.sh
bash scripts/task19-bench.sh
```

The correctness runner starts disposable PostgreSQL on a random loopback port.
The release benchmark measures SQLite, PostgreSQL and PostgreSQL with Moka for
100 and 5,000 authored nodes. Its isolated PostgreSQL uses `pg_stat_statements`
to verify the hot sample groups by counting successful identity probes and
entity reads. The production adapter requires no PostgreSQL extensions.
The earlier measurements are preserved as the
[Redis/bitcode baseline](RUNTIME-STORAGE-BENCHMARK.md).

## HTTP snapshot response cache

The snapshot route has a separate in-process Moka cache in server state. It
stores immutable serialized bytes and a strong ETag per source identity and
node format. The loopback and public viewer routes share this cache. Every
request revalidates against storage before serving bytes or returning 304.
Only snapshot reads populate entries. A read concurrent with a commit can
observe either revision at its authoritative read point.

PostgreSQL probes the canonical project ID, database namespace, project
generation, schema and revision. Administrative PostgreSQL recovery to a
different revision history must rotate the database namespace before serving
requests. The SQLite backend probes the canonical project,
schema and revision from the live database. On Unix it checks the database
file identity and current asset readiness, which can change independently of
revision. SQLite storage expects a single owner of the data root and offline
administrative database edits. Backends without a reliable identity use a
fresh snapshot read for each request.

A miss binds encoded bytes to the identity of the snapshot read. A late fill
at an older revision uses a separate key. Entries expire 1 hour after
admission, with a 64 MiB weighted capacity and a 16 MiB admission limit per
entry. The weight includes the body, tag, identity and a fixed overhead
allowance. Older entries can remain until eviction or expiration. Oversized
responses still serve normally.

## Revision metadata cache

SQLite data roots and PostgreSQL backends also own a separate in-process derived
index cache. `Storage::shared_snapshot` binds an immutable snapshot Arc to its
matching index Arc using that read's complete `SnapshotCacheIdentity`. Commit
validation, page guards/history and MCP `get_edit_context` share these indexes.
This does not admit oversized snapshots into either payload cache or change
their 16 MiB admission / 64 MiB capacity.

Metadata entries use a 1-hour insertion TTL and 256 MiB weighted capacity,
including a reserved 4 MiB per entry for lazy dependency adjacency/font-ID sets.
Larger dependency graphs are used without retention; topology can still cache.
Expanded node payloads are temporary and are dropped after graph construction.
Weights count index strings/container allowances, without serializing snapshots;
they do not bound process RSS, concurrent builds or reader-held Arcs.

Exact identity keys isolate older/newer readers and late fills. Only durable
commits publish candidate indexes. PostgreSQL uses the authoritative namespace,
generation, schema and revision. SQLite checks file incarnation/readiness and
reopens pinned writers after device/inode replacement. Backends without a
reliable identity build fresh indexes. Cold dependency construction remains
project-wide; page-scoped expansion is pending. See the slice 3 evidence and
limits in [TASK25-REPORT](TASK25-REPORT.md).

Task25 slice 4a adds a separate lazy read `ScopeIndex` to each matching revision's
metadata: source-vector positions, children, navigation sources and note positions.
It is built from an authoritative read vector rather than Candidate's ID order,
so sort-key changes and cold/warm snapshots retain identical expansion ordering.
The existing metadata weight now reserves node/link metadata and 2 MiB for notes
before this index materializes. It retains no node properties or note text. The
default 5,000-note cap fits that note reservation; a larger benchmark-only index
above the charged reservation is used without retention. Payload admission limits
are unchanged.

Only builtin adapters attach a private committed authority binding to
`SharedSnapshot`; `trusted_scoped()` checks the exact snapshot/index Arc pair.
Custom backends (even with a cache identity), or replacing either public Arc,
lose that binding and can retain their legacy read path. The conservative
`scoped()` fallback fully validates. Trusted scoped expansion/resolution skips
full validation and whole-Candidate cloning; cold snapshot loading/index building
and the existing edit-context dependency graph costs remain project-wide until
their respective consumers change. See the 4a API/limits and checks in
[TASK25-REPORT](TASK25-REPORT.md).
