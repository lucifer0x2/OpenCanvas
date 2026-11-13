# Task25 — proposed split for slices 4–5

Original assessment on `feat/incremental-validation`, source `a98012d`.
The user subsequently authorized slice 4 on 2026-11-13. Parts 4a–4d are committed
through `16b3ced`; the combined default gate passes (496 passed, 37 ignored).
Slice 5 remains a proposal and slice 6/caps remain pending. Task25 required
stopping after the slice 1–3 checkpoint when 4 or 5 was large; both were large.
The coordinator verification and matrix results belong in [TASK25-REPORT](../TASK25-REPORT.md).
Product requirements remain [Task25](../../tasks/25-incremental-validation.md).

The checkpoint is now measured: 463 enabled tests passed across the combined gate
and fixture/artifact rerun (36 default-ignored), clippy/package fmt passed, matrix
8/8 completed with five samples each. PG 30× strict/scoped/context medians are
2.673 / 3.528 / 33.981 seconds, missing 1.5 / 1.5 / 1 seconds. A no-regression
claim at 1× was not justified at that checkpoint. The table below records the
approved slice-4 split and the still-proposed slice-5 work; new evidence is in the report.

## Why separate handoffs are needed

- `reads.rs` currently gets an owned full snapshot; `reads/tree.rs` expands the
  project and context calls full validation/resolution again. Context also emits
  component definitions, diagnostics, notes and all resources unless compacted.
- `scene-input/input.rs`, context viewports, server jobs and `export/mod.rs` have
  separate full expansion/resolution entry points. Fixing tree reads alone does
  not scope render/inspect/export or the prerender admission path.
- `reads/find.rs` computes `scannedNodes` and `totalMatches` over the entire scope
  before paging. Its cursor is a revision/query hash plus match offset, not a
  traversal cursor. A simple early break changes the contract.
- Snapshot compression changes representation identity and HTTP validators. Page
  routes need a separate additive contract before viewer adoption.
- `events.rs` reconstructs commits from the durable log; broadcast messages only
  wake the stream. Adding pages to the broadcast alone cannot fix replay.
- Viewer navigation currently reconciles against one complete accepted snapshot;
  page payloads need manifest membership, separate scope keys and stale-response
  handling together. This affects URL fallback and virtual-node deep links.

## Proposed independently verifiable tasks

Each row is one fresh implementation worker, one focused commit and report entry.
All use this workspace/branch after verifying HEAD/status. Workers must not spawn
or reuse agents, change storage layout, push, deploy or mutate live services.

