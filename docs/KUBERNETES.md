# Local Kubernetes (PostgreSQL)

This deployment runs the actual PostgreSQL storage backend and Chromium worker
on the existing local OrbStack cluster. It is separate from the native server,
Docker Compose and `~/.opencanvas`; it does not migrate existing projects.

Use one application replica with `Recreate`. MCP sessions, render jobs, SSE
wakeups and Moka snapshots are process-local. Restarting loses sessions/jobs and
causes downtime; PostgreSQL retains authored projects, revisions and resources.
This is a local deployment, without HA, backup automation, TLS or public ingress.

## Build and apply

Prerequisites: OrbStack Docker and Kubernetes enabled, `kubectl`, Bash, OpenSSL,
and Node 18+ for the optional smoke. Run from the repository root:

```sh
kubectl --context orbstack get nodes
bash deploy/kubernetes/local.sh build
bash deploy/kubernetes/local.sh apply
bash deploy/kubernetes/local.sh status
```

All Kubernetes commands explicitly use context `orbstack` and namespace
`opencanvas-local`. The app image is `opencanvas:kubernetes-local`, built from
current source with the existing multi-stage Dockerfile. OrbStack shares its
Docker image store with Kubernetes; `imagePullPolicy: Never` prevents fetching
an unrelated registry image. `apply` always replaces the app Pod, so repeating
build + apply uses the rebuilt tag and current ConfigMap.

The PostgreSQL 16 Alpine image is pinned to the cached multi-platform digest
in both its StatefulSet and the app's startup gate. The image's entrypoint
initializes storage ownership; the app and startup gate run as uid/gid 1000.
PostgreSQL requests a 2 GiB ReadWriteOnce PVC using the cluster's default
StorageClass (`local-path` on the verified host). No host data directory is mounted.

The first apply generates a random password directly into the Kubernetes Secret
`postgres-credentials`. No credential file is created or printed. Later applies
keep the Secret and PVC; if a PVC exists without its Secret, setup refuses to
generate a replacement password. Restore the original Secret in that situation.
Do not enable shell tracing or print Secret data. Kubernetes Secrets are not a
production secrets-management system; access follows the local cluster's RBAC.

Apply first validates against the Kubernetes API with `--dry-run=server`, then
waits for PostgreSQL and application rollout. It creates no Ingress, NodePort,
LoadBalancer, public viewer listener or public endpoint.

For the opt-in read-only listener on loopback 3858 and later user-managed Funnel,
see [Kubernetes public viewer](KUBERNETES-PUBLIC-VIEWER.md). Ordinary `local.sh
apply` restores local-only mode; use its dedicated public apply/access commands
to keep both ports enabled. The shared manifests live under `deploy/kubernetes/base`.

## Access

In a separate terminal:

```sh
bash deploy/kubernetes/local.sh access
```

