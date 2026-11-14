# React frontend optimization — 2026-11-12

Branch: `codex/react-frontend-optimization`, based on
`feat/viewer-frame-paint` at `f54f7c2a841c246b27c70b783d9b6d28778a8b9f`.
The initial working tree was clean. Scope follows `main.tsx` into `App.tsx`
and its dependencies, using the `react-engineering` skill. Preview internals,
backend, deployment and user design data are outside this change.

## Baseline and method

The coordinator built pristine baseline sources in
`/private/tmp/opencanvas-react-baseline`, using the same installed dependencies
as the working branch. `vite build --outDir …/dist` passed: 198 modules,
entry JavaScript 673.65 kB (gzip 210.45 kB), CSS 18.51 kB (gzip 4.73 kB).
This measures emitted bundle size, not a browser latency or frame-rate SLA.

The coordinator also ran the existing production repaint harness against that
baseline: `node web/tests/all-frames-repaint.mjs --root
/private/tmp/opencanvas-react-baseline --label react-baseline --edits 3 --dpr 2`.
It uses a temporary SQLite backend, a 12-frame page and Chromium at DPR 2.
Baseline: cold 3,522 ms, warm 919 ms; median one-frame update at Fit 203 ms,
one scene paint/index, four React and four Konva commits, no update long tasks.
Timing includes the local backend and is diagnostic rather than an SLA.
The first attempt had misplaced WASM files in the temporary copy and was stopped
with browser/backend cleanup; its output is excluded. The corrected run passed
with no browser errors. Generated JSON/RGBA remain ignored under `artifacts/`.

## Changes and state ownership

- `main.tsx` loads only the selected route after ViewerGate grants access.
  RouteBoundary provides accessible loading, failure and reload retry states.
  Preview implementation is unchanged.
- App is 95 lines rather than 236. It owns shared navigation, viewport preset
  and shell preferences. SceneWorkspace owns scene subscriptions and retry;
  ViewerStatus renders provenance, notices and connection state. Progressive
  frame completions do not rerender Topbar or ScopeControls.
- HTTP remains the authority for snapshot and project-list membership.
  A pure selection/notice reducer makes fallback and explanation atomic.
  No-op list reconciliation retains references; retry keeps the accepted
  snapshot, ETag and monotonic revision. Closed SSE sources cannot call the
  next project's handlers or schedule duplicate reconnects.
- Layer expansion remains per row. Derived selection ancestry updates only
  the affected branches. Scene-node WeakMap indexes avoid repeated scans;
  only the owning CanvasFrame receives a changed selection prop.
- useCanvasFrames separates geometry from scene/resource publications.
  Packing caches retain geometry rather than obsolete scene/resource objects.
  Camera remains local to useCanvasTransform; same-scope refreshes preserve
  mounted Konva nodes, pan/zoom and active drag. Hook dependencies are explicit.
- FontManager mounts its content on first opening and retains it when closed,
  preserving recoverable form work. Unopened managers do no font/capability
  requests. Inventory fetching has its own hook; immutable import retries and
  mutation feedback stay in useFonts. A synchronous ref prevents duplicate
  submits before React rerenders. Selection reads four signature bytes;
  submission revalidates and encodes the complete bounded file once.
  Aborted downloads cannot trigger a browser save after blob loading.
- Persisted selection/preferences are validated; unavailable browser storage
  leaves the current tab usable. No dependency or backend contract changed.

Font state review: Content still composes four inventory, four mutation-feedback
and one capability responsibilities. Splitting clarifies real responsibilities;
it does not claim to reduce the total state count. Busy tracks submission,
retryable tracks an immutable frozen request, and error/notice also serve download
results. Keeping these explicit states avoids combining independent outcomes
into a general action store. The form's seven units remain five editable fields
plus validation/checking; no server data or derived fields are copied into it.

## Final bundle

`npm --prefix web run build` passed, with 209 modules and no large-chunk warning.
The build fingerprint was independently checked against current production
sources, including dirty/untracked source files.

| JavaScript | Baseline kB / gzip kB | Final kB / gzip kB |
| --- | --- | --- |
| Entry before authorization | 673.65 / 210.45 | 228.64 / 71.91 |
| Canvas route | included | 408.51 / 127.15 |
| Shared route code | included | 22.59 / 9.23 |
| Preview route | included | 20.29 / 7.78 |

