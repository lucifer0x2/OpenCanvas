# Task22 report — design-session token efficiency

Spec: [`tasks/22-design-token-efficiency.md`](../tasks/22-design-token-efficiency.md).
Each item has its own section; items land independently. Nothing here is
committed or deployed; the root session reviews the diff first.

## 22a — `import_asset` from the project inbox

Branch `feat/import-asset-inbox` (from `main` @`0f42dd1`), 2026-11-09.

### Result

- `import_asset` takes the bytes from exactly one of `path` or `base64`.
  `tools/list` shows the flat union (both optional, `mediaType` still required);
  `schemas::validate` enforces a strict `oneOf` of two closed shapes, the same
  pattern as `render_frame` (`crates/server/src/schemas/import.rs`).
- `path` is inbox-relative or absolute inside `<data-root>/<slug>/inbox`.
  Traversal (`..`, `.`), symlinks (file or any directory component), FIFOs and
  directories fail with `PATH_REJECTED`; a missing file or inbox fails with
  `FILE_NOT_FOUND`; files over 10 MiB fail with `VALIDATION_FAILED` field `path`
  before being read (metadata length, then a bounded `take(max + 1)` read).
- Works on SQLite and PostgreSQL over MCP; both verified (see Checks).
- Tool description, `instructions.rs` and `skills/author-frame.md` now say: save
  the file in the inbox and pass `path`, never print or paste base64, `base64`
  only without filesystem access.

### Design decisions

- **Shared inbox walk.** `crates/storage/src/batch_file/inbox.rs` moved to
  `crates/storage/src/inbox.rs` and gained `read(root, project, input, Purpose)`.
  It contains the regular-file check, the size bound and the stream bound that
  previously lived in `Project::read_batch_file_operations`. `Purpose::{Batch,
  Asset}` selects only the error wording and the byte limit (2 MiB / 10 MiB). Batch
  codes, messages, fields and check order are unchanged; `batch_file`,
  `inbox_guards`, `structured_batch` and `scoped_concurrency` pass unmodified.
- **Inbox root is engine-neutral.** `Storage` carries an optional local inbox
  root (`Storage::with_inbox_root`), and `ProjectHandle::read_inbox_asset(path)`
  resolves `<root>/<slug>/inbox` with the project's slug from `project_meta`.
  `Storage::from(Store)` sets the SQLite data root automatically. In PostgreSQL
  mode, startup passes the configured data root (`--data-root`,
  `OPENCANVAS_DATA_ROOT`, default `~/.opencanvas`). PostgreSQL still stores
  nothing there and creates no directory; the root is canonicalized per read, so
  it may appear after startup. A storage without a root returns
  `STAGING_UNSUPPORTED`.
- **Signature checks.** Storage already rejected a signature that contradicts
  `mediaType` (PNG incl. IHDR, JPEG, WebP, TTF, OTF, WOFF2) in
  `assets::asset_dimensions`. Both backends call it for every import, so the check
  covers `path` and `base64` without new code. Existing base64 behaviour is
  unchanged: no existing test or fixture sends a mismatched signature, and all of
  them pass. The new tests pin the rejection for both sources
  (`VALIDATION_FAILED`, field `mediaType`).
- **Hashing and replay.** The canonical request hash and the asset hash were
  already computed from decoded bytes (`hash` field, never base64 or filename).
  The server reads `path` into the same `ImportAssetRequest.bytes`. The same key
  with the same bytes replays across `path`/`base64` in either order, and changed
  bytes return `IDEMPOTENCY_KEY_REUSED`. The file read and the import run in one
  blocking storage task.
- **Module split.** The `import_asset` request handling moved from `domain.rs` to
  `domain/import.rs`, and its schema from `schemas.rs` to `schemas/import.rs`, so
  both parent files shrank.

### Sizes

| Measure | Before | After |
| --- | --- | --- |
| `tools/list` total (`TOTAL_MAX` 32 768) | 29 516 B | 29 865 B (+349) |
| `import_asset` tool (`PER_TOOL_MAX` 8 192) | 1 205 B | 1 554 B |
| `instructions` (`CONNECT`) | 10 134 B | 10 327 B (+193) |
| `skill://opencanvas/author-frame.md` | 20 494 B | 20 926 B (+432) |

