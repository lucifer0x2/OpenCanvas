# Storage backend contract

The application runtime uses `Storage`, a cloneable handle to
`Arc<dyn StorageBackend>`. `ProjectHandle` stores a canonical project ID and that
same handle. Server HTTP/MCP reads, mutations, history and resource reads use these
facades; scene/render/export jobs receive the same `Storage` dependency.
`StorageBackend` is object-safe, synchronous, `Send + Sync`, and accepts only
shared domain models, requests, results and client-safe `StorageError` values.
Connections, transactions, database paths and driver types do not cross it.

SQLite remains the default runtime engine. The existing `Store` and `Project`
APIs remain available for SQLite callers. `From<Store> for Storage` wraps a
`SqliteBackend`, so existing `AppState::new(store, config)` callers keep working.
The adapter delegates validation and durable writes to the existing implementation.
Its bounded strong project pins keep the SQLite Weak registry hot without putting
strong project references inside the root. Lookup checks identity when directories
are removed or replaced; an old ID must never resolve to a replacement project.

## Implementing and injecting an adapter

Implement every `StorageBackend` method in a separate crate or module. Its domain
operations are list/create/project metadata, snapshot, apply/dry-run batch,
commit replay, asset import/read, font read and staged operations read. There are
no successful default methods. The full signatures are in
[`backend.rs`](../crates/storage/src/backend.rs).

At the composition root, substitute the implementation once:

```rust,ignore
let backend = MyBackend::connect(configuration)?;
let storage = opencanvas_storage::Storage::new(backend);
let state = opencanvas_server::AppState::new(storage, server_config);
let app = opencanvas_server::router(state);
```

An already shared adapter can use `Storage::from_shared(Arc<dyn StorageBackend>)`.
Do not add engine selection to individual routes, MCP tools or jobs. The current
binary still constructs `Store`; selecting another production engine requires an
explicit composition-root configuration change.

All methods are **blocking**. Server access runs through the existing
`spawn_blocking` boundary. An adapter must bound driver/network waits and use an
appropriate synchronous driver or a deliberately managed worker. Calling
`block_on` on an arbitrary async reactor is not supplied by this interface.
Release transactions and database locks before returning: layout, rendering,
event publication and response streaming happen after storage returns.

## Required semantics

A new database driver does not automatically provide the application contract.
An adapter must implement and verify these behaviors:

- Accept canonical IDs or slugs for lookup; return canonical metadata and require
  each mutation request's project ID to match the selected project. Reused entity
  IDs and idempotency keys remain isolated between projects.
- Validate the complete candidate model and commit a batch atomically. A late
  invalid operation must leave entities, revision, history and replay keys intact.
  Concurrent requests with the same expected revision have one winner; losers
  receive `REVISION_CONFLICT` with the current revision.
- Commit durably before success or event publication. Store the original
  idempotency result and request fingerprint atomically with the mutation. Exact
  retries replay the original result even after later commits; changed requests
  under the same key receive `IDEMPOTENCY_KEY_REUSED`.
- Dry runs perform the same candidate checks without persisting entities,
  revisions, history, resource writes or an idempotency reservation.
- Snapshots represent a consistent revision. `commits_after` returns ordered,
  whole commits and maintains the existing reset/retention behavior.
- Asset/font metadata and bytes stay project scoped and consistent. Return the
  exact stored bytes and media type, preserve import replay and corruption/path
  checks, and keep internal paths and credentials out of public errors.

The existing model validators and resource rules remain part of that contract.
The trait introduces a substitution boundary; it does not translate SQL schemas,
implement transactions for a new engine or provide migration tooling.

## Staging and engine-specific administration

Legacy `BatchFileRequest.path` is still accepted by the application API.
`ProjectHandle` asks the backend to read its data-only operations, then calls the
ordinary apply/dry-run domain method. SQLite uses its guarded per-project inbox,
including containment, symlink, size and JSON checks. The backend owns what a
staging key means; there is no assumption that another engine shares a local
filesystem. An adapter without staging must return an explicit unsupported error.

WAL checkpoints, SQLite diagnostics, orphan inventory, SQLite backup/bundle
administration and `db.sql` migration stay on the existing concrete APIs. They
are not fabricated as generic operations. A future engine needs its own relevant
administration and deployment decisions.

## Verification and current limits

The `backend_contract` and `backend_contract_resources` storage tests exercise
SQLite through `Arc<dyn StorageBackend>` and the facades: lookup/scope, replay,
conflicts, rollback, dry run, resource bytes, staging guards, path replacement
and final root-lock release.

The server `storage_backend` test injects an independent in-memory fixture into
the real `AppState` and router. It verifies HTTP project create/list/snapshot,
MCP mutation/replay/change history and HTTP asset/font downloads. The fixture has
no SQLite `Store`, driver or filesystem implementation; call tracing and
`memory_` identities prove that these routes reach the injected backend. Its
unsupported operations fail explicitly. This narrow fixture proves substitution,
not the completeness or durability of a second production engine.

Task19 implements `PostgresBackend` through this trait in runtime storage. It
reuses the full product candidate validators and commits entity deltas, revision,
history, replay and project-scoped resource bytes atomically. Optional Moka
keeps one typed snapshot per project in process with a one-hour TTL. Each hit
requires an authoritative PostgreSQL check of namespace, canonical project ID,
generation and revision. See the [adapter guide](POSTGRES-BACKEND.md),
[Moka cache guide](MOKA-SNAPSHOT-CACHE.md) and
[original Task19 results](TASK19-REPORT.md).

The server defaults to SQLite. Set `DATABASE_PROVIDER=postgresql` with
`DATABASE_URL` to select PostgreSQL and enable its snapshot cache.
SQLite migration, production cutover/rollback and deployment remain separate
work. Task19 uses full candidate validation. Existing render/export and
end-to-end regression results are tracked separately in
[`TASK16-REPORT.md`](TASK16-REPORT.md).
