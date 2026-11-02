# PostgreSQL batch and realtime contract spike

Task15, 2026-11-02. Workspace `codex/task15-realtime-db-spike`, baseline
`438254936823cb4ef8639d85c1404302c608f7f7`. This lane changes only the isolated
spike. It does not change the production storage engine, MCP sessions or services.

## Scope and changes

The shared Rust core remains an explicitly limited fixture: page/frame/group/text;
create, shallow property update, reparent and subtree delete; the listed numeric
and string properties only. Candidate validation, request canonicalization and
operation ordering were reviewed; existing core tests pass. This is not production
validator, storage, error-contract or MCP protocol parity.

The PostgreSQL adapter keeps a project row lock for each batch, looks up a stored
idempotency result before checking expected revision, and commits touched entities,
revision, commit log, stored response and NOTIFY in one transaction. Snapshot and
changes queries use `REPEATABLE READ READ ONLY` across their multiple SELECTs.

Hardening added here:

- A shared internal snapshot function accepts an interleave callback, allowing a
  test to commit on another connection between reading revision and reading nodes.
  The normal snapshot uses an empty callback and the same transaction path.
- Future change cursors return `INVALID_CURSOR` with `currentRevision` (HTTP400
  through the existing adapter). Incomplete commit ranges return `REPLAY_GAP`.
  SSE sends `reset_required` with `invalid_cursor`/`replay_gap` and closes, rather
  than waiting forever or silently skipping missing history.
- The dedicated LISTEN connection identifies itself as
  `oc_spike_listener:<process-id>`, without credentials. A failed LISTEN setup
  aborts and awaits its driver before retrying, avoiding detached driver tasks.

## Verification

Real PostgreSQL 16.9 in the harness-owned, isolated Docker fixture; temporary
project IDs only. Run tests serially against the fixture, outside the sandbox when
its localhost network policy blocks PostgreSQL. Do not print the private `PG_URL`.

```sh
# Load PG_URL from the selected private harness runtime before these commands.
rtk bypass cargo test --manifest-path spikes/realtime-db/core/Cargo.toml --locked
rtk bypass cargo test --manifest-path spikes/realtime-db/postgres/Cargo.toml \
  --test contract --locked -- --test-threads=1 --nocapture
rtk bypass cargo clippy --manifest-path spikes/realtime-db/core/Cargo.toml \
  --all-targets --locked -- -D warnings
rtk bypass cargo clippy --manifest-path spikes/realtime-db/postgres/Cargo.toml \
  --all-targets --locked -- -D warnings
```

Core: **6 passed**. Real-DB suite: **10 passed, 0 failed**, 2.23 seconds on the
first isolated fixture. Strict Clippy for both crates/all targets: passed. The
fresh integrated harness also records **10 passed**, 2.16 seconds, in
[`rust-contract.txt`](evidence/pg-1793594925396-085690ee/rust-contract.txt) before
benchmarking. Later durability-lane issues in that run are tracked separately;
this result is limited to the contract suite.

| Test evidence | What it proves |
| --- | --- |
| `atomic::held_project_lock_does_not_block_another_project` | Holds project A row lock, observes its waiting writer via `pg_blocking_pids`, commits B with identical node/key before releasing A. This is an overlapping lock test. |
| `atomic::same_project_race_and_stale_replay_are_atomic` | Two connections race one expected revision; exactly one commits, loser reports current revision. Replay after a later commit equals the original complete response except `replayed`; changed body collides. |
| `atomic::late_validation_and_late_sql_failure_leave_no_partial_records` | Invalid candidate preserves snapshot. A test-project-only trigger then fails the final idempotency insert, after earlier SQL writes; nodes/revision/commits/idempotency all roll back. Corrected retry succeeds with the same key. |
| `reads::snapshot_keeps_revision_and_paired_entities_from_one_database_snapshot` | A second transaction commits between the two snapshot SELECTs; response retains old revision and both old entities. A fresh snapshot returns new revision and both new values. Weakening isolation makes the old-snapshot assertion fail. |
| `reads::concurrent_snapshots_match_gapless_revision_and_both_entity_values` | Thirty concurrent paired updates; each snapshot's revision matches both entities, and changes' current revision matches its complete ordered commit range. |
| `reads::dry_run_and_validation_failure_reserve_no_key_or_durable_rows` | Preview preserves snapshot and node/revision/log/idempotency counts. Different body can subsequently commit using the preview key. |
| `notifications::notifications_are_post_commit_dry_run_silent_and_listener_recovers` | Dry run emits no LISTEN notification; real commit does. Terminates the exact listener backend, observes a different registered backend, verifies new notification and durable catch-up. |
| `notifications::polling_and_reconnected_stream_catch_up_without_any_notify_listener` | No listener runs at all: live SSE still catches a new commit through polling; a new stream resumes exactly revisions3/4 after disconnect. |
| Cursor tests | Future HTTP-style change cursor fails explicitly; missing fixture commit fails explicitly; future SSE cursor emits reset and closes within a deadline. |

The expected `postgres: db error` and `LISTEN reconnect: db error` in test output
are deliberately injected faults. Initial sandbox-only attempts could not connect.
A later evidence capture targeted the already retired first fixture and also
failed to connect; raw output is retained as
`postgres/tests/evidence/retired-runtime-connection-failure.log`. These connection
failures are not counted as successful verification.

## Boundaries

The Rust suite uses separate PostgreSQL connections in one test process. The
harness's existing two-gateway semantics test supplies the distinct-process race,
lost-ACK replay and cross-gateway subscription evidence; these tests do not claim
to replace that check. Durable render jobs/resources/staging and their evidence
belong to the other spike lane; this adapter schedules no render jobs on batch or
dry run, so this suite proves no job scheduling behavior beyond that fixture.

The connection pool is intentionally process-global and assumes one database URL
per process; it is not a general multi-database pool. Its finite16-connection
capacity can still be exhausted by blocked work. SSE uses a 250ms polling repair
path, has no pruning/paging retention policy or Last-Event-ID compatibility, and
does not prove Kubernetes rolling upgrades or production resource behavior.

Owned Rust files are each at most 176 lines after formatting; no size exception.
