# URL-owned viewer navigation — 2026-11-12

The React/SWR checkpoint is commit `72a449901c36d34a101b0f8fd5ad4dd7687e5464`.
This follow-up moves current project/page/frame/node selection to the browser URL.
It remains uncommitted for review; no runtime, database or native service changes
are part of this work.

## Address and history contract

- `?project=p&page=page&frame=frame` opens that scope without selecting a node.
- Add `&node=instance%2Flabel` to select a virtual instance child. IDs are decoded
  once by URLSearchParams; ancestry validation still uses the authored node index.
- `frame=%3Aall%3A` is the existing reserved All-frames scope.
- Node absence means no selection. Deselecting removes the parameter rather than
  writing an empty string or selecting the frame again.
- Partial addresses retain their project intent while reads are pending. The
  authoritative project list chooses an available project; the correct project's
  accepted snapshot chooses an available page/All scope and validates any node.
  Removed projects, pages, frames and layers keep explicit fallback notices.
- A link without `page` infers it from the addressed root frame, or failing
  that from the node's ancestry (virtual instance IDs included), before frame
  and node are validated: `?project=p&node=t` opens t's page in All frames with
  t selected. Only a frame/node absent from the snapshot falls back to the first
  page with the usual notice. An explicit `page` always wins; a frame or node
  elsewhere then follows the normal removed-target fallbacks.
- A matching project's accepted snapshot reconciles the address even while the
  project list is pending or failed; only a loaded list that omits the project
  defers to the project fallback. Foreign snapshots never rewrite the address.
  A missing `frame` still reads as empty (not All frames) until that snapshot
  supplies the default, keeping written links free of guessed parameters.
- Empty workspaces remove all four navigation parameters. Missing pages do not
  produce `page=` entries, and empty nodes do not produce `node=` entries.
- Explicit project/page/frame/layer/canvas choices use `pushState`. No-op choices
  do not add history. Defaults, missing-target fallbacks and accepted revisions use
  `replaceState`. Back/Forward restores selection and scope without reloading.
- Address-bar changes preserve unrelated query parameters, the fragment and
  other code's `history.state`. Copy link remains a separate canonical root
  address with only design IDs, excluding transient query/hash/viewport/revision.

## State ownership and boundaries

`useNavigationUrl` observes the current href through `useSyncExternalStore` and
listens for popstate plus the viewer's own history-write notification. There is
no second mutable selection in React or localStorage. `readSelection` and
`saveSelection` were removed; old `opencanvas.selection` values are ignored and
left untouched. Shell, viewport, follow and panel preferences retain their
existing owners and storage contracts.

`useProject` parses selection from that href and owns only an address-scoped
notice. The existing pure `projectSelection` transitions still reconcile
membership, snapshot ancestry and fallback messages; the hook applies their
result to the URL instead of storing selection in a reducer. Automatic
transitions capture their owning href and reject writes after it changes.
Snapshot acceptance is gated on the matching project and is skipped only when a
loaded authoritative list omits it, including cached-snapshot reconciliation
after history navigation. Page inference for partial links lives in
`web/src/pageInference.ts`, called by the pure `reconcileSelection`.
Public user-event callbacks retain their identities and read the live URL when
invoked. This preserves memoized layer/frame rendering without a selection ref
or other mutable copy of navigation state.

SWR still owns project lists and latest accepted snapshots. Scope keys never
enter snapshot cache keys, so page/frame/node history changes do not read another
snapshot. Warm A→B→A uses accepted payloads. Foreign/late project responses cannot
rewrite current navigation. The viewer retains a single live SSE subscription.

An unlisted follow hint first awaits the actual project-list revalidation result.
An unchanged list therefore settles an unavailable target rather than leaving a
pending project selection. Pending follow navigation is invalidated by user URL
changes, follow disable, project changes, retry, stream disconnect or unmount.
Confirmed new projects still follow with the existing explanatory notice.

The hook fixture now declares its initial scope in the URL instead of preloading
selection storage. The repaint fixture also supplies an explicit All-frames
address for cold and warm tabs. Existing assertions remain; tests for the obsolete
storage authority were replaced with partial/optional-node address behavior and
an explicit browser assertion that conflicting legacy storage is untouched.

## Verification

Worker commands actually run:

```sh
npm --prefix web run typecheck
node --test --test-concurrency=1 web/tests/node-links.test.ts web/tests/project-selection.test.ts web/tests/state.test.ts web/tests/frame-scope.test.ts
cd web
node --test --test-concurrency=1 tests/url-navigation.test.mjs tests/url-follow-navigation.test.mjs tests/project-hooks.test.mjs tests/swr-project-cache.test.mjs tests/snapshot-index.test.mjs
npm run test:navigation
node tests/layers-render-smoke.mjs
node --test --test-concurrency=1 tests/project-hooks.test.mjs tests/swr-project-cache.test.mjs
```

TypeScript passed. All 22 focused pure tests and 15 Chromium tests passed. The
combined browser run included the defaults scenarios before their final file
split. After the split, `test:navigation` passed all 12 navigation tests, including
the identical cases now in `url-defaults.test.mjs`. The repeatable final gate is
`npm --prefix web run test:navigation`, also included in `test:react` with test
concurrency one.

After the stable event-callback correction, typecheck, all 12 navigation tests,
both existing project/SWR browser regressions and the unchanged Layers render
smoke passed again. The render probe observes exactly three of 27 visible rows
and one frame on sibling selection, and no canvas/layer renders on notice toggle;
Stage identity, expansion and keyboard/context-menu actions remain intact.

The browser scenarios cover three-/four-key addresses, virtual IDs, optional-node
reload and deselection, missing/partial/invalid targets, empty workspaces,
fallback notices, Back/Forward across projects/pages/frames/nodes, query/hash and
history.state preservation, no-op/revision history counts, conflicting stored
selection, one SSE and warm snapshot request counts. They also invoke an old
same-project acceptance callback after URL navigation, deliver an old project's
response after a history change, and test asynchronous follow completion after
user navigation, follow disable and unmount. Existing project-hooks, canonical
SWR cache and snapshot-index regression assertions passed unchanged.

The initial sandbox browser attempt failed with loopback `listen EPERM`; the
same isolated fixtures passed outside the sandbox. No production service was
used by these fixtures. `git diff --check` and
`node --check web/tests/all-frames-repaint.mjs` passed. No ESLint gate is configured.

All changed/new frontend source and test modules are at most 250 lines after
formatting and splitting the URL suite. The largest are the existing focused
`state.ts` (222 lines) and history/reload test (195 lines); no module crosses the
required split-review threshold. No dependency or router was added. The repaint
benchmark itself was not rerun because this change only updates its address
setup; earlier performance figures do not measure this follow-up.

Coordinator independently verified the final source:

- `npm --prefix web run build` passed TypeScript and Vite after the stable-callback
  change. The existing SWR `use client` bundler warning remains nonfatal.
- `npm --prefix web test` passed 114 unit tests and 10 snapshot HTTP tests.
- `npm --prefix web run test:react` passed the shell/storage/auth/lazy-route
  scenarios, 17 hook/cache/navigation/capability tests, Layers render smoke and
  font-workflow regression. Layers retained the exact three-row/one-frame bound.
- `node web/tests/canvas-continuity.mjs` and
  `node web/tests/all-frames-smoke.mjs` passed, preserving camera/drag, SSE,
  scope changes, shared caches, fallback and failure/retry behavior.
- `npm --prefix web run test:actions` passed all nine real temporary-backend
  groups: virtual deep links, copy/native text copy, selection/deselection,
  context actions, actual PNG/SVG, missing nodes, unavailable scenes and the
  Vite proxy Origin checks. Its temporary backend/browser/data were cleaned up.
- Final raw `git diff --check` and modified-file size audit passed.

The optimization checkpoint remains committed at `72a4499`; this URL-navigation
delta is separate and uncommitted. No push or native deployment occurred.

The temporary [production preview](REACT-SWR-PREVIEW.md) on
`http://127.0.0.1:3877` now serves this feature build. Runtime verification checked
all 18 served assets against the frozen/repository output, six raw snapshots
against native, and proxied durable SSE replay. Actual Chromium verified the
three-key scope, optional node, layer/canvas selection, deselection, Back/Forward
including All frames, selected/deselected reload, unrelated query/hash and
history.state in both WASM and server modes. Both rendered with Live status and
zero browser errors/authored requests. The coordinator independently checked
the new entry bytes, six snapshot hashes and unchanged native/PG listeners,
and inspected the final no-selection screenshot. Only the owned preview was
restarted; native services and the shared database were unchanged.
