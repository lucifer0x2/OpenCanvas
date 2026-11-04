# Kubernetes public read-only listener

The opt-in overlay adds the existing public viewer router on container/Service
port 3858 to the [local PostgreSQL deployment](KUBERNETES.md). Local port 3857
retains MCP and writes. Both routes share projects, resources, jobs and events.
One app replica and the existing PostgreSQL Secret/PVC remain in place.

With a current image, the public router serves projects marked `public` and
their notes, fonts and assets, with optional password protection. It allows
GET/HEAD and the sign-in and export POST endpoints. Exports leave authored
revisions intact. MCP,
project creation, font imports and other mutations are blocked independently
of peer address or forwarded headers. Rendering shares the local worker budget.

## Enable and forward

First complete `local.sh build` and `local.sh apply` from the parent runbook.
The existing image already supports the public flags; this overlay needs no
rebuild. Bash, Node 18+ and kubectl are required, including Node for origin
validation even when not running smoke checks.

```sh
# Supply your machine's actual HTTPS origin, without a DNS trailing dot.
export OPENCANVAS_PUBLIC_ORIGIN=https://machine.tailnet-name.ts.net
bash deploy/kubernetes/public-viewer.sh apply
# Keep running in another terminal:
bash deploy/kubernetes/public-viewer.sh access
```

`apply` requires an explicit origin on every invocation. It validates the
ConfigMap and Kustomize overlay with server-side dry run before applying. The
origin goes into the cluster ConfigMap `opencanvas-public-viewer`, rather than
tracked configuration. The overlay passes `--public-viewer-port 3858` and
`--public-viewer-origin $(PUBLIC_VIEWER_ORIGIN)` using that ConfigMap. Repeated
apply keeps the origin and ports consistent and replaces the app Pod to refresh
environment values. It does not rotate credentials, replace PVCs or migrate data.

Both Services remain ClusterIP; there is no Ingress, NodePort or LoadBalancer.
The foreground access command binds **127.0.0.1 only** and forwards both ports:

- Local viewer/MCP: `http://127.0.0.1:3857`, MCP `/mcp`.
- Public router upstream: `http://127.0.0.1:3858` with configured public Host.

Stop the old forward before starting this combined command. After any app Pod
replacement, restart forwarding; kubectl attaches to a particular Pod and its
old connection can fail only on the next request. `public-viewer.sh restart-app`,
`restart-postgres`, `wait` and `status` reuse the local lifecycle helpers.

**`local.sh apply` restores the default local-only configuration**, removing the
public flags, public env reference and Service port. The public ConfigMap remains
unused. Re-enable by running public apply again. `local.sh restart-app` preserves
the currently applied configuration. Ordinary local Kustomize rendering remains
at `deploy/kubernetes`; the manifests live in `deploy/kubernetes/base` and the
opt-in overlay is `deploy/kubernetes/public-viewer`.

## Host guard and later Funnel

Bare localhost navigation/curl to 3858 returns 403 because public Host must match
the configured origin. A supplied Origin must match too. Test locally with:

```sh
public_host=${OPENCANVAS_PUBLIC_ORIGIN#https://}
curl --fail -H "Host: $public_host" -H "Origin: $OPENCANVAS_PUBLIC_ORIGIN" \
  http://127.0.0.1:3858/api/v1/viewer
# {"readOnly":true}
```

Public scene resources, font downloads, PNG and export links use the configured
HTTPS origin. Before Funnel is configured, test their paths through loopback
with the public Host header; they are not working Internet URLs yet. Local MCP
and worker callbacks retain `http://127.0.0.1:3857` URLs.

Funnel/Serve is **not enabled or altered by these scripts**. When ready, the user
can run the [Tailscale Funnel CLI](https://tailscale.com/docs/reference/tailscale-cli/funnel):

```sh
tailscale funnel --bg http://127.0.0.1:3858
```

Point Funnel at **3858 only**. DNS, TLS, tailnet policy and external reachability
remain Tailscale configuration. Never point it at local MCP/write port 3857.

## Targeted smoke and cleanup

```sh
smoke_dir=$(mktemp -d /tmp/opencanvas-kubernetes-public.XXXXXX)
state="$smoke_dir/state.json"
node docker/kubernetes-smoke.mjs create http://127.0.0.1:3857 "$state"
node docker/kubernetes-public-smoke.mjs "$OPENCANVAS_PUBLIC_ORIGIN" "$state"
```

The first command authors one labelled disposable project via local MCP. The
second checks public HTML/readOnly/list/snapshot/font, Host/Origin rejection,
spoofed forwarded headers, blocked MCP/create/import/mutation methods, scene
resource URL rewriting, PNG/SVG exports, unchanged authored snapshot and local
MCP access. It uses native Node HTTP to preserve a custom Host header. The local
PNG is 320×200 and SVG embeds the font. All requests/polling are time-bounded.

After independent checks, remove only the exact generated fixture with the
existing ID+slug-guarded helper. Restart to clear transient jobs/cache, then
restart the combined forwarding command:

```sh
bash deploy/kubernetes/cleanup-smoke.sh "$state"
bash deploy/kubernetes/public-viewer.sh restart-app
rm -- "$state"
rmdir -- "$smoke_dir"
```

No project-delete product API exists. Never use the cleanup helper for real data.
Local node/PVC persistence is not a backup; this remains one replica without HA.
Funnel/TLS end-to-end verification is outside this listener setup.

## Verification on 2026-11-03

- Bash/Node syntax, both Kustomize renders and server-side dry runs passed.
  The default local-only server dry run removes public flags/env/port as intended.
- Two public applies completed, without rebuilding the existing image. App and
  PostgreSQL remained healthy; existing Secret/PVC identities were preserved.
- Local MCP authored revision 1 and rendered a 320×200, 8544-byte PNG. Public
  smoke passed before and after repeated apply: 34 rejected MCP/mutation probes,
  Host/Origin guards, readOnly capability, snapshot/font reads, public scene/PNG
  URLs and SVG/PNG export downloads. SVG embedded the font; authored snapshot
  remained identical. Local jobs retained loopback URLs and MCP protocol
  `2025-11-25` continued working.
- Independent coordinator checks confirmed the public capability, matching
  local/public project lists, font download, Host/Origin rejection and blocked
  writes with spoofed loopback headers, plus real local MCP initialization/list.
- The exact fixture ID+slug was removed, app restarted to clear transient jobs,
  and combined loopback forwarding restored. No fixture project remains.

Funnel/Serve was not enabled or changed, so Internet DNS/TLS access was not tested.
All new implementation/runbook modules are under 200 lines.
