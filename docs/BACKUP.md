# Local project backup and recovery

This milestone exposes Rust storage APIs; no backup CLI or browser control is
added. Supply an explicit root to `Store::open`. Opening a store does not create
a project. The root lock lasts while any Store/Project handle still owns it.

```rust,ignore
let bundle = project.export_bundle()?;
// The calling application can persist bundle.bytes at its chosen location.
let copy = store.import_bundle("new-copy", &bundle.bytes)?;
let recovered = another_store.restore_bundle("recovered", &bundle.bytes)?;
```

`ProjectBundle` contains `manifest: BundleManifest` and `bytes: Vec<u8>`. The
manifest reports source project ID, captured revision R, database schema version
and SHA-256/byte length for each file.

## Identity, history and consistent capture

Import creates a new project ID at revision 0, preserving authored node, token,
style, asset and font IDs/data/hashes. Commit/idempotency history is cleared, so a
former key can be used for a new request. Restore preserves source identity,
revision R and both histories. Retrying an original request with its original
project ID/base revision/key returns its recorded result. A new slug changes the
folder and metadata slug only. Existing destination slugs and existing source
identities in the same data root return `PROJECT_EXISTS`; no overwrite occurs.
Recover into an unused root if the identity still exists in the current root.

Export uses SQLite's online backup API through an independent reader. It does
not copy a live main DB/WAL/SHM. The completed target switches to DELETE journal
mode, making `db.sql` self-contained. Its owned snapshot selects R; resource reads
use that snapshot. No writer mutex or read transaction survives into binary I/O
or TAR serialization. Writers can advance the live project during export. Busy
capture returns retryable `STORAGE_BUSY`.

Immutable content-addressed assets become durable before their DB references
commit. Export checks every needed size/hash. There is no resource deletion or
GC: older render snapshots and exports retain their bytes under application
writes. `orphan_asset_keys` is read-only inventory and retains bundled-font copies
referenced by committed font metadata. Future GC needs ownership/pinning first.

## Format, validation and limits

Bundle version 1 is uncompressed TAR containing regular USTAR files, ending in
zero blocks. Allowed paths are exactly `manifest.json`, `db.sql`,
`assets/<64 lowercase hex SHA-256>` and `licenses/OFL.txt`. The exact pinned Noto
Sans font/license accompany uploaded resources. Live unreferenced files and
SQLite sidecars are excluded. Restored resource binaries reside in `assets/`;
the built-in font also remains available from the bundled runtime.

Bounds: 128 MiB archive, 64 MiB standalone DB, 10 MiB per asset/font, 2 MiB
manifest, 4,096 non-manifest files. DB preflight caps nodes/styles at 20,000 each,
tokens at 4,096, asset rows at 4,093 and font rows at 4,096 before snapshot loading.
File bytes reserve 2 MiB for manifest overhead; headers/final TAR must also fit
the archive limit.

Import requires the exact manifest file set and verifies every hash/size. It
rejects duplicate paths, absolute/relative traversal, backslashes, links,
directories, GNU/PAX extensions, truncation and nonzero trailing data. It never
uses TAR extraction or accepts client filesystem paths. Slugs follow project
creation guards. Destination/root/asset symlinks are rejected, including a root
replaced by a symlink before use. These checks do not promise immunity to a
hostile same-user process continuously racing filesystem replacements.

Before activation, validation checks exact SQLite schema objects, version,
integrity/FKs, singleton/row metadata, typed authored references, resource
signatures/dimensions/hashes, fonts and the supported retention policy. Commits
must cover 1..=R, with matching idempotency results and reconstructed canonical
request hashes. Numeric sort keys preserve gaps from delete/reparent; ordering is
`(sortKey,id)`. Digests establish integrity, not authentication.

Validation and synced writes occur in private staging; a directory rename makes
the validated destination visible. The root registry lock serializes activation
with create/open. Failed validation leaves no destination/staging directory.
Malformed/unsupported bundles return `BUNDLE_INVALID`; path/conflict failures
retain specific codes. Errors contain no private paths.

## Migration policy

Internal database schema 4 is separate from layout input schema 1. Opening an
on-disk schema-1, schema-2 or schema-3 project upgrades before writer setup. Every step
(schema 2 adds `retention_metadata`, schema 3 adds the `notes` table, and schema 4
marks support for component/instance nodes without changing SQL columns) and the
project metadata/PRAGMA version change run in one transaction. Authored IDs,
revision, binary references and idempotency history stay intact. Supported
retention is commits from 1 and lifetime idempotency, without expiration.

SQL failure returns `MIGRATION_FAILED` and rolls back DDL/data/version changes.
Resolve the failure before reopening; do not bypass it by editing versions.
Future versions or disagreeing PRAGMA/metadata versions return
`SCHEMA_UNSUPPORTED`. Bundle v1 accepts database schemas 1–4. It validates the exact
schema for the declared version before upgrading only its private staging database;
unexpected tables, views, triggers or mismatched versions remain rejected. Export
always writes schema 4 and retains authored components, instance references and
overrides; virtual children are never persisted. There is no public bundle route yet.
Note rows are bounded (5000), their JSON `id` must equal the row and each note must
anchor to an existing node. Schema 4 is one-way:
an older binary reports `SCHEMA_UNSUPPORTED` rather than reading it. No downgrade or overwrite restore is provided.

## Executed verification

The targeted command is `cargo test -p opencanvas-storage --offline --test
migration --test bundle_roundtrip --test bundle_archive --test bundle_database
-- --test-threads=2`; final results are in `docs/checkpoints/m4-storage.md`.

Fixtures cover real v1 upgrade and trigger-induced failure after DDL; authored,
image and uploaded-font import/restore/restart/replay; order gaps and orphan
inventory; writer plus repeated backup/restore and owned resolver snapshots;
archive/schema/metadata/replay/resource rejection; bounds; and symlink guards.
All data roots are temporary. This does not claim power-loss hardware simulation
or real render/browser E2E. Root coordinates integrated acceptance separately.
