# PostgreSQL runtime adapter

`PostgresBackend` implements the same blocking `StorageBackend` contract as
SQLite, including complete product validation, notes, tokens, styles, instances,
themes, resources, dry runs, history and exact idempotency replay.

Set `DATABASE_PROVIDER=postgresql` and supply a nonempty `DATABASE_URL` in the
environment before launching the server. The database role needs permission to
initialize and use the runtime schema. The server connects during startup,
enables an in-process snapshot cache and exits on initialization failure.
PostgreSQL mode stores nothing in the data root; it only reads `import_asset`
`path` files from `<data-root>/<slug>/inbox` (default `~/.opencanvas`) and
creates no directory there.

For an embedded server, inject the adapter at the composition root:

```rust,ignore
use opencanvas_storage::{PostgresBackend, SnapshotCache, Storage};

let backend = PostgresBackend::connect(&database_url)?;
// Optional. The durable SQL namespace prevents cache mixing across databases.
let cache = SnapshotCache::new(backend.cache_namespace());
let storage = Storage::new(backend.with_cache(cache)?);
let state = opencanvas_server::AppState::new(storage, server_config);
let app = opencanvas_server::router(state);
```

An embedded server can omit cache construction and `with_cache` for PostgreSQL
alone. A cache configured for another database namespace returns
`INVALID_CACHE_CONFIG`.

## Connections and timeouts

`PostgresBackend::with_config(PostgresConfig::new(database_url))` accepts these
public configuration fields:

| Field | Default | Meaning |
| --- | --- | --- |
| `pool_size` | 4 | Dedicated reusable synchronous client workers; allowed 1–32 |
| `connect_timeout` | 3 seconds | Driver socket connection attempt limit |
| `statement_timeout` | 15 seconds | PostgreSQL statement limit |
| `lock_timeout` | 3 seconds | PostgreSQL lock acquisition limit |
| `operation_timeout` | 30 seconds | Total caller checkout and result wait limit |

All methods block and belong on the runtime's existing `spawn_blocking` boundary.
Workers own the synchronous driver's reactor; no reactor is entered on a caller
thread. There is no mutex serializing independent project reads or writes.
Connections are reused and closed connections are discarded. Network keepalive
and TCP user timeout are also configured; operating-system support varies.

The caller wait limit also covers a stalled authentication exchange or broken
network after a statement was sent. A stalled worker retains its bounded pool
slot until transport recovery; additional workers are not spawned. A timed-out
write may have committed: retry its identical idempotency key and body to recover
the original result. No successful acknowledgment precedes durable SQL COMMIT.

The initial transport uses `NoTls`. Use it for a trusted local connection or an
explicitly secured local tunnel/proxy. `sslmode=require` is unsupported and fails
without exposing the connection string. Native TLS configuration is a later
deployment concern, not an implicit downgrade of a required TLS connection.

## Schema and persistence

Construction transactionally initializes version 1 of the separate
`opencanvas_runtime_v1` schema and its durable random cache namespace. An
unsupported schema registry version fails before product table initialization.
Construction creates no projects or demo data and touches no local data root.

Projects have monotonic identity generations. Mutations acquire the project row
`FOR UPDATE`, then run a separate replay query before checking the expected
revision. The read-committed replay query sees a writer that released that lock.
All changed entity rows, revision, whole commit record, canonical request hash,
original response and any asset bytes commit together with
`synchronous_commit=on`. Entity upserts/deletions use set-based arrays and persist
only entities whose serialized domain value changed.

Snapshots and resource/font reads use repeatable-read transactions. Entity
payloads and history/replay values are opaque escaped JSON in SQL `TEXT`;
project names are UTF-8 `BYTEA`. This preserves existing NUL and JSON-number
semantics that PostgreSQL `JSONB`/literal text would otherwise reject. Assets are
project-scoped `BYTEA`, verified against their manifests; oversized resource
payloads are rejected before download. The pinned bundled font is seeded only
by explicit project creation. `apply_batch_file` inbox staging returns
`STAGING_UNSUPPORTED`; `import_asset` paths use the local inbox above.

Validation loads the full authored candidate and uses the existing product
validators. Moka is optional for snapshot reads. Every hit requires a
successful PostgreSQL identity probe, and writes bypass the cache. Rotating
the SQL namespace makes an already connected backend bypass its old cache until
it reconnects with the new namespace. See [snapshot cache behavior](MOKA-SNAPSHOT-CACHE.md).

SQLite migration, production cutover/rollback, backup/restore administration,
retention pruning and Kubernetes are separate work. A
database restore must rotate its cache namespace if it reuses prior project
generation/revision values while preserving different authored content.
