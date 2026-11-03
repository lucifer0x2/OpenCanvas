# PostgreSQL durability fixture (Task15 AC5/AC8)

Measurements and checks below describe the dated historical run. Generated raw
logs, JSON/JSONL, manifests and archives are no longer versioned in the current
tree; see [output policy and reruns](evidence/README.md).

This is an isolated fixture, not a production storage port or renderer. Existing
SQLite runtime, MCP sessions, real services and user resources are unchanged.

## Storage and ownership

`postgres/durable-schema.sql` adds four tables in the same PostgreSQL database:
`shared_blobs`, `resources`, `staged_batches`, and `render_jobs`. Tiny test asset,
font and output bytes use PostgreSQL `bytea`, allowing the harness to back up DB
and resources together. This choice does not establish production object-store
capacity, quotas, garbage collection, auth or tenant isolation. Resource fixture
bytes and output strings are opaque test data; they are not rendered PNG/font
conformance tests. No uploaded content is executed.

A render-input job stores an immutable snapshot at the requested revision,
pipeline fingerprint, resource hashes and render parameters. Submission fails if
the captured revision differs. The input content hash identifies the job; another
revision or pipeline produces a separate job and output. No mutable “latest scene”
alias exists. Later authored edits cannot mutate a previously pinned snapshot.

Claim uses an atomic UPDATE with `FOR UPDATE SKIP LOCKED`, increments its attempt
counter and sets expiry using database time. Pending jobs and expired attempts
remain claimable after process loss. Completion checks running status, exact
attempt, worker ID and unexpired lease while holding a row lock. It inserts the
immutable content-addressed artifact and links it to the job in that transaction.
Expired, superseded or repeated completions receive `409 STALE_ATTEMPT` and leave
the output untouched. There is no render worker daemon or lease heartbeat here;
the HTTP fixture represents worker claim/completion calls explicitly.

Blob upload and every resource/artifact read calculate SHA-256 from actual bytes.
An optional claimed upload hash must match; corrupted stored bytes produce
`500 INTEGRITY_ERROR`. IDs cannot overwrite existing bytes through this API.

## Fixture HTTP routes

All bodies/results are JSON. Local transport limits the body to 2 MiB; JSON byte
arrays therefore have a smaller practical binary capacity than 2 MiB.

| Route | Body/result |
| --- | --- |
| `POST /resources` | `{kind:"asset"|"font",bytes:[u8],sha256?:hex}` → resource ID/hash/bytes |
| `GET /resources/<id>` | Verified bytes, hash and registered kinds |
| `POST /uploads` | A normal inline `BatchRequest` → `uploadId`, hash, byte length |
| `POST /batch-file` | `{projectId,uploadId}` → normal batch result |
| `POST /jobs` | `{projectId,revision,pipelineFingerprint,resourceIds,renderParams:{...}}` → immutable job |
| `GET /jobs/<id>` | Job input, status, attempt, owner, expiry, artifact ID |
| `POST /jobs/claim` | `{workerId,leaseMs:100..60000}` → `{job:null|job}` |
| `POST /jobs/<id>/complete` | `{workerId,attempt,bytes:[u8]}` → completed job |
| `GET /artifacts/<id>` | Verified immutable bytes/hash linked to a completed job |

The staging upload is strict JSON, bounded at 2 MiB and validated by the shared
core (at most 500 operations). Its database row scopes the ID to a project; a
different project cannot execute that ID. A Pod-local path is never opened.
`/batch-file` loads and verifies bytes, then calls the same batch executor as inline
`/batch`, preserving dry-run, revision, collision and replay behavior. Upload
staging itself is durable metadata; dry-run execution does not commit authored
state, commits/events, jobs or an idempotency reservation. Uploads currently have
no expiry, deletion or quota policy.

## Compatibility and lifecycle hooks

Default builds implement the v1 project API. A distinct binary compiled with
`--features compat-v2` accepts/returns `description` on project creation/read and
requires the harness's additive migration column. Default v1 code does not read
or remove that column, allowing real code rollback while preserving v2 data.
This single-field fixture is not proof that all future migrations are reversible.

`/health` verifies a real database query with a two-second timeout. SIGTERM/SIGINT
stops accepting requests, drains finite requests for at most two seconds, then
aborts remaining SSE/request tasks. Clients resume SSE using durable revisions.
This does not establish Kubernetes HA, ingress readiness or zero downtime.

## Validation

From the spike worktree root:

```sh
cargo fmt --manifest-path spikes/realtime-db/postgres/Cargo.toml --check
cargo test --manifest-path spikes/realtime-db/postgres/Cargo.toml --bin oc-spike-postgres -- --test-threads=2
cargo clippy --manifest-path spikes/realtime-db/postgres/Cargo.toml --all-features --bin oc-spike-postgres -- -D warnings
node --check spikes/realtime-db/postgres/tests/durability.mjs
```

These checks passed during implementation (one binary hash/corruption unit test).
`postgres/tests/durability.mjs` exports `exercise(urls,lifecycle)` for the outer
harness. It asserts scoped/limited uploads, inline replay, staged dry-run, byte
hashes, pinned inputs, pending survival, expiry/retry, owner/attempt fencing,
independent old/new revision/pipeline artifacts and retrieval after gateway death.
The test forks `tests/fixture-worker.mjs`, waits for its actual job claim over IPC,
sends SIGKILL, verifies that exit signal, waits for the DB lease to expire and
retries the job from another gateway. This is a real worker-process death fixture,
while its bytes remain synthetic and no renderer is run.
Lifecycle callbacks own all gateway starts/stops. Standalone execution omits
process-death operations and reports `lifecyclePerformed:false`; it is not AC5
process-death evidence. Runtime evidence and DB backup/restore are owned by the
outer harness and reported separately.

The final targeted run
`evidence/pg-1793595329682-25e5601b/report.json` passed
`durableJobsResourcesStaging`, including actual SIGKILL of the claimed worker,
both gateway-death checks and the oversized-upload boundary. Its
`additiveMigrationAndBackupRestore` phase also passed actual v2 → v1 code rollback,
later v2 description reads, and pg_dump restore of seven hash-verified blobs,
three resources, two jobs and two staged uploads. This targeted run skipped the
contract/load matrix; it does not replace that evidence or an independent final
integration run. Gateway/database recovery and history-growth phases passed too.

Earlier runs exposed a Docker random-port restart issue in the harness and a
TCP reset hiding the oversized-upload JSON rejection. The transport now sends
its error/FIN and drains pending input for at most 250 ms with constant memory.
Failed attempts were recorded separately and were not counted as passing evidence.

All new Rust modules and the modified HTTP/main modules are below 200 lines.
