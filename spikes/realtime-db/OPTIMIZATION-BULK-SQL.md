# PostgreSQL batch: bulk SQL — 2026-11-02

Measurements and checks below describe the dated historical run. Generated raw
logs, JSON/JSONL, manifests and archives are no longer versioned in the current
tree; see [output policy and reruns](evidence/README.md).

The spike batch writer now persists all touched node rows and commit metadata in
one parameterized statement. This report covers the implementation lane and its
isolated contract run; it makes no latency claim. Root's independent harness and
wire-protocol measurements are reported separately.

## Changes and transaction invariants

- `postgres/src/batch_write.rs` partitions the validated final candidate into
  disjoint upsert/delete sets. A single `jsonb_to_recordset` UPSERT and `ANY(text[])`
  DELETE replace the awaited SQL loop. Finding rows no longer rescans the entire
  candidate once per changed ID.
- One writable CTE also updates project revision, inserts commit history and the
  complete replay response, then calls transactional `pg_notify`. Metadata CTEs
  pass project/revision through `RETURNING`. Node CTE execution order is irrelevant:
  their ID sets cannot overlap, and the existing composite FK remains deferred.
  No schema or index was changed.
- Full candidate apply/validation still happens before any write. COMMIT must
  succeed before a result is returned. SQL errors roll back nodes, revision,
  history, replay and queued notification together.
- Project `FOR UPDATE` and replay lookup intentionally remain separate statements.
  Under READ COMMITTED the replay lookup obtains a fresh snapshot after waiting
  for another writer. Replay/collision still precede revision rejection and node
  loading; matching replay returns the original stored revision/result even after
  a later commit. Dry run validates but performs no writes.
- Lock/replay/node reads and the write statement use tokio-postgres typed-query
  APIs. `db::state` and `db::revision` only changed their query invocation; row
  mapping, the pool, snapshot isolation and transaction boundaries are unchanged.
  Explicit built-in parameter types avoid the driver's separate prepare phase.
  No pipeline overlap or lock/replay snapshot combination was introduced.

## SQL command count model

These are source-derived counts, including BEGIN and terminal COMMIT/ROLLBACK.
They exclude connection setup, asynchronous statement close messages and internal
SQL/constraint work. An early-error transaction drop queues ROLLBACK; count it once
that connection is drained. They are not measured network packet or RTT counts.

| Request path | Before | After |
| --- | ---: | ---: |
| Valid durable success touching N IDs | N + 9 | 6 |
| Matching stored replay | 4 | 4 |
| Revision conflict, after replay check | 4 | 4 |
| Key/body collision | 4 | 4 |
| Fresh valid dry run | 5 | 5 |
| Request rejected by canonical-hash validation | 0 | 0 |

After success is BEGIN → project lock → replay lookup → nodes read → combined
write → COMMIT, regardless of touched count (tested for 1, 500 and 1,000 IDs).
The public core requires 1–500 operations and always records at least one touched
ID for a valid batch; a successful `changedIds=[]` request is not reachable.
Zero-operation requests fail before SQL; replay/conflict/dry-run paths write zero
node rows. Creating then deleting a node still records its touched ID and leaves
no final row. The write statement supports empty upsert or delete sets.

## Executed verification

`cargo fmt --manifest-path spikes/realtime-db/postgres/Cargo.toml --check` and
strict offline Clippy with `--all-targets --all-features -- -D warnings` passed.

The real-DB target passed **19/19 tests: 18 DB scenarios plus one blob/hash unit**:

```sh
cargo test --offline --locked \
  --manifest-path spikes/realtime-db/postgres/Cargo.toml \
  --test contract -- --test-threads=1
```

It used a new isolated PostgreSQL 16.9 container created through `startEngine`.
Run `pg-1793608038446-c9607491` was removed through `stopEngine`; no inherited
runtime, real service or user data was used. Raw output:
`contract.txt`.

Six added regression scenarios cover:

1. Mixed creates, updates, subtree deletes and reparenting match the full candidate
   exactly, including a parent created later, nullable page parent, Unicode props
   and i64 sort-key extremes. The same batch's dry run changes no durable records.
2. Delete/recreate of one ID and create/delete of another persist only final rows;
   replay after a later commit preserves the complete result and collision fails.
3. Separate create/update/delete batches each exercise the 500-operation limit.
4. One subtree-delete operation touches 1,000 IDs and leaves no nodes.
5. A trigger holds the first real writer inside its replay insert on an advisory
   lock; `pg_blocking_pids` proves a second same-key request waits on that writer's
   project lock before release. It then replays successfully, including after a
   subsequent commit. This exposes stale same-statement snapshot regressions.
6. A test trigger corrupts only the SQL candidate after domain validation. Deferred
   FK failure at COMMIT rolls back mixed upserts/deletes and all metadata, emits no
   notification, and allows the same key to commit once the trigger is removed.

Existing tests also retain candidate failure and idempotency-insert SQL failure,
same-project conflict, independent projects, fast stale/replay reads under a nodes
table lock, consistent snapshots and durable SSE/listener recovery.

Modified/new Rust modules: `batch.rs` 73 lines, `batch_write.rs` 70, `db.rs` 152,
`main.rs` 86, test module registry 46, `bulk_nodes.rs` 147, `bulk_replay.rs` 86,
`bulk_failure.rs` 73. Every module remains below 250 lines.

## Remaining scope

Root still runs the full independent harness, matrix and measured SQL/wire counts.
This lane did not run a separate latency matrix, restart/backup workload or real
Kubernetes/MCP/product migration. A valid batch still loads, clones and validates
the complete project candidate; the per-project revision lock still serializes
same-project writers. This change does not establish production latency or HA.