Limits were not raised; 2 903 B of `tools/list` headroom remains. There is no
instructions budget test. Commands: `cargo test -p opencanvas-server --test
tools_budget -- --nocapture` before any edit and after the last one.

### Checks run

All passed. No failures were observed, so nothing is excluded as pre-existing.
`stroke_props` passed in this run.

- `cargo test -p opencanvas-server --lib --test tools_budget --test tool_schemas
  --test skill_layer --test structured_batch --test scoped_concurrency --test
  structured_authoring --test import_inbox --test changes --test conflict_changes
  --test startup_env --test startup_backend --test storage_backend --test
  font_portability --test native_binary`: lib 81 (including 2 new schema tests),
  tools_budget 1, tool_schemas 2, skill_layer 2, structured_batch 3,
  scoped_concurrency 2 (+1 PostgreSQL ignored), structured_authoring 3,
  import_inbox 3 (new), changes 5, conflict_changes 3, startup_env 9,
  startup_backend 5, storage_backend 1, font_portability 2, native_binary 1.
- `cargo test -p opencanvas-server --test animation_context --test
  navigation_context --test states_context --test export_svg --test image_fit
  --test multi_resources --test public_resources --test font_face_context --test
  native_png --test native_compute --test render --test text_rendering`: 5, 4, 3,
  2, 2, 2, 1, 2, 4, 3, 2 and 1 passed (base64 import users and instruction checks).
- `cargo test -p opencanvas-storage`: 163 passed, 0 failed, 28 ignored, including
  `batch_file` 4, `backend_contract_resources` 4, `assets` 2 and `inbox_assets`
  2 (new, +1 PostgreSQL ignored).
- PostgreSQL ran on a fresh, disposable Homebrew PostgreSQL 16.15 cluster in the
  session scratch directory: TCP loopback on a random port, trust auth, stopped
  and deleted afterwards. It did not run through Docker's
  `scripts/task19-check.sh`. With `TASK19_POSTGRES_URL` set:
  - `cargo test -p opencanvas-storage --test postgres_resources --test
    postgres_contract --test inbox_assets -- --include-ignored --test-threads=1`:
    2, 2 and 3 passed.
  - `cargo test -p opencanvas-server --test postgres_import_inbox --test
    postgres_backend --test scoped_concurrency -- --include-ignored`: 1 (new), 2
    and 3 passed.
- `cargo fmt -p opencanvas-server -p opencanvas-storage -- --check`: clean.
- `cargo clippy -p opencanvas-storage -p opencanvas-server`: no warnings. Also
  clean: `cargo clippy` on the new test targets (`import_inbox`,
  `postgres_import_inbox`, `inbox_assets`).

New tests cover:

- Schema: `path` only, `base64` only and `path` with `font` accepted. Both,
  neither, empty/null/number `path`, array `base64`, extra keys, missing or unknown
  `mediaType` rejected. The published schema equals the union of the strict
  shapes.
- Security over MCP: `..`, `./`, absolute outside the inbox, `/etc/hosts`, file
  symlink, directory-component symlink, directory, missing file, file over 10 MiB.
  No error leaks the data-root path, and the revision is unchanged.
- Signatures: PNG as WOFF2/JPEG, WOFF2 as PNG/TTF (path), WOFF2 as PNG and JPEG as
  WebP (base64), each with matching-type positives.
- Replay: base64 then path, and absolute path then base64, under one key.
  Changed bytes under a used key conflict. The same contract runs at storage level
  for SQLite and PostgreSQL.

### Remaining limitations

- `apply_batch_file` on PostgreSQL still returns `STAGING_UNSUPPORTED` (unchanged
  by request). Only `import_asset` uses the PostgreSQL inbox root. Enabling batch
  files there would reuse the same `Storage` root and needs its own decision.
- Replaying with `path` re-reads the file, so a retry after a lost response needs
  the same bytes still in the inbox. `SERVER-OPERATIONS.md` says to keep the file
  until success.
- Absolute paths must use the canonical inbox path. An alias through a symlinked
  data root is rejected, as for batch files.
