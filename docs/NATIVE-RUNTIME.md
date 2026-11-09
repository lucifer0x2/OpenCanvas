# Native macOS runtime

Prepared on 2026-11-06 from the **running Docker image revision
`727cd33092ba5cd4aaea63b33eb5eadf979f29c9`**, rather than the older checkout.
The rehearsal passed, and cutover completed on2026-11-06: native local MCP/viewer
now serves`3857` and the public viewer serves`3858`. The two old Docker containers
are stopped with restart policies`no`; their volume remains intact for rollback.
PostgreSQL 16.15 uses the user-selected loopback port **2703**.

## Files and service boundaries

The dedicated root is `~/Library/Application Support/OpenCanvas`:

| Path | Purpose |
| --- | --- |
| `runtime/releases/<revision>/` | Native server/layout/Node, matching image web/WASM/worker/fonts and copied macOS Chromium |
| `runtime/scripts/` | Small launchd wrappers and signal forwarding |
| `postgres/` | Isolated PostgreSQL cluster; does not use Homebrew's existing data directory |
| `run/` | Private PostgreSQL Unix socket directory |
| `config/native.conf`, `config/pgpass` | Private libpq/admin/backup configuration |
| `config/server.env` | Private app database credentials and listener configuration |
| `backups/`, `backup-tools/` | Verified local backup sets and independent backup tools |
| `logs/` | launchd stdout/stderr and installation diagnostics |

The root, cluster, socket, config, backups and logs are mode `0700`; private
configuration is `0600`. Do not print config files or process environments.
The runtime has its own Node executable and browser installation and can run
without this checkout. The pinned Homebrew PostgreSQL installation remains an
external dependency at `/opt/homebrew/opt/postgresql@16/bin`.

The PostgreSQL app role `opencanvas` is a login with no superuser, role creation
or database creation privileges. TCP and application Unix-socket connections
use SCRAM. The macOS owner `luis` authenticates as itself through `peer` on the
private Unix socket for cluster administration and scratch restore verification.
The old `/opt/homebrew/var/postgresql@16` cluster is untouched.

## Reproduce the preparation

Run from the repository. Each output path must be new; installation refuses to
overwrite an existing root or an occupied port2703. The source container remains
running throughout this preparation.

```sh
revision=727cd33092ba5cd4aaea63b33eb5eadf979f29c9
bash scripts/native/prepare-runtime.sh "$revision" opencanvas-postgres-opencanvas-1
OPENCANVAS_PUBLIC_ORIGIN=https://viewer.example.invalid \
OPENCANVAS_PUBLIC_ALLOWED_ORIGINS=https://alternate.example.invalid \
  bash scripts/native/install-runtime.sh ".runtime/native/release-$revision"
bash scripts/native/launch-runtime.sh postgres
bash scripts/native/capture-docker-db.sh opencanvas-postgres-postgres-1 \
  .runtime/native/rehearsal .runtime/compose-postgres/preflight/fingerprint.sql
```

Replace the example HTTPS origins with your own. The installer requires
`OPENCANVAS_PUBLIC_ORIGIN` before creating any files or initializing PostgreSQL;
`OPENCANVAS_PUBLIC_ALLOWED_ORIGINS` is optional and can be omitted for one origin.
It writes these caller-provided values as quoted shell assignments in private
`config/server.env`, with no hostname defaults. The trusted proxy remains loopback.

`prepare-runtime.sh` archives the exact Git revision, builds only native Rust
server/layout binaries offline under `.runtime/native/target`, and copies the
portable viewer, scene WASM, renderer, worker and Playwright JavaScript from the
same running image. Native Node24.21.0 and the installed macOS browser are copied
into the release. `REVISION`, `SOURCE_IMAGE` and `SHA256SUMS` record provenance.
The generated archive, dump, snapshots and images stay ignored and private.

The initial build briefly used the checkout's generated Cargo target before it
was isolated. The checkout's debug layout binary was rebuilt from its own HEAD
afterward; tracked sources were not changed by that build.

Source `config/native.conf` privately with `set -a`, restore the custom dump using
`pg_restore --single-transaction --no-owner --no-acl --dbname=opencanvas`, and run
the same fingerprint SQL through native `psql`. Compare the complete output
with `cmp` before loading the server LaunchAgent.

The capture exports one read-only PostgreSQL snapshot and uses it for both the
table/row/binary/auth-state fingerprint and custom dump. It also compares sequence
state and rejects large objects. `pg_restore --list` and full archive decoding
check the dump before it is accepted.

```sh
bash scripts/native/launch-runtime.sh server
node scripts/native/verify-runtime.mjs http://127.0.0.1:3867 \
  .runtime/native/rehearsal/http http://127.0.0.1:3857
```

