# PostgreSQL selected-row delta writes

Task17, 2026-11-02. This is the isolated Task15 spike. The SQLite runtime,
production validator and user data are outside this change.

## Behavior and proof boundary

An update-only batch on a certified revision reads the unique IDs named by its
operations through `project_id=$1 AND node_id=ANY($2)`. It clones only those
returned nodes, runs the existing ordered command executor, validates their final
properties with the **same function** used by full candidate validation, and
persists the selected final rows. It does not load, clone, sort or validate the
complete project. The selected SQL result is already ordered by node ID.

All current spike properties are node-local scalars: `name`, `text`, `fill`, `x`,
`y`, `width`, `height`, `fontSize`, `opacity`. Updates shallow-merge props and do
not change IDs, kinds, parent links or sort keys. Starting from a valid baseline,
the only predicates whose inputs can change are the changed nodes' property
predicates. The other nodes' properties and every topology predicate retain
their inputs. This is the limited reason selected-row validation is sufficient.
It is not text/layout/paint/theme/instance parity with the product validator.

`scalar_update_ids` classifies successfully parsed update-only batches without
validating properties. Structural, mixed and malformed operations go through the
full path; an early missing-node error must precede a later malformed operation.
Both paths validate properties after **all** operations, so an invalid scalar
temporarily overwritten with a valid one remains accepted. Repeated IDs preserve
operation order and yield sorted unique `changedIds` as before.
Target IDs containing NUL also use the full path: PostgreSQL `TEXT` cannot bind
them, so selecting them early would replace ordered core errors with SQL errors.

## Certificate and startup lifecycle

`projects` adds `generation TEXT NOT NULL DEFAULT gen_random_uuid()::text`, nullable
`validated_revision BIGINT`, and nullable `validator_version TEXT`. Existing and
new projects start uncertified. `delta-migration.sql` adds these columns
idempotently to old fixtures. `db::initialize(client)` applies base schema,
additive migration, durable schema and indexes at gateway startup before Redis
initialization or accepting requests. DDL is never run by a hot pool lease.

The initial project-row lock reads revision and certificate in one statement. A
fast path requires both the locked revision and `spike-scalar-v1` to match the
certificate exactly. Missing fields, wrong version or wrong certified revision
fall back to full validation. Any candidate-validator rule, supported kind or
command-semantic change must bump `VALIDATOR_VERSION`. Direct SQL mutation outside
the command boundary must invalidate the certificate; it is not an authentication
mechanism against an actor allowed to rewrite database rows and metadata.

A successful full or delta candidate stamps both certificate fields in the same
persistence statement as entities, revision, history, exact replay response and
transactional NOTIFY. PostgreSQL COMMIT must succeed before ACK. SQL errors and
deferred-constraint COMMIT failures roll back the certificate with everything
else. Dry run rolls back and never creates or advances a certificate. Generation
separates recreated projects with the same external ID; Redis uses it only for
read identity, never to authorize a write.

Replay remains a separate READ COMMITTED statement **after** the project lock;
it precedes expected-revision and node reads. A stale revision still rejects
before node reads. The selected-row path uses the existing atomic set-based
persistence statement, with no Redis access or cache fill on the write path.

## Evidence and validation

Core differential tests compare the selected overlay against the complete
candidate, including all nine supported properties, geometry bounds, text kind
and length rules, unknown props, invalid objects, repeated IDs, missing nodes,
transient-invalid-then-fixed values and error precedence. Existing full-candidate
tests continue covering forward-parent creation and structural operations.

New real-PostgreSQL contracts cover:

- A certified update loads and validates two unique nodes from a 41-node project;
  final state and changed IDs match `apply` on the complete snapshot.
- Missing, wrong-version and wrong-revision certificates force full validation;
  mixed forward-parent creation retains final-candidate validation timing.
- An uncertified invalid untouched row rejects and cannot produce a certificate;
  a later valid repair through full validation can certify it.
- Dry run leaves initial and existing certificates, nodes, history and replay
  rows unchanged; replay cannot stamp a later head.
- Late property validation, missing IDs, SQL failure and deferred COMMIT failure
  preserve nodes, revision, certificate, history and idempotency atomically.
- Additive migration runs twice over a temporary old-schema fixture; old rows
  remain uncertified and recreation receives a new generation.
- NUL-containing missing target IDs preserve full-path validation errors,
  including an earlier invalid props object, with all durable state unchanged.

Profile JSON preserves the original phase names and four `coreUs` fields. It adds
sanitized `writeEvidence`: `path`, `fallbackReason`, `queriedNodeCount`, and
`validatedNodeCount`. Counts describe returned/materialized and validation-target
rows, **not** physical tuples inspected by a PostgreSQL query plan. `nodeCount`
is the selected candidate size for delta profiles. `cloneUs`/`executeUs`/
`validateUs` measure selected work; `sortUs` is zero. No IDs, keys, props, SQL or
connection URLs are emitted by these counters.

Commands run by worker A:

```sh
cargo test --manifest-path spikes/realtime-db/core/Cargo.toml
cargo clippy --manifest-path spikes/realtime-db/core/Cargo.toml --all-targets -- -D warnings
cargo check --manifest-path spikes/realtime-db/postgres/Cargo.toml --features compat-v2
cargo clippy --manifest-path spikes/realtime-db/postgres/Cargo.toml --features compat-v2 --bin oc-spike-postgres -- -D warnings
```

Results from worker A: core 10 tests passed; both clippy checks and integrated
PostgreSQL binary check passed. A fresh completion worker then verified core
10 tests, real PostgreSQL contracts 33 (v1) / 34 (`compat-v2`) serially on the
owned disposable PG/Redis fixture, and strict `--offline --locked --all-targets`
Clippy for core and both PostgreSQL feature configurations. Core/PostgreSQL
format checks passed. Sanitized raw logs, including the NUL regression failing
before the fix, are in `evidence/task17-final-checks-20261102/`.

## Modules and limits

New core modules: `delta.rs` (74 lines), `delta_tests.rs` (156). Existing core
files touched: `lib.rs` 140, `command.rs` 111, `validation.rs` 118. New PostgreSQL
`delta.rs` is 83 lines; modified `batch.rs` 109, `batch_write.rs` 71, `db.rs` 192,
`profile.rs` 100. New contract modules: `delta.rs` 223 and `delta_failures.rs` 104
after the NUL target-ID regression. The 67-line contract fixture helper now runs
the same initializer as the gateway.
All are below the 250-line review threshold and have one responsibility.

No benchmark improvement is asserted here. Worker C owns frozen-source evidence,
one-writer comparisons, Redis lifecycle runs and fixture cleanup. The delta gate
is intentionally conservative; structural/mixed batches remain proportional to
the whole project. Request-local execution can still be quadratic in up to 500
selected IDs because it reuses the existing executor; it is independent of the
number of untouched project nodes.