The entry reduction is about 66%; it is deferred loading, not a 66% reduction
of all code needed by the canvas. Opening the authorized canvas also fetches its
route/shared chunks, totaling about 659.74 kB (gzip 208.29 kB). Additional chunk
requests are the trade-off. WASM/resource bytes are outside these JS figures.

## Production repaint comparison

Final command: `node web/tests/all-frames-repaint.mjs --label react-final
--edits 3 --dpr 2 --compare react-baseline`. Both runs used the same fixture,
dependencies and WASM bytes; final production sources were frozen.

| Observation | Baseline | Final |
| --- | ---: | ---: |
| Cold ready, ms | 3,522 | 2,972 |
| Warm ready, ms | 919 | 1,134 |
| Median edited-frame ready at Fit, ms | 203 | 165 |
| Median edited-frame ready at 100%, ms | 235 | 177 |
| Scene paint / index calls per edit | 1 / 1 | 1 / 1 |
| React / Konva commits per edit | 4 / 4 | 4 / 4 |
| Cold React commits | 25 | 23 |
| Pan frame interval p95 at Fit, ms | 16.8 | 16.8 |
| Retained offscreen pixels at Fit | 1,366,684 | 1,366,684 |
| Retained offscreen pixels at 100% | 45,000 | 45,000 |

Both Fit and 100% captures compared 2,881,536 pixels each: zero differing pixels.
No browser errors or update long tasks occurred. This short local comparison
does not establish a general latency gain: warm load was slower in the final
sample, and host load/timing varies. The reliable acceptance results are unchanged
pixels, bounded caching, retained camera behavior and targeted render counts.

## Verification

Coordinator independently ran:

- `npm --prefix web test`: 110 unit tests plus 10 snapshot HTTP tests passed.
  The HTTP command uses Node's experimental transform-types flag because an
  existing API error class uses TypeScript parameter properties.
- `npm --prefix web run test:react`: all four sequential browser groups passed:
  shell/storage/lazy/auth, project hooks in StrictMode, real layer/frame render
  counts, and deferred font work/file reads/submit races/immutable retry.
- `node web/tests/canvas-continuity.mjs`: passed revision/drag/resize/failure/
  retry/fit/scope/stale-job/GET-only assertions.
- `node web/tests/all-frames-smoke.mjs`: passed on final sources. The coordinator
  also copied the updated test and its helper into the pristine baseline source
  copy and ran it there successfully, confirming the document-scoped cache and
  default-All expectations are preexisting behavior rather than relaxed checks.
- `npm --prefix web run test:actions`: all nine groups passed, including actual
  exports, native clipboard, unavailable backgrounds and the Vite proxy.
- `npm --prefix web run test:fonts`: passed the race fixture and real backend
  import/download/conflict/restart/focus/narrow-layout smoke.
- An earlier focused run of 31 selection/state/node-link/frame-scope/HTTP tests
  and the standalone project-hooks browser regression passed.

The render probe observes exactly three of 27 visible rows and one frame
rerendering when selection moves between sibling layers. Toggling notices
renders no CanvasView, frames or rows. These are actual execution counters
attached only by the test harness, not production instrumentation.

Workers also ran the existing viewer smoke and `npm --prefix web run test:fonts`
successfully. Font checks include original download SHA-256, committed lost
response replay, real revision conflict and explicit resubmit, backend restart,
project isolation, public read-only controls, keyboard/focus and 390px layout.

The older generic viewer and continuity fixtures assumed a single-frame server
workflow while the product already defaults to All. Their setup now states that
scenario explicitly; continuity also checks the current project-to-All transition,
retained Mobile preset, skeletons without stale scene/bounds/hits, and exact fit.
Assertions were retained or updated to the existing product contract.
The node-actions unavailable fixture now explicitly selects server mode for its
intercepted pending/failed responses, and asserts interception plus no mounted
Stage. Canonical links and clipboard assertions are unchanged.

No ESLint script/config is provided in this package. TypeScript build, targeted
runtime tests and diff checks are the available checks. The coordinator inspected
the desktop fixture screenshot. No deployment, restart, commit or push occurred.

