# Task 07 report — agent read efficiency

Each slice appends its own section. Numbers come from `spikes/agent-cost/run.mjs`
artifacts under `artifacts/agent-cost/`; compare with
`node spikes/agent-cost/compare.mjs baseline after-07X`.

## 07A — Agent-cost harness (2026-10-28)

Added `spikes/agent-cost/` (harness, README) and `artifacts/agent-cost/`
(`baseline.json` plus two repeat runs). No product code, `spikes/e2e`,
`samples/` or other spikes were changed; `spikes/e2e/server.mjs`,
`spikes/e2e/protocol.mjs` and `samples/welcome-stacks/author.mjs` are imported.

### Commands run

```sh
npm run build:renderer                                            # renderer/dist/index.js 11.2kb
cargo build --offline -p opencanvas-layout -p opencanvas-server  # Finished, binary already current
node spikes/agent-cost/run.mjs --label baseline                  # exit 0, 6/6 scenarios
node spikes/agent-cost/run.mjs --label baseline-run2             # exit 0, 6/6 scenarios
node spikes/agent-cost/run.mjs --label baseline-run3             # exit 0, 6/6 scenarios
node spikes/agent-cost/compare.mjs baseline baseline-run2
```

Binary `target/debug/opencanvas-server` sha256 prefix `8bda7ffba651f054`, git
HEAD `2f91bfb` plus the existing dirty tree; Node v24.21.0, darwin-arm64. Every
scenario recorded `cleanup:{serverStopped:true, rootRemoved:true}`; afterwards
no `opencanvas-agent-cost-*` temp root or harness server process remained, and
the user server on 3847 was untouched. Earlier development runs (`dev-check`,
deleted) are not used as numbers.

### Baseline (`artifacts/agent-cost/baseline.json`)

Bytes = JSON-RPC request + response messages (the response carries the result
twice: a JSON text block and `structuredContent`). "Structured" is the payload
alone, which the 64 KiB `budget` governs. Per-session handshake: initialize
2,736 B, `tools/list` 15,167 B (not included in scenario totals).

| Scenario | Calls | Bytes | Structured | Wall ms | Breakdown |
| --- | ---: | ---: | ---: | ---: | --- |
| S1 find "Tổng quan" + props | 4 | 85,986 | 38,782 | 74.5 | list_projects 595 B; outline 2 pages 40,279 B; subtree 1 page 45,112 B |
| S2 full stacks context | 1 | 134,015 | 61,032 | 186.1 | 1 page, 181 nodes; `resolvedProps` 20,397 B; 5 tokens, 2 styles, 1 font |
| S3 render+inspect ×3 | 12 | 475,112 | 216,262 | 584.7 | enqueue 3 (2,551 B); poll 6 (355,562 B); inspect 3 (116,999 B) |
| S4 conflict recovery | 8 | 178,939 | 80,546 | 346.9 | base 2 (2,544 B); commit 2 (1,386 B); **recover 4 (175,009 B, 301.6 ms)** |
| S5 outline, 1 vs 20 projects | 42 | 925,442 | 418,740 | — | p50/p95 **20.98/21.53 ms** (1) vs **24.59/25.87 ms** (20) |
| S6 5,000 nodes | 38 | 2,592,867 | 1,176,808 | 13,741 | outline 25 pages, p50 297.4 ms/page; context 13 pages (2,500 nodes), p50 481.0 ms/page |

S3 per viewport (1040/800/390): 2/2/2 polls, ready after 554/549/544 ms from
enqueue. S5 `wallMs` (2,320 ms) spans filler seeding and the restart, so the
latency rows are the S5 result.

### Run-to-run variance (three runs: baseline / run2 / run3)

Call counts and bytes were identical in all three runs except S3, where one 390
job needed one more poll in run2 (12 → 13 calls, +811 B).

| Metric | baseline | run2 | run3 | Spread (max−min) |
| --- | ---: | ---: | ---: | ---: |
| S1 wall ms | 74.50 | 75.25 | 74.52 | 0.75 (1.0%) |
| S2 wall ms | 186.07 | 193.65 | 186.67 | 7.58 (4.1%) |
| S3 wall ms | 584.68 | 840.38 | 582.22 | 258 (44%) — one extra 250 ms poll |
| S3 polls | 6 | 7 | 6 | 1 |
| S4 wall ms | 346.94 | 310.03 | 308.38 | 38.6 (12.5%) |
| S4 recover ms | 301.57 | 267.65 | 264.33 | 37.2 (14.1%) |
| S5 p50, 1 project | 20.98 | 18.93 | 19.15 | 2.05 (10.8%) |
| S5 p50, 20 projects | 24.59 | 24.81 | 25.21 | 0.62 (2.5%) |
| S6 outline p50/page | 297.38 | 298.94 | 297.63 | 1.56 (0.5%) |
| S6 context p50/page | 481.04 | 484.99 | 479.54 | 5.45 (1.1%) |
| S6 wall ms | 13,741 | 13,798 | 13,740 | 58 (0.4%) |

Guidance for later slices: treat ms deltas under ~5% (S1, S2, S6) or ~15%
(S4, S5 single-project) as noise; compare S3 by polls and bytes, not wall time,
since a poll quantizes it to 250 ms. A development run right after the build
was much slower (S1 262 ms, S3 13 polls, ready ≈1.1–1.3 s), so discard the
first run after a fresh build.

### Observations (measured, for later slices; no change made)

- **S3 bytes are dominated by the ready poll**, not `inspect_layout`: each ready
  `render_frame {jobId}` returns ~54 KB structured (~117 KB on the wire) because
  the job carries its whole scene; 75% of S3 bytes. Inspect is ~17.5 KB
  structured per viewport. Relevant to 07B/07E sizing.
- S1 outline needs **2 pages for 182 nodes** although `limit` is 200: the
  default `depth` 8 ends the first page when deeper nodes start.
- S4 recovery is almost entirely readback: 4 calls and 175 KB (outline 2 pages,
  context 1 page, retry), against 1.4 KB for both original commit attempts.
- S5: 20 projects add ~4–6 ms (~0.3 ms per extra project) per `get_outline` on
  this machine. S6: ~300 ms per outline page and ~480 ms per context page of
  200 nodes; S2's single 181-node context page already takes ~186 ms.

### Design decisions

- **Metering via `fetch` tap, not a second MCP client.** `protocol.mjs` calls the
  global `fetch`; `meter.mjs` wraps it for `POST …/mcp`, measures the request
  body and copies the response stream while `protocol.mjs` reads it. The record
  uses the matched JSON-RPC message size, so SSE framing/priming does not count.
  Calls must be sequential (they are in every scenario).
