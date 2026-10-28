# Agent-cost harness (Task 07A)

Measures what an MCP agent pays — tool calls, bytes and latency — for the main
read flows, so every Task 07 slice can report a before/after delta from the
same harness. It only measures; it changes no product code.

## Run

From the repository root, with the production binary and renderer bundle built
the same way `npm run test:e2e` builds them:

```sh
npm run build:renderer
cargo build --offline -p opencanvas-layout -p opencanvas-server
node spikes/agent-cost/run.mjs                      # writes artifacts/agent-cost/baseline.json
node spikes/agent-cost/run.mjs --label after-07B    # writes artifacts/agent-cost/after-07B.json
node spikes/agent-cost/compare.mjs baseline after-07B
```

Flags: `--label <name>` (default `baseline`), `--only S1,S3` (subset),
`--build` (run the two build commands first), `--opt path=value` (repeatable,
see Extension point). One full run takes about 35 s, mostly S6 seeding.
Exit code is 1 when any scenario fails; the file is still written with
`ok:false` and the scenario's `error`, so a broken run is never mistaken for a
measurement.

Every scenario gets its own OS temporary data root and its own backend process
on a random loopback port (`spikes/e2e/server.mjs`, port 0). The project is
seeded through real MCP mutations with an unmetered session, then the backend is
**restarted** before measuring, so the measured session starts cold: no
prerender jobs from the seeding commits, no in-memory render cache — the state
an agent sees when it opens an existing project. MCP sessions, the backend
process group (worker and Chromium included) and the temp root are removed in
`finally`; each scenario records `cleanup:{serverStopped, rootRemoved}`. Port
3847 and `~/.opencanvas` are never used.

## Scenarios

| ID | Flow | Calls made (baseline) |
| --- | --- | --- |
| S1 | Find the text node whose copy is "Tổng quan" and read its props | `list_projects` → `get_outline` (all pages) → `get_subtree` per page root until a node's `props.text` matches. Outline names are IDs, so text is only visible in props; the matching subtree page already carries the props. |
| S2 | Full design context of the stacks frame | `get_design_context {projectId, frameId}` with defaults, following `nextCursor`. |
| S3 | Render + inspect at 1040×720, 800×720, 390×844 | Three `render_frame` enqueues, then every 250 ms one `render_frame {jobId}` poll per pending job until all leave `pending`, then `inspect_layout` per job (following `nextCursor` if a server ever returns one). Polls are counted per viewport. |
| S4 | Two clients, same base revision | A and B each `get_subtree` their node (depth 0) on the same revision. A commits an `update_node`; B gets `REVISION_CONFLICT` (only `currentRevision`), re-reads `get_outline` + `get_design_context` (all pages), then retries with the new revision and a new idempotency key. |
| S5 | `get_outline` latency, 1 vs 20 projects | Stacks project alone: 1 warm-up + 20 measured first-page outlines. Then 19 small filler projects are added, the backend restarted, and the same 1 + 20 calls repeated. |
| S6 | 5,000-node project | Same shape as the storage spike's "large" fixture (`spikes/storage/README.md`): one page, two 1440×900 frames, alternating rectangle/text leaves, token + style. Full `get_outline`, then full `get_design_context` of `frame-a` (2,500 nodes). Not rendered: it exceeds the 2,048-node scene cap. |

The stacks project is the Task02 sample (`samples/welcome-stacks`, imported, not
modified): 182 nodes, frame `welcome-dashboard-stacks`.

## Output — `artifacts/agent-cost/<label>.json`

```text
schema        "opencanvas.agent-cost/1"
label, generatedAt, ok
environment   {node, platform, gitHead, serverBinary:{sha256 (first 16 hex), modified}}
options       effective options (defaults + overrides)
overrides     only what --opt changed; {} for the baseline
handshake     {initializeResponseBytes, toolsListResponseBytes}  paid once per MCP session
summary       {S1: totals, …}  quick diff target
scenarios.Sx
  id, title
  totals      {calls, requestBytes, responseBytes, bytes, structuredBytes, callMs, wallMs}
  phases      {<phase>: {calls, requestBytes, responseBytes, bytes, structuredBytes, callMs}}
  result      scenario facts (S3 polls/readyMs per viewport, S5/S6 latency p50/p95, …)
  calls       [{tool, phase, client?, requestBytes, responseBytes, structuredBytes, ms, error?}]
  cleanup     {serverStopped, rootRemoved}
```

- `requestBytes`: the JSON-RPC request body as sent.
- `responseBytes`: the matched JSON-RPC response message, without SSE framing or
  priming events. The server returns tool results with both a JSON text block
  and `structuredContent`, so this is roughly 2× the payload; it is what an MCP
  client receives.
- `structuredBytes`: compact JSON of `structuredContent` alone — the payload
  that the server's 64 KiB `budget` applies to.
- `ms`: wall time of one `call` as seen by the client. `callMs` sums them;
  `wallMs` spans first metered call start to last metered call end. It includes
  S3's poll sleeps and, for S5, the filler seeding and restart between the two
  sets (use S5 `result.latency` there).
- Percentiles are nearest-rank over the measured samples (warm-ups excluded).

Bytes and call counts are deterministic across runs except S3, whose poll
count depends on render timing. Latency varies; see `docs/TASK07-REPORT.md` for
measured run-to-run spread. Discard the first run after a fresh build: it was
noticeably slower (cold file cache and Chromium start).

## Extension point

Defaults in `options.mjs` are the baseline and use only tools and arguments that
exist before Task 07B–07F. Later slices opt in from the command line:

```sh
node spikes/agent-cost/run.mjs --label after-07B --opt s3.inspectArgs='{"fields":["bounds"]}'
node spikes/agent-cost/run.mjs --label after-07C --opt s2.contextArgs='{"compact":true}'
node spikes/agent-cost/run.mjs --label after-07E --opt s3.pollArgs='{"waitMs":2000}'
```

`*Args` objects are merged verbatim into that scenario's tool calls. A new tool
is added as a named strategy in the scenario module (`strategies` in
`s1-find-text.mjs` and `s4-conflict.mjs`) and selected with
`--opt s1.strategy=<name>` / `--opt s4.strategy=<name>`. A slice that only
changes server internals (07F) runs without overrides. Unknown option paths are
rejected.

## Modules

| File | Responsibility |
| --- | --- |
| `run.mjs` | CLI, scenario order, environment facts, writing the artifact |
| `options.mjs` | Baseline defaults (extension point) and argument parsing |
| `meter.mjs` | Taps `fetch` used by `spikes/e2e/protocol.mjs`; per-call records, totals, latency stats |
| `env.mjs` | Temp data root, backend start/restart, MCP sessions, cleanup |
| `seed.mjs` | Stacks sample, small filler and 5,000-node fixtures through MCP |
| `scenarios/common.mjs` | Cursor-following reads and the shared scenario summary |
| `scenarios/s1…s6-*.mjs` | One scenario each |
| `compare.mjs` | Markdown before/after table for two labels |
