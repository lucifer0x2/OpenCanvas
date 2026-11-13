# Task27 report — native deploy with process-compose

Branch `feat/native-process-compose` from `main` @ `0f42dd1`, worktree
`OpenCanvas-native-deploy`. Nothing was committed, pushed or deployed; the live root,
its LaunchAgents, PostgreSQL 2703 and the server on 3857/3858 were not touched (read-only
inspection of the layout, scripts and release checksums only).

## Status

Implemented and checked, including the real process-compose. `brew install process-compose`
fails because homebrew-core has no such formula (2026-11-13). With the user's approval the
root session installed it from the upstream tap: `brew install f1bonacc1/tap/process-compose`
gave **process-compose v1.124.0** (commit `f471d83`) at `/opt/homebrew/bin/process-compose`.
The worker first proved the control flow against a scratch-only stand-in (see below); the
root then reran both process-compose tests against the real binary, and both passed:

```sh
LC_ALL=C bash scripts/native/tests/deploy-integration.sh .runtime/native/build-output/0f42dd1   # PASS, 12/12
LC_ALL=C bash scripts/native/tests/migrate-integration.sh .runtime/native/build-output/0f42dd1  # PASS, 6/6
```

## What was built

| File | Lines | Role |
| --- | ---: | --- |
| `scripts/native/deploy-common.sh` | 207 | Shared helpers: root/live detection, confirmation, private env reads, atomic `current` switch, history, lock, schema versions, process-compose client, readiness, verify |
| `scripts/native/build-release.sh` | 137 | `<git-ref> <output-dir>`: git archive, native offline build, Node + browser copy, `REVISION`/`PROVENANCE`/`SHA256SUMS` |
| `scripts/native/deploy.sh` | 123 | Lock, build or reuse, backup, switch, server-only restart, verify, auto-switch-back, `--dry-run`, `--keep N` |
| `scripts/native/rollback.sh` | 92 | Previous release from history (or named), schema guard, same switch path, `--force` |
| `scripts/native/switch-release.sh` | 55 | Internal switch/restart/wait/verify with switch-back; requires the caller's lock |
| `scripts/native/status.sh` | 69 | Releases, process states, health, schema, newest backup, agent, history |
| `scripts/native/migrate-to-process-compose.sh` | 234 | One-time cutover and exact `--revert` |
| `scripts/native/compose-config.sh` | 115 | Generates and validates `process-compose.yaml` |
| `scripts/native/run-compose.sh` | 20 | LaunchAgent entry: `process-compose up` headless on the Unix socket |
| `scripts/native/compose.launchd.plist.in` | 21 | `org.opencanvas.compose` (RunAtLoad, KeepAlive, ExitTimeOut 120) |
| `scripts/native/run-server.sh` (changed) | 29 | Pins the physical release at start; waits on the DATABASE_URL port instead of fixed 2703 |
| `scripts/native/tests/native-fixture.sh` | 127 | Disposable root with spaces, fresh `initdb` cluster, free ports, backup tools |
| `scripts/native/tests/release-smoke.sh` | 40 | Runs one release directly and runs `verify-runtime.mjs` |
| `scripts/native/tests/deploy-integration.sh` | 125 | Deploy/rollback acceptance under process-compose |
| `scripts/native/tests/migrate-integration.sh` | 107 | Cutover/revert on a live-like layout with simulated agents |
| `scripts/native/tests/fake-launchctl.sh` | 48 | launchctl test double (bootstrap/bootout/print) |
| `scripts/native/tests/seed-project.mjs` | 27 | Creates one project with a frame through MCP |
| `docs/NATIVE-RUNTIME.md` | +61 | "Deploy and rollback" section (file now 318 lines) |

All handwritten files stay under 250 lines. Old scripts (`prepare-runtime.sh`,
`install-runtime.sh`, `launch-runtime.sh`) are kept until the user has cut over.

## Design notes

- Layout: `runtime/releases/<sha>/`, `runtime/current -> releases/<name>` replaced by
  `rename(2)` (`mv -fh`), append-only `runtime/deploy-history.log` with tab-separated
  `time action from to result detail`. The previous release is the `from` of the last `ok`
  line whose `to` is current. `server.env` has `OPENCANVAS_RELEASE=<root>/runtime/current`;
  `run-server.sh` resolves it with `pwd -P` at start, so a running server never reads files
  of a release switched under it.
- `process-compose.yaml`: `is_strict`, `disable_env_expansion`, `ordered_shutdown`,
  `/bin/bash -c`, commands built with `printf %q` inside YAML single quotes (paths with
  spaces), `is_template_disabled`; `postgres` readiness `pg_isready` on the private socket,
  `server` HTTP readiness `GET /api/v1/viewer` on the local port and `depends_on postgres:
  process_healthy`; `restart: on_failure`, `backoff_seconds: 5`; `shutdown.signal: 2` with
  `parent_only: true` so only the wrapper gets SIGINT and forwards one SIGINT (a second one
  would make the server exit at once); timeouts 30 s (server) and 60 s (PostgreSQL), after
  which process-compose SIGKILLs the process group. Plain-text per-process logs with
  rotation in `logs/`. API only on `run/process-compose.sock`. Keys and the
  `process list -o json` fields (`name,status,is_ready,pid,restarts`) were checked against
  the upstream source (`src/types/{project,process,logger}.go`, `src/health/probe.go`).
