# Task08 report — live workspace events in the viewer

Date 2026-10-28. Branch `main`, uncommitted. User decision applied: **Follow agent defaults to
off** (`FOLLOW_AGENT_DEFAULT` in `web/src/hooks/useFollowAgent.ts`), persisted in local storage
(`opencanvas.followAgent`) inside try/catch. Effective on port 3847 only after the user restarts
that server; this task never touched it.

## What changed

Server (`crates/server`)

- `workspace_events.rs` (new, 112): `classify(message, own)` maps a broadcast message to
  `Replay` (own commit only), `Forward(event)` or `Ignore`; `project_created` broadcast builder;
  `GET /api/v1/events` handler (no cursor, no replay, heartbeat 10 s, ends on
  `shutting_down()`, lagged → `reset_required {reason:"buffer_overflow"}` then end, unknown
  query → `INVALID_REQUEST`).
- `events.rs` (125): after replay the stream now waits in an inner loop and only `break`s back
  to the durable replay for a commit of its own project. Another project's commit is forwarded
  as `project_activity {projectId,revision}`, `project_created` as-is, own `scene_ready` as
  before; none of them calls `commits_after`. Hints carry no SSE `id`. Lagged and shutdown
  behaviour unchanged.
- `mcp.rs` `create`: sends `{"event":"project_created",projectId,slug,name,revision}` after a
  successful creation only (shared by MCP and `POST /api/v1/projects`).
- `http.rs`: one route `/api/v1/events` (behind the same `guard`). `lib.rs`: module declaration.

Viewer (`web/src`)

- `useProject.ts` (207 → 138) split into `useProjectList.ts` (list, refetch, merge/bump),
  `useProjectStream.ts` (the tab's single `EventSource`: project URL with cursor when a project
  is selected, `/api/v1/events` otherwise), `useSnapshot.ts` (authoritative snapshot with the
  existing coalescing/never-backwards rules), `useFollowAgent.ts`; `useProject` composes them and
  keeps every field `App.tsx` used, adding `activity`, `follow`, `setFollow`.
- Every stream open and `reset_required` refetches both the snapshot and the project list.
- Pure reducers in `state.ts`: `mergeProjects`, `bumpRevision`, `markActivity`,
  `clearActivity`, `pickProject`, `followTarget`; 4 new unit tests.
- `App.tsx` (244 → 231): selector extracted to `components/ProjectPicker.tsx` (`●` on marked
  options, count badge next to the selector); `components/FollowToggle.tsx` checkbox in the
  status bar.

E2E: `spikes/e2e/live-workspace.mjs` (group) + `spikes/e2e/live-tab.mjs` (tab, request
listener, event log), wired into `run.mjs` with one import and one call line (232 → 234).

Docs: `docs/CONTRACT.md` (events + `GET /events`, non-durable, list API is the source of
truth), `docs/VIEWER.md` (new "Live workspace" section; Consistency paragraph updated).

## Measured (production `npm run test:e2e`, temp data root, random port)

Event sequence actually received by the viewer's EventSource (`<xxxx>` = projectId prefix):

```
/api/v1/events                   project_created   alpha r0   (empty workspace)
/api/v1/projects/<alpha>/events  commit            alpha r1   (auto-selected, own commit)
/api/v1/projects/<alpha>/events  project_created   beta  r0
/api/v1/projects/<alpha>/events  project_activity  beta  r1   (A not refetched)
/api/v1/projects/<beta>/events   commit            beta  r1   (user selects B; cursor :0 replay)
/api/v1/projects/<beta>/events   project_created   gamma r0   (after backend restart; follow on)
/api/v1/projects/<gamma>/events  project_activity  alpha r2   (follow → alpha)
/api/v1/projects/<alpha>/events  commit            alpha r1, r2 (replay from :0)
```

EventSource per tab (Playwright `request`/`requestfinished`/`requestfailed` listener):
**max 1 open at any time**, 6 opened in total across empty → A → B → backend restart reconnect
→ C → A. Same result in 3 extra standalone runs of the group.

Latency, MCP call start → UI state observed by Playwright (full run; 3 standalone runs within
the same range):

| Step | ms |
| --- | --- |
| `create_project` in empty workspace → project selected | 31 (24–26) |
| `create_project` of B while viewing A → option in selector | 29 (28–30), AC < 2000 |
| commit to B while viewing A → `●` mark on B | 16 (17–19) |
| follow on: `create_project` → selected | 29 (27–34) |
| follow on: commit to A → selected | 16 (12–14) |

Also asserted: A's snapshot request count unchanged 500 ms after B's commit; selecting B shows
`Authored r1` and clears the mark; list refetched after reconnect; follow defaults to off and
persists `"true"`; viewer API traffic GET-only and no page errors (`assertReadOnly`) in both
the live group and the main E2E viewer.

Server proof that a foreign commit does not replay (`tests/workspace_events.rs`): A's stream is
opened, then A's `commits` table is renamed so any replay fails. B's `create_project` and
commit yield `project_created`, `project_activity`, and an injected own `scene_ready` is
forwarded, with no `reset_required` and no `id:` line; injecting A's own commit then produces
`reset_required` `replay_unavailable`, proving the trap was armed. Mutation check: making
`classify` replay on any commit makes this test fail.

## Commands run (real results)

| Command | Result |
| --- | --- |
| `cargo test -p opencanvas-server --offline` | pass: 56 tests (7 unit + 49 integration in 18 binaries, incl. 3 new); `transport.rs` 4/4 unchanged |
| `cargo clippy -p opencanvas-server --offline -- -D warnings` | pass, no warnings |
| `cargo fmt --all --check` | pass |
| `npm --prefix web run build` (runs `tsc --noEmit`) | pass; existing >500 kB chunk warning |
| `npm --prefix web test` | 8/8 pass |
| `npm --prefix web run test:browser` | pass (fixture smoke, 9 checks) |
| `npm run test:e2e` | exit 0; 16 checks + `liveWorkspace` evidence in `artifacts/e2e/evidence.json` |
| `git diff --check` | pass |

Loopback/browser commands ran with the sandbox disabled (EPERM on bind otherwise). Port 3847 /
PID 12919 untouched; temp data roots removed.

## Deviations and notes

- UI text is English like the rest of the viewer: the toggle is labelled **"Follow agent"**
  (the task's "Theo dõi agent"). It sits in the status bar, not the top bar, so it stays visible
  at 390 px; no CSS file was in scope, so it reuses existing classes (`connection`,
  `status-dot`).
- A `project_created` of another project shows a notice but no `●` mark; the mark is reserved
  for `project_activity`, so the AC "commit to B marks B" is observable.
- Extra files beyond the listed names, under owned directories, for the modular rules:
  `useSnapshot.ts`, `useFollowAgent.ts`, `components/ProjectPicker.tsx`,
  `components/FollowToggle.tsx`, `spikes/e2e/live-tab.mjs`; `types.ts` got `ActivityEvent`.
- First snapshot and stream now start from two hooks in the same effect pass (snapshot first);
  every stream open refetches the snapshot, so no commit is lost. Project switch still uses
  cursor `<id>:0` (pre-existing behaviour), which replays that project's log up to 1000 commits.
- The viewer still reads only the first page (100) of `GET /api/v1/projects`.
- `spikes/e2e/README.md` (not owned) does not yet list the live workspace group.
- No file touched here exceeds 250 lines; closest: `run.mjs` 234, `App.tsx` 231.
