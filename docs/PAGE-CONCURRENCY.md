# Page-scoped optimistic concurrency

Implemented in source on 2026-11-09. Runtime deployment is a separate action;
this task does not deploy, restart services or mutate live designs. The checkout
and native release have different baselines, so the native candidate must retain
the release's renderer/model changes when this feature is integrated.

`apply_batch` and `apply_batch_file` accept optional `pageIds`. With it,
`expectedRevision` means “the project revision of the design I actually read”.
Storage allows later unrelated commits, while guarding those pages and their
transitive dependencies. Omitting `pageIds` preserves exact global revision
matching. There is still one monotonically increasing project revision for
snapshots, SSE, render jobs, ETags and cache identity; no separate page counter
is introduced.

## Using the MCP contract

Read authored data using `get_subtree` or `get_design_context`, and retain its
revision and page identity. No extra revision-only call is required. For example,
two agents read different existing pages at revision 42 and independently call:

```json
{"name":"apply_batch","arguments":{"projectId":"PROJECT","expectedRevision":42,"pageIds":["page-a"],"idempotencyKey":"edit-a","operations":[{"op":"update_node","id":"title-a","props":{"text":"Updated A"}}]}}
```

```json
{"name":"apply_batch","arguments":{"projectId":"PROJECT","expectedRevision":42,"pageIds":["page-b"],"idempotencyKey":"edit-b","operations":[{"op":"update_node","id":"title-b","props":{"text":"Updated B"}}]}}
```

Both commit if their pages/dependencies are independent, receiving revisions 43
and 44 in writer order. Each scoped response also supplies:

```json
{"editGuard":{"projectId":"PROJECT","expectedRevision":44,"pageIds":["page-b"]}}
```

Use the guard from that successful commit for subsequent work on the same scope.
For diagnostics, `get_edit_context {projectId,pageIds}` returns a guard and
dependency metadata from one authoritative snapshot: `expectedRevision`,
`pageIds`, `dependencyCount`, up to 100 sorted `dependencies` keys, and
`dependenciesTruncated`. It returns no design payload, binary bytes or internal
paths. Read authored data with `revision` equal to this guard, or use the revision
of an existing authored read directly. **Never relabel an older design read with
a newly fetched revision.**

`dryRun:true` performs the same guard and candidate checks without writing,
consuming a key, publishing events or starting rendering. Its `editGuard` uses
the preview's authoritative `baseRevision` and remains provisional. The preview
does not reserve a revision or authorize skipping checks during commit.

After a lost commit response, retry the identical request/key, including
`pageIds` and the original revision. Exact replay precedes stale checks and
returns the original committed revision. Changing scope, revision or operations
under a used key returns `IDEMPOTENCY_KEY_REUSED`.

On `REVISION_CONFLICT`, inspect `changedSince`/`get_changes`, reread the affected
authored pages and dependencies, rebuild the batch and use a new key. On
`SCOPED_OPERATION_UNSUPPORTED`, review the restriction below and reread before
using strict mode. Raising the revision alone discards the concurrency guard.

## Server-derived coverage and restrictions

Clients supply page IDs, never dependency lists. Storage derives the union of
the current and candidate dependency closures under its writer boundary:

- Every guarded page and its complete authored subtree.
- Tokens recursively through aliases, defaults and every named theme; styles
  and all nested supported reference values, including instance overrides.
- Component masters and nested instances. A referenced master guards its entire
  containing page, conservatively including unrelated content on that page.
- Fonts resolved from expanded, placed instances under every theme, the selected
  font families/faces and their backing assets. Family keys catch registration
  changes that can alter face selection.
- Navigation target pages, recursively, when node links reference another page.
  Newly attached dependencies are checked too.

Allowed scoped work is node property editing, descendant creation, reordering,
deletion and reparenting inside existing pages. Cross-page moves require both
source and destination page IDs, even if descendant rows themselves do not
change. All effective side effects must be covered. Full validation on the
current authoritative candidate still enforces ID collisions, references,
layouts, themes and operation ordering, with atomic rollback on failure.

Scoped v1 explicitly rejects:

- Token/style writes, resource imports/registrations and every note operation.
  These retain strict revision matching. Deleting a node that cascades notes is
  rejected as well, including notes attached after the original read.
- Page creation, deletion, reordering and effective page property changes.
- `flowStart` writes and root-frame deletion, reparenting or reordering.
- The first root frame on an empty page, or root creation with an explicit
  index. Appending a root after existing roots without `index` is supported;
  harmless numeric sort-key compaction preserves relative root order.
- Missing/ambiguous scope coverage, incomplete legacy history, malformed or
  unknown history envelopes, more than 1,000 intervening commits, or more than
  8 MiB of intervening encoded history. Refetch to obtain a new base.

