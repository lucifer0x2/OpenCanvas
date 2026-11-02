# Task17 final integration checks — 2026-11-02

All commands ran from the repository root against the working tree containing
the NUL target-ID fallback fix. PostgreSQL contracts used a C-verified disposable
Task17 PG/Redis fixture, exported from its private environment file. Credentials
are omitted. The fixture also had `infra/migration-v2.sql` installed separately;
the common initializer applies delta metadata, not the opt-in v2 description.

| Command | Result | Raw log |
| --- | --- | --- |
| `cargo test --manifest-path spikes/realtime-db/core/Cargo.toml --offline --locked` | 10 passed | `core-test.log` |
| `cargo test --manifest-path spikes/realtime-db/postgres/Cargo.toml --offline --locked --test contract -- --test-threads=1` | 33 passed | `v1-contract.log` |
| Same contract command with `--features compat-v2` | 34 passed | `v2-contract.log` |
| `cargo clippy --manifest-path spikes/realtime-db/core/Cargo.toml --offline --locked --all-targets -- -D warnings` | Passed | `core-clippy.log` |
| `cargo clippy --manifest-path spikes/realtime-db/postgres/Cargo.toml --offline --locked --all-targets -- -D warnings` | Passed | `v1-clippy.log` |
| Same PostgreSQL Clippy command with `--features compat-v2` | Passed | `v2-clippy.log` |
| `cargo fmt --manifest-path spikes/realtime-db/core/Cargo.toml -- --check` | Passed | `core-fmt.log` (empty success) |
| `cargo fmt --manifest-path spikes/realtime-db/postgres/Cargo.toml -- --check` | Passed | `postgres-fmt.log` (empty success) |
| `cargo build --manifest-path spikes/realtime-db/postgres/Cargo.toml --offline --locked --bin oc-spike-postgres` | Passed, default v1 built last | `default-build.log` |

`nul-regression-before.log` records the genuine failing PostgreSQL regression
before the fix: `INTERNAL_ERROR` instead of full apply's `VALIDATION_FAILED`.
The successful v1/v2 logs contain that same regression after conservative
classification fallback. It also verifies an earlier `props:null` retains error
precedence and every failure leaves revision/certificate/entities/history/replay
unchanged. The core classifier regression covers NUL fallback too.

Initial setup attempts without exported variables and without sandbox socket
access failed before exercising the DB behavior. Successful contract runs used
the exported fixture environment and approved local socket access. No user data,
production service, commit or push was involved. Build/freeze and timed evidence
are recorded separately by the Task17 evidence worker.

Final default v1 binary SHA-256:
`8ac73be9d0d99eaf2f0faea61759568fe4e3e168fc05f8214c21090ed4893a91`.
`BINARY-SHA256.txt` records its path. `SOURCE-SHA256.txt` freezes 47 core/PostgreSQL
source, test, SQL, manifest and lock files; that manifest's SHA-256 is
`0558ce4a7feb3c9033d399e65c93213c01aa2ea79a436cefe712e8bcc0d5bf81`.
No code/build/test/DB traffic from this worker occurs in the subsequent evidence
worker's quiet lifecycle/benchmark window.
