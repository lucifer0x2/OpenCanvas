# Task14 Lane A — instance colour schemes

Implemented 2026-11-02 on the shared `claude/design-properties` checkout. Existing
frontend and other lanes' changes were preserved. No commit/push, production
listener restart, or write to the user's data root was performed.

## Behavior and boundaries

- Frames/instances accept a declared token theme or `default`; unknown scopes
  return `UNKNOWN_THEME` at `props.theme`. Global request errors remain
  `UNSUPPORTED_THEME`. Nearest placed scope wins over the global fallback.
- Expansion keeps a master root's explicit theme, lets an instance replace it,
  and uses the instance's placed ancestors instead of the master's ancestors.
  Nested frame/instance scopes still win. Resolution uses a borrowed context;
  it does not clone the full candidate per node.
- One paint helper implements stroke object merge, shadow array replacement,
  and per-state visual-property merge (including nested stroke merge). Root
  instance fill/stroke/shadow/radius/opacity apply to the expanded root. The
  validator checks fully merged properties in their placed scopes, with
  `props.overrides.<target>...` error paths. Structure/layout/sizing/component
  overrides and interaction-state shadow remain unsupported.
- Styles, aliases, state paint, shadows and exact font selection use the effective
  node theme. Authored snapshots and bundle content remain authored. Scene and
  inspect nodes expose `effectiveTheme`; context exposes it once outside
  `resolvedProps`. Token/style resource `resolvedValue` describes the global
  request theme. Compact resource closure preserves named-theme declarations and
  their aliases even when a selected nested frame inherits a scope outside it.
- Preview discovers new states introduced only in instance overrides. Motion
  token/style/timing resolution uses the scene node's effective theme.
- Props guidance and machine capabilities describe the contracts; the capability
  fingerprint changes automatically because the catalog/capability document changed.

## Checks actually run

All storage fixtures use temporary roots. HTTP/MCP tests bind loopback port 0 and
launch the real Node/Chromium worker; drop/cleanup removes the temporary resources.

| Command | Result |
| --- | --- |
| `cargo check -p opencanvas-storage -p opencanvas-layout -p opencanvas-server -j 2` | Passed |
| `cargo test -p opencanvas-storage --test instances -j 2` | 4 passed |
| `cargo test -p opencanvas-storage --test instance_schemes --test instances --test resolution --test states --test font_face -j 2` | 22 passed across 5 suites |
| `cargo test -p opencanvas-storage --test instance_scheme_fonts --test instance_schemes --test paint_props_reuse -j 2` | 6 passed across 3 suites |
| `cargo build -p opencanvas-layout --bin layout-step -j 2` | Passed; updated `target/debug/layout-step` used by real-worker checks |
| `cargo test -p opencanvas-server --test instance_schemes --test instances_states --test props_reference --test multi_viewport --test instances_resources -j 2 -- --test-threads=1` | 8 passed across 5 suites |
| `cargo test -p opencanvas-server --test instance_schemes --test instances_states --test props_reference --test compact_context --test inspect_paging --test multi_viewport -j 2 -- --test-threads=1` | Intermediate run: compact 5 and inspect 2 passed; the following new scheme test exposed an incorrect test expectation about compact differing props, subsequently fixed and passed above |
| `cargo clippy -p opencanvas-storage -p opencanvas-layout -p opencanvas-server --lib -j 2 -- -D warnings` | Passed |
| `npm --prefix web run typecheck` | Passed |
| `node --test --test-concurrency=1 web/tests/preview-keyframes.test.ts web/tests/preview-model.test.ts web/tests/preview-schemes.test.ts` | 14 passed |
| `node --check web/tests/instance-schemes-smoke.mjs` | Passed |
| Targeted `rustfmt --edition 2021 ...`, targeted `npx prettier --write ...`, `git diff --check` | Passed |

The initial sandboxed HTTP test run was blocked at loopback bind (`EPERM`), then
the same temporary-listener tests ran with approved escalation. Earlier assertions
expecting rejected shadow/state overrides and the pre-theme inspect projection
were updated to the new positive contract; invalid conic paint/state shadow and
atomic rollback checks remain.

The real scheme test checks default/night/day scopes simultaneously in one scene,
context expanded/resolved states, master stroke width 2, root paint, themed shadow,
inspect metadata, cache job reuse, exact PNG samples, and themed SVG fills. Storage
adds nested instances/scopes, default scope precedence, aliases/styles/font refs,
stroke/state merge preservation, shadow layer replacement, invalid values/target
kinds, unknown themes, authored immutability and late-invalid rollback. Font tests
verify the compact font closure selects the placed bold face while the source
definition selects the regular face.

## File sizes and module review

New responsibility boundaries are `instance_paint.rs` (38 lines, shared patch
semantics), `validate/scope.rs` (67 lines, borrowed scoped resolution), and focused
scheme/font/preview fixtures and tests. Modified runtime files remain at most
220 lines: refs 220, instance validation 215, context tree 215, layout engine 210,
layout model 206, preview motion 181, catalog 180, compact resources 162,
property validation 159; remaining runtime files are smaller. `renderer/types.ts`
is 138 lines. The new preview scheme test was separated from keyframe tests to
keep both focused and below 250 lines.

`crates/server/tests/inspect_paging.rs` remains 251 lines. It has two coherent
pagination/projection scenarios plus their fixture; splitting only for one line
would scatter the byte/projection oracle and cursor checks. No modified runtime
file exceeds 250 lines, and no modified handwritten file exceeds 350 lines.

## Limits and pending evidence

U07 golden rebuild and the fuzz ≤3% comparison are for the design worker after the
API is available. This implementation does not modify U07 and does not claim the
golden acceptance or the projected ~4 KB/instance payload. No load/performance
benchmark of maximally expanded themed projects was added. SVG's existing
stroke/shadow omission diagnostics remain; this task changes resolution, not SVG
paint support. P6 contrast checking remains outside Lane A.

Root coordinator verification (reported separately from this worker's commands):

- `npm run build -- --offline`: synchronized debug server/layout-step/renderer/web
  build passed.
- `node web/tests/instance-schemes-smoke.mjs`: passed with the synchronized build;
  real preview paints night/day scopes, fetches and plays an override-only hover
  state, and uses 750 ms/night colour for the frame's scoped motion.
- Independent server regression: 25 tests across 11 suites passed, including
  scoped schemes, compact/paged inspect/multi-viewport, legacy instance behavior,
  and the other Task14 lanes. Coordinator also reports web 26 tests, renderer
  typecheck, global formatting and diff checks passed.

The preview smoke starts an isolated production-code debug server on port 0,
uses a temporary data root and cleans server/browser/data in `finally`; it does
not use or restart the production listeners.
