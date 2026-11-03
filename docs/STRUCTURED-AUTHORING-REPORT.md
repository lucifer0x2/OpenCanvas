# Structured authoring workflow — Task14 lane C

Date: 2026-11-02. Workspace: OpenCanvas, shared `claude/design-properties` checkout.

## Result

Connect instructions and the published `skill://opencanvas/author-frame.md` now
direct OpenCanvas design authors to discover MCP `tools/list` input schemas and
submit structured JSON through `apply_batch`. The existing four-resource catalog
is preserved. The selected project's design-memory instructions are preserved,
including confirmed requirements, authorized filesystem access, limited design
scope and truthful handling of unavailable access.

The guide covers named token/style/master/instance reuse, frame/instance themes,
schema-correct text updates, current revisions, stable lost-response replay,
explicit conflict inspection/re-read/rebuild with a new key, dry runs and an
authorized pure-JSON inbox fallback through `apply_batch_file`. Successful commits
supply the next revision without a revision-only read. Dry runs do not reserve a
revision or consume an idempotency key. File calls remain MCP mutations, use the
same operations and 500-operation limit, and never auto-chunk or execute code.

This is client guidance for design work. It cannot enforce a client's system
prompt or prevent arbitrary external programs. It does not prohibit application
implementation, JS test fixtures or authorized project-memory filesystem access.
The server schema/validator continues to enforce shapes and data validity; it
does not infer code provenance from repetition or reject valid repeated edits.

## Verification

`crates/server/tests/structured_authoring.rs` fetches the actual author resource
over MCP and parses its marked JSON examples, rather than duplicating their
payloads in tests. Every mutation passes the real published schema and MCP
runtime. Tests start their own `127.0.0.1:0` listener and temporary data root;
the server task is aborted and its temporary store removed on drop.

Verified scenarios:

- Create a text style, themed token, reusable frame master and two instances;
  edit `update_node {id,props:{text,fontSize}}`; inspect both expanded instances
  for inherited text, font size and different resolved scheme colors.
- Dry-run token/file updates return provisional results, keep the revision and
  permit subsequent commits with the same key. Inline dry run emits no commit
  event. Committed `changedIds` match the dry-run predictions.
- Save the documented pure JSON array in the temporary inbox and commit it with
  the documented MCP file call. Replaying the exact file request returns revision
  4 with `replayed:true`; the equivalent inline operations replay the same commit.
- Accept 500 repeated, valid `update_node` operations. Replay a lost response;
  then handle an explicit stale-revision conflict by inspecting `changedSince`,
  reading the target and rebuilding against the read revision with a new key.
- Reject invented `update_text`, `nodeId`, `props.content`, executable `eval`
  operations and extra script/code fields. Invalid attempts preserve both the
  authored text and revision.

Actual commands:

```sh
cargo test -p opencanvas-server --test structured_authoring -- --test-threads=1
cargo test -p opencanvas-server --test structured_authoring --test skill_layer --test tool_schemas -- --test-threads=1
rustfmt --edition 2021 --check crates/server/src/instructions.rs crates/server/tests/structured_authoring.rs crates/server/tests/authoring_support/mod.rs
cargo clippy -p opencanvas-server --test structured_authoring -- -D warnings
rtk bypass git diff --check -- crates/server/src/instructions.rs crates/server/skills/author-frame.md
wc -l -c crates/server/src/instructions.rs crates/server/skills/author-frame.md crates/server/tests/structured_authoring.rs crates/server/tests/authoring_support/mod.rs
```

The first sandboxed test attempt could not bind the temporary listener
(`PermissionDenied`, before any workflow ran). The authorized rerun passed 3/3.
After adding the no-event assertion, the combined targeted run passed 7/7 across
authoring, resource compatibility and tool schemas (4.81 seconds). Formatting and
diff whitespace checks passed. Targeted Clippy passed with `-D warnings`.

## Scope and remaining work

No real server was started/restarted and no user design/data root was accessed.
No commit or push was made. Lane A owns paint/theme implementation and discovery
capabilities; lane B owns bulk/dry-run schema, atomicity and filesystem hardening.
Those broader backend guarantees have their own targeted tests and reports.

No transport performance gain is claimed. The source U07 measurements
652.8 KB/3,140 operations to 224.4 KB/792 operations came from master/instance
reuse. They are not a JS-to-MCP transport improvement. The predicted ~4 KB per
scheme, U07 rebuild with only text/style overrides and golden-image fuzz ≤3%
remain for the separate U07 design worker; this synthetic workflow does not
establish those acceptance criteria or pixel equivalence.

File sizes: instructions 105 lines; workflow test 137 lines; example/discovery
helper 76 lines. The author skill is 272 Markdown lines. It was evaluated for
splitting: keeping the checklist, fewshots and existing author guidance in one
published resource avoids a new discovery round trip/resource-count change;
it has one authoring responsibility. No modified handwritten Rust file exceeds
250 lines. There are no runtime public API changes in this lane.