## Size and completion audit

At the preceding optimization checkpoint, all 47 changed/new frontend code and
test files were at most 250 lines.
Files above the soft 200-line target: CanvasView 202, usePageScenes 226,
state 242, app-shell smoke 219 and continuity smoke 249. These retain their
focused viewer/camera/transition or end-to-end scenario responsibilities;
no file crosses the mandatory split-review threshold. `git diff --check` passed.
AGENTS.md remains 71 lines and below 10 KiB; closed runtime log entries were
moved intact to `docs/agents/history/2026-11-12-closed-runtime.md`.

All temporary test servers, browser processes and user-data fixtures were cleaned
by their harnesses. No real viewer/database/runtime was changed. The working tree
contains the source/tests/report changes for review; no commit or push was made.

## Follow-up: project/revision node indexes and sidebar preferences

The user requested retaining indexes across same-revision JSON refetches and
project switches. Node arrays are authored data and change with project revision.
The project cache retains one latest index per visited project; revision
matches reuse it, newer revisions replace it, and older snapshots cannot replace
the latest entry. Array-based helpers retain a small auxiliary lookup so their
interfaces stay unchanged. useSnapshot binds accepted arrays before its accepted
callback and selection reconciliation, avoiding a redundant first build. App uses
the explicit snapshotNodeIndex entry point. A changed revision rebuilds even when
the array reference happens to be shared. Same-revision snapshot/resource metadata
is still accepted; only the authored node index is reused.

The user also changed App's left/right panel state to localStorage keys
`left-open` and `right-open`, both defaulting to true. Those edits are preserved.
Earlier bundle/repaint measurements above describe the preceding optimization
checkpoint, before these follow-up source changes.

Coordinator verification on the follow-up: build/typecheck passed; 27 focused
node-index/state/link/frame/layer/project unit tests passed; two StrictMode
snapshot-index/project-hook browser tests passed; App shell smoke passed including
sidebar close/reopen across reload and malformed/denied storage. The cache tests
assert no traversal of cloned arrays on matching revisions, project isolation,
A→B→A reuse, updated props/sort order, stale-revision protection and helper reuse.
The browser checks priming before reconciliation and preserves new font metadata
at the same revision. Worker also verified 10 snapshot HTTP tests and Layers
render smoke. No follow-up source/test file exceeds 250 lines; production files
are nodeIndex 58, useSnapshot 100 and App 95 lines. The latest index of each
visited project stays in tab memory until reload; no past-revision history is
retained by the project Map. No commit/push/deploy or native database access.
Coordinator also reran All frames smoke successfully on the completed follow-up:
WASM/server/fallback, frame selection/links/bounds, SSE/reload, camera/scope and
shared font/snapshot caches passed. Final diff check passed.

## Follow-up: SWR project server state and explicit ownership

Installed SWR 2.5.1 (`^2.5.1`, lockfile pinned). App now explicitly composes
`useProjectList`, navigation-only `useProject`, `useSnapshot`, and `useProjectLive`.
`useProject` returns neither the project list nor a snapshot and performs no I/O.
The scoped SWR provider sits below ViewerGate and is discarded on authorization
unmount. Left/right sidebar keys remain `left-open`/`right-open`, default true.

SWR owns list/snapshot data, validation status and errors. HTTP pagination is
authoritative for membership; SSE revision hints are a separate SWR resource,
derived into listed projects without adding unlisted ones. Snapshot request
keys name the latest endpoint, project, known minimum revision, representation,
visibility scope and metadata epoch. Their results are small revision pointers;
payloads/ETags are stored only under the actual accepted canonical revision.
This handles a latest response advancing beyond the list/event watermark without
pretending it is historical data for the older revision. A per-provider resource
only coordinates in-flight promises, AbortControllers and retention keys; it
contains no second snapshot-data cache or React fetch/loading state machine.

