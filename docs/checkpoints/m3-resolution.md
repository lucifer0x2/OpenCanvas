# M3 token/theme resolution checkpoint — 2026-10-28

## Resumed task completed — current handoff

The earlier paused/unverified status below is historical. A fresh worker resumed
the shared `main` working tree, retained all prior changes, and verified the M3
resolver without needing a production fix. No commit/reset/checkout or dependency
installation was performed. No agent/session was spawned and no background
process remains. Storage fixtures use temporary roots, never `~/.opencanvas`.

Completed changes in this resumed task:

- Split `tests/resolution.rs` (now 233 lines) into focused limits (151), safety
  (37) and text-style (69) modules under `tests/resolution/`.
- Expanded from 5 to 9 actual tests: project theme union 32/33 boundary,
  snapshot token count 4096/4097 boundary, malformed unused branch/name rejection,
  text-style dependencies and node overrides, and unknown-theme errors on both
  public APIs. Existing alias/depth/atomic/revision scenarios are retained.
- Added `spikes/pipeline/constraints-fixture.mjs` (72 lines) and
  `constraints.test.mjs` (67) with two real-worker tests for exact nested
  row/column fixed/fill/hug min/max bounds, padding/gap, explicit line-height
  baselines and font widths, authored immutability, overflow extents and
  clipped/unclipped RGBA samples. Generated ignored scene evidence:
  `spikes/pipeline/output/constraints.scene.json`.
- Updated `docs/RESOLUTION.md` with the supported contract, actual evidence and
  precise remaining scope. Every resumed modified file is below 250 lines.

Commands actually completed:

```sh
cargo test -p opencanvas-storage --offline --test resolution -- --test-threads=2
# 9 passed, 0 failed, 0.25s (last run)
cargo test -p opencanvas-storage --offline --test references --test style_aliases -- --test-threads=2
# 5 passed across 2 suites, 0.17s
node --test --test-concurrency=1 spikes/pipeline/constraints.test.mjs
# 2 passed, 0 failed/skipped, 505.48ms (last run, approved sandbox escalation)
rustfmt --edition 2021 crates/storage/tests/resolution.rs
npx prettier --write spikes/pipeline/constraints-fixture.mjs spikes/pipeline/constraints.test.mjs
# both formatting checks/actions succeeded
```

The first sandboxed Node invocation failed with `listen EPERM: 127.0.0.1`.
The approved escalated rerun ran real Chromium 156.0.8078.4 on Node v24.21.0.
Full pipeline/storage suites were not rerun: root requested targeted verification
while M4 is still changing shared storage. No M3 resolver blocker remains.

Public API and limits are unchanged from the contract below. Missing refs,
wrong types, cycle/depth and malformed/theme limits fail before durable commits.
Token count boundary is verified at the shared validator/readback seam, not by
a 4096-token durable write load test. Server/E2E owns revision/theme scene cache
invalidation and context/pagination; these tests do not claim those are complete.
The root/server owners were informed of actual passes and no API changes.

## Historical paused checkpoint

Paused at the main agent's instruction: user token limit is near; stop expansion
and preserve current work. This task is **unfinished and unverified**. No agents
were spawned, no dependencies installed, and no background process remains.
Workspace: `/Users/luis/Workspace/OpenCanvas`. Handoff originally reported no Git;
latest read-only `rtk git status` now reports branch `main`, with shared changes
from multiple workers. No commit made by this worker.

## Files implemented

- `crates/storage/src/validate.rs`: new token modules and public re-exports.
- `crates/storage/src/validate/candidate.rs`: concrete theme context, all-theme
  validation, token graph validation after each ordered token upsert.
- `crates/storage/src/validate/refs.rs`: token lookup through theme/alias resolver,
  new theme-aware node/literal-token APIs, safer malformed text-style readback.
- `crates/storage/src/validate/tokens.rs`: envelope/alias schema, typed target
  validation, finite limits/themes, guarded iterative literal resolution.
- `crates/storage/src/validate/token_graph.rs`: bounded union-branch graph checks
  reject cycles/depth, including unused and cross-theme branches.
- `crates/storage/tests/resolution.rs`: five tests covering authored/computed
  separation, multihop aliases, default/dark fallback, dependent styles, token
  updates/revision/idempotency, atomic bad refs/type/cycles, stale corrupted
  snapshot safety, invalid themed property ranges, malformed schemas and caps.
- `docs/RESOLUTION.md`: public contract, API, limits and verification disclaimer.

M4 owns `lib.rs` and has already added the requested new function exports there;
this worker did not edit that file. No storage DTO or persistence schema changed
for tokens. Server/root were sent the API and contract early. Server integration
is owned elsewhere and has not been checked by this worker.

## Exact checks and results

Ran `rustfmt --edition 2021` on the changed validator modules/test file: exit 0.

Ran:

```sh
cargo test -p opencanvas-storage --test resolution -- --test-threads=2
```

Exit 101 during compilation; **zero tests executed**. Compiler reported:

1. Missing `bundle_import` module referenced from `src/lib.rs:13`.
2. Missing `bundle_validation` module referenced from `src/lib.rs:14`.
3. `u64: FromSql` unsatisfied at `src/migration.rs:29`.

These were concurrent M4 edits. Sent exact blockers to M4. By the later
read-only status, both module files existed; no rerun was done after the stop
instruction, so compile success and M3 behavior remain unknown. Do not report
passing tests or acceptance from this checkpoint.

Line-count check: validate.rs 10; candidate.rs 199; refs.rs 160; tokens.rs 163;
token_graph.rs 65; resolution.rs 303. Production modules remain under 250 lines.
The 303-line test file is one focused contract suite; reviewed for splitting.
Keeping its five related setup/scenario tests together currently aids reading;
if expanded, split negative schema/cap tests into another focused test module.

## Exact resume steps

1. Confirm concurrent M4 compiler work is complete; read its checkpoint/status.
2. Run the targeted `--test resolution` command above. Fix any local compiler or
   behavioral error through a fresh implementation task/worker as main requires.
3. Since shared validation changed, run the full storage suite once:
   `cargo test -p opencanvas-storage --features fault-injection -- --test-threads=2`.
   Existing style-alias rejection tests must remain passing. Use temporary roots.
4. Verify server scene jobs/context use `resolved_nodes_for_theme` and
   `resolved_tokens_for_theme`, with structured unknown-theme errors and cache
   keys including revision + concrete theme. This is server-owner work.
5. Complete the not-yet-started focused real-worker layout regression fixture
   for nested row/column fixed/fill/hug/minmax/padding/gap/overflow/line-height
   coverage. Do not alter layout/renderer implementation without main assigning
   that scope; report any actual engine bug to main.
6. Update `docs/RESOLUTION.md` with real check results and remaining limits;
   main updates AGENTS progress. No M3 acceptance claim until checks complete.

No pipeline fixture, renderer, layout engine, worker, Cargo manifest, model DTO,
DB/store or AGENTS progress file was edited by this worker.
