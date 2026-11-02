# PostgreSQL fast-path optimization — 2026-11-02

This change is confined to the isolated Task15 spike. It does not migrate the
product runtime, open user data, or start native listeners on 3847/3848.

## Changes

- `postgres/src/batch.rs` rejects a stale expected revision after canonical request
  validation/hash, the locked project-row read and idempotency lookup, before the
  nodes SELECT. Existing-key collisions still take precedence. Identical committed
  requests still replay the complete stored response even when their revision is
  old. Dry runs still validate against current revision and never replay a write.
- `GET /projects/{id}/revision` returns exactly `{projectId,revision}` through one
  project primary-key SELECT, without loading nodes. Malformed IDs return 400;
  missing projects return 404. The route precedes the broad snapshot handler and
  preserves the literal `/projects/revision` snapshot route.
- SSE's initial existence check uses the same project-only read. Durable catch-up,
  cursor checks, repeatable-read snapshots and notification behavior are unchanged.
- Startup applies `indexes.sql` after the base and durable schemas. Index selection
  and EXPLAIN evidence are documented by the separate index lane.

No revision or snapshot cache was introduced. Successful batches still acquire the
project row lock, load the current candidate tree, validate the complete candidate,
and atomically write nodes/revision/commit/idempotency before notification/response.
The revision endpoint is a current statement-level read, not a promise that a later
batch will succeed: concurrent commits can still require revision-aware retries.

## Regression checks

The original 10 real-DB contract tests remain. Two additional tests are integrated
into the same `contract` test target:

1. Seed a project and hold `ACCESS EXCLUSIVE` on `nodes` from another connection.
   Stale rejection, stale committed-key replay, collisions (including dry run),
   stale dry runs, initial request validation, missing-project lookup, revision HTTP
   reads and the SSE initial response must complete while the lock is held. A fresh
   valid batch then demonstrably waits on that lock (`pg_blocking_pids`), commits
   only after release, and produces exactly the expected durable row counts.
2. Exercise the actual TCP HTTP adapter for revision zero/current, missing and
   malformed project IDs, ordinary snapshots and the literal `revision` project
   path. Fixtures use per-process/per-test IDs; no global project is created.

Run only against the disposable spike PostgreSQL fixture, with `PG_URL` supplied
by the harness. The nodes-lock probe intentionally needs serial execution:

```sh
cargo test --offline --manifest-path spikes/realtime-db/postgres/Cargo.toml --test contract -- --test-threads=1
cargo clippy --offline --manifest-path spikes/realtime-db/postgres/Cargo.toml --bin oc-spike-postgres -- -D warnings
cargo fmt --manifest-path spikes/realtime-db/postgres/Cargo.toml -- --check
```

Executed checks, all exit 0:

- `cargo build --offline --locked --manifest-path spikes/realtime-db/postgres/Cargo.toml`
- The Clippy and formatting commands above, plus `cargo test --offline
  --manifest-path spikes/realtime-db/postgres/Cargo.toml --test contract --no-run`
  and `git diff --check -- spikes/realtime-db/postgres`.
- Harness-owned `PHASES=rustContract,indexEvidence node
  spikes/realtime-db/harness/run-all.mjs` completed against disposable PostgreSQL
  16.9. [Raw Rust output](evidence/pg-1793597486325-94df21f9/rust-contract.txt)
  records **13 passed**: 10 existing DB contract scenarios, the 2 new scenarios,
  and the existing blob hash/corruption unit test now included through HTTP imports.
  The regression target itself finished in 2.43 seconds.
- [Integration report](evidence/pg-1793597486325-94df21f9/report.json) records passing
  real-DB contracts and index plans/mutations, matching before/after source
  fingerprint `a792088d464857c9c551608463a7df857014a84f25503d8f85c45efcd3e19aed`,
  and cleanup of that run's gateway processes/container/anonymous volume. Other
  phases were explicitly skipped in this targeted run; the full final run belongs
  to the independent harness lane.

Comparative workload timings and full recovery/durability results are reported by
the harness lane; this module report makes no unmeasured speedup claim.

## Module boundaries and limits

`batch.rs` owns transactional command execution; `db.rs` owns database reads;
`http.rs` owns routing; `main.rs` owns startup. The focused regression module is
`tests/contract/fast_paths.rs`. Modified Rust files are below 200 lines at initial
handoff (largest: fast-path regressions, 193 lines). No large module was expanded.

This change does not optimize full-tree successful candidate validation or change
the spike's deliberately limited domain subset. Runtime/product validator parity,
production ingress/auth, Kubernetes HA and real-data migration remain outside this
optimization. Performance claims require the independent matched-build benchmark;
the lock regression proves absence of node reads on fast paths, not a latency SLA.