Normal same-revision events/list refreshes, StrictMode and A→B→A reuse accepted
snapshots. Focus/reconnect does not refetch immutable authored snapshots;
reconciling the list discovers a newer revision, while durable SSE replay covers
the initial snapshot/stream race. Multiple revision keys serialize reads of the
same latest endpoint, preserving a newer commit received during an in-flight
read. Reset, visibility changes and manual retry advance a metadata epoch, abort
an obsolete pending read and retain conditional GET/304 behavior. Visibility
also invalidates other cached project payloads. Snapshot 404 and authoritative
membership removal evict inaccessible data. Accepted snapshots prime node indexes
before selection reconciliation; stale and foreign payload checks remain in the
existing HTTP reader. A real provider unmount aborts pending snapshot work;
StrictMode replay reacquires the shared request before disposal.

Retention is eight latest project payloads and no historical snapshots. Eviction
advances that project's epoch, so an old immutable pointer or SWR dedupe record
cannot strand a missing payload on immediate revisit. Small request/pointer/epoch
records can accumulate for a resident project until eviction or authorization
unmount; this saves payload memory but is not a byte-bounded cache. Existing
node-index retention remains separate and unchanged.

Worker verification: `npm --prefix web run typecheck` passed. Focused Chromium
tests `project-hooks.test.mjs`, `snapshot-index.test.mjs`, and
`swr-project-cache.test.mjs` passed, covering request counts, canonical response
revision, pagination/membership, same-revision reuse, in-flight newer commits,
foreign/stale rejection, ETag304 identity, explicit retry abort/recovery, 404
eviction, list failure/retry and authorization lifetime isolation. The index
test additionally visits eight more projects and checks at most eight payloads,
then verifies a fresh GET after eviction even inside SWR's dedupe interval.
The former artificial fixture sequence commit9→304(snapshot3)→commit4 now uses
backend-valid monotonic revisions; 304 identity coverage remains at an explicit
same-revision visibility invalidation. Existing fallback/follow/cursor assertions
remain. `test:react` includes these tests and the font/capability SWR regressions.

No ESLint configuration exists. Vite's build warning that SWR's module-level
`use client` directive is ignored is harmless in this client-only viewer; no
bundler configuration was changed to suppress it. Earlier bundle/repaint numbers
above predate this dependency/refactor and are not measurements of this follow-up.

Font inventory and listener capabilities also use SWR. Inventory keys contain the
resource kind, project and visible revision watermark; unopened managers perform
no reads, reopening and A→B→A reuse cached inventory, and import/conflict/manual
refresh explicitly revalidate. Capability consumers share one pending read per
provider and preserve pending `null` and fail-closed behavior. Inventory metadata
for visited revisions remains until provider disposal; full snapshot payloads
have the separate eight-project retention policy described above. Font mutation
state, frozen import retries and download cancellation stay outside the read cache.

Coordinator independently verified this follow-up:

- `npm --prefix web run build` passed on final source, including TypeScript.
  Entry JavaScript is 246.37 kB / gzip 78.77 kB; canvas route is 408.01 kB /
  gzip 127.08 kB. The SWR directive warning above remains visible.
- `npm --prefix web test` passed 113 unit tests and 10 snapshot HTTP tests.
- `npm --prefix web run test:react` passed shell/storage/auth/lazy routes,
  five sequential hook/cache/capability browser tests, Layers render smoke and
  font-workflow regression. Layers still update three of 27 rows and one frame
  on sibling selection, with no canvas/layer render from a notice toggle.
- `node web/tests/canvas-continuity.mjs` and
  `node web/tests/all-frames-smoke.mjs` passed, covering camera/drag, revision and
  scope changes, SSE, caches, failure/retry and WASM/server fallback.
- After the final pending-membership correction, rebuilt and reran
  `node --test --test-concurrency=1 tests/project-hooks.test.mjs
  tests/snapshot-index.test.mjs tests/swr-project-cache.test.mjs` from `web/`:
  three passed. This includes eviction/revisit and an authoritative empty list
  aborting a pending first snapshot; its late response cannot repopulate cache.
- Independently ran font-workflow, font-inventory-cache and viewer-access browser
  tests together: three passed. `git diff --check` passed.

No touched frontend source/test exceeds 250 lines. Largest new module is
`data/snapshotResource.ts` at 244 lines; it owns snapshot request coordination,
while key definitions/types live separately. App is 127 lines; navigation/list/
live/snapshot hooks are 47/84/91/69. The existing focused state/page-scene modules
remain 242/226 lines. No ESLint gate is configured. No commit, push, deploy,
service restart or user database access occurred.
