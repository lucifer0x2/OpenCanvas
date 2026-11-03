# Task14 lane B — bulk JSON and dry run

Completed 2026-11-02 on the shared `claude/design-properties` checkout. No commit,
push, real-server restart or user data write. Scope: P4–P5 only; theme/instance
and authoring work belong to the other lanes.

## Contract and implementation

- `apply_batch_file {projectId,expectedRevision,idempotencyKey,path,dryRun?}`
  reads one pure JSON operation array from that project's inbox. Relative paths
  start in inbox; absolute paths must use its canonical path. New projects create
  inbox; older projects require the filesystem-capable caller to create it.
- The read uses descriptor-relative `openat` with no-follow traversal, a final
  regular-file `fstatat` preflight, nonblocking/no-controlling-terminal open, and
  post-open regular-file validation. Symlink files, symlink inbox/intermediate
  directories, traversal, outside paths, directories and FIFOs are rejected.
  Client errors never echo a local absolute path.
- Metadata is checked before reading, and stream reading is capped at 2 MiB + 1
  byte to catch concurrent growth. The JSON must contain 1–500 operations. The
  normalized complete request retains the existing 2 MiB cap. No code execution,
  heuristic rejection of repeated operations, or automatic partial chunking.
- File JSON is checked against the original public operation schema before serde
  normalizes optional fields. Both transports converge on the same storage
  candidate validation, writer lock, revision check, atomic persistence and
  canonical hash. Hashing excludes the file path and dryRun flag, and preserves
  the existing request project/revision/key/operations semantics.
- `apply_batch dryRun:true` and file previews return
  `{projectId,dryRun:true,baseRevision,changedIds,diagnostics:[]}`. They check
  request bounds/IDs, read-only existing-key collisions, revision, ordered
  operations, pruning, references and the full candidate. Matching prior keys do
  not return a committed response during preview: candidate/revision checks run.
  Failed validation returns the normal error envelope.
- Preview does not persist, increment revision, reserve a key, publish events,
  enqueue prerender, or modify assets. Successful previews have no render/layout
  diagnostics because they deliberately do not run rendering or measurement.
  The public Rust `BatchRequest` and `BatchResult` structs remain unchanged.

Usage and limits: [BULK-OPERATIONS.md](BULK-OPERATIONS.md).

## Checks actually run

All test roots are temporary; MCP servers bind `127.0.0.1:0` and are dropped by
the harness. The first sandboxed MCP run failed at loopback bind with
`Operation not permitted`; the commands below passed with sandbox escalation.

```text
rtk bypass cargo test -p opencanvas-storage --test batch_file --test transactions
  batch_file: 4 passed; transactions: 4 passed

rtk bypass cargo test -p opencanvas-server --test structured_batch --test tool_schemas --test tools_budget -- --nocapture
  structured_batch: 3 passed; tool_schemas: 2 passed; tools_budget: 1 passed

rtk bypass cargo clippy -p opencanvas-storage --lib --test batch_file --test transactions -- -D warnings
  passed

rtk bypass cargo clippy -p opencanvas-server --lib --test structured_batch --test tool_schemas --test tools_budget -- -D warnings
  passed

rustfmt --edition 2021 --config skip_children=true --check <all lane B Rust files>
  passed

rtk bypass git diff --check
  passed
```

The initial broad `cargo clippy -p opencanvas-storage -p opencanvas-server --lib
--tests -- -D warnings` failed on existing unused shared-support members in the
unrelated `context`, `animation_context` and `multi_viewport` integration targets.
It was narrowed to both libraries and the affected integration tests above; no
unrelated suppressions were added.

Coverage includes file/inline cross-transport replay, filename/whitespace changes,
changed content with a reused key, conflict `changedSince`, atomic late-error
rollback, strict original file schema, traversal/symlink/directory/FIFO/size/JSON/
operation-count failures, 500 valid repeated operations, preview key collisions,
unchanged DB and WAL bytes during preview, preview/commit changed-ID equivalence,
and storage restart after both commit and preview. The real MCP side-effect test
checks commits/idempotency row counts, empty asset storage, and no broadcast
events. A temporary executable worker probe stays untouched during both preview
transports and is invoked by a real commit with the same key, proving the
prerender check is live.

Measured final tools/list: **17 tools, 26,183 B**, under the unchanged 32 KiB
catalog limit. `apply_batch` is 5,515 B; `apply_batch_file` is 942 B, each below
8 KiB. No pre-change byte measurement was obtained because the initial baseline
attempt was blocked at socket bind; no before/after savings are claimed.

## Module boundaries and sizes

`project.rs` now keeps project reads/history/path checks; `batch.rs` owns candidate
mutation orchestration; `batch_file.rs` owns the bounded JSON read contract;
`batch_file/inbox.rs` isolates Unix descriptor safety. Server batch request
orchestration and batch schemas live in separate modules. Adversarial filesystem
tests were extracted when the combined test file exceeded 250 lines.

| Rust file touched by lane B | Lines |
| --- | ---: |
| storage/src/batch.rs | 130 |
| storage/src/batch_file.rs | 77 |
| storage/src/batch_file/inbox.rs | 111 |
| storage/src/project.rs | 119 |
| storage/src/store.rs | 175 |
| storage/src/lib.rs (shared declarations) | 68 |
| storage/tests/batch_file.rs | 176 |
| storage/tests/inbox_guards/mod.rs | 96 |
| server/src/domain.rs | 104 |
| server/src/domain/batches.rs | 38 |
| server/src/mcp.rs | 166 |
| server/src/schemas.rs | 179 |
| server/src/schemas/batch.rs | 31 |
| server/tests/structured_batch.rs | 185 |
| server/tests/tool_names/mod.rs | 38 |

No lane B Rust file exceeds 250 lines. Cargo.lock is generated; its change records
the direct libc dependency already present transitively. Secure inbox reads are
implemented for Unix and verified on macOS; non-Unix returns
`UNSUPPORTED_PLATFORM`. Validation is a structural preview, not a reservation or
render guarantee; a concurrent commit can invalidate its base revision.