Strict page/flow/global-root topology changes invalidate all stale scoped
guards, protecting default navigation selection. Safe root appends preserve
unrelated page guards. Root deletion is rejected in scoped mode even when all
incoming-link source pages are listed; strict mode retains atomic link cleanup.

## Atomicity and persistence

SQLite executes under its existing project writer mutex, then checks the actual
parent revision again inside the `IMMEDIATE` persistence transaction. PostgreSQL
locks the project row and reads/checks/validates/writes within that transaction;
previews use one repeatable-read transaction. There is no server read/check/write
race. Global revisions advance once per committed batch.

Every new strict/scoped batch and resource import records a versioned private
history envelope containing the canonical original request, operation array,
effective page/entity/family footprint, and a checksum covering the envelope.
The durable commit's `baseRevision` remains the actual global parent, which can
differ from a scoped request's older `expectedRevision`. This catches delete/
recreate and change/revert ABA across commits. Missing older coverage fails
closed. No authored snapshot or database schema migration is needed.

Public `commits_after`/`get_changes` unwrap operation arrays; internal envelopes
and request bodies are not exposed. Bundle validation verifies envelope shape,
checksum, canonical request/operation consistency, identity and durable replay
hashes. Import retains its existing new-identity/revision-0/history-reset rules.
Restore preserves replay but scrubs and reseals all imported footprints to
wildcards, so even deliberately forged valid-checksum footprints cannot grant
old guards permission to write. Fresh guards work after restore. Old bundles
with operation arrays remain readable. Older binaries do not understand the
new private history envelope; downgrade/mixed-version history readers are not
supported.

Custom `StorageBackend` implementations retain source compatibility through
default scoped methods returning `STORAGE_UNSUPPORTED`. SQLite and PostgreSQL
implement the scoped operations. PostgreSQL continues returning
`STAGING_UNSUPPORTED` for file inboxes. Public routing/privacy, existing snapshot
representations and cache identity are unchanged.

## Verification

Worker checks used only temporary data roots. Actual commands/results:

- `rtk bypass cargo test --offline -p opencanvas-storage --test scoped_bundle
  --test scoped_concurrency --test scoped_navigation -- --test-threads=1`:
  5 passed, 1 PostgreSQL case ignored. Covers parallel pages, same-page conflicts,
  ABA, preview/replay/key collision, transitive tokens/styles/nested masters,
  expanded-instance font overrides with changed font bytes, restrictions,
  navigation gaps and untrusted bundle footprints.
- Storage `scoped_history` and existing `bundle_roundtrip`: 6 passed. Covers
  restart, import/restore and stale scoped replay with original read revision
  differing from actual parent, malformed/gapped/legacy history, and root deletion
  with incoming cross-page links.
- Real SQLite MCP `scoped_concurrency`: 2 passed; existing `structured_batch`:
  3 passed. Inline/file scoped batches, preview/replay, dependency conflicts,
  unwrapped changes, legacy compatibility and schema validation are exercised.
- A disposable native PostgreSQL 16 cluster in a temporary directory on loopback
  55479 ran storage `scoped_concurrency -- --ignored --test-threads=1` and server
  `scoped_concurrency -- --ignored --test-threads=1`: 1 + 1 passed. Includes
  durable scoped replay after closing/reopening backend pools. Trap cleanup
  stopped the temporary server and removed its directory. Live port 2703 and
  live runtime configuration were not accessed. Initial sandbox attempts could
  not create PostgreSQL shared memory or bind MCP loopback; authorized escalated
  temporary runs succeeded.
- `rtk bypass cargo clippy --offline -p opencanvas-storage -p opencanvas-server
  --lib -- -D warnings`, format and diff checks passed.

For portable reproduction with fresh Docker PostgreSQL, run each ignored suite
through `scripts/task19-check.sh`, for example:

```sh
scripts/task19-check.sh rtk bypass cargo test --offline -p opencanvas-storage --test scoped_concurrency -- --ignored --test-threads=1
scripts/task19-check.sh rtk bypass cargo test --offline -p opencanvas-server --test scoped_concurrency -- --ignored --test-threads=1
```

Root independently reported 36 relevant storage/server checks passed, including
both backends and actual MCP. After the navigation correction, root reran all
eight new SQLite storage checks successfully. An independent reviewer found no
remaining P1/P2 findings after the relative-order correction.

Modified product Rust files are 32–238 lines; new modules are `scoped.rs` 179,
`scoped/dependencies.rs` 131, `scoped/history.rs` 165 and
`scoped/navigation.rs` 51 lines. No modified product Rust file exceeds 250 lines;
the largest is existing PostgreSQL resources at 238. Modules separate guard
policy, reference closure, durable envelopes and navigation topology. No release
build, commit, push or live installation is part of this change. MCP clients caching
tool schemas/instructions need refresh/reconnection after an authorized release.