| Part | Goal and likely ownership | Acceptance / focused checks | Depends on |
| --- | --- | --- | --- |
| 4a | Pure page/target closure and scoped expansion/resolution in scene-input; small storage accessor using matching revision indexes | Full-reference parity for placed themes, cross-page/nested masters, overrides, fonts, navigation, stable virtual IDs/order and errors; unrelated-page growth does not increase expanded/resolved work. Preserve the full validator for untrusted snapshots. | 1–3 |
| 4b | MCP outline/subtree/context consume shared authoritative snapshots and 4a; focused modules under `server/src/reads/` | Byte-level response/cursor parity for compact/default/rows, components, notes, filters, diagnostics, missing/virtual targets, themes and error precedence. Keep full resource arrays when the existing contract does. Test cold/warm revision isolation. | 4a |
| 4c | Render/inspect, context viewports, export and job preparation/admission consume the same scoped inputs | Existing scene input, viewport, render-node/pixel budgets and provenance remain exact. Native/Chromium, export settings, fonts, stale jobs, queue/failure/cache fixtures; do not turn a bounded frame request into a whole-project expansion. | 4a; integrate 4b |
| 4d | Page-at-a-time find traversal with bounded result materialization and exact existing counters/cursor | All criteria, authored instance roots/virtual descendants, preorder/ties, paths, Unicode preview, budgets and revision/cursor errors match reference on first/middle/last pages and zero matches. Resolve the exact-total/early-stop issue below before coding. | 4a |
| 5a | Full snapshot gzip/br negotiation, representation cache and conditional GET | Decompressed bytes identical for objects/rows; encoding-specific strong validators, Vary, Accept-Encoding negotiation and 304 behavior, privacy/identity/readiness isolation, no SSE compression. | Can follow checkpoint independently |
| 5b | Add manifest/page GET and page cache with 4a closure | Additive local/public HTTP contracts, correct closure/resources/order, authoritative identity/ETag, deletion/restore/resource/master invalidation, old full snapshot clients preserved. Document whether page validator survives unrelated revisions. | 4a, 5a integration |
| 5c | Durable `changedPageIds` derivation for live/replayed SSE | Old and new page footprints for moves/deletes, transitive shared masters/resources, global fallback, replay/gaps/reconnect and privacy. Existing history/layout compatibility stays intact; do not infer old footprints solely from latest nodes. | 4a + agreed 5b invalidation |
| 5d | React/SWR page resource ownership and manifest-backed URL navigation | Typecheck, unit/HTTP and real Chromium hooks/navigation/Layers/font/All-frames regressions. Warm A→B→A, Back/Forward, partial/virtual URLs, project switching, late responses, one SSE, reconnect and unrelated-page no-refetch assertions. Read React skill/rules/preferences before editing or reviewing code. | 5b + 5c |

## Decisions to settle within those handoffs

1. **Find totals:** preserving exact `totalMatches`/`scannedNodes` means a cold
   arbitrary text/name query must examine the scope, even if only one page of
   matches is emitted. Bound expanded payload/result retention page by page;
   investigate revision/query count metadata for subsequent pagination. Do not
   relabel partial counts as totals, remove fields, change cursor offsets or claim
   cold global search is independent of project size. The literal early-stop
   requirement must be reconciled with these existing response semantics.
2. **Trusted reads:** skipping full validation is permitted only for snapshots
   obtained under the validated committed-revision invariant. Do not silently
   weaken standalone scene-input APIs, bundle/import validation or custom backend
   behavior without an explicit trust boundary and negative tests.
3. **Page revision:** the project revision remains the source of provenance and
   concurrency. Separate a closure-content validator from that global revision if
   unchanged-page reuse is desired; never attach a newer revision to old payloads.
   Manifest/global resources and public privacy/readiness changes must participate.
4. **SSE history:** make page invalidation reproducible from durable data, including
   historical old/new dependencies. Legacy or uncertain history must conservatively
   invalidate/refetch; broadcasts cannot be the sole source of page identities.
5. **Write budget:** 4–5 optimize reads/HTTP/viewer, while PG 30× still spends
   1.224–1.376 s validating plus full loads, candidate/reference-index work and
   cache fill. A separately scoped commit-performance follow-up is needed before
   claiming the write target, potentially reusing the 4a expansion API while
   preserving global guards and differential error parity. Do not silently add
   that write refactor to a read/UI handoff or treat slice 6 cap changes as a fix.

## Checkpoint and authorization boundary

Gates and the exact Task24 matrix are finished and target misses/regressions are
reported. The user approved the smaller slice-4 sequence; it is now implemented.
Do not start slice 5 from that authorization. Its proposed sequence still needs
user approval under Task25's **Sequence and authorization** boundary.

Slice 6 remains a later task after correctness/performance evidence. Keep
production node/note/bundle limits and the 2,048/frame renderer budget unchanged
in this checkpoint. Measurements at 200k do not authorize a 1M production cap.

Use temporary SQLite roots/disposable PostgreSQL only. Only the benchmark may GET
native3857 snapshot/assets; no writes to native3857/3858, PG2703,
`~/.opencanvas` or preview3877. Reports contain conclusions and rerun commands,
never private snapshots or generated benchmark output.
