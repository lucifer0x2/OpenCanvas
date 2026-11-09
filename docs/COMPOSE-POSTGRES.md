# Local PostgreSQL in Docker Compose

`compose.postgres.yaml` runs one OpenCanvas instance with PostgreSQL 16 on an
internal Docker network. Only OpenCanvas publishes a host port. The base stack
serves the viewer and MCP on loopback 3857. The `opencanvas-postgres` Compose
project stores authored data and resources in its `postgres-data` named volume.
The PostgreSQL image matches the Kubernetes digest and the app reuses the local
`opencanvas:kubernetes-local` image. The default `compose.yaml` serves SQLite.

Both Compose configurations pass `RENDER_BACKEND` into the app container and
default it to `rust`. Set `RENDER_BACKEND=chromium` in the Compose env file to
select Chromium instead. The public-viewer overlay preserves this setting.
After changing it, recreate just the app with
`compose_pg up -d --no-deps --force-recreate opencanvas`, keeping the same overlay
arguments when enabled. Native backend limits are documented in
[SERVER.md](SERVER.md#experimental-native-render-backend).

## Start and stop

Run from the repository root with Docker Compose on OrbStack. If the app image
is missing, build it with `docker --context orbstack build -t
opencanvas:kubernetes-local .`. Set `OPENCANVAS_IMAGE` in the env file to select
another locally available tag.

Create the credential file once for a new stack. Store it outside the repo and
retain the same password with the initialized volume. The app embeds the hex
password in the database URL and PostgreSQL uses it for the `opencanvas` role.

```sh
config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/opencanvas"
compose_env="$config_dir/compose-postgres.env"
(umask 077; set -C; mkdir -p "$config_dir" && password=$(openssl rand -hex 24) && \
  printf 'OPENCANVAS_DATABASE_PASSWORD=%s\n' "$password" > "$compose_env")

compose_pg() {
  docker --context orbstack compose --env-file "$compose_env" -f compose.postgres.yaml "$@"
}

compose_pg config --quiet
compose_pg up -d --wait --wait-timeout 180
curl --fail http://127.0.0.1:3857/api/v1/projects
compose_pg ps
```

Use `compose_pg stop` to stop both services while keeping the database volume.
Keep `config --quiet` when validating with real credentials. Compose's rendered
configuration includes the password and database URL. Set `OPENCANVAS_PORT` to
change the listener and published port together.

## Public viewer

Add these settings to the same env file before enabling the public overlay.
Replace the example with your HTTPS origin without a path. Set both ports
explicitly because the shared overlay also supports the SQLite defaults.

```dotenv
OPENCANVAS_PORT=3857
OPENCANVAS_PUBLIC_PORT=3858
OPENCANVAS_PUBLIC_ORIGIN=https://machine.tailnet-name.ts.net
```

For password protection behind Funnel, set
`OPENCANVAS_PUBLIC_TRUSTED_PROXY_IP` to the peer IP that the app sees on
connections from the proxy. On this OrbStack setup the loopback-published
port reaches the container from the Compose bridge gateway. Find the gateway
with the following command and verify it against the published-port peer if
the network or proxy setup changes.

```sh
docker --context orbstack network inspect opencanvas-postgres_default \
  --format '{{range .IPAM.Config}}{{println .Gateway}}{{end}}'
```

```sh
compose_pg -f compose.public-viewer.yaml config --quiet
compose_pg -f compose.public-viewer.yaml up -d --wait --wait-timeout 180
```

The public listener serves projects marked `public` in read-only mode and
allows their exports. All projects are private by default. Use
`update_project` on MCP port 3857 to publish selected projects.
For loopback testing, send the configured public Host and Origin headers to
`http://127.0.0.1:3858/api/v1/viewer` and expect `{"readOnly":true}`. If enabling
Funnel separately, point it at loopback 3858. Set or clear the shared password
and manage sessions through MCP on 3857. See the
[MCP tool arguments](SERVER.md#public-viewer-password). Use the HTTPS Funnel
URL for browser sign-in so it accepts the secure cookie.

An image from before project visibility support serves the full project list
on the public port. If rolling back to such an image, stop the public listener
or disconnect its proxy before starting the older image.

## Restore a Kubernetes database dump

Rehearse using a separate project name (`-p`) and env file with free loopback
ports. Freeze writes to the Kubernetes source before taking the final custom
format PostgreSQL dump. Keep the source PVC and credentials for rollback. Stop
the Kubernetes port-forward before publishing the same ports through Compose.

With the final dump in `$backup`, start PostgreSQL on a fresh Compose volume
before starting the app. Run the restore in a single transaction.

```sh
compose_pg up -d --wait --wait-timeout 180 postgres
compose_pg exec -T postgres pg_restore --username=opencanvas --dbname=opencanvas \
  --single-transaction --no-owner --no-acl < "$backup"
```

Start the app with the appropriate command above after restore. Before resuming
writes, verify project IDs and revisions, resources, MCP and PNG rendering on
the local listener. If using the public overlay, also verify read-only access
with the expected Host and rejection of mutation requests. For rollback before
Compose accepts writes, stop Compose, restart the source workloads and restore
their loopback port-forward. If Compose has accepted writes, preserve its
database and reconcile those writes before reactivating the old source.

## Local OrbStack cutover

The `opencanvas-postgres` project now serves the migrated database on this host.
The app publishes loopback `3857` for MCP and `3858` for the read-only viewer.
The database is on the Docker volume `opencanvas-postgres_postgres-data`.
The local env file is private and Git-ignored. Backups and parity checks are
also under `.runtime/compose-postgres/`.

The 2026-11-04 app rollout uses the local `opencanvas:project-privacy-local`
image. The existing PostgreSQL container and volume serve the same authored
snapshots. Existing projects start private and the HTTPS Funnel viewer shows
an empty list until a project is published.

```sh
docker --context orbstack compose --env-file .runtime/compose-postgres/prod.env \
  -f compose.postgres.yaml -f compose.public-viewer.yaml ps
```

The source app and database were scaled to zero and the PVC was retained. On this host,
`orb stop k8s` stopped the Kubernetes core containers and briefly restarted
Docker. Compose recovered automatically and both snapshot checksums still
matched. The PostgreSQL volume and final dump are separate from the source PVC.
