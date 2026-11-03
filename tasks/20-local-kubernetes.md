# Task20 — Local Kubernetes deployment

**Status:** Implemented and verified on 2026-11-03. Commands, observed results and limitations: [local Kubernetes runbook](../docs/KUBERNETES.md).

User authorized a new branch from `feat/postgres-runtime-storage` and local Kubernetes setup on 2026-11-03.

## Workspace and scope

- `/Users/luis/Workspace/OpenCanvas`, branch `codex/kubernetes-local`, base `d93db0742c151d127698290344c6ad797396309b`.
- Existing local Docker/Kubernetes is OrbStack, context `orbstack`, Kubernetes `v1.35.6+orb1`; use an isolated `opencanvas-local` namespace. No kind cluster is needed for this host.
- Fresh worker implements directly and does not create agents. Root coordinates and independently verifies. No commit/push, native-server restart, real-data migration, or writes to `~/.opencanvas`.
- Own `deploy/kubernetes/**`, deployment scripts under `scripts/` or `docker/`, `docs/KUBERNETES.md`, README Kubernetes link; minimal Dockerfile changes only if required. Do not change unrelated Rust/viewer code.

## Goal and boundaries

Deliver reproducible local Kubernetes manifests and runnable setup for the actual PostgreSQL runtime, then deploy and smoke-test it on this machine. Use current source in a distinct local image tag. Keep OpenCanvas one replica with Recreate strategy because MCP sessions/jobs/SSE wakeups remain process-local. No HA or zero-downtime claim.

Use ClusterIP services only; expose to the host through explicit loopback port-forward on a free port different from 3847/3848. Host-facing port must match the internal application port because Host/Origin guards and generated URLs depend on it. Do not expose public ingress, NodePort or LoadBalancer.

## Acceptance and checks

1. Small focused manifests/Kustomize modules for namespace, PostgreSQL StatefulSet + persistent PVC, app Deployment and services; app config chooses PostgreSQL and bounded pools/timeouts. Pin practical images using available versions. Keep all secrets out of tracked files and stdout; generated local credentials live only in Kubernetes or ignored/private temporary files. Re-running setup must preserve credentials/data.
2. Correct lifecycle: readiness checks actually exercise database access without Host-guard failures; distinguish DB-dependent readiness from liveness; startup budget; graceful application stop compatible with current SIGINT-only server and image STOPSIGNAL. Include Chromium shared memory, writable temp directories, nonroot app, resource requests/limits and disabled service-account token where appropriate.
3. Reproducible commands/scripts for build, apply, rollout, access, restart and cleanup with explicit context/namespace. Never delete namespace/PVC implicitly. Existing services and native processes stay intact. Prefer existing OrbStack, optionally document kind image loading.
4. Actually build the current Docker image and deploy it. Validate via server-side dry run where available, readiness and runtime smoke: HTTP + real MCP initialization/mutation/readback, Chromium layout/render PNG and correct resource URLs. Use clearly labelled temporary smoke project; remove it afterwards through supported means or isolate fixture storage/namespace if necessary. Never delete real data.
5. Demonstrate durable project/revision readback after app Pod restart and PostgreSQL Pod restart. Do not claim coverage without running it. Leave requested deployment running, empty of smoke fixture if supported; report any retained fixture explicitly.
6. Run targeted syntax/manifest checks, inspect diffs and module sizes. Aim 100–200 lines/file; evaluate >250 and avoid >350 handwritten code. No full unrelated suites. Report all changed files >250 lines, commands/results, running resources and access instructions.

## Output

Source manifests/scripts, `docs/KUBERNETES.md` with runbook, actual verification results and limitations; final handoff with files, checks, endpoints and remaining blockers. Root owns progress log and AGENTS state.
