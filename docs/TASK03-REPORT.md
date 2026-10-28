# Task03 — multi-viewport context

Implementation and checks completed in the shared `main` working tree, 2026-10-28.
No commit/push/reset/checkout, model/config/auth change, subagent/session, server
3847 operation or write to `~/.opencanvas`. Existing Task01/02 changes retained.
AGENTS/task status belong to root and were not edited by this worker.

## Decision and final API

Extended the existing `get_design_context` tool rather than adding a tool: the
same scoped authored/resolved context, theme/revision, diagnostics and cursor
already exist. Only `viewports` opts into computed work. Legacy single `viewport`
and no-viewport calls stay authored-only with the old viewport echo.

Request example:

```json
{"projectId":"p_example","frameId":"welcome-dashboard-stacks","viewports":[{"width":1040,"height":720},{"width":800,"height":720}],"properties":["name"],"budget":8192}
```

New response fields (other context/node fields omitted):

```json
{
  "computed": {
    "status":"ready",
    "coordinateSpace":"frame-local",
    "comparisonScope":"all-descendants-excluding-root",
    "comparedNodes":180,
    "viewports":[
      {"viewport":{"width":1040,"height":720},"status":"ready","jobId":"job_1","sceneKey":"key1"},
      {"viewport":{"width":800,"height":720},"status":"ready","jobId":"job_2","sceneKey":"key2"}
    ]
  },
  "reflow":true,
  "nodes":[{"id":"welcome-dashboard-stacks","computedBounds":[{"x":0,"y":0,"width":1040,"height":720},{"x":0,"y":0,"width":800,"height":720}]}],
  "truncated":true,
  "nextCursor":"opaque continuation"
}
```

IDs/keys/cursor above are illustrative. Bounds have one slot per requested
viewport. Pending/failed slots are null. `computed.status` aggregates viewport
states with failed > pending > ready. `reflow:null` means no successful comparison
yet (including a one-entry array). When ready at ≥2 viewports, compare exact
descendant x/y/width/height across the full frame, independent of depth/property
filters or page boundaries, excluding only the viewport-resized root. The root
still receives bounds. Both authored and resolved props remain separate.

False emits Task01's `NO_REFLOW_INFORMATION`, preserving authored dimensions.
All-free scopes retain its existing message; stacks with unchanged bounds get
`reason:UNCHANGED_BOUNDS` and an honest observed-equality message. True suppresses
that diagnostic for the successful measured comparison. Pending/failure never
claims `reflow:false`; an authored all-free diagnostic may still be present.

## Cold work, cache and failure behavior

- Reused `SceneRequest`, the current immutable snapshot, process-local Jobs cache
  and persistent Chromium worker; no new layout implementation or measurement
  path. New private Jobs entry point retains errors for context readback, while
  normal render_frame enqueue still retries errors. Newest matching cache record
  wins after explicit retries. This is the only jobs-interface change.
- Context never polls/waits for the worker. Cold calls return pending job IDs;
  poll them using existing render_frame/HTTP, then repeat the same context
  request without cursor until ready. Begin pagination then. A normal cache hit
  returns complete geometry immediately. Subsequent pages can become pending
  after cache eviction; restart the ready read then.
- Resource errors, worker timeout and queue saturation produce null comparisons
  and explicit per-viewport errors. A strict PNG error also prevents comparison
  success, even if a non-strict scene is available. Correct the issue and retry
  the failed viewport explicitly through render_frame at the same revision,
  theme and scale 1; context picks the new job. Queue rejection has no job ID and
  can be retried by repeating context.
- Cold work currently includes strict PNG generation as well as layout and font
  measurement. This intentionally reuses the existing tested jobs path; it is
  more work than a future bounds-only path. Warm render_frame requests hit the
  same job/scene keys (integration assertion).
- Bounds validation rejects incomplete/non-finite/duplicate or foreign scene
  nodes before comparing; it never interprets missing nodes as unchanged.
- Limits: 1–4 viewport entries, dimensions >0 and ≤100,000, summed area ≤16M at
  scale 1; existing 2,048 scoped render nodes, 1 active + 8 queued, 32 records,
  45-second active-job deadline and worker 30-second deadlines remain. Queue
  waiting is outside the active-job timer; the context read never waits on it.
  No arbitrary breakpoint/schema expansion.
- Bounds stay with each complete node through the existing 64 KiB/200-entry/
  depth-8 pagination. Fixed metadata is byte-budget checked too. Every successful
  page carries truncated/nextCursor. An indivisible item or fixed metadata that
  exceeds budget returns CONTEXT_ITEM_TOO_LARGE; invalid count/area returns
  INVALID_REQUEST. These use the existing error envelope with no partial page.
  Cursor hashes include ordered viewports, theme, filters, scope and revision.