- Viewer: [http://127.0.0.1:3857](http://127.0.0.1:3857)
- MCP: `http://127.0.0.1:3857/mcp`

This runs `kubectl --context orbstack -n opencanvas-local port-forward --address
127.0.0.1 service/opencanvas 3857:3857`. Stop forwarding with Ctrl-C; workloads
continue running. Port-forward attaches to a particular Pod and exits when that
app Pod is replaced, sometimes only when the next request arrives: stop the old
forward with Ctrl-C and run `access` again after restart or apply.

Port 3857 is deliberately different from native 3847/3848. Host port and internal
app port must match: generated render URLs and the Origin guard use the app's
configured port. If changing it, update `PORT` in `base/config.yaml`, the Service and
container port in `base/app.yaml`, the `access` command and smoke script together.
Do not publish a different host port or open these unauthenticated APIs publicly.

## Lifecycle and resource limits

The PostgreSQL startup gate allows about three minutes for first initialization.
An app startup probe allows another three minutes for HTTP to open. Readiness
executes a timed loopback `GET /api/v1/projects`, exercising real DB access while
using a permitted Host. Liveness fetches viewer HTML without DB access, so a DB
outage removes readiness without repeatedly restarting a healthy app process.
PostgreSQL uses `pg_isready` for its probes; app readiness supplies the actual
database-query check.

The app is nonroot, drops capabilities, disables privilege escalation and runs
with a read-only root filesystem. `/tmp` and `/data` are bounded writable
emptyDirs; PostgreSQL holds all authored state/resource bytes, so app scratch
does not need a PVC. Chromium gets a 512 MiB memory-backed `/dev/shm`, counted
within the app's 1536 MiB memory limit. Both workloads have CPU/memory requests
and limits and disable service-account token mounts.

The image has `STOPSIGNAL SIGINT`, which the verified OrbStack/Docker runtime
honors. The server currently handles SIGINT rather than SIGTERM; it drains for
at most three seconds within a 30-second Pod termination budget. `Recreate`
prevents app overlap during replacement. Other runtimes must honor image stop
signals before using these manifests; a runtime that overrides this with SIGTERM
will not exercise the server's graceful stop path.

See Kubernetes' [probe documentation](https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-startup-probes/)
and [Deployment strategy documentation](https://kubernetes.io/docs/concepts/workloads/controllers/deployment/),
plus [image stop signals](https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#stop-signals).

## Restart and verify persistence

```sh
# With access running; use a new private state filename each time.
smoke_dir=$(mktemp -d /tmp/opencanvas-kubernetes-smoke.XXXXXX)
state="$smoke_dir/state.json"
node docker/kubernetes-smoke.mjs create http://127.0.0.1:3857 "$state"
bash deploy/kubernetes/local.sh restart-app
# Re-run access in the other terminal, then:
node docker/kubernetes-smoke.mjs check http://127.0.0.1:3857 "$state"
bash deploy/kubernetes/local.sh restart-postgres
node docker/kubernetes-smoke.mjs check http://127.0.0.1:3857 "$state"
```

The smoke uses real MCP initialize/notification/tool calls, creates one labelled
`kubernetes-smoke-<time>` project, commits page/frame/text at revision 1, reads via
HTTP and MCP, computes layout with Chromium and downloads a 320×200 PNG. It
checks the generated image URL unchanged, font download, scene revision and
resource readiness. HTTP requests have timeouts and render polling is bounded.
The state file contains only fixture identity/revision and is created mode 0600.

`check` reconnects MCP and verifies the same project/revision/text after each
restart. Job IDs can change after app restart; authored identity must not.
There is no product delete-project API. To remove only this disposable fixture:

```sh
bash deploy/kubernetes/cleanup-smoke.sh "$state"
rm -- "$state"
rmdir -- "$smoke_dir"
```

The helper validates generated ID/slug patterns, deletes with both exact values
using SQL-quoted psql variables and requires exactly one removed project.
Foreign keys cascade only its dependent rows. It never drops a database/schema,
deletes a PVC or accepts a non-smoke slug. Do not use it for real project data.

## Stop and cleanup

```sh
# Stop workloads, retaining credentials, services and durable data:
bash deploy/kubernetes/local.sh stop
# Resume:
bash deploy/kubernetes/local.sh apply
```

Nothing deletes the namespace or PVC implicitly. To permanently erase this
isolated deployment and all of its projects, first stop port-forward and then
explicitly run:

```sh
kubectl --context orbstack delete namespace opencanvas-local
```

The local-path provisioner's reclaim policy controls physical PV cleanup.
Native processes and data are outside this namespace and these manifests.

## Verification on 2026-11-03

Verified on OrbStack Kubernetes `v1.35.6+orb1`, local arm64 Docker:

- Current-source image build passed, including Rust server/layout and viewer typecheck/build.
- Bash/Node syntax, Kustomize rendering and server-side dry run passed.
- PostgreSQL/app Ready; PVC Bound 2 GiB; Services ClusterIP only.
- MCP mutation committed revision 1; HTTP/MCP readback and Chromium render/PNG
  passed, 8544-byte 320×200 PNG and exact `http://127.0.0.1:3857` image URL.
- Same project/revision/text persisted across app Pod replacement and PostgreSQL
  Pod replacement; render and resource readback passed after both.
- Repeated apply preserved the existing Secret UID and PVC UID. A later app
  restart with no preStop hook logged `Stopping…`, verifying image SIGINT handling.
- Independent coordinator checks passed viewer 200, invalid Host/Origin 403,
  same Origin 200, MCP protocol `2025-11-25`, revision 1 snapshot and ETag 304
  with an empty response body.
- With PostgreSQL deliberately stopped, the app became NotReady and its DB
  endpoint returned 503 while viewer/liveness stayed 200. Restoring PostgreSQL
  returned the same app Pod to Ready without a container restart.
- The exact generated smoke ID/slug was removed with the helper, HTTP project
  listing confirmed empty, and the app was restarted to clear transient render
  jobs/cache. No smoke project or test rows are retained; deployment/PVC remain.

Local node/PVC persistence is not a backup. Restart verification does not cover
node loss, migration, multiple app replicas or external cluster installation.

All new implementation/runbook modules are under 200 lines. Existing `PLAN.md`
is a longer product-requirements document, preserved as the source of truth.
