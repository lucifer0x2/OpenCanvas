# Task20b — Kubernetes public read-only listener 3858

**Status:** Implemented and independently verified on 2026-11-03. See [public listener runbook](../docs/KUBERNETES-PUBLIC-VIEWER.md) for commands, results and limitations. Listener/loopback forwarding is running; Funnel remains unconfigured.

User explicitly requested adding read-only listener `3858` on 2026-11-03 after Task20.

## Handoff

- Workspace `/Users/luis/Workspace/OpenCanvas`, branch `codex/kubernetes-local`; preserve the existing uncommitted Task20 work and unrelated log entries. One fresh worker, no child agents; no commit/push.
- Goal: configure and deploy the existing public viewer router on port3858 in local namespace `opencanvas-local`, context `orbstack`; make it reachable by host-loopback port-forward for the user to point Funnel at later.
- Own focused configuration/scripts under `deploy/kubernetes/**`, public smoke under `docker/`, `docs/KUBERNETES-PUBLIC-VIEWER.md` and a short link/update in `docs/KUBERNETES.md`/README as needed. Root owns PLAN, AGENTS, ACCEPTANCE, LOG and task file. No unrelated Rust/frontend changes.
- Read AGENTS.md, relevant PLAN/README, Task20 handoff/runbook, docs/SERVER.md public-viewer section, existing public router and public tests. Use apply_patch, RTK for supported routine commands; keep modules focused under200lines where practical and report >250line files.

## Origin and scope

Root verified the local Tailscale daemon is Running and obtained its HTTPS origin from status, stripping the trailing DNS dot. Supply that machine-specific value through explicit origin env/config, optionally autodetected from Tailscale status; do not record it in tracked files. No serve config existed at handoff.

Only enable listener and loopback forwarding. Do not enable or alter Tailscale Funnel/Serve, install ingress, expose NodePort/LoadBalancer, alter other namespaces, restart native3847/3848, change DB credentials/PVC or migrate data. The public router intentionally shares all projects/resources read-only plus export generation.

## Acceptance

1. Reuse server flags `--public-viewer-port 3858 --public-viewer-origin ORIGIN`; existing local3857 MCP/write access stays intact. Public ClusterIP service/containerport3858. Prefer a small opt-in Kustomize overlay and explicit lifecycle commands so ordinary local setup remains reproducible without host-specific tracked origin. Public apply must be repeatable and keep origin/ports consistent; document how normal apply affects the opt-in listener.
2. Current image already contains public viewer support; no rebuild/full suite unless code actually requires one. Apply via server-side dryrun and actual rollout in the authorized namespace. Preserve existing Secret/PVC and user projects, no temporary overwrite of real origin.
3. Leave loopback port-forwards3857 and3858 running after rollout; no wildcard bind. Verify public viewer200 with configured Host, public API/access readOnly, bundled font read; reject wrong Host/Origin, MCP GET/POST, create-project, font import and other mutation methods even with spoofed loopback/forwarded headers. Do not expect bare localhost requests to public port to pass the Host guard; document local curl test using public Host.
4. Smoke real project authored via local3857, public readback and export/PNG/resource URL rewriting to configured HTTPS origin as appropriate; no authored revision changes from public reads/exports. Use uniquely labelled disposable fixture and clean only exact generated ID+slug in this isolated DB afterwards; existing cleanup helper may be reused. Restore app transient state if needed, leave user deployment and forwards running.
5. Check syntax, Kustomize render, targeted runtime checks, diff and module sizes. Root will independently verify read-only access and healthy local MCP. No claim Funnel/TLS end-to-end verified because Funnel remains off.

## Output

Configuration/scripts + public runbook, actual commands/results, hostname handling, current listening ports, restrictions and any blockers. Message root when a stable forwarding window is available before fixture cleanup; root verifies independently. Do not reuse old Task20 worker.