- **Fresh backend per scenario, restarted after seeding.** Seeding commits
  trigger the server's 4-frame prerender; the stacks frame's 1040×720 render
  would otherwise already be cached and S3 timing would depend on prerender
  progress. Restarting gives each scenario the cold "open existing project"
  state and makes runs comparable.
- **S1 baseline strategy** is list_projects → full outline → page subtree until
  `props.text` matches: outline names are IDs, so the copy is only discoverable
  in props. The subtree page that contains the match already holds its props,
  so no extra read is counted.
- **S4 baseline recovery** re-reads outline + frame context because the
  conflict error only carries `code/currentRevision/message/retryable`
  (recorded in the result). The edits are disjoint, so one retry commits; any
  unexpected error fails the scenario loudly rather than looping.
- **S5 filler projects are 3-node projects** to isolate per-project open cost
  from project size; the measured project is the 182-node stacks sample.
- **S6 reads frame-a's context but does not render it**: 2,500 nodes exceed the
  2,048-node scene cap, as the storage README notes for this fixture.
- **Extension point** (`options.mjs`): baseline defaults use only today's tools;
  later slices pass `--opt s2.contextArgs='{"compact":true}'`,
  `--opt s3.pollArgs='{"waitMs":…}'`, `--opt s3.inspectArgs=…`, or add a named
  strategy in S1/S4 and select it. Inspect reads already follow `nextCursor`
  when present. Overrides are written to the artifact.
- Artifacts are pretty-printed with stable key order; `compare.mjs` prints a
  Markdown table of totals, phase calls/bytes/ms and changed result facts.

## 07B — `inspect_layout` cursor and field selection (2026-10-28)

### What changed

- `crates/server/src/inspect.rs` (new, 113 lines): `inspect` moved out of `mcp.rs`
  (175 → 147 lines). New optional `fields` (`bounds`/`lines`, default both), `limit`
  (1–200, default 200), `budget` (1,024–65,536, default **none**) and `cursor`.
- `reads/pagination.rs` (104 → 174 lines incl. a 37-line unit test): `scope_of(identity,
  cursor)` hashes a `Value` identity and decodes the offset; cursor decoding is shared with
  `scope()` through a private `offset()`; `scope()` and `page()` behave as before.
  `page_by_count(...)`: `page` without budget/depth/resource cap (see decision 2).
- `schemas.rs`: only the `inspect_layout` schema and description. `docs/SERVER.md`: new
  section "Inspecting a render job". Test `crates/server/tests/inspect_paging.rs` (new,
  247 lines). `spikes/agent-cost` unchanged: its `s3.inspectArgs` extension point and
  `nextCursor` following were sufficient.
- Outside the ownership list, one line each, needed to wire the module: `lib.rs`
  `mod inspect;` and `reads.rs` `mod pagination` → `pub(crate) mod pagination`.

### Design decisions

1. **Default stays byte-identical, including no byte cap.** The v1 tool had no budget,
   so `budget` omitted means no cap; a default of 65,536 would have changed the
   response of any scene whose first 200 nodes exceed 64 KiB. `nextCursor` is present
   **only when `truncated:true`** (removed when null), so a complete default response
   has exactly the v1 keys. Proven by test against a copy of the pre-change function
   (oracle first checked against the unmodified server: pass), and by the harness:
   S3 inspect phase 116,999 B before and after.
2. **`page_by_count` for the no-budget path.** The first after-07B run used `page()`
   with `budget = usize::MAX`: inspect went from ~4.6 ms to **54–62 ms per call** (3
   runs) because `page()` re-serializes the whole response after every entry (finding
   6; 07F owns that fix). Without a budget there is nothing to measure, so the helper
   slices by count and produces the same bytes and cursor; a unit test compares it
   with `page(…, usize::MAX, 0, …)` byte for byte over 5 limits × 5 offsets. With an
   explicit `budget`, `page()` is used unchanged. After: 4.7–5.4 ms per call.
3. **Paging key stays `nodes`**, so `page()`'s 100-item cap on non-`nodes` arrays
   (Task06 finding) never applies; `limit:200` really returns 200.
4. **Cursor identity** = `["inspect_layout", projectId, jobId, sceneKey, nodeIds,
   normalized fields, limit, budget]`. Fields are normalized to canonical order so
   `["lines","bounds"]` equals the default. No revision is bound: the job's scene is
   immutable, so a commit after the render does not invalidate the cursor (tested);
   job eviction ends the read with `JOB_NOT_FOUND`.
5. Errors use existing codes: out-of-range `limit`/`budget`/`fields` → schema
   `INVALID_REQUEST` (re-checked in code); changed arguments or tampered cursor →
   `INVALID_CURSOR`; one node over `budget` → `CONTEXT_ITEM_TOO_LARGE`.

### Commands run

```sh
cargo test -p opencanvas-server --offline --test inspect_paging   # oracle vs unmodified server: 1 passed (temp test, removed)
cargo test -p opencanvas-server --offline                         # 29 passed, 0 failed (14 binaries incl. 3 lib unit tests)
cargo clippy -p opencanvas-server --offline -- -D warnings        # clean
cargo fmt --all --check                                           # clean
git diff --check                                                  # clean (new untracked files checked with rg: no trailing ws)
cargo build --offline -p opencanvas-layout -p opencanvas-server   # binary sha256 a3251e3cfa56f8fb
node spikes/agent-cost/run.mjs --label after-07B                  # exit 0, 6/6 scenarios
node spikes/agent-cost/run.mjs --label after-07B-bounds --only S3 --opt s3.inspectArgs='{"fields":["bounds"]}'  # exit 0
node spikes/agent-cost/compare.mjs baseline after-07B
node spikes/agent-cost/compare.mjs baseline after-07B-bounds
```

### 250-node scene read fully via cursor (`inspect_paging.rs`)

Frame with 249 children (rectangles, short texts, one long narrow text with >10 lines),
rendered at 400×3000: 250 scene nodes. Every read below returned all 250 IDs in scene
order, no duplicate, no gap:

| Arguments | Pages |
| --- | ---: |
| default (v1 call + cursor) | 2 (200 + 50) |
| `limit:100` | 3 |
| `fields:["bounds"], budget:2048` | 10 |

Default first page equals the v1 oracle plus `nextCursor`; a `nodeIds` subset equals the
v1 oracle byte for byte, without a `nextCursor` key.

### Harness (stacks sample, 181 scene nodes, 1 page per viewport)

