# opencanvas-storage

Reusable authored-domain storage, no HTTP/MCP/renderer dependencies. `Store::open`
requires an explicit data root and owns a process-wide filesystem lock until the
last project handle is dropped. It creates no project. `create_project` is the
only project initialization path. Project IDs are stable UUID-based opaque IDs;
slug accepts 1–64 lowercase ASCII letters/digits/internal hyphens.

```rust,no_run
use opencanvas_storage::*;
let store = Store::open("/explicit/test-or-user-data-root")?;
let project = store.create_project("website", "Website")?;
let snapshot = project.snapshot()?; // revision 0, font_bundled metadata
# Ok::<(), StorageError>(())
```

Public DTOs use camelCase JSON, `Operation` uses a discriminant `op` with
snake_case values. `BatchRequest` contains `projectId`, `expectedRevision`,
`idempotencyKey`, `operations`. Supported operations:

- `create_node`: `id`, `kind`, `parentId`, `props`, optional sibling `index`.
- `update_node`: `id`, `props` merged by top-level property. This is not JSON Patch;
  nested layout/crop objects are replaced, and null is not a delete instruction.
- `reparent_node`: `id`, `parentId`, optional `index` (append by default).
- `reorder_node`: `id`, `index` among siblings; `delete_subtree`: `id`.
- `upsert_token`: `token: {id,name,type,value}`; types color/dimension/number,
  literals only. `upsert_style`: `style: {id,name,kind,value}`; kinds fill/text.

Refs are `{tokenId: id}` in supported typed properties, `{styleId: id}` in fill,
or top-level `styleId` for a fill/text style. Styles may bind tokens but cannot
alias styles. Forward refs are rejected: create/upsert a target before referring
to it in the same ordered batch. A final full-model validation also catches type
changes that would invalidate existing refs. `resolved_nodes(&snapshot)` applies
styles and refs without changing authored JSON, and converts public image
`fit: "fit"` (stretch) to renderer `"fill"`. Other public fit values: contain/cover.

Page roots; page/frame/group containers; rectangle/text/image leaves. Root frames
require fixed width/height. Sizes accept finite numbers, fill or hug; row/column/
free layouts accept numeric or dimension-token padding/gap. Fill in a hug or
unspecified parent axis is rejected. Supported properties are explicitly checked
in `src/validate/properties.rs`. Crop is image-only, normalized source x/y/width/
height with positive dimensions wholly inside [0,1]. Hex color formats only.
Numeric dimensions/positions <= 100,000 absolute; font size/line height 1–10,000;
text <=64 KiB. Text requires text/fontId/fontSize/lineHeight, authored or style.

Caps: 500 ops and 2 MiB canonical JSON per batch, 20,000 nodes per project,
128 ancestor depth, 10 MiB per asset, PNG pixel budget 40 million. These storage
caps are distinct from the renderer's per-frame 2,048-node/16-million-pixel caps.
IDs/idempotency keys are 1–128 ASCII letters/digits/underscore/hyphen.

Each cached `Arc<Project>` has one writer mutex; snapshots open independent read
connections, materialize in a read transaction, then release it. The caller can
serialize/send/render the owned snapshot without retaining a DB transaction.
The writer checks canonical request-hash replay before revision, applies candidate
operations in RAM, validates, diffs rows and commits snapshot+revision+commit+
idempotency together. Only the durable result is returned. Render failure never
rolls back authored state. `commits_after` returns ordered durable replay batches
(cap 1,000/call). Commit and idempotency retention: entire project lifetime in v1;
`retention` in snapshots announces this policy. Future pruning requires a schema
and API policy update. No historical full-snapshot API is promised.

SQLite is bundled/pinned by Cargo.lock (rusqlite 0.41.0, libsqlite3-sys 0.39.1,
SQLite 3.54.1). Startup rejects runtime <3.51.3. WAL, synchronous FULL, FK ON,
1,500-ms busy timeout, 1,000-page autocheckpoint, 16-MiB journal size limit;
fullfsync and checkpoint_fullfsync ON for macOS. Passive checkpoint is explicit,
never forced per batch. `STORAGE_BUSY` is retryable and distinct from
`REVISION_CONFLICT`; validation/key reuse are not retryable. Public errors omit
filesystem paths and SQLite messages. Unknown schemas are rejected without
attempting a destructive migration. Backups/migrations/bundle import are M4.

`import_asset(ImportAssetRequest)` accepts bounded bytes, not a local path.
PNG/JPEG/WebP/TTF/OTF/WOFF2 signatures are checked; PNG dimensions are bounded.
Full decode and font shaping readiness are checked by the render worker, so
metadata `ready` means durable bytes are present, not successful decode. Optional
`FontRegistration` records imported font metadata in the same DB commit. Binaries
are immutable SHA-256 keys: temp write -> file sync -> rename -> directory sync ->
DB commit. `read_asset` checks containment and hash. Snapshot reads expose missing
binary status. Orphans can be listed but are never deleted automatically; handles
for render/export and future revision manifests must be considered before GC.

Tests use temp roots and do not write `~/.opencanvas`. Run:

```sh
cargo test -p opencanvas-storage --features fault-injection -- --test-threads=2
cargo build --release -p opencanvas-storage --example storage_spike
/usr/bin/time -l target/release/examples/storage_spike > spikes/storage/results.json
```

The opt-in `fault-injection` feature terminates child tests with `exit(91)` at
selected boundaries, bypassing Rust destructors/SQLite close. It is absent from
production defaults. It verifies abrupt-process recovery, not physical power-loss
testing or durability of a storage controller that ignores fsync.
