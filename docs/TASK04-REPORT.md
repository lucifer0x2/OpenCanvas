# Task04 — row wrap implementation report

2026-10-28. Worker implementation complete; root independent review/acceptance is
complete. User approved “Duyệt, triển khai row wrap” against TASK04-PROPOSAL.md.
Workspace main; no commit/push/reset/checkout, agent/session spawn, model/auth/config
change, real project mutation, or real3847 restart. Existing01/02/03/05 dirty work
and hook cache were retained. AGENTS/task status remain root-owned.

## PLAN-first sequence and module boundaries

The **first mutation was apply_patch to PLAN.md**, adding approved schema, guards,
semantics, fixture AC, versioning and limits. Worker then emitted commentary
“PLAN.md đã được cập nhật là thay đổi đầu tiên…” before the first production patch.
Production followed: extract Sizing/LayoutRule to model/layout.rs with re-export;
add typed bool default false/skip false; map Taffy wrap/start only when true;
extract storage sizing/layout guards from properties.rs before adding wrap rules.
No new dependency, DB migration or scene/API/snapshot version bump.

New focused modules: layout validation/layout.rs and wrap_tests.rs; storage
validate/layout.rs and tests/wrap.rs; pipeline responsive-fixture/assert/test;
server tests/wrap.rs; E2E wrap-design.mjs and wrap.mjs. Existing engine remains
183 lines, with only an equivalent or_else→or lint correction. Renderer/worker/web
behavior and existing no-wrap samples/known-overflow golden are unchanged.

## Behavior and evidence

Literal boolean layout.wrap defaults false (false omitted when serialized).
True only supports row containers with positive definite width or a finite fill
chain. Direct computation anchors at selected frame viewport; storage validates
fixed root-frame props in every theme. Every immediate width-fill child requires
explicit positive minWidth, including resolved dimension refs. Inner card text
fill needs no extra minimum. Fixed/hug children and empty rows remain valid.
Both seams reject min>max on either axis; equality and fixed-size clamping remain.
Existing fill-in-hug guards hold on both axes. Oversized minima still overflow.

Taffy FlexWrap Wrap and AlignContent START are opt-in only. Gap applies between
cards and rows; align-items START preserves unequal card heights; last-row fill
can span the entire row. Hug height sums line maxima/gaps/padding; fixed/min/max
height still clamps. Measurement remains cache-only, at most6 passes, one batch
per pass. Canonical fixture measurements use2 passes/1 batch, repeat0 batches;
geometry uses1 pass/0 batches. Authored input stays immutable.

| Width | Cards per row | Card widths | Geometry hug | Real text hug |
| --- | --- | --- | ---: | ---: |
| 1040 | 3 | 320/320/320 | 80 | 128 |
| 640 | 2+1 | 288/288/592 | 156 | 224 |
| 390 | 1+1+1 | 342/342/342 | 212 | 320 |

Min280/gap16/frame padding24. Boundaries920→3,919→2+1,624→2+1,623→1+1+1.
Assertions cover exact card widths, row heights, all descendant bounds before
clipping, text line widths/heights, card height=text+32, and no overlaps. Nested
finite fill chains, fixed/hug children, fixed row align-start and max-height
overflow have focused controls. Omitted and false wrap produce identical scenes;
all old11 pipeline tests and frozen old dashboard overflow expectations pass.

Direct semantic failures occur before measurement; malformed boolean produces
actual CLI INVALID_JSON (tested null/string/number/ref). Storage invalid mode/type,
minima, unknown field, min/max, reparent, enabling-wrap around an invalid child,
ancestor-width loss, token default/theme updates and late invalid operation all
roll back snapshot/revision/commit log. Reusing each rejected key for a valid
request succeeds, proving no idempotency reservation. Valid values persist after
storage reopen. MCP apply/retry/context/subtree tests preserve authored/resolved wrap.

Actual MCP→SQLite→multi-viewport context/ready reflow=true→render/inspect→process
restart→readback/PNG completed on a temporary root and random port. All3 viewport
bounds and authored revision1 survive restart; new job IDs retain scene keys.
Independent decoded PNG comparisons:0 differences across3,511,040 channels at
1040×844;2,160,640 at640×844;1,316,640 at390×844. The production viewer frame/mobile
presets consume identical scenes/lines/PNG with0 differences at1040/390; viewer
API requests are GET-only and page errors are empty. No web change was needed.

Final E2E completed **2026-10-28T09:05:38.191Z**, Node24.21.0 /
Chromium156.0.8078.4 / MCP2025-11-25. Evidence and actual visuals:

- `artifacts/e2e/wrap.json`, `evidence.json`, `wrap-{1040,640,390}-job.json`.
- `artifacts/e2e/wrap-{1040,640,390}.png`, `wrap-viewer-{1040,390}.png`.
- Worker opened real1040/390 PNGs and checked3→1 columns, readable Vietnamese,
  long unbroken text and clear gaps with no card/text overlap.
- Pipeline `spikes/pipeline/output/wrap-*` is ignored/recreated by tests.

## Commands actually run