| S3 inspect phase (3 calls) | baseline | after-07B (default) | after-07B-bounds |
| --- | ---: | ---: | ---: |
| bytes (req+resp) | 116,999 | 116,999 (0) | 87,349 (−29,650, −25.3%) |
| structured per call | 17,520 / 17,519 / 17,857 | same | 13,081 / 13,080 / 13,078 (−25.3% to −26.8%) |
| callMs (3 calls) | 13.81 | 15.00 (+8.6%) | 14.94 (+8.2%) |

`fields:["bounds"]` saves only ~25% here because the stacks text nodes are mostly one
line; whole S3 bytes drop 5.9% (475,112 → 447,084), since the ready `render_frame`
polls dominate S3 (07A observation). The page count is 1 either way (181 < 200).

Not attributable to 07B, reported unrounded: S3 renders were slower in every run this
session (readyMs 809–1,093 vs 544–554 in the baseline, so 3–4 polls per viewport
instead of 2; S3 calls 12 → 14–17). The render/poll path is unchanged by this slice.
S1, S2, S4, S5 and S6 bytes are identical to the baseline; S5 p50 +12%/+22% and S6
wall −33% are timing variance or other slices' changes, not this one. `tools/list`
grew 17,174 → 17,796 B (+622 B) for the new schema and description (after-06 → after-07B).

## 07C — Compact design context, opt-in (2026-10-28)

### What changed

- `crates/server/src/reads/compact.rs` (new, 134 lines): `Referenced::collect` (reference
  closure), `resolved_props` (differing keys), `kept` (resource filter) and `mark`
  (response declarations). Each is a no-op without `compact:true`.
- `reads/tree.rs` (128 → 133 lines): four call sites into `compact` plus the import; no
  logic of its own. `reads.rs` (84 → 89): `mod compact;` and
  `#[serde(default, skip_serializing_if = "Option::is_none")] compact: Option<bool>`,
  with `false` normalized to absent before the scope hash.
- `schemas.rs` (123 → 125): `compact: boolean` only in the `get_design_context` schema,
  plus one description sentence. `get_outline`/`get_subtree`/`find_nodes` reject it
  through `additionalProperties:false` (`INVALID_REQUEST`, tested).
- `instructions.rs` (38 → 41), `skills/implement-frame.md` step 2, `docs/SERVER.md`
  (new section "Compact context: `compact:true`").
- Tests: `crates/server/tests/compact_context.rs` (new, 211 lines, 5 tests) and
  `tests/compact_support/mod.rs` (new, 138 lines: fixture with every reference kind,
  a second frame and 300 filler tokens).

### Response with `compact:true`

Every page adds `resolvedPropsMode:"differing"` and `resources:"referenced"`.

- `resolvedProps` keeps a key when the resolved value differs from authored or is
  absent from authored (keys added by a style). The full resolved object is authored
  props **without `styleId`**, overlaid with these keys. Storage's resolver drops only
  `styleId` (`refs.rs` `resolve_props`), so this rebuild is exact; the test rebuilds
  every node and compares it with the default read, for the `default` and `night`
  themes. An unchanged node keeps `resolvedProps: {}` rather than losing the key, so
  the node shape stays the same.
- `tokens`/`styles`/`fonts`/`assets` keep only items referenced by the frame scope,
  with the item shape unchanged (`resolvedValue` included). Closure: node props →
  styles → their tokens and fonts → token `aliasId` targets in the default **and
  every theme branch**, to a fixed point. All theme branches are included, not only the
  requested theme: an implementation that maps tokens should see the targets for every
  theme, and the set then does not depend on `theme`.

### Design decisions and deviations

1. **The key set is mirrored, not called.** Storage's helpers (`branches`, `alias`,
   `Candidate`) are `pub(super)` inside `crates/storage/src/validate/`, and the task
   forbids changing storage. Validation (`refs.rs`, `properties.rs`, `node.rs`,
   `tokens.rs`) allows these reference keys: `{tokenId}` (props, layout padding/gap,
   text-style values, fill style value), `styleId` (top-level prop and fill `{styleId}`),
   `fontId` (props, text style), `assetId` (props) and `{aliasId}` (token branches).
   Storage rejects them anywhere else, so `compact.rs` walks every object key and
   matches those five names. **Pin test**
   `compact_resource_set_is_exactly_what_storage_needs` builds a reduced `Snapshot`
   (scope nodes + compact resources only) and calls storage's public
   `resolved_nodes_for_theme`: it must resolve to identical props for every declared
   theme (sufficiency), and removing any single kept resource must fail validation
   (minimality). Run for both fixture frames. A mutation check (dropping `aliasId` from
   the walk) made 3 of the 5 tests fail; the change was then reverted.
2. **Set collected over the full scope**, before the `properties` filter and paging, the
   same way diagnostics are computed. Every page of one read uses one resource set, and
   a filter can never hide a reference that the agent later needs.
3. **`compact:false` = absent.** Without the normalization, an explicit `false` would be
   serialized into the scope hash and change `nextCursor`. The test compares every page,
   with cursors, of default and `compact:false` reads (budget 2,048, both frames).
   The golden test `traversal_tests::outline_subtree_and_context_bytes_match_golden`
   passes without changes.
4. **Ownership notes.** `mod compact;` in `reads.rs` is within ownership. No file
   outside the list was touched. The harness extension point `s2.contextArgs` was
   enough, so `spikes/agent-cost` is unchanged.

### Commands run

```sh
cargo test -p opencanvas-server --offline --test compact_context -- --nocapture  # 5 passed
cargo test -p opencanvas-server --offline        # 34 passed, 0 failed (lib 3 incl. golden, 13 integration binaries)
cargo clippy -p opencanvas-server --offline -- -D warnings   # clean
cargo fmt --all --check                          # clean (after rustfmt on the two new test files)
git diff --check                                 # clean; new files checked with rg: no trailing whitespace
cargo build --offline -p opencanvas-layout -p opencanvas-server   # binary sha256 ef5d352e7906ac1d
node spikes/agent-cost/run.mjs --label after-07C                  # exit 0, 6/6, every cleanup true
node spikes/agent-cost/run.mjs --label after-07C-compact --only S2 --opt s2.contextArgs='{"compact":true}'
node spikes/agent-cost/run.mjs --label after-07C-compact-run2 --only S2 --opt s2.contextArgs='{"compact":true}'
node spikes/agent-cost/run.mjs --label after-07C-default-run2 --only S2
node spikes/agent-cost/compare.mjs baseline after-07C-compact
```

A first S2 warm-up run after the build was discarded (07A guidance); its file was deleted.

### S2 on the stacks sample (181 nodes, 1 page)

