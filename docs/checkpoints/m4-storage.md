# M4 storage checkpoint — runtime hardening, 2026-10-28

Workspace `/Users/luis/Workspace/OpenCanvas`, shared dirty branch `main`. No
commit/reset/checkout/push, dependency changes, child agents or independent chats
were made by this continuation worker. Original compile-only work was reviewed
and completed in place. No background process remains after commands complete.

## API and changes

```rust
Project::export_bundle(&self) -> Result<ProjectBundle>
Store::import_bundle(&self, slug: &str, bytes: &[u8]) -> Result<Arc<Project>>
Store::restore_bundle(&self, slug: &str, bytes: &[u8]) -> Result<Arc<Project>>
```

API/dependencies unchanged from previous checkpoint. Import uses new ID/revision
0/cleared replay; restore preserves ID/R/history and rejects existing identity or
slug. `docs/BACKUP.md` gives format, limits and operation semantics.

- `backup.rs`: complete SQLite backup target switches to standalone DELETE mode
  before size cap/read; busy capture is retryable.
- `db.rs`, `migration.rs`: retention stays within the snapshot transaction and
  validates the supported policy; signed SQLite revision representation.
- `bundle.rs`: bounded allowlisted regular USTAR; rejects malformed, truncated,
  duplicate, traversal, link, extension and trailing-data archives.
- `bundle_validation.rs`, new `bundle_history.rs`: exact schema/integrity/FKs,
  row-count preflight, resource identities/signatures/dimensions, authored refs,
  complete replay and reconstructed canonical request hashes.
- `bundle_import.rs`, `store.rs`: private staging then synced atomic rename;
  canonical root checks before staging/activation/create/list/open reject a root
  replaced with a symlink. No existing slug/identity is overwritten.
- `assets.rs`, with explicit root permission: reuse signature/dimension helper;
  orphan inventory retains bundled-font copies referenced by font metadata. No
  asset import semantics or GC/deletion added.
- Tests: `migration.rs`, `bundle_roundtrip.rs`, `bundle_archive.rs`,
  `bundle_database.rs`, focused `bundle_support/mod.rs` helper.

Root review caught one first-draft compatibility bug: DeleteSubtree/ReparentNode
can leave valid numeric sort-key gaps. History validation now preserves them and
deterministic `(sortKey,id)` ordering. Regression covers both operations through
import/restore. No resolver/validator behavior was changed.

## Actual commands/results

1. Initial fixture compile failed on SQLite FromSql u64/usize; changed fixture
   reads to i64. First roundtrip used unsupported style kind paint; corrected to
   supported fill style.
2. Migration + roundtrip targeted command: 5 passed / 2 suites / 4.72 s.
3. Four-suite targeted command: 10 passed / 4 suites / 9.02 s; then root symlink
   and request-hash checks: 11 passed / 4 suites / 9.34 s.
4. Final raw-output command: `rtk bypass cargo test -p opencanvas-storage --offline
   --test migration --test bundle_roundtrip --test bundle_archive --test
   bundle_database -- --test-threads=2`: **12/12 passed**. Archive 4/4 (1.28 s),
   database 2/2 (3.41 s), roundtrip 3/3 (5.34 s), migration 3/3 (0.14 s), after
   0.98 s compilation. Root notified code is stable for the one integrated full
   storage run; this worker does not duplicate it.
5. Targeted rustfmt used `--edition 2021 --config skip_children=true` on owned
   modules/tests, preserving M3-owned validator children.

All roots are tempfile-managed, never `~/.opencanvas`. Concurrent workload uses
one writer (16 asset imports + 16 authored updates) and eight backups; each is
restored separately and checked for revision/history/resources. An owned older
snapshot remains resolvable. A real render worker/browser is not part of these
storage tests.

## Remaining limits/integration

- Rust API only; no backup CLI/HTTP/MCP endpoint added.
- No asset GC, downgrade, compression, overwrite restore, v1 DB bundle or hostile
  filesystem-race immunity. Storage backup correctness is covered under normal
  application writes and owned reader/resolver snapshots.
- Root runs full storage with fault-injection, then coordinates real backend,
  viewer/PNG/restart acceptance and overall docs.
- Final targeted `rustfmt --check --edition 2021 --config skip_children=true`
  passed. New owned source/test modules remain below 250 lines; largest source
  is bundle.rs (207), largest new test bundle_roundtrip.rs (208). Existing
  assets.rs is 213 lines after the approved helper/inventory change. No new
  size exception is needed. Full source counts are in the tool output.
