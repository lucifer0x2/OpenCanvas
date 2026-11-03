# PostgreSQL harness acceptance report

Measurements and checks below describe the dated historical run. Generated raw
logs, JSON/JSONL, manifests and archives are no longer versioned in the current
tree; see [output policy and reruns](evidence/README.md).

Implementation workspace: isolated Task15 worktree, baseline `438254936823cb4ef8639d85c1404302c608f7f7`.
This report covers the PostgreSQL-only fixture, not a production storage switch.
The user's subsequent PostgreSQL selection supersedes the original two-engine comparison.

This worker report predates root's final default run, summarized with exact versions in
the [Task15 acceptance report](../../docs/TASK15-REPORT.md). Historical failed
invocations found an unsupported fixture property; another
found oversized HTTP bodies could hide a validation response behind TCP reset. Both were
corrected and owned containers were removed. No inherited runtime was stopped.

Worker evidence before the coordinator's final default run:

- `evidence/pg-1793595051187-69d099ad/`: Rust contract10/10 and all six matrix cells
  passed. The later sole-DB outage failed because Docker reassigned an automatic port;
  the controller was interrupted and its owned gateways/container/volume cleaned explicitly.
- `evidence/pg-1793595329682-25e5601b/`: final targeted durability, rolling gateway,
  sole-DB outage,20-commit history, v1/v2 rollback and backup/restore all passed with
  unchanged source fingerprint and automatic cleanup. Matrix/Rust phases were explicitly
  skipped in this targeted invocation. All three Node harness tests passed afterward.
- The targeted gateway restart recorded43 raw non-200/network failures,26 successful
  writes,304ms SSE reconnect gap, zero missing/duplicate commits and112ms retry p99.
  Sole-DB stop/start recorded16 raw failures,27 successful writes and1,092ms retry p99;
  its existing SSE connection survived while durable catch-up paused. This is an outage,
  not zero downtime. Restored data verified seven blob hashes, three resources, two jobs
  and two staged uploads. These small sample quantiles are observations, not tail-SLA estimates.

Additional harness defects found by investigation were fixed: explicit OS-selected random
port mapping is stable across database restart; recovery traffic has a30s total retry
deadline and2s per-request timeout; each gateway pool has unique readiness logs; empty
backup tables are handled. Failed invocations were recorded separately and never
counted as successful acceptance.

| AC | Status | Scope and acceptance evidence |
| --- | --- | --- |
| 1 | Pass in spike subset | Real PostgreSQL transactions across two processes; revision race, per-project lock independence, fault-injected rollback, paired sentinel snapshots and writer-specific final values. See Rust contract log and matrix. |
| 2 | Pass in spike subset | Drop HTTP response after durable commit, replay through another gateway, exact prior revision/result and collision; canonical body hashing tested. |
| 3 | Pass in spike subset | Cross-gateway SSE and durable reconnect; complete ordered commit ranges, no mixed snapshots, invalid/future/gapped cursors and polling repair; dry-run writes no rows/key/event/job. |
| 4 | Partial | Separate process rolling restart under overlapping write/read/SSE traffic and sole-DB stop/start outage. Raw errors, retry latency, SSE gaps/missing/duplicates recorded. Module publish is N/A for PostgreSQL. Actual rolling DB upgrade, Kubernetes and HA remain **not tested**. |
| 5 | Partial | Durable pending job survives gateway death; separate fixture worker is actually SIGKILLed; lease expiry/retry/fencing and pinned revision/pipeline prevent stale replacement. Resource and output hashes survive gateway loss. Outputs are fixture bytes, no real renderer process is ported. |
| 6 | Partial | Narrow additive schema with distinct compiled v1/v2 clients, actual binary rollback, shared fixture DB/resource/staging/job backup and restore. No real SQLite import, external object store, general schema downgrade or production release rollback. |
| 7 | Partial pending final default run | 100/5,000 nodes × 1/4/8 writers, 12 successful edits/writer passed. Raw errors/conflicts, attempt and retry-to-success p95/p99 recorded. Extra-history/restart investigation used20 commits; default runner requires1,000 commits on5,000 nodes. **Measured synthetic fixture only; no SLA/capacity claim.** |
| 8 | Pass in spike subset | Project-scoped SHA upload IDs stored in common PostgreSQL, shared cross-gateway lookup, 500-op/2-MiB guards, inline replay compatibility and no Pod-local path assumption. Production quotas/object-store lifecycle remain **not tested**. |

The runner records each phase status independently. A complete `status:pass` means its
executed fixture phases passed; it does not turn the limitations above into passes.
Each module has one responsibility and is below 250 lines. Dependencies/artifacts and
raw evidence are outside that handwritten-module guideline.

The production migration would first freeze/authenticate writes, capture a SQLite
snapshot plus resource manifest, import to a separately validated PostgreSQL project,
verify revision/entity/resource hashes, then switch traffic once. Keep the untouched
SQLite source for rollback before accepting PostgreSQL-only writes. A reverse migration
after new writes needs its own tested plan. None of this cutover has been executed.

No real user data, `~/.opencanvas`, 3847/3848 runtime or shared Docker workload was touched.
Kubernetes product provisioning, MCP session compatibility, general domain validators,
object storage, ingress/auth and DB operating topology require follow-up design and tests.