The verification reads every authored snapshot, negotiates actual MCP, reads an
outline and renders an existing frame through native Node/Chromium. It verifies
the PNG signature and dimensions and checks JavaScript MIME plus actual WASM
magic bytes, avoiding the SPA fallback's misleading HTTP200. Rendering and MCP
session creation do not change authored project data.

## launchd operation

`~/Library/LaunchAgents/org.opencanvas.postgres.plist` and
`org.opencanvas.server.plist` use `RunAtLoad` and `KeepAlive`, with a five-second
restart throttle. `run-server.sh` gives PostgreSQL up to30seconds to become ready;
a remaining startup failure is restarted by launchd. PostgreSQL explicitly uses
`LC_ALL=C` because the locale-free launchd environment otherwise caused the
macOS PostgreSQL startup error “postmaster became multithreaded during startup”.

Both wrappers translate launchd `SIGTERM` into `SIGINT` and wait for their child.
For PostgreSQL this requests a clean fast shutdown. For OpenCanvas it enters the
server's existing bounded graceful shutdown. launchd's default process-group
cleanup remains enabled so an unexpectedly terminated wrapper cannot strand
its server. Lifecycle acceptance confirms both the server's graceful `Stopping`
message and that no server/worker remains after bootout. Start PostgreSQL before
the server, and stop the server first.

```sh
launchctl print "gui/$(id -u)/org.opencanvas.postgres"
launchctl print "gui/$(id -u)/org.opencanvas.server"
launchctl bootout "gui/$(id -u)/org.opencanvas.server"
launchctl bootstrap "gui/$(id -u)" "$HOME/Library/LaunchAgents/org.opencanvas.server.plist"
```

These are per-user LaunchAgents: they start when this user logs in and require
the logged-in user session. They do not start before login, during sleep or
while the Mac is powered off. A LaunchDaemon is outside this setup.
When reloading the server, wait up to20seconds after bootout for the job entry,
old process and both listeners to disappear before bootstrap. A graceful stop
can outlive the bootout command; immediate bootstrap can fail with launchd error5.

## Cutover and rollback

Do this only after the independent rehearsal review has passed. Keep Funnel
unchanged: its configured HTTPS hostname continues to proxy loopback3858.
The native public listener trusts only the loopback proxy and keeps the same
public origin so preserved viewer cookies/sessions use the same client identity.

1. Boot out the rehearsal native server. Stop **only** the Docker app to freeze
   authored writes; leave Docker PostgreSQL running for the final capture.
2. Capture a new final dump and full fingerprint into a new private directory.
   Recreate only the dedicated native `opencanvas` database as its cluster owner,
   restore as the app role, and require exact complete fingerprint parity.
3. Change only the generated native listener settings to local3857/public3858.
   Bootstrap the existing server LaunchAgent. Verify all original snapshot hashes,
   MCP, PNG, public read-only/auth behavior, WASM and the unchanged Funnel URL.
4. Install the separately reviewed daily backup LaunchAgent and run one real
   backup/restore check; see [native backups](NATIVE-BACKUP.md). Retain14 completed
   daily sets. Backups remain on this Mac and do not protect against its loss.
5. Disable restart policies on the two old Docker containers, stop Docker
   PostgreSQL and retain both containers and the original volume. Record the
   previous restart policies privately for rollback. Do not remove the volume.

Before native accepts authored writes, rollback is: boot out the native app,
start the retained Docker PostgreSQL, wait for readiness, then start its app.
The retained Docker database already contains the final frozen source state.
Restore the recorded container restart policies if returning to Docker.

```sh
docker --context orbstack start opencanvas-postgres-postgres-1
docker --context orbstack exec opencanvas-postgres-postgres-1 \
  pg_isready -U opencanvas -d opencanvas
docker --context orbstack start opencanvas-postgres-opencanvas-1
```

**After any native database write, the old Docker database is stale.** This
includes viewer authentication/session updates as well as authored changes.
Keep native as the source of truth; take a fresh native backup and migrate those
writes into the rollback database before reactivating Docker. Never switch back
to the frozen database and discard accepted writes.

## Recorded rehearsal evidence

On2026-11-06: offline native layout and release-server builds passed for727cd33;
full source/native fingerprints matched exactly after restore. HTTP readback
retained5projects with sorted revisions`8,10,18,60,137`; all5raw authored snapshots
matched the live Docker source. Actual MCP negotiated`2025-11-25`; native worker
produced a valid320×200 PNG of69,709bytes. Scene WASM MIME and magic bytes passed.
Artifacts are under `.runtime/native/rehearsal/`; no authored mutation was used.
`bash -n`, ShellCheck and Node syntax checks passed for the new runtime tooling.
All new handwritten files remain below200lines.

