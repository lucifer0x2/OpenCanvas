# Native PostgreSQL backup and recovery

The native macOS deployment uses PostgreSQL16 on port2703 and a private install
root at `/Users/luis/Library/Application Support/OpenCanvas`. Backup tools operate
on this cluster only through its private `config/native.conf`; they do not use
Docker, the SQLite bundle API, or a database URL in command arguments. Installing
the files does not start a job or touch a database. Off-machine copies are a
separate follow-up; these local copies do not protect against losing the Mac.

## Configuration and installation

`scripts/native/install-runtime.sh` creates `config/native.conf` and
`config/pgpass`. Both must be regular files owned by the current user with mode600.
The config is trusted shell assignments, so do not source a downloaded config.
Paths containing spaces are supported. The agreed interface is:

```sh
OPENCANVAS_ROOT='/Users/luis/Library/Application Support/OpenCanvas'
PG_BIN='/opt/homebrew/opt/postgresql@16/bin'
PGHOST='/Users/luis/Library/Application Support/OpenCanvas/run'
PGPORT=2703
PGDATABASE=opencanvas
PGUSER=opencanvas
PGPASSFILE='/Users/luis/Library/Application Support/OpenCanvas/config/pgpass'
PGADMIN_HOST='/Users/luis/Library/Application Support/OpenCanvas/run'
PGADMIN_USER=luis
BACKUP_KEEP=14
# Optional: BACKUP_DIR='/another/private/local/directory'
# Optional: PGADMIN_PASSFILE='/private/admin/passfile'
```

`PGUSER` can read/dump the application database without superuser or CREATEDB.
Scratch restore verification uses the native cluster owner's Unix socket peer
authentication; it creates a fresh database owned by the application role, then
restores with `--role=opencanvas`. The socket directory is private to the Mac user.
An administrator using password authentication can set `PGADMIN_PASSFILE` instead.
Passwords remain in private passfiles; inherited `PGPASSWORD`, `DATABASE_URL` and
`PGSERVICE` are cleared by these tools.

Run from the repository to install tools and a daily LaunchAgent definition:

```sh
root="$HOME/Library/Application Support/OpenCanvas"
bash scripts/native/install-backup.sh "$root/config/native.conf"
```

This copies the tools to `$root/backup-tools` and writes
`~/Library/LaunchAgents/org.opencanvas.backup.plist`. After the native cluster is
running and the migration is accepted, activate the schedule explicitly:

```sh
launchctl bootstrap "gui/$(id -u)" "$HOME/Library/LaunchAgents/org.opencanvas.backup.plist"
```

The LaunchAgent runs at03:00 local time and on loading/login. `--if-due` skips
when a successful set already records today's **local calendar date**; loading
the job therefore catches a missed day without duplicating today's backup.
The Mac must be running with this user logged in for the user LaunchAgent to run.
Launchd can deliver a missed calendar event after sleep; a powered-off Mac cannot
produce backups. Logs are private under `$root/logs/backup.{stdout,stderr}.log`.
If PostgreSQL is unavailable, the job fails without pruning; the next daily
event, login, or a manual run retries. It does not loop or wake the Mac.

## Capture, verification and retention

An immediate manual backup does not apply the daily skip:

```sh
"$root/backup-tools/backup.sh" "$root/config/native.conf"
```

Each backup takes a PostgreSQL custom-format whole-database dump with
`--no-owner --no-acl`. This includes every application schema/table, project IDs,
revisions, generations, bytea asset/font bytes, commit/idempotency history, public
visibility, viewer credentials/sessions, and PostgreSQL large objects. The dump
captures PostgreSQL's consistent transaction snapshot while writes can continue.
Cluster roles, PostgreSQL configuration, and the private passfile are not in the
dump; recreate the restricted app role with the native runtime installer during
disaster recovery. New database credentials may differ from the lost cluster.

The script holds an exclusive directory lock, writes a private staging directory,
and restores the dump into a freshly named `opencanvas_verify_*` database using
`--single-transaction --exit-on-error --no-owner --no-acl`. It deletes this scratch
database after verification. Existing database names, including `opencanvas`,
cannot be supplied as a destination to the verification command. There is no
`--clean`, database replacement, or drop of a destination it failed to create.

The restored archive gets an independent canonical inventory: schema SQL hash,
each user table's row count and sorted JSON row hash, sequence state, and large
object count/content hash. This includes bytea bytes and auth/history rows. The
inventory describes the restored dump; it is **not** a second live-source
fingerprint taken under the same snapshot. The integration test compares it to
its quiescent source, including all eight application tables and binary data.

A successful set contains `database.dump`, `fingerprint.tsv`, `metadata.txt`,
`SHA256SUMS`, and the `COMPLETE` marker. SHA-256 covers the three payload/metadata
files. A same-filesystem directory rename publishes the set atomically only
after restore succeeds. Files are created with umask077 and directories mode700.
Allow disk space for the dump and one full scratch database during capture.

