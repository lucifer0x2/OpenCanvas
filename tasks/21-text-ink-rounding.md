# Task21 — U07 text ink diagnostics across runtimes

**Status:** verified and deployed on3857; see
`docs/TEXT-INK-DIAGNOSTICS-FIX.md`. All ten warnings resolved; PNG/data unchanged.

User authorized review and fix of ten `TEXT_CONTENT_OUTSIDE_BOX` warnings on
2026-11-03. Source project3847 remains read-only; destination3857 must retain
its authored content. See `docs/API-MCP-MIGRATION.md` for measured evidence.

## Goal and scope

Explain and correct the destination-only warnings in `u07-a3-tile`, without
changing authored text, font, sizing, node IDs or resource bytes. Investigate
Canvas ink metrics versus layout advance width and actual clip boundaries.
Do not broadly suppress overflow diagnostics or relabel deliberate clipping.

Workspace `/Users/luis/Workspace/OpenCanvas`, branch `codex/kubernetes-local`,
shared dirty tree. Preserve all existing changes. Fresh worker only; no child
agents, commits, push, DB manipulation or native-service restart.

Likely modules: `crates/server/src/text_diagnostics/overflow.rs` and focused
tests; `renderer/measurement.ts` only if evidence establishes a measurement bug.
Keep public API stable, modules focused and preferably below250 lines. Add a
short report `docs/TEXT-INK-DIAGNOSTICS-FIX.md` with diagnosis and verification.

## Acceptance

- Explain all ten reported nodes using actual immutable scene measurements.
- A narrow correction eliminates rasterization/metric-rounding false positives.
- Regression controls retain genuine advance overflow, significant ink overhang,
  vertical overflow, and ancestor/own clipping, including a subpixel clip case.
- Missing ink evidence, hidden nodes, intentional overflow, diagnostic budgets
  and provenance retain their behavior.
- Run focused Rust tests and applicable renderer checks; no full suite by default.
- Root reviews the source delta before deployment. After review, build and update
  only the Kubernetes app, preserving Secret/PVC/PG and public viewer overlay.
  Restore both loopback forwards3857/3858 if replacement ends them.
- Render the actual U07 frame on3857 and verify its ten warnings are resolved.
  Validate current project snapshots/resources/revisions remain unchanged.
- No authored design mutations or generated blobs in Git; keep temporary evidence
  private and ignored. Record checks actually run and any remaining limitations.

## Deployment context

Kubernetes context `orbstack`, namespace `opencanvas-local`, app image
`opencanvas:kubernetes-local`. Existing public viewer overlay is active;
plain `local.sh apply` would disable it. Native3847/3848 is user-managed.
Read `docs/KUBERNETES.md`, `docs/KUBERNETES-PUBLIC-VIEWER.md` before deployment.
Tool session handles may still point3847; use explicit3857 MCP for destination
render verification, never mutate the native project through stale handles.
