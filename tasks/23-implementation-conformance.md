# Task23 — Implementation conformance check (expected vs actual)

**Status:** ready to assign. Not started. Requested by the user on 2026-11-09.

## Why

AXIS Courses Dev workers build a Moodle theme from OpenCanvas frames. They report
"khớp thiết kế" based on side-by-side images and prose numbers. The lead or the user then
keeps finding misses:

- 3 extra issues after D-notify-1, 5 after D-notify-2 (`axis-courses/.company/board/D.md:87-88`);
- M1 off design on 5 points (`.company/dev/D-retro-m1.md`).

Measurement files hold only actual values (`evidence/notify-local/1/*.json`). Nothing
records the expected design value or the design revision used. Frames are 1440 wide but
captures are at 1920 (sometimes at 90 % zoom or 2x DPR scaled down), so composites cannot
be compared by geometry.

Typical misses are all measurable:

- decoration: border, chevron, icon box, shadow, background colour;
- text fit: truncation, extra line wraps;
- offsets: `#page` 8–21 px;
- theme CSS that silently did not apply to Moodle DOM;
- layout shift when a control is clicked.

The retro's P3 asks for a DOM-vs-design measurement. No worker can do it reliably by hand.

Goal: a tool that turns "khớp" into a measured table. OpenCanvas exports the expected
values per node at an exact revision, viewport, theme and state. A script measures the
live page and writes expected / actual / delta / status per node and property. The spec
and raw measurements never pass through the model; only the final table does.

## Scope

Workspace `/Users/luis/Workspace/OpenCanvas`, branch `feat/page-concurrency`.

- Do not touch the untracked `docs/GRAPHITE-RENDERER-RESEARCH.md`.
- No deploy, no native-service restart, no DB manipulation, no push.
- Keep modules focused, preferably below 250 lines.
- Independent of Task22. Coordinate only if both edit `crates/server/skills/implement-frame.md`.

### 23a — Expected-value spec (server)

- Add a read-only HTTP endpoint on the private listener only, for example
  `GET /api/v1/projects/{project}/conformance-spec`. Follow the auth of the other
  `/api/v1/projects` routes.
  - The public viewer listener and public sessions must not expose it. Extend
    `public_viewer`/`project_privacy` tests.
- Inputs: `frameId`, `revision` (required: exact, immutable), `viewport` (width×height),
  optional `theme`, `state`, `nodeIds` (default: every visible node in the frame), `fields`.
- Source of truth is the rendered scene for that key, the same data `inspect_layout`
  serves, plus `resolvedProps` from `get_design_context`. Do not re-derive layout.
  Reuse the existing render job or scene cache and its job key.
- Per node, expected values in CSS terms:
  - frame-local box `x,y,width,height`;
  - background (resolved RGBA or gradient);
  - border width, colour and style per side;
  - radius per corner;
  - shadow list;
  - opacity;
  - for text: font family, weight and style, size, line height, letter spacing, colour,
    alignment, transform, decoration, exact lines (count and text) and whether the design
    clips or overflows;
  - plus `id`, `name`, `kind`, `parentId`, authored token/style references (so a mismatch
    can name the token), `visible`, and the interaction-state variant when `state` is given.
- Each response carries `projectId`, `revision`, `frameId`, viewport, theme, state, the
  scene key and a schema version. Paged with a byte budget like the other reads. Values use
  the CSS mapping already documented in `skills/implement-frame.md` and `props.md`; share
  that code rather than duplicating rules.
- An MCP tool is **not** required. If one is added, it must stay within `tools_budget`
  without raising `TOTAL_MAX`, and it should return only a pointer or summary, not the full
  spec.

### 23b — Measuring script (`tools/conformance/`)