Only after a new successful publication does retention keep the newest14
completed sets, ordered by their UTC timestamp names. Failed dumps/restores never
prune prior sets; incomplete directories are ignored. A failed process normally
removes its staging directory, scratch database and lock. SIGKILL or power loss
can leave `.backup.lock`, `.pending.*`, or an `opencanvas_verify_*` scratch database.
If a lock persists, inspect its `pid` and running processes before removing that
specific stale lock. Inspect leftover scratch databases before deleting them;
do not delete the application database. Ordinary failures never break a lock
owned by another invocation. Logs/state are local and not committed to Git.

Recheck any completed set without changing the live database:

```sh
set_path="$root/backups/backup-YYYYMMDDTHHMMSSZ-XXXXXXXX"
"$root/backup-tools/restore-verify.sh" "$root/config/native.conf" "$set_path"
```

This verifies checksums before restoring, compares the canonical content
inventory after restore, and drops its scratch database. Checksums detect
corruption; they do not authenticate an archive from an untrusted sender.

## Recover into a new database

Keep the application stopped during recovery and start a new process only after
validation; do not reuse a running app's Moka cache against replaced data. Unload
the backup LaunchAgent and native app if loaded, leaving PostgreSQL running:

```sh
launchctl bootout "gui/$(id -u)/org.opencanvas.backup"
launchctl bootout "gui/$(id -u)/org.opencanvas.server"
```

Confirm any backup invocation has exited and its lock is clear before changing
configuration; inspect persistent locks as described above. Keep the schedule
unloaded until the recovered database's backup configuration is verified, so it
cannot silently capture the old database during recovery.

Preserve the current database and a private copy of the original completed set
outside `BACKUP_DIR` until recovery is accepted: the validation backup below
applies normal retention. First run `restore-verify.sh` above. Then restore to a
**new** database name using the cluster owner's private socket. If `createdb`
finds an existing destination, this subshell exits before `pg_restore` can run:

```sh
(
  set -euo pipefail
  umask 077
  source "$root/config/native.conf"
  recovery_db="opencanvas_recovered_$(date -u +%Y%m%d%H%M%S)"
  "$PG_BIN/createdb" --no-password --host="$PGADMIN_HOST" --port="$PGPORT" \
    --username="$PGADMIN_USER" --maintenance-db=postgres --template=template0 \
    --owner="$PGUSER" "$recovery_db"
  "$PG_BIN/pg_restore" --no-password --host="$PGADMIN_HOST" --port="$PGPORT" \
    --username="$PGADMIN_USER" --role="$PGUSER" --dbname="$recovery_db" \
    --single-transaction --exit-on-error --no-owner --no-acl "$set_path/database.dump"
  "$root/backup-tools/backup-fingerprint.sh" "$root/config/native.conf" "$recovery_db" \
    > "$root/backups/recovery-fingerprint.tsv"
  cmp "$root/backups/recovery-fingerprint.tsv" "$set_path/fingerprint.tsv"
  printf 'Validated fresh recovery database: %s\n' "$recovery_db"
)
```

After comparison succeeds, record the validated database name printed above.
While the app and backup schedule remain stopped, privately edit **both** files
to select that exact name:

- `config/server.env`: change the database selected by `DATABASE_URL`.
- `config/native.conf`: change `PGDATABASE`; backup tools do not read the app's
  `DATABASE_URL` and otherwise continue backing up the old database.

Retain the existing private credential handling and file modes. The installed
`config/pgpass` entry uses a database wildcard, so a new database name needs no
password change; if a custom passfile restricts database names, update it
privately too. Do not print credentials or put a database URL/password in argv
or shell history. Verify the two private configs select the same recovered
database, then create an immediate backup (without `--if-due`):

```sh
"$root/backup-tools/backup.sh" "$root/config/native.conf"
```

Identify the newly published completed set, check its `metadata.txt` records
`database=<validated recovered name>`, and run `restore-verify.sh` against that
new set as shown above. Do not accept recovery if this backup or verification
fails. Then start a fresh native app through the runtime procedure and check
project IDs/revisions, asset/font bytes, MCP, rendering, and viewer sign-in/privacy
before accepting traffic. Reload the backup schedule only after these checks:

```sh
launchctl bootstrap "gui/$(id -u)" "$HOME/Library/LaunchAgents/org.opencanvas.backup.plist"
```

A failed restore leaves an unused recovery database for inspection, not an
overwritten live database. Restoring to a previously running database requires
either a cold app restart or cache-namespace rotation; the new-name procedure
uses a cold app.

## Focused verification

```sh
shellcheck scripts/native/backup*.sh scripts/native/restore-verify.sh \
  scripts/native/install-backup.sh scripts/native/tests/backup-integration.sh
bash scripts/native/tests/backup-integration.sh
```

The integration test starts an isolated disposable PG16 cluster under
`/private/tmp` using a private socket with no TCP listener. It checks real
whole-database restore, quiescent source equality, binary/history/auth/sequence
and large-object preservation, corruption/fingerprint rejection, TERM cleanup
after CREATE succeeds, collision no-drop, failed dump/restore no-prune, exclusive
locking, retention14, local-day catchup, private modes and plist installation.
It never connects to the native or Docker runtime. PostgreSQL shared memory may
require running this test outside the sandbox; no dependency installation is
needed when the Homebrew PG16 tools are present.