| S2 | baseline | after-07C default | after-07C `compact:true` (run 1 / run 2) |
| --- | ---: | ---: | ---: |
| calls | 1 | 1 | 1 |
| bytes (req+resp) | 134,015 | 134,015 (0) | 91,620 (−42,395, −31.6%) |
| structuredBytes | 61,032 | 61,032 | 41,758 (−19,274, −31.6%) |
| `resolvedProps` bytes | 20,397 | 20,397 | 1,066 (−94.8%) |
| tokens / styles / fonts / assets | 5 / 2 / 1 / 0 | same | 5 / 2 / 1 / 0 |
| callMs | 186.07 | 192.06 / 186.40 | 131.77 / 132.92 (−29%) |

The ~20 KB saving matches the task estimate and comes entirely from `resolvedProps`.
On this sample the stacks frame references every resource the project has, so the
resource filter removes nothing. The call is faster because `pagination::page`
re-serializes the response after every entry (finding 6, owned by 07F), and the
response is now smaller.

### Resources part: 300-token fixture (`compact_shrinks_a_300_token_project_context`)

Project with 308 tokens (300 unreferenced fillers), 3 styles, 1 font, 1 asset;
frame `card` references 7 tokens, 2 styles, 1 font, 1 asset. Structured bytes
(compact JSON of each page, which the 64 KiB budget governs), default budget:

| Frame `card` | pages (calls) | structured bytes | resource items bytes |
| --- | ---: | ---: | ---: |
| default | 4 | 30,801 | 27,613 (308 tokens) |
| `compact:true` | 1 | 2,942 (−90.4%) | 1,513 (7 tokens) |

The default read needs 4 pages because of the 100-item cap per resource array per page.

### Other scenarios and handshake (after-07C, default arguments)

S1, S2, S4, S5 and S6 bytes are identical to the baseline (85,986 / 134,015 / 178,939 /
925,442 / 2,592,867 B). S3 took 17 calls / 479,167 B against 12 / 475,112 B. This
comes from render polls (readyMs varies between runs, see 07B), and the render path is
unchanged here. Timing deltas against after-07B (S5 p50 18.82/25.20 ms, S6 context p50
361.9 ms/page) are within the earlier run-to-run spread or come from other slices.
The handshake grows with the new text: `tools/list` 17,796 → 18,241 B (+445 B, schema
property and description), `initialize` 3,068 → 3,331 B (+263 B, instructions sentence).
Every session pays these 708 B once, whether or not it uses `compact`. One compact S2
read saves 42,395 B.

### Open risks

- An agent that ignores `resolvedPropsMode` would read a missing key as "not set". The
  mode is declared on every page, and the tool description, instructions, skill and
  SERVER.md state the rebuild rule. The default stays the full form.
- The mirrored key set must change together with storage if storage gains a new
  reference kind. The pin test then fails for any fixture that uses the new kind,
  but only if the fixture is extended to cover it.

## 07F — Internal performance, no contract change (2026-10-28)

### What changed

1. **Pinned project handles** — `state/pins.rs` (new, 78 lines) and `state.rs` (76 → 94).
   `AppState` keeps the `Arc<Project>` of every project it opened, keyed by ID and by slug.
   Storage's registry holds only `Weak`, so before this change every read of a `p_<uuid>` ID
   ran `list_projects()` (opened every DB in the root), then opened a RW connection and ran
   migrations. Opening still goes through `Store::open_project`, so the root and project path
   checks run on every call; the pin only turns the registry lookup into a hit.
   `AppState::project(id, f)` is used by `domain.rs` for `apply_batch`/`import_asset`;
   `http.rs` and `events.rs` (not owned) still call `open_project` and hit the registry too.
2. **Cumulative byte accounting in `pagination::page`** (`pagination.rs` 174 → 141). The
   response is serialized once; each step adds the exact compact-JSON growth: entry length,
   its comma, `,"field":[]` when an array key is created, and the cursor length change
   (`false→true` −1 byte, `null→"<cursor>"` +len−2, hence `len − 3`). Mutations to the
   response are the same as before (including the empty array that stays after a popped
   first entry), so only the size decision changed. `page_by_count` (07B workaround) is
   removed; `inspect.rs` (113 → 108) calls `page()` with `budget.unwrap_or(usize::MAX)`.
3. **Tool catalog cached** in a `OnceLock` (`schemas.rs` 125 → 142): tools and a
   name→schema map are built once; `validate` no longer rebuilds every schema per call,
   `tools()` clones the cached `Vec` (`list_tools`/`get_tool` in `mcp.rs` unchanged).
4. **Targeted prerender** — `reads/targets.rs` (new, 116 lines incl. 2 unit tests) and
   `domain.rs` (86 → 84). Each changed ID that is a node maps through `TreeIndex::ancestors`
   to its outermost enclosing frame (itself if it is one); the viewer lists top-level frames.
   Matching frames are prerendered in storage order, at most 4. When no changed ID resolves
   to a frame — token/style-only changes, but also deletions (removed IDs are not in the new
   snapshot) and page edits — the first 4 frames in storage order are prerendered as before.
5. **`scope_ids` via `TreeIndex::preorder`** (`viewports.rs` 163 → 157), replacing the
   fixed-point rescan of all nodes. Same set (frame + all descendants); `multi_viewport`,
   `multi_failures`, `multi_resources` pass unchanged.

Outside the ownership list, one line: `reads.rs` `pub(crate) mod targets;` (89 → 90), the same
kind of wiring 07B did for `pagination`. `targets.rs` lives under `reads/` because
`TreeIndex` is `pub(super)` there. `jobs.rs`, `mcp.rs`, `tree.rs`, `lib.rs` untouched.

### Design decisions and risks

- **Import/restore and project creation are not blocked.** `bundle_import.rs` stages into a
  new temp dir, refuses an existing destination slug or (for restore) an existing identity,
  takes the registry lock only around the final rename and then calls `open_project(slug)`;
  `create_project` creates a new slug directory. None of them touches an open project, and
  the server does not expose import/restore at all. `export_bundle` uses SQLite backup, so a
  WAL kept by a long-lived writer is captured.
- **Stale handle after out-of-band replacement (found and mitigated).** With a pin, a
  project directory deleted and recreated under the same slug while the server runs made
  the old ID resolve to the pinned handle, whose path checks pass on the new directory. A
  mutation of the identity check made `get_outline` by the old ID return the **new**
  project's data (test output, revision 0, other projectId). Mitigations in `pins.rs`:
  `snapshot` rejects a project whose `projectId`/`slug` does not match the key, drops the
  pin and reopens once; any open error on a pinned key drops it and retries once (without
  this, a deleted project read as `STORAGE_IO` instead of `PROJECT_NOT_FOUND`); re-pinning a
  slug to another project drops the old handle. Both behaviours are pinned by
  `tests/project_pins.rs` and each mitigation was mutation-checked (test failed without it).
  **Residual:** a *write* sent by the old ID after such a replacement and before any read by
  that ID still goes through the old writer into the removed database. Closing it needs an
  identity check per write (a full snapshot read) or a storage API; neither is in scope.
  It requires replacing a project directory behind a running backend, which holds the
  data root's exclusive lock; PLAN has no project deletion. Documented in `SERVER.md`.
