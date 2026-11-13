# Task27 — Native deploy with process-compose, user-run deploy/rollback

**Status:** ready to assign. Not started. Requested by the user on 2026-11-13: supervise the
native runtime with process-compose and give the user `deploy.sh` / `rollback.sh` to run
themselves instead of an agent.

## Why

The native macOS runtime (`docs/NATIVE-RUNTIME.md`) works but is hard to operate:

- A release is assembled by `prepare-runtime.sh` from a running Docker image (web, WASM,
  worker, Playwright copied out of the container) plus native Rust binaries. Later releases
  were stacked as patch overlays; the live one is
  `4905dd5…+native-overlay+react-singleton+scope-cache+frame-cache` with a long `PROVENANCE`.
- `config/server.env` hard-codes `OPENCANVAS_RELEASE` to one release directory, so every
  switch edits private config by hand.
- PostgreSQL and the server are two LaunchAgents with wrapper scripts; ordering is a
  30-second `pg_isready` loop inside `run-server.sh`. Verification and rollback are manual.

## Goal

One command builds a release from one git commit natively and switches to it with health
checks and automatic rollback; one command rolls back. process-compose supervises
PostgreSQL and the server under a single LaunchAgent. Agents never deploy: the user runs
the cutover and every live deploy.

## Scope

One worker, worktree `/Users/luis/Workspace/OpenCanvas-native-deploy`, branch
`feat/native-process-compose` from `main` @ `0f42dd1`. Shell scripts in
`scripts/native/`, each ≤ 250 lines, shared helpers in one common file; pass `shellcheck`.

1. `build-release.sh <git-ref> <output-dir>`: resolve the ref to a full SHA, `git archive`
   it to a private source dir, build everything natively from that tree (release server,
   layout binary, web, renderer, scene WASM, worker; follow `Dockerfile` and
   `npm run build:release` / `scripts/app.mjs` for the steps), copy Node and the Playwright
   browser as `prepare-runtime.sh` does. Write `REVISION`, `PROVENANCE` (commit, ref,
   build time, tool versions, supported database schema version) and `SHA256SUMS`.
   No Docker, no patch overlays. Build caches go under `.runtime/native/` (ignored).
2. Release layout: `runtime/releases/<sha>/`, an atomic `runtime/current` symlink, and an
   append-only `runtime/deploy-history.log` (time, from, to, result). `server.env` points
   `OPENCANVAS_RELEASE` at `runtime/current`, so switching never edits config.
3. `process-compose.yaml` generated into the runtime root: processes `postgres` and
   `server` reusing the existing graceful-shutdown behavior (PostgreSQL SIGINT, server
   SIGINT); readiness probes (`pg_isready`; an HTTP check on the local server port);
   `server` depends on `postgres` being healthy; restart on failure with backoff; per-process
   logs in `logs/`; the process-compose API bound to loopback or a Unix socket, never the
   default `:8080`. Paths contain spaces (`Application Support`): quote correctly.
4. One LaunchAgent `org.opencanvas.compose` (RunAtLoad, KeepAlive) runs process-compose
   headless. The backup LaunchAgent stays as is (process-compose is not a scheduler).
5. `migrate-to-process-compose.sh [--revert]`, run once by the user: back up, stop the
   `org.opencanvas.postgres` and `org.opencanvas.server` agents, install the compose agent,
   point `server.env` at `current` (initially the live release), start, verify; `--revert`
   restores the two old agents and config exactly.
6. `deploy.sh <git-ref> [--dry-run] [--yes] [--keep N]`: lock against concurrent runs; build
   (or reuse an existing identical release); run the existing backup before switching;
   switch `current`; restart only `server` through process-compose; wait for readiness; run
   `verify-runtime.mjs`; on any failure switch back, restart, verify, record the failure and
   exit non-zero. Same SHA already current is a no-op. `--keep` prunes old releases but never
   the current or previous one.
7. `rollback.sh [release] [--dry-run] [--yes]`: default to the previous successful release
   from history; same switch/restart/verify path. Refuse when the database schema version
   is newer than the target release supports, and point to restore from the pre-deploy
   backup (`docs/NATIVE-BACKUP.md`), unless `--force`.
8. `status.sh`: current and previous release, process states, health, last history lines.

All user-facing scripts: `set -euo pipefail`, `--help`, clear step output, never print
secrets or the contents of `config/`. Root defaults to
`~/Library/Application Support/OpenCanvas`; `OPENCANVAS_ROOT` overrides it for tests.
On the default live root, ask for confirmation unless `--yes`.

## Constraints for the worker

- Never run any script against the live root, never `launchctl` the live agents, never stop,
  restart or connect to the live PostgreSQL cluster (port 2703) or the live server
  (3857/3858), never write `~/.opencanvas`. Read live files only to understand the layout.
- Integration tests use a `mktemp` root, a fresh `initdb` cluster and free ports chosen at
  run time, and run process-compose directly (not through launchd). Clean everything up.
- Cluster, ports, auth, privacy, public viewer and Funnel behavior stay unchanged.
- No deploy, no push, no commit; the root session reviews the diff first.
- process-compose comes from Homebrew (`brew install process-compose`) only if the user has
  approved it; record the version used.

## Acceptance

- `scripts/native/tests/deploy-integration.sh` passes on a disposable root: install and
  start under process-compose; deploy release A, verify; deploy release B, verify; rollback
  to A, verify; a broken release B' (server exits at start) makes `deploy.sh` exit non-zero
  and leaves A serving and verified; a concurrent deploy is refused by the lock;
  `--dry-run` changes nothing; the schema guard refuses an incompatible rollback.
  Tests may reuse one built release directory to avoid rebuilding twice.
- `build-release.sh` builds `main` @ `0f42dd1` natively, and `verify-runtime.mjs` passes
  against it on disposable ports.
- `migrate-to-process-compose.sh` and `--revert` are exercised on a disposable root that
  mimics the current layout (two agents simulated without launchd).
- `shellcheck` clean for every new or changed script; existing `backup-integration.sh`
  still passes.
- `docs/NATIVE-RUNTIME.md` gets a short "Deploy and rollback" section (user commands,
  cutover steps, how to revert). Report `docs/TASK27-REPORT.md`: commands run, results,
  build time, process-compose version, line counts, limitations.

## Out of scope

Deploying to the live root, the cutover itself, changing the backup schedule, TLS, Linux
or Kubernetes, removing the old scripts (keep them until the user has cut over).