- Schema guard: the PostgreSQL registry version (`opencanvas_runtime_v1.schema_registry`)
  and the newest `projects.schema_version` are compared with the target's
  `database_schema_version`/`scene_schema_version` from `PROVENANCE`; for older overlay
  releases they are parsed from the `REVISION` commit in this repository
  (`4905dd5` and `727cd33` both resolve to 1 and 4). Unknown support refuses unless `--force`.
- Locking: `runtime/.deploy.lock` (mkdir + pid, stale lock of a dead pid removed) shared by
  deploy and rollback. Ctrl-C is ignored inside `switch-release.sh`, so an interrupt takes
  effect only after the switch completes or switches back.
- Cutover backs up `server.env`, both plists and `runtime/scripts` to
  `backups/pc-migration-<UTC>/` with `MANIFEST.sha256` (prefix outside backup retention),
  runs `backup-tools/backup.sh`, stops server then PostgreSQL, moves the old plists out of
  LaunchAgents, installs wrappers and the compose agent and verifies. Any failure or
  interrupt after the stop runs `--revert`. `--revert` restores the files byte for byte
  (checked against the manifest), starts PostgreSQL, then the server, and verifies.
  `deploy.sh --keep` never prunes the release that `--revert` restores.
- Test hooks (environment): `OPENCANVAS_ROOT`, `OPENCANVAS_PROCESS_COMPOSE`,
  `OPENCANVAS_READY_TIMEOUT`, `OPENCANVAS_BUILD_RELEASE`, `OPENCANVAS_LAUNCH_AGENTS`,
  `OPENCANVAS_LAUNCHCTL`.

## Checks run

| Command | Result |
| --- | --- |
| `CARGO_TARGET_DIR=/private/tmp/opencanvas-task27-target CARGO_BUILD_JOBS=4 OPENCANVAS_BROWSERS=<main checkout>/.runtime/browsers bash scripts/native/build-release.sh 0f42dd1 .runtime/native/build-output/0f42dd1` | Pass, **157 s** wall (empty target dir, cached registry and npm cache). Viewer bundle `index-CfxC__M2.js`, same name as the live frame-cache release. |
| Same with ref `main` into a second output | Pass, 14 s (warm; Cargo reused its artifacts); rebuilt web/renderer/worker and the binaries hash-identical to the first build |
| `bash scripts/native/tests/release-smoke.sh .runtime/native/build-output/0f42dd1` | Pass: fresh cluster, free ports, `verify-runtime.mjs` `{"ok":true,"projectCount":1,"protocol":"2025-11-25","pngBytes":6218,"wasmBytesVerified":true}`, graceful `Stopping` on wrapper SIGTERM |
| `deploy-integration.sh` with `OPENCANVAS_PROCESS_COMPOSE=<scratch stand-in>` | Pass, all 12 steps (install/start, deploy A, no-op, deploy B, rollback, broken B' non-zero with A verified and PostgreSQL untouched, lock refuses deploy and rollback, dry runs unchanged, schema guard, `--keep 2`, status, ordered SIGTERM stop) — **stand-in, not process-compose** |
| `migrate-integration.sh` with the same stand-in | Pass: dry run unchanged, failed cutover auto-reverts, cutover, deploy after cutover, `--revert` restores files exactly and serves the original release — **stand-in, not process-compose** |
| Root rerun of `deploy-integration.sh` and `migrate-integration.sh` with the real process-compose v1.124.0 (`LC_ALL=C`, no `OPENCANVAS_PROCESS_COMPOSE`) | Pass, 12/12 and 6/6, including YAML `--dry-run` validation, UDS-only API with no TCP listener, readiness probes and ordered stop; no temporary root or process left |
| `~/.local/bin/shellcheck -x scripts/native/*.sh scripts/native/tests/*.sh` (0.11.0) | Clean |
| `LC_ALL=C bash scripts/native/tests/backup-integration.sh` | Pass. Without `LC_ALL=C` this agent shell (no locale variables) hits the documented "postmaster became multithreaded" PostgreSQL start error. |
| Read-only `shasum --check` of the live release's `SHA256SUMS` | Pass (rollback to it would pass the release check) |

The stand-in (scratch directory only, not in the repository) implemented the documented
CLI subset (`up`, `process list -o json`, `process restart|start`, SIGTERM ordered stop)
and ran the generated YAML commands through `bash -c`. It proves the scripts' control flow
with a real PostgreSQL, real server releases, real backups and `verify-runtime.mjs`; it does
not prove process-compose's own probe timing, restart and YAML validation; the real-binary
rerun above covers those. Every temporary root, cluster and process was removed after each run.

## Limitations and follow-ups

- Run the two integration tests with the real process-compose and record its version.
- `verify-runtime.mjs` needs at least one project with a frame (true on the live root).
- `deploy.sh` builds from local git objects: fetch the commit first. Builds are offline;
  a new dependency needs a one-time online `npm ci`/`cargo fetch` in a checkout.
- The cutover itself, TLS and the backup schedule are out of scope; the old launchd scripts
  remain for `--revert`. process-compose children run in their own process groups; if
  launchd SIGKILLs process-compose after `ExitTimeOut` (120 s), children could outlive it.
- Rollback does not take a backup (deploy does); rollback across a schema change is refused.