| Command | Result |
| --- | --- |
| `cargo test -p opencanvas-layout --offline` | 7 passed, exit0 |
| `npm run test:pipeline` | 16 passed,0 failed/skipped/cancelled; old11 retained |
| `cargo test -p opencanvas-storage --offline --features fault-injection -- --test-threads=2` | 41 passed/14 suites, exit0; final13.02s |
| `cargo test -p opencanvas-server --offline -- --test-threads=2` | 19 passed, old18 retained, exit0 |
| `cargo test -p opencanvas-server --offline --test wrap --test multi_viewport --test skill_layer -- --test-threads=2` | Final embedded skill compatibility:5 passed, exit0 |
| `npm run test:e2e` | Build renderer/layout/server/web TypeScript+production,16 groups pass, exit0 |
| `node --test --test-concurrency=1 spikes/pipeline/responsive.test.mjs` | Final stronger exact-width assertions:5 passed |
| `node spikes/e2e/run.mjs` | Final stronger exact-width assertions:16 groups pass, cleanup=true |
| `node spikes/responsive-proposal/run.mjs` | Disposable native copy:15 cases (14 scenes+expected rejection), cleanup=true |
| `cargo clippy -p opencanvas-layout -p opencanvas-storage -p opencanvas-server --offline --lib -- -D warnings` | Exit0 |
| `cargo fmt --all --check` | Exit0 |
| `prettier --check` + `node --check` on changed/new JS modules | Exit0; explicit modules in summary |
| `rtk bypass git diff --check` | Exit0 |
| Source/test line scan across crates/spikes/renderer/worker/web/src | No handwritten code>250 lines |

Initial pipeline sandbox attempt failed loopback listen EPERM. Approved escalation
reran actual Chromium successfully, without skip/mock. Storage test compile first
failed because Commit has no PartialEq; test now compares serialized logs. Initial
Clippy exposed preexisting warnings and one new collapsible guard: equivalent
expression/guard fixes applied, no behavioral relaxation. The existing public
LayoutStep enum has a single **enum-local** clippy::large_enum_variant expect with
reason: preserve Ready{scene:Scene} public API instead of unrelated boxing. No
crate/global lint allow. Shared server test helpers still emit dead_code warnings;
production strict lint is clean. Web build retains its existing large-chunk warning.

The proposal reproducer now detects native modular DTO source and copies/builds
without reinjecting fields/mapping. The no-min control must reject in native mode.
Historical prototype `evidence.json`/text PNGs are preserved; new results are
`spikes/responsive-proposal/native-evidence.json`/`native-text-*.png`. The README
clearly distinguishes historical prototype evidence and production acceptance.

## Sizes, cleanup and limits

Changed/new code sizes: model210, model/layout27, tree94, engine183, validation133,
validation/layout57, wrap_tests120; storage properties174 (was242), layout136,
wrap tests205; server wrap tests75; pipeline fixture62/assert63/test136; E2E wrap138,
wrap-design40/run232 (responsive remains240); proposal prepare66/run193. No file
exceeds250; modules each own DTO, validation, fixture, bounds assertions or one
integration seam. New public API surface is just LayoutRule.wrap via existing exports.

Final E2E reports temporaryDataRemoved=true after closing MCP/browser/server process
group and deleting temp root. Proposal copy/browser also cleaned. Real3847 listener
PID12919 was observed unchanged; it still serves older loaded code and was never
started/stopped/restarted by this task. No ~/.opencanvas data was created.

Backend and layout must upgrade together: rebuild binary and restart backend for
startup pipeline fingerprint; no mixed-version/downgrade/hot-replacement promise.
Old engine may ignore unknown wrap. No %, breakpoint variants, column wrap, align
API, renderer/web behavior, actual dashboard authoring or mobile-dashboard claim.
Sidebar176+body minimum480, fixed chart/progress and fixed text-height limitations
of old dashboards remain. Root owns final task status and independent acceptance.

## Root independent acceptance

Root reviewed production/module deltas and PLAN-first evidence; no required
finding remained. Actual independent commands all passed:

- `cargo test -p opencanvas-layout -p opencanvas-storage --offline --features opencanvas-storage/fault-injection -- --test-threads=2`: layout7/7, storage41/41.
- `npm run test:pipeline`:16/16, no failures/skips/cancellations;2.636s.
- `cargo test -p opencanvas-server --offline -- --test-threads=1`:19/19 integration tests.
- `node spikes/e2e/run.mjs`:16 groups, completed2026-10-28T09:11:22.774Z, temporaryDataRemoved=true.
- Strict library Clippy layout/storage/server, fmt, changed JS format/syntax, diff and size checks passed.

Root opened final production wrap1040/390 PNGs:3→1 columns, text inside cards,
clear row gaps. Root E2E confirms all three text hug heights128/224/320,
post-restart PNG differences0 and viewer differences0 at1040/390; no browser errors.
Read-only process inspection after checks found only the unchanged user backend
PID12919 on3847 and workerPID13009. No fixture backend/worker remains. Source
scan found no handwritten code above250 lines (largest existing file244).
Task04 is accepted within the approved row-wrap scope; runtime3847 was not rolled
forward, and the old dashboards were not reauthored. No commit/push.