Worker-active lifecycle tests passed with launchd's default process-group
cleanup: normal bootout retained the server's `Stopping` message and left no
server/Node/Chromium; a forced wrapper crash removed all old release processes
and launchd restarted a healthy app. A clean PostgreSQL restart and app-before-PG
startup recovered automatically, with transient unavailable responses only.

The independently reviewed final cutover used the frozen source set
`backups/migration-source-20261106T031458Z/` under the dedicated root. Its custom
dump, archive checksum, raw full-table fingerprints, native comparison and all
five HTTP snapshot hashes are retained there outside checkout and daily rotation.
Raw native/source fingerprints matched exactly both before app start and after
read-only HTTP/MCP/PNG smoke. All five snapshot hashes still match the frozen
Docker source; revisions remain`8,10,18,60,137`.

The first real daily-format backup is
`backups/backup-20261106T031604Z-lNJC5H3h/`: its fresh scratch restore and full
content verification completed successfully before`COMPLETE`publication.
`org.opencanvas.backup` is loaded, its RunAtLoad check exited0, and its calendar
schedule is03:00 local with14completed daily sets retained. The source migration
set has a different prefix and is excluded from daily retention.

Final local checks: native3857HTTP/MCP/PNG/WASM pass; public3858viewer returns
`{"readOnly":true}`, unauthenticated project list401and MCP403. The unchanged
HTTPS Funnel viewer endpoint returns200/readOnly. Both old Docker containers
are`exited`with restart`no`; prior policies are saved privately in the final
source set's`docker-restart-policies.txt`. The original Docker volume remains.
Root independently rechecked five raw snapshots, listener ownership, loopback
binding, app-role limits, durable PostgreSQL settings and HTTPS viewer after
cutover. No commit, push, authored mutation or Funnel configuration change occurred.

## Public viewer domain allowlist

The public listener can retain its primary HTTPS origin and accept additional
HTTPS origins through the private `config/server.env` file:

```sh
OPENCANVAS_PUBLIC_ORIGIN=https://viewer.example.invalid
OPENCANVAS_PUBLIC_ALLOWED_ORIGINS=https://alternate.example.invalid
OPENCANVAS_PUBLIC_TRUSTED_PROXY_IP=127.0.0.1
```

`OPENCANVAS_PUBLIC_ALLOWED_ORIGINS` is a comma-separated list of additional
origins, with at most eight unique origins including the primary origin. The
installed launcher already exports private configuration assignments, so this
setting does not require a launcher change. Keep `server.env` mode600 and never
print it: the same file contains the database credential.

Every request must use an allowed `Host`; an `Origin` header must match that
particular host. Sign-in requires that matching origin. The selected host also
controls generated resource/job URLs and the JWT audience. Cookies remain
Secure, HttpOnly and host-only; visitors sign in separately on each domain.
Keeping the original primary origin preserves its existing cookie audience.

This application setting approves a host at loopback3858. DNS, a valid HTTPS
certificate and a proxy route for a custom domain are separate requirements.
The existing Funnel route continues to use its configured HTTPS hostname and
proxy `http://127.0.0.1:3858`. The custom subdomain is approved; its apex hostname
is excluded. Actual hostnames are kept only in private runtime configuration
and ignored local evidence. No DNS, certificate or proxy configuration was
changed by this application update.

On2026-11-06, the subdomain resolved to Cloudflare IPv4/IPv6 addresses. Initial
TLS handshakes failed; by04:05UTC, verified HTTPS returned200/readOnly through
Cloudflare. The original Funnel HTTPS viewer also continues to return200/readOnly.
No certificate verification bypass is required.

The installed release is
`runtime/releases/727cd33092ba5cd4aaea63b33eb5eadf979f29c9+domain-allowlist.33bbe9f75b87/`.
It combines the exact727cd33 source with the recorded `SOURCE_PATCH` and
`PATCH_SHA256`; only the native Rust server binary changed. All original
web/WASM/worker/layout/Node hashes match the base release. The base release and
private pre-update configuration remain available for an app/config rollback;
the PostgreSQL database must remain the native source of truth.

Independent acceptance verified both HTTPS hosts, projects401, MCP403,
cross-host Origin403 and excluded apex403. Both JWT audiences were tested only
on an isolated clone. All five raw snapshots and the complete live database
fingerprint matched before/after the server replacement and subdomain update.
A fresh backup passed scratch restore before replacement. PostgreSQL2703,
daily03:00 backup retention14 and Funnel configuration remained unchanged.
The canary process/database were removed after acceptance. Local ignored
evidence is under `.runtime/native/domain-allowlist/`; no live sign-in, authored
mutation, commit or push was used.