- PostgreSQL defaults the inbox root to `~/.opencanvas` when no data root is given.
  The native runtime passes `--data-root …/runtime/unused-sqlite`, so its inbox is
  `…/runtime/unused-sqlite/<slug>/inbox` until the deployment chooses a clearer
  root. Nothing was deployed or restarted.
- Inbox reads require Unix (unchanged); other platforms return
  `UNSUPPORTED_PLATFORM`.
- `crates/server/skills/author-frame.md` is now 323 lines. It is a skill
  document, not handwritten code, so it was not split.

## 22b — `import_asset` sources: inbox path and cross-project copy (OC-8)

Branch `feat/import-asset-sources` (from `main` @`e413793`), 2026-11-14.

### Result

- 22a merged onto current `main`. The only conflict was the module list in
  `crates/storage/src/lib.rs` (main's `revision_indexes` next to the branch's
  `inbox`). `apply_batch_file` and `import_asset` share one resolver,
  `crates/storage/src/inbox.rs`.
- `import_asset` also takes `sourceProjectId` (ID or slug) plus exactly one of
  `assetId` or `fontId`, with no `mediaType`, `font`, `path` or `base64`. The
  server reads the source through `ProjectHandle::read_asset` / `read_font`, which
  check the bytes against the manifest hash. It then imports them as an ordinary
  content-addressed import (`crates/server/src/domain/import/copy.rs`). The asset
  keeps its ID. `fontId` also copies the registration (ID, family, weight, style,
  license), so text can use the font at once. Replay with the same key returns
  `replayed:true`. `font_bundled` is rejected (`INVALID_REQUEST`) because it is
  built in. Missing items return `PROJECT_NOT_FOUND`, `ASSET_NOT_FOUND` or
  `FONT_NOT_FOUND`.
- One item per call. Storage commits one asset per import, so copying several
  faces in one call would need a new multi-asset transaction on both backends.
- The server does not download URLs. A download would add server-side request
  forgery risk, and the loopback MCP client can save the file in the inbox itself.
- The 10 MiB cap stays. Typical fonts are much smaller: Lato TTF is 641 KiB and
  the bundled Noto Sans is 2.0 MiB; WOFF2 files are usually under 200 KiB. Only
  full CJK families exceed 10 MiB, and those can be subset or converted to WOFF2.
  The blocker was carrying base64 through the conversation, not the cap.

### Sizes (before = `main` @`e413793`)

| Measure | Before | After |
| --- | --- | --- |
| discovery workflow total (cap 100 000) | 99 645 B | 99 953 B (+308; ±1 B per run) |
| `tools/list` total (`TOTAL_MAX` 32 768) | 29 734 B | 30 029 B (+295) |
| `import_asset` tool | 1 205 B | 1 500 B |
| `skill://opencanvas/author-frame.md` read | 21 697 B | 21 709 B (+12) |

The thresholds stay the same. The 22a wording would have pushed the discovery
total to 100 739 B, so the import paragraph and tool description were shortened,
and new ID fields use a plain bounded string. The author-frame changes also
remove the stale U07 payload-measurement note and the duplicate traversal
sentence; tool errors report traversal.

### Checks

Render tests need `npm ci`, `npm run build:renderer`, `layout-step` and a
Playwright headless shell (`PLAYWRIGHT_BROWSERS_PATH`).

- `cargo test -p opencanvas-storage --test inbox_assets --test batch_file --test
  assets`: 2 (+1 PostgreSQL ignored), 4, 2 passed.
- `cargo test -p opencanvas-server --lib --test import_copy --test import_inbox
  --test discovery_workflow --test tools_budget --test tool_schemas --test
  skill_layer --test structured_authoring --test structured_batch --test
  font_portability --test postgres_import_inbox --test startup_env --test
  startup_backend`: 147, 2, 3, 1, 1, 2, 2, 3, 3, 2, 0 (2 PostgreSQL ignored),
  9, 5 passed; `font_face_context` 2 passed.
- PostgreSQL copy test `postgres_copies_a_font_with_its_registration_between_projects`
  compiles but was not run (no disposable cluster).
