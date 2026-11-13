# Task25 handoff — 2026-11-13

User requested a prompt for another session to continue. Stop implementation in
this session. Workspace `/Users/luis/Workspace/OpenCanvas`, branch
`feat/incremental-validation`. Product source HEAD is
`a98012d33b14738c115414dd029df9240bc017bc`; a later docs-only checkpoint may follow.

## Read first

`AGENTS.md`, `PLAN.md`, `tasks/25-incremental-validation.md`,
`docs/TASK25-REPORT.md`, `docs/TASK24-REPORT.md`,
`docs/LARGE-PROJECT-PROPOSAL.md`, `docs/PAGE-CONCURRENCY.md`,
`docs/MOKA-SNAPSHOT-CACHE.md`, `docs/URL-NAVIGATION.md`, and active LOG.
Task25 contains the complete six-slice requirements and acceptance criteria.

## Committed work

- `81f883c`: Task24 benchmark baseline, committed with explicit user permission.
- `7cc36e0`: packaged Task25 before product implementation.
- `6c4a156`: slice 1, incremental affected-set validation and borrowed theme context.
- `b1622cb`: slice 2, indexed scoped guards and sparse history footprints.
- `1bd675a`: archived closed React/Task24 coordination logs.
- `a98012d`: slice 3, shared revision indexes and authoritative cache identity.

Main coordinates/verifies; each implementation task uses a fresh subagent with
`fork_turns="none"`, same default model/effort. Finished workers must not be reused.
All prior workers are finished. Preserve the existing branch and commits.

Targeted checks and commands are in the report: slice 1 full scene/storage suites
passed at that stage; slice 2 had 73 targeted cases including temporary PG; slice
3 had 51 targeted cases including cache/identity/read/commit consumers and PG.
Clippy three touched libraries and targeted fmt passed. Coordinator independently
ran 7 scene differential and 4 scoped parity tests successfully. Web unchanged:
117 unit + 10 HTTP and React shell/22 browser checks/Layers/font workflow passed.
These are stage-specific results, not a completed final gate at `a98012d`.

## State at handoff

Combined default test command was started but terminated during compilation when
the user requested handoff. Exit 143; no test results yet. Its process group was
confirmed stopped. Partial log: `/tmp/opencanvas-task25-default-gates.log`.
The next run can reuse compiled artifacts. No Task24 matrix was started.

No changed Rust file exceeds 250 lines. Global fmt historically has unrelated
differences in render/path_scan.rs, shadow.rs and stroke.rs; do not overwrite them.
The checkpoint also removes an extra blank EOF in the archived log.

## Next actions

1. Check git status/HEAD, preserve other changes, then finish default gates:
   `cargo test --offline --no-fail-fast -p opencanvas-scene-input -p opencanvas-storage -p opencanvas-server -j2 -- --test-threads=2`.
   Run clippy for the three touched crates and fmt/diff checks. Loopback/Chromium
   tests require sandbox escalation; authorization to run these checks already exists.
2. Run exactly `TASK24_COPIES_LIST="1 15 30" bash scripts/task24-bench.sh`.
   Keep at least five samples, both engines and default shared-master variant.
   Do not overlap heavy tests/builds with measured cells. Docker/PG image is available;
   the OrbStack socket requires escalation. Script uses disposable PG and cleans up.
3. Update TASK25 report with actual before/after medians/p95/phases/counts/RSS versus
   Task24. Target PG 30× strict/scoped approximately ≤1.5 s and context ≤1 s;
   do not claim these targets before measurements or hide regressions/failures.
4. Assess slices 4–5 and stop/report before starting them if large, as user requested.
   They touch page expansion, tree/context/find/export/viewports/jobs, HTTP/cache/SSE
   and SWR navigation. Existing find responses include totalMatches/scannedNodes:
   preserve response/cursor semantics. Propose concrete smaller handoffs first.
   Slice 6 follows; do not raise caps early or claim default 200k support now.

Cold dependency construction still expands/resolves fonts project-wide once per
revision. Warm closure uses cached adjacency/font IDs. Metadata cache is separate
from unchanged 16 MiB payload admission: TTL 1 hour, 256 MiB proxy capacity with
4 MiB reserved dependency retention per entry. It never retains expanded payloads.
Storage still loads full authored rows; page-scoped reads remain unimplemented.

## Non-negotiable boundaries

Never write to native 3857/3858, PostgreSQL 2703, `~/.opencanvas` or preview3877.
Only the benchmark may GET native3857 for source snapshot/assets. Mutations use
temporary SQLite/disposable PG only. No deploy/restart/push. No storage-layout
change. Keep private source and generated benchmark outputs unversioned.
Commit each further implementation slice separately, without AI attribution.
Append assignments/results/blockers to LOG. Reply to the user in Vietnamese.