- **Bounded pins:** after 128 keys (ID and slug count separately) the set is cleared and
  projects reopen on demand, so a large root does not keep unbounded SQLite writers open.
- **Byte identity, proven against a copy of the old algorithm.**
  `reads/pagination_tests.rs` (new, 220 lines) holds `page_v1`, a verbatim copy (checked
  with `diff` against the removed body: identical). Two tests compare whole reads (following
  every cursor) for 39 budgets from 100 B to `usize::MAX` × 4 limit/depth combinations over
  220 mixed entries (escapes, non-ASCII, emoji, floats, depths 0–4, a pre-existing and a
  newly created resource array, the 100-item cap), and single pages at offsets around
  9/99/999 → 10/100/1000 and the end, 63 budgets × 3 limits, where cursor digits and base64
  length change. Mutating the constant to `len − 2` or `len − 4` fails both tests. The
  golden byte test and `inspect_paging.rs` pass unchanged.
- **Mixed node + token batch** targets the frames of the changed nodes only; the "keep old
  behaviour" rule applies when no node resolves to a frame. A deletion therefore falls back
  to the first four frames (the parent of a removed subtree is not in `changedIds`).

### Commands run

```sh
cargo build --offline -p opencanvas-layout -p opencanvas-server   # before: sha256 ef5d352e7906ac1d
node <scratch>/prerender-queue.mjs before-{1,2,3}                 # before measurement (see below)
cargo test -p opencanvas-server --offline --lib pagination        # 3 passed; mutants len−2 / len−4: 2 failed each
cargo test -p opencanvas-server --offline --test project_pins     # 1 passed; 2 mutants: failed as expected
cargo test -p opencanvas-server --offline --test prerender_targets # 1 passed (real Chromium, ~7 s)
cargo test -p opencanvas-server --offline        # 40 passed, 0 failed (lib 7 incl. golden; 15 integration binaries)
cargo clippy -p opencanvas-server --offline -- -D warnings   # clean
cargo fmt --all --check                          # clean
git diff --check                                 # clean; new files checked with rg: no trailing whitespace
cargo build --offline -p opencanvas-layout -p opencanvas-server   # sha256 11c50b8bb2550e8f
node spikes/agent-cost/run.mjs --label after-07F-warmup          # first run after build, discarded and deleted
node spikes/agent-cost/run.mjs --label after-07F                 # exit 0, 6/6, every cleanup true
node spikes/agent-cost/run.mjs --label after-07F-run2            # exit 0, 6/6
node spikes/agent-cost/run.mjs --label ablation-07F-no-pins --only S1,S2,S5,S6   # temp build without pin()
node spikes/agent-cost/compare.mjs after-07C after-07F
node <scratch>/prerender-queue.mjs after-{1,2,3}
npm run test:e2e                                 # exit 0, browserErrors []
```

The ablation build only removed the `self.pin(...)` call; the source was restored from a
copy and rebuilt (`diff -q` clean). That rebuild has sha256 `a52443e249107f61` (incremental
debug builds are not bit-reproducible here); it is the binary the prerender "after" runs
and `npm run test:e2e` used, from identical source.

### Harness (stacks sample; bytes identical to after-07C in every scenario)

S1, S2, S4, S5 and S6 calls and bytes are identical to after-07C and to the baseline; S3
bytes are identical to the baseline (12 calls, 475,112 B). `tools/list` 18,241 B and
`initialize` 3,331 B, unchanged. Timings (after-07F / run2):

| Metric | baseline | after-07C | after-07F | run2 | Δ vs after-07C |
| --- | ---: | ---: | ---: | ---: | ---: |
| S1 wall ms | 74.50 | 74.47 | 17.54 | 17.29 | −56.93 (−76.4%) |
| S2 call ms | 186.07 | 192.06 | 24.88 | 23.21 | −167.18 (−87.0%) |
| S4 wall ms | 346.94 | 345.77 | 75.46 | 75.64 | −270.31 (−78.2%) |
| S4 recover ms | 301.57 | 297.91 | 46.61 | 46.67 | −251.30 (−84.4%) |
| S5 p50, 1 project | 20.98 | 18.82 | 3.48 | 3.49 | −15.34 (−81.5%) |
| S5 p95, 1 project | 21.53 | 20.17 | 3.99 | 3.93 | −16.18 (−80.2%) |
| S5 p50, 20 projects | 24.59 | 25.20 | 3.45 | 3.44 | −21.75 (−86.3%) |
| S5 p95, 20 projects | 25.87 | 27.08 | 3.88 | 3.84 | −23.20 (−85.7%) |
| S5 p50 ratio 20/1 | 1.17 | 1.34 | 0.99 | 0.99 | — |
| S6 outline p50/page | 297.38 | 82.84 | 32.64 | 32.64 | −50.20 (−60.6%) |
| S6 context p50/page | 481.04 | 361.90 | 191.70 | 191.40 | −170.20 (−47.0%) |
| S6 wall ms | 13,741 | 6,714 | 3,330 | 3,319 | −3,384 (−50.4%) |
| S3 inspect callMs (3) | 13.81 | 15.35 | 13.30 | 13.50 | −2.05 |

Attribution from the ablation (pins off, everything else on): S2 22.33 ms, S1 19.14 ms,
S5 p50 4.26 ms (1 project) / **10.23 ms (20 projects)**, S6 outline p50 33.51, context p50
195.33. So the per-call gains in S1/S2/S4/S6 come from the pagination fix; the pins remove
the per-project scan: with 20 projects the pinned read is 3.45 ms against 10.23 ms, and the
1-vs-20 ratio drops to 0.99. S3 render timing (readyMs 545/540/536, 2 polls per viewport)
matches the baseline; after-07C's 17 calls were render-time variance (07B/07C notes).

### Prerender queue: editing the 6th frame of a 6-frame project

Scratch script (`prerender-queue.mjs`, temp root and random port through
`spikes/agent-cost/env.mjs`, not committed): seed 1 page + 6 frames (320–370×200, one text
each), restart, subscribe to SSE, edit frame 6's text, collect `scene_ready` for that
revision until 3 s quiet, then match `render_frame` job IDs per frame. Then, worker warm, a
second edit and the time from the commit response to frame 6 ready (50 ms polls). Three runs
each:

| | prerendered frames | prerender jobs | commit ms | frame 6 ready ms |
| --- | --- | ---: | --- | --- |
| before (ef5d352e) | frame-1, 2, 3, 4 (3/3 runs) | 4 | 22.73 / 23.10 / 20.26 | 109.94 / 109.50 / 110.06 |
| after | frame-6 (3/3 runs) | 1 | 12.72 / 8.99 / 9.98 | 106.62 / 107.42 / 108.32 |

The commit now enqueues one job instead of four and warms the frame that changed. Frame-6
ready time is about the same (≈−2.5%, within the 50 ms poll quantum) because these frames
render in ~20 ms each, so four jobs ahead of it cost little; with stacks-sized frames
(~540 ms per render in S3) the four untouched frames would occupy the single worker for
seconds. Not measured with large frames. Seeding commits (every frame changed) still
prerender the first four, and so does a token-only commit (`tests/prerender_targets.rs`).

### Open risks

- Residual stale-handle write after out-of-band project directory replacement (above).
- Prerender after a subtree deletion falls back to the first four frames, not the frame the
  subtree was removed from.
- Pins hold one SQLite writer (plus WAL/SHM descriptors) per opened project for the life of
  the backend, up to the 128-key bound.

## 07E — Bounded wait on render job polls (2026-10-28)

### What changed

- `render_frame {projectId, jobId, waitMs}`: `waitMs` is an integer 0–10,000, accepted only
  on the `jobId` (poll) branch of the schema. Absent or 0 is the previous immediate poll.
  Otherwise the call returns as soon as the job leaves `pending` (`ready` or `failed`), or
  the record as it is when the wait expires (still `pending`). Same response shape in every
  case. Out of range, non-integer, or `waitMs` on the enqueue form → `INVALID_REQUEST`
  (schema validation, the existing envelope).
- `crates/server/src/jobs/wait.rs` (new, 42 lines): `Jobs::wait(id, wait, cancel)` and
  `MAX_WAIT_MS`. `jobs.rs` only gained the completion signal: a `watch::Sender<u64>` bumped
  at the end of `finish` (233 → 239 lines, public API unchanged apart from the added
  `wait`/`MAX_WAIT_MS`). `mcp.rs` render branch: project check first, then wait when
  `waitMs > 0` (147 → 154). `schemas.rs`: `waitMs` on the poll branch and a description
  sentence (142 → 143).
- `docs/SERVER.md`, `instructions.rs` (connect text) and both skills tell agents to wait with
  `waitMs` instead of polling in a tight loop, including on the job IDs that
  `get_design_context` returns. `get_design_context` itself still never waits (Task 03).

### Design decisions

- **Only on `jobId` polls.** The task specifies `render_frame {projectId, jobId, waitMs}`;
  the enqueue form rejects `waitMs`. An agent enqueues, then waits on the returned job ID:
  one extra call compared with waiting on enqueue, but the enqueue response, cache-hit
  semantics and `QUEUE_FULL` handling stay exactly as they were.
- **No storage held.** The wait runs on the async task: it never enters `AppState::db`
  (no `spawn_blocking` thread, no pinned `Arc<Project>` use, no read transaction). It holds
  the job-table mutex only for each `get` (a clone of the record), never across the park.
  Covered by `waiting_holds_no_storage_and_ends_on_shutdown`: `get_outline` and an
  `apply_batch` commit both finish while a 10 s wait is parked.
- **No lost wake-ups.** The waiter subscribes to the `watch` before its first read; a job
  finishing between that read and the park still marks the receiver changed. Any completion
  wakes every waiter, which re-reads its own record (at most 32 records, one worker, so the
  thundering herd is bounded and cheap).
- **Shutdown.** `cancel` is `AppState::shutting_down()`; a wait in progress returns its
  current record at once, and a wait started after shutdown does not park.
- **Ordering of errors.** `JOB_NOT_FOUND` and the other-project `INVALID_REQUEST` are
  returned before any waiting; a record evicted during the wait returns `JOB_NOT_FOUND`.
- **Timeout result.** After the deadline the record is re-read once, so a job finishing
  exactly at the deadline is reported as finished. No `waited`/`timedOut` field was added:
  the status already tells the caller whether to poll again, and it keeps the shape stable.