## Actual verification

The first sandboxed integration command failed before any test body could bind
random loopback: `Os {code:1, PermissionDenied, Operation not permitted}` at
support/mod.rs:33. Approved escalation then allowed loopback/Chromium; there was
no automatic approval rejection. No unrestricted sandbox mode was used.

An initial fixture-only failure (a frame with no parent page) was corrected to
valid storage authoring. No production change was needed for that failure.

Final commands actually completed:

```sh
rtk bypass cargo test -p opencanvas-server --offline -- --test-threads=1 --nocapture
# exit 0: 18 integration tests passed across 8 suites; no ignored/filtered tests.
# Includes all 6 new Task03 tests plus the existing 12 tests.
rtk bypass cargo clippy -p opencanvas-server --offline -- -D warnings
# exit 0 (production target).
cargo fmt --all --check
# exit 0.
rtk bypass git diff --check
# exit 0, including final documentation changes.
```

New tests cover:

- Actual MCP initialization/tools/calls on temporary roots/random ports, creating
  the unchanged JS welcome free/stacks fixtures via MCP and computing 1040×720
  and 800×720 with actual Chromium. Free: false; stacks: true, even with only the
  root on page one (`limit:1, depth:0, properties:[name]`). Root bounds alone
  differ in both cases, proving the root is excluded from comparison.
- Every scoped node's bounds through 8 KiB pages, no duplicate/missing nodes,
  stable global reflow/diagnostics, changed ordered-viewports/theme/property
  query and changed revision rejecting cursors. Both legacy request shapes.
- Fixed stacks with unchanged child bounds and the honest diagnostic, one/four
  viewport arrays, negative/empty/excess/area limits, mutually exclusive legacy
  viewport and metadata/item budget errors. Real theme cache separation.
- A deterministic hanging worker: immediate pending/null bounds, timeout
  readback without retry loops, explicit retry selects the newest job; queue
  saturation is retryable and never fabricates false.
- Actual missing-image strict failure produces null comparison and retained
  errors; restoring only the temporary asset and explicit retries recovers.
- A fresh worker with real bundled-font text measurement (direct temporary
  storage commit avoids automatic mutation render): pending then ready reflow,
  exact cache reuse by render_frame, nonempty real text line list.

Last full-suite measurements on this machine (single samples, not latency
budgets/benchmarks): true cold worker enqueue/read **4.390 ms**, total through
ready **327.160 ms** at 320×200/160×200. Sample fixture reads **14.984/33.439 ms**
(free/stacks) and additional settle **348.383/270.280 ms** at 1040×720/800×720.
Sample runs use normal MCP apply_batch, which already starts an automatic render,
so they are not pure cold-start measurements. The settle helper polls at 50 ms.
Read CPU/JSON costs remain, and large frames/slow resources can cost far more.

Only existing shared test-helper dead_code warnings appeared in test targets;
strict production Clippy is clean. No layout/storage/web/sample/E2E source
changes, live viewer QA or Task05 work. No 3847 restart performed; the existing
live process has not loaded this code delta. Read-only process inspection after
tests found only the known existing live server PID12919 and its worker PID13009,
not an additional test worker.

## Files and sizes

Production: schemas.rs, instructions.rs, reads.rs, reads/tree.rs,
reads/pagination.rs, reads/diagnostics.rs, new reads/viewports.rs, jobs.rs.
Tests: new multi_viewport.rs, multi_failures.rs, multi_resources.rs and
multi_support/mod.rs. Docs: SERVER.md, CONTRACT.md and this report.
Task01-owned changes already present in shared files were preserved.

After rustfmt all modified/new code files are ≤250 lines; no size exception:

| File | Lines |
| --- | ---: |
| src/schemas.rs | 114 |
| src/instructions.rs | 34 |
| src/reads.rs | 70 |
| src/reads/tree.rs | 152 |
| src/reads/pagination.rs | 104 |
| src/reads/diagnostics.rs | 37 |
| src/reads/viewports.rs | 163 |
| src/jobs.rs | 233 |
| tests/multi_viewport.rs | 185 |
| tests/multi_failures.rs | 91 |
| tests/multi_resources.rs | 108 |
| tests/multi_support/mod.rs | 63 |

Paths in this table are relative to crates/server. Root retains responsibility
for AGENTS/task status updates and independent acceptance.

## Root independent acceptance

Root reviewed the implementation and ran the full offline server suite again:
**18/18 integration tests passed** with loopback/Chromium and temporary roots.
Production Clippy `-D warnings`, `cargo fmt --all --check` and diff-check passed.
No required findings remained. The user backend on3847 was kept unchanged and
continues to serve its previously loaded binary; no live rollout was performed.