- A Node CLI using the repo's pinned `playwright` (1.64.1), run as
  `npm run conformance -- <args>`. Arguments:
  - `--server` (default `http://127.0.0.1:3857`), `--project`, `--frame`, `--revision`;
  - `--viewport 1440x900` (default: the frame's own size);
  - `--theme`, `--state`;
  - `--url` (the implementation page);
  - `--map map.json`;
  - `--storage-state` (an optional Playwright auth file prepared by the user; the script
    never types passwords);
  - `--out <dir>`.
- The browser runs at exactly the design viewport, `deviceScaleFactor: 1` and 100 % zoom.
  It waits for `document.fonts.ready` and network idle. Abort with a clear error if the
  page viewport, zoom or DPR do not match.
- Map file: `root` (CSS selector for the frame root), and `nodes` as nodeId → selector
  (optional `nth`, and `ignore` with a reason).
  - Optional `interactions`: a list of steps (click, hover, focus) with the nodeIds whose
    boxes must not move. The script measures before and after each step and reports
    layout shift.
  - Optional `setup` steps before measuring, for example forcing a slow network to capture
    a loading state.
- Measure with `getBoundingClientRect` relative to the root, `getComputedStyle` and
  text-line rects (`Range.getClientRects`). Report truncation or overflow
  (`scrollWidth > clientWidth`, ellipsis), extra wraps, and selectors that match nothing
  or several elements.
- Compare with tolerances that are configurable and recorded in the report:
  - geometry ±1 px by default;
  - colours exact RGBA, or ΔE ≤ 2 when `--color-tolerance` is set;
  - font values exact;
  - line count exact.
  - Unmapped visible design nodes are listed as `not-checked`, never as passing.
- Outputs:
  - `report.md`: a compact table node | property | expected | actual | delta | status,
    fails first, plus a summary header with revision, viewport, URL, map hash and counts;
  - `report.json`;
  - the page screenshot and the `render_frame` PNG at the same viewport and scale, side by
    side, with failing boxes outlined.
  - Exit code non-zero when any fail exists.
- `--suggest`: propose a starter map by matching design text nodes to DOM text, and
  containers by bounding-box overlap. Output only for a human or agent to edit; never use it
  silently for the check.
- Keep the table short enough to paste into a task report. Raw measurements stay in files.

### 23c — Guidance

- `skills/implement-frame.md`: a short "Conformance check" section covering when to run it,
  the map file, reading the table, and that `not-checked`/`fail` rows block "matches
  design".
- `tools/conformance/README.md` with one worked example.
- Do not edit AXIS Courses prompts from this repo. The report lists suggested lines for
  `prompts/worker-dev.md` (states table plus conformance report required before "XONG").

## Acceptance

- A fixture frame in a temporary SQLite workspace plus a static HTML implementation in
  `tools/conformance/fixtures/`. An end-to-end test proves the tool detects intentional
  deviations:
  - a background colour, a 4 px padding offset, radius, border on one side, shadow and font
    weight;
  - a truncated label, an extra line wrap and a missing node;
  - a selector matching two elements;
  - layout shift after a click.
  - It must also show a matching version producing zero fails.
- Viewport, zoom and DPR guard tests. Tolerance tests. Tests that `revision` is
  mandatory and that the spec is byte-stable for one scene key.
- Endpoint tests:
  - private listener only;
  - auth as for the other project routes;
  - unknown frame, node or revision errors;
  - paging and budget.
- Run focused Rust tests, the new Node tests and `tools_budget`; record their output.
  Pre-existing failures in `docs/NATIVE-COMBINED-CANDIDATE.md` remain out of scope.
- Report `docs/TASK23-REPORT.md`: design decisions, a sample `report.md` from the fixture,
  checks actually run, and limitations. Limitations must cover pseudo-elements, icon fonts,
  gradients and what is not measured. The root session reviews the diff before any commit.
  Deployment is a separate user decision (`AGENTS.md`).

## Out of scope

- Pixel-diff as a pass/fail gate (the side-by-side image is for humans only).
- Moodle-specific selectors, logins or data seeding; those live in AXIS Courses map files.
- Automatic selector inference beyond `--suggest`.
- Writing to designs, and changes in the AXIS Courses repository.