- HTTP `/api/v1/jobs/:id` is unchanged (not in this slice's ownership; the viewer uses SSE).

### Commands run

```sh
cargo check -p opencanvas-server --offline                  # clean
cargo test -p opencanvas-server --offline --test render_wait # 5 passed, 1.47 s
cargo build --offline -p opencanvas-layout -p opencanvas-server   # sha256 575335fd23b1d1f5
node spikes/agent-cost/run.mjs --label after-07E-warmup --only S3 --opt s3.pollArgs='{"waitMs":2000}'  # first run after build, discarded and deleted
node spikes/agent-cost/run.mjs --label after-07E --only S3 --opt s3.pollArgs='{"waitMs":2000}'         # exit 0
node spikes/agent-cost/run.mjs --label after-07E-run2 --only S3 --opt s3.pollArgs='{"waitMs":2000}'    # exit 0
node spikes/agent-cost/run.mjs --label after-07E-default --only S3                                     # exit 0
node spikes/agent-cost/run.mjs --label after-07E-default-run2 --only S3                                # exit 0
node spikes/agent-cost/run.mjs --label after-07E-wait10000 --only S3 --opt s3.pollArgs='{"waitMs":10000}'  # exit 0
node spikes/agent-cost/run.mjs --label after-07E-default-all                                            # exit 0, 6/6
node spikes/agent-cost/compare.mjs after-07E-default after-07E
node spikes/agent-cost/compare.mjs after-07F after-07E-default-all
cargo test -p opencanvas-server --offline        # 43 passed, 0 failed (lib 7; 15 integration binaries incl. render_wait 5)
cargo clippy -p opencanvas-server --offline -- -D warnings   # clean
cargo fmt --all --check                          # clean
git diff --check                                 # clean
```

Mutant check: with the `send_modify` line in `finish` commented out, `render_wait` fails
2 of 5 (`…_is_ready`, `…_fails`: the wait only ends at the 10 s deadline); restored source
(`diff -q` clean) passes 5/5. That rebuild relinked the binary (now sha256
`76368d826a40ef5a`, same source as `575335fd23b1d1f5`; debug builds are not bit-reproducible
here). Every harness run recorded `cleanup:{serverStopped:true, rootRemoved:true}`; no
harness server or temp root remained, and the user server on 3847 was untouched.

### S3 at the same 250 ms cadence (stacks sample, 181 nodes)

The harness still sleeps 250 ms before each poll round; only `pollArgs` differs.

| Run | Calls | Bytes | Polls (1040/800/390) | readyMs (1040/800/390) | Wall ms | Poll callMs |
| --- | ---: | ---: | --- | --- | ---: | ---: |
| baseline (07A) | 12 | 475,112 | 2/2/2 = 6 | 554/549/544 | 584.68 | 28.31 |
| after-07E-default | 12 | 475,112 | 2/2/2 = 6 | 549/544/540 | 578.13 | 25.03 |
| after-07E-default-run2 | 12 | 475,112 | 2/2/2 = 6 | 547/542/539 | 576.85 | 24.98 |
| after-07E (`waitMs:2000`) | 9 | 472,721 | 1/1/1 = 3 | 347/404/460 | 497.00 | 196.16 |
| after-07E-run2 (`waitMs:2000`) | 9 | 472,721 | 1/1/1 = 3 | 351/408/463 | 500.31 | 198.99 |
| after-07E-wait10000 | 9 | 472,724 | 1/1/1 = 3 | 352/409/465 | 502.67 | 200.90 |

`compare.mjs after-07E-default after-07E`: calls −3 (−25.0%), polls 6 → 3 (−50.0%), bytes
−2,391 (−0.5%; the three removed polls were small `pending` records, while the ready record
carrying the scene still dominates, see 07A), wall −81.13 ms (−14.0%). Poll `callMs` rises
171 ms because the time now passes inside the call instead of in the client's sleep.
readyMs now measures completion rather than the next 250 ms tick; the 800/390 values include
waiting sequentially behind the earlier viewport's call, since the harness polls in order.
The saving per job grows with render time: in 07B's slower runs (3–4 polls per viewport)
a `waitMs` poll still needs one call per viewport whenever the job finishes within the wait.
For an LLM agent each removed poll is a whole model round, which this harness does not price.

Default arguments are unchanged byte for byte: `after-07E-default-all` vs `after-07F` has
identical calls and bytes in S1–S6 (S3 475,112 B, 12 calls). The handshake grows because
the published text grew: `initialize` 3,331 → 3,511 B (+180, connect instructions) and
`tools/list` 18,241 → 18,507 B (+266, `waitMs` schema and description).

### Tests (`crates/server/tests/render_wait.rs`, deterministic fake worker)

- `wait_returns_as_soon_as_the_job_is_ready`: `waitMs:10000` returns `ready` with `imageUrl`
  after ≥500 ms and <5 s; equals an immediate poll; a wait on a finished job returns at once.
- `wait_returns_as_soon_as_the_job_fails`: hanging worker, 600 ms job deadline;
  `waitMs:10000` returns `failed`/`JOB_TIMEOUT` in <5 s.
- `wait_expires_with_the_current_pending_state`: `waitMs:0` is byte-identical to no
  `waitMs`; `waitMs:300` returns the same pending record after ≥300 ms and <2 s.
- `waiting_holds_no_storage_and_ends_on_shutdown`: reads and a commit proceed during a
  10 s wait; `begin_shutdown` ends it with `pending` well within 3 s; later waits don't park.
- `wait_ms_is_bounded_and_only_for_job_polls`: 10001, −1, 1.5, "100", null and `waitMs` on
  enqueue → `INVALID_REQUEST`; another project's job → `INVALID_REQUEST` and an unknown job
  → `JOB_NOT_FOUND`, both in <1 s.

### Open risks

- Each `waitMs` call occupies one HTTP request for up to 10 s. There is no per-session cap on
  concurrent waiters; each costs one parked task and one `watch` receiver, no thread.
- Waiters are woken by any job completion and re-read their record; cheap at 32 records,
  but it would need per-job signals if retention grew a lot.
- `cargo clippy --tests` still reports pre-existing dead-code errors in other test files'
  shared support (not part of the required check); `render_wait.rs` adds none.

## 07D — `get_changes` and `REVISION_CONFLICT.changedSince` (2026-10-28)

Approved by the user on 2026-10-28 (Tasks 06/07/08); the PLAN §6 row was added by the
coordinator before this slice.

### What changed

- New tool `get_changes {projectId, sinceRevision, limit 1–100 (default 100),
  includeOperations=false, budget 1,024–65,536 (default 65,536)}` →
  `{schemaVersion, projectId, fromRevision, currentRevision, commits:[{revision, changedIds,
  operations?}], changed:{nodes, tokens, styles, assets, fonts, removed}, truncated,
  nextRevision, limits:{commits, bytes}}`. Tool count 14 → 15.
- `REVISION_CONFLICT` from `apply_batch` and `import_asset` gains
  `changedSince:{fromRevision, toRevision, changedIds ≤200, truncated}` inside `error`.
  `code`, `message`, `retryable`, HTTP status (409) and `currentRevision` are unchanged;
  `StorageError` and storage are untouched.
- Files: `reads/changes.rs` (new, 177: request, page builder, grouping),
  `reads/changes/history.rs` (new, 50: contiguous log read cut at the snapshot revision),
  `reads/changes/conflict.rs` (new, 58: conflict enrichment); `error.rs` 52 → 66 (optional
  detail map on `ApiError`, `with_detail`); `domain.rs` 84 → 88; `mcp.rs` 154 → 155 (one
  dispatch arm); `reads.rs` 90 → 91; `schemas.rs` 143 → 151; `instructions.rs` 42 → 46;
  `skills/author-frame.md` 59 → 66; `docs/SERVER.md` (new "Changes since a revision" section,
  tool count, conflict sentence). Tests: `tests/changes.rs` (new, 5 tests),
  `tests/conflict_changes.rs` (new, 3 tests), tool-count lines in `transport.rs` and
  `skill_layer.rs`. Harness: `changedSince` strategy in `scenarios/s4-conflict.mjs`.

### Design decisions

- **One snapshot defines the page.** `get_changes` reads the pinned snapshot first
  (`state.snapshot`), then `commits_after` through `state.project`, and keeps only commits
  ≤ the snapshot revision, so `changed` grouping, `currentRevision` and the commits agree even
  when a commit lands between the two reads.
- **Gaps are errors, not silence.** Commits must be `since+1, since+2, …`; a missing row
  (a bundle import clears the log) returns `RESET_REQUIRED` with `currentRevision`, the code
  storage and SSE already use. `sinceRevision` ahead of the project returns the same. A page
  that ends before a gap is still served.
- **Whole commits within the budget, never over it.** The base is measured with the widest
  final values (`truncated:false`, `nextRevision = currentRevision`); each commit adds its
  exact compact-JSON growth (entry plus newly seen IDs in their group). A single commit larger
  than the budget returns `CONTEXT_ITEM_TOO_LARGE`. No re-serialization per commit.
  `nextRevision` is always present: equal to `currentRevision` when caught up, otherwise the
  `sinceRevision` for the next page (rather than an opaque cursor; revisions are already the
  log's natural cursor).
- **Grouping** uses the current snapshot; an ID present in several kinds goes to the first of
  nodes/tokens/styles/assets/fonts, absent IDs to `removed`, in first-seen order. Storage's
  `changedIds` are the touched IDs: creating a child does not list its parent (verified:
  creating `other` under `frame` reports `["other"]`). Ancestry is therefore the agent's to
  check; `TreeIndex` was not needed server-side.
- **`includeOperations`** returns the logged operations; `import_asset` entries drop the asset's
  internal `relativeKey`, as `list_assets` does.
- **Conflict enrichment is best effort and bounded**: at most 100 commits read, 200 distinct
  IDs; `truncated:true` when IDs or commits were cut or a log row is missing. If the log read
  fails the plain storage error is returned. It is omitted when `expectedRevision ≥
  currentRevision` (nothing changed "since"). Details are merged into `error` without
  overriding storage fields; with no detail `ApiError::value()` is byte-identical to before.
- **Agent rule** (connect instructions, `author-frame.md`, SERVER.md): if `truncated` is false
  and no changed ID is an edited node or one of its ancestors, resend with
  `expectedRevision: toRevision` and a new `idempotencyKey`; otherwise re-read the IDs with
  `find_nodes {ids}` or page `get_changes`. Storage validates every batch again.
- **Harness strategy `changedSince`**: S4's base read is `get_subtree depth 0`, which has no
  ancestry, so the strategy spends one `find_nodes {ids:[nodeId]}` to learn the edited node's
  path, then retries on `toRevision` when disjoint (re-reads overlapping IDs otherwise; falls
  back to the baseline readback on `truncated` or `pathTruncated`). An agent that already knows
  the path (for example from `find_nodes`) recovers in one call.

### Commands run

```sh
cargo build -p opencanvas-server --offline                       # clean
cargo test -p opencanvas-server --offline --test changes --test conflict_changes  # 5 + 3 passed (after fixing one expectation, see above)
cargo build --offline -p opencanvas-layout -p opencanvas-server  # final binary sha256 b498de953fef5d22
node spikes/agent-cost/run.mjs --label after-07D-warmup --only s4 --opt s4.strategy=changedSince  # first run after build, discarded and deleted
node spikes/agent-cost/run.mjs --label after-07D --only s4 --opt s4.strategy=changedSince          # exit 0
node spikes/agent-cost/run.mjs --label after-07D-run2 --only s4 --opt s4.strategy=changedSince     # exit 0
node spikes/agent-cost/run.mjs --label after-07D-default --only s4                                 # exit 0
node spikes/agent-cost/run.mjs --label after-07D-default-run2 --only s4                            # exit 0
node spikes/agent-cost/compare.mjs after-07D-default after-07D
node spikes/agent-cost/compare.mjs after-07E-default-all after-07D-default
cargo test -p opencanvas-server --offline        # 53 passed, 0 failed (lib 7; 17 integration binaries incl. changes 5, conflict_changes 3)
cargo clippy -p opencanvas-server --offline -- -D warnings   # clean
cargo fmt --all --check                          # clean
git diff --check                                 # clean
```

Mutant check: replacing the per-commit budget test `used + grow > budget` with
`used > budget` fails 2 of 5 `changes` tests (budget sweep and the oversize commit); the
restored file (`diff` clean) passes. The harness runs above were taken after a description
trim, on the final binary; an earlier pair with a longer `get_changes` description had the
same S4 calls/bytes and a 161 B larger `tools/list`. Every harness run recorded
`cleanup:{serverStopped:true, rootRemoved:true}`; port 3847 and `~/.opencanvas` were untouched.

### S4 conflict recovery (stacks sample)

| Run | Calls | Bytes | Structured | Recover calls | Recover bytes | Recover ms | Wall ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline (07A) | 8 | 178,939 | 80,546 | 4 | 175,009 | 301.57 | 346.94 |
| after-07E-default-all | 8 | 178,939 | 80,546 | 4 | 175,009 | 45.16 | 76.56 |
| after-07D-default | 8 | 179,145 | 80,643 | 4 | 175,009 | 47.09 | 76.33 |
| after-07D-default-run2 | 8 | 179,145 | 80,643 | 4 | 175,009 | 47.48 | 80.12 |
| after-07D (`changedSince`) | 6 | 6,858 | 2,249 | 2 | 2,722 | 22.27 | 52.28 |
| after-07D-run2 (`changedSince`) | 6 | 6,858 | 2,249 | 2 | 2,722 | 21.78 | 50.98 |

Recovery with `changedSince`: `find_nodes` 2,027 B + retry `apply_batch` 695 B, against
outline ×2 (40,279 B) + context (134,035 B) + retry. Recovery calls 4 → 2 (−50%), recovery
bytes 175,009 → 2,722 (−98.4%), scenario bytes 179,145 → 6,858 (−96.2%) against the same
build's default; recovery time −25 ms (−53%) on this machine, and ~−280 ms against the 07A
baseline, most of which 07F's pins had already removed.

Default arguments: only the losing `apply_batch` response grows, by +206 B on the wire
(+97 B structured, the text block and `structuredContent` both carry it):
`"changedSince":{"changedIds":["overview-nav"],"fromRevision":1,"toRevision":2,"truncated":false}`.
Every other call is byte-identical to after-07E-default-all; S1–S3, S5, S6 were not rerun
since no code they call changed (`error.rs` serializes identically without a detail).

Handshake: `tools/list` 18,507 → 19,567 B (+1,060: the `get_changes` tool and the
`apply_batch` description sentence); `initialize` 3,511 → 3,943 B (+432: the conflict
recovery rule in the connect instructions).

### Open risks

- The rebase decision is the agent's: `changedIds` does not include parents of created
  children or nodes whose resolved value changed through a token/style edit (the token ID is
  listed instead). The server still validates the rebased batch fully; it cannot detect a
  semantically stale but valid edit, which is the same as before 07D.
- A conflict now costs one extra bounded log read (≤100 commits) on the losing path only;
  winners and non-conflict errors pay nothing.
- `get_changes` with `includeOperations:true` returns other clients' operation payloads
  (props, token values); this is the same data any reader of the snapshot can see.
