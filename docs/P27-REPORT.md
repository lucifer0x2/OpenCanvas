# P27a+b — flat interaction states

Implemented on `codex/p27-interaction-states`, based on `a0d7743`. No commit,
push, checkout/reset/stash, production server restart or real data write was
performed. Fixtures use temporary storage roots and `127.0.0.1:0` listeners.

## Contract and architectural choices

The supported contract is [INTERACTION-STATES.md](INTERACTION-STATES.md).
The actual catalog has `stroke:{color,width?,align?}` and text color `fill`.
There is no base `fontWeight` or shadow property. State allowlists therefore
accept fill/opacity/radius and stroke on box kinds, rejecting those nonexistent
properties and all layout/navigation/animation keys. This avoids introducing
typographic layout changes through state overrides.

Storage's focused `validate/states.rs` handles state shape/key/range validation
and typed reference resolution. Candidate validation already walks every named
theme, so it also checks dependent token/style updates in otherwise unused
state/theme branches. Authored snapshots remain intact.

`reads/states.rs` computes differing resolved variants only on explicit context
opt-in. `statesMode:none` normalizes to absence before cursor hashing; resolved
mode remains in scope identity. Existing resource collection recursively finds
refs in states, including refs excluded by the caller's property filter.

`jobs/states.rs` applies resolved paint values after Rust layout and Chromium text
measurement, before strict PNG rendering. All bounds/lines remain resting values,
even for state inside stroke width changes. State stroke replaces the whole
object; only the resting border reserves layout space. Each node uses only its
own declaration. The input already includes state in cache identity; the scene
and PNG provenance retain it. Missing declarations produce `STATE_NOT_DECLARED`.

P07a requires explicit keyframes and has no `to` field. Hover requires exactly
one keyframe. That schema stays intact: no undocumented implicit state target or
missing-keyframe fallback was introduced. Static renders ignore animations and
show the chosen interaction variant. The optional viewer picker was omitted.

## AC evidence

| AC | Evidence |
| --- | --- |
| 1 | Storage `states_resolve_themed_token_style_and_stroke_refs_without_changing_authored` and server context test update a valid token state, increment revision and read authored refs unchanged. |
| 2 | Storage and MCP tests reject unknown state/width keys with full paths and allowedKeys, then verify unchanged revision. Storage additionally covers nested states, unsupported fontWeight, bad ranges/refs, page/text scope and empty-object clearing. |
| 3 | Server context tests resolve night fill to `#abcdef`, omit resolvedStates on nodes without states, and test differing/default/compact/filter semantics and referenced resources. Omitted and explicit none mode produce identical serialized bytes and cursor; mode mismatch rejects. Root also passed the existing `outline_subtree_and_context_bytes_match_golden` test against the unchanged pre-P27 `traversal_golden.jsonl`, including default/night, filtered, paginated and computed context fixtures. |
| 4 | Real Chromium test decodes RGBA default/hover PNGs, requires at least one changed pixel, and asserts every changed pixel lies inside the hover button bbox. Undeclared disabled RGBA equals default and includes STATE_NOT_DECLARED. |
| 5 | Same-revision/viewport state jobs have different scene keys; repeated hover returns the same job ID. |
| 6 | Every named state has exact default scene bounds and text lines; inspect_layout node results match exactly. Focus changes inside stroke widths from 1 to [8,4,6,2] without moving the text or sibling. Hover leaves undeclared child fill unchanged; pressed changes both declaring button and sibling. |
| 7 | Instructions and implementation skill describe state mapping and whole-frame preview. Tools/list measured 24,909 baseline vs 25,104 current bytes: +195, within 400. Test reconstructs the baseline by reversing only the two schema changes. Root also passed the existing catalog budget test (25,056 bytes for the compact tools array measured by that test). |
| 8 | Every changed/new file is below 350 lines; largest is SERVER.md at 344. Its operations sections moved without removal to SERVER-OPERATIONS.md (199 lines). Production Rust files are all below 250. |

## Commands actually run

```sh
cargo test --offline -p opencanvas-storage --test states -- --test-threads=2
# 3 passed (initial targeted run)

rtk bypass cargo test -p opencanvas-server --test states_context \
  --test states_render --test tool_schemas -- --test-threads=1 --nocapture
# 3 context + 1 real Chromium render + 2 schema tests passed

rtk bypass cargo test --offline -p opencanvas-server --test states_context \
  --test states_render --test tool_schemas -- --test-threads=1 --nocapture
# final run after multi-node/default comparison assertions: same 6 passed

rtk bypass cargo test --offline -p opencanvas-storage --test states \
  --test references --test stroke_props --test prop_catalog -- --test-threads=2
# final: 3 states + 3 refs + 6 stroke + 3 catalog = 15 passed

rtk bypass cargo clippy --offline -p opencanvas-storage -p opencanvas-server \
  --lib -- -D warnings
# passed, including after the page key-validation correction

cargo fmt -p opencanvas-storage -p opencanvas-server --check
git diff --check
# passed
```

Initial offline server tests stopped before execution because several already
locked dev crates were missing locally. A normal retry could not resolve
static.crates.io inside the sandbox; the approved escalated retry downloaded the
locked dependencies and ran the loopback/Chromium checks. Cargo manifests and
lockfile were unchanged. One added regression run found `states:null` on page
resolving before key validation (missing allowedKeys). Resolution now leaves page
states to the existing catalog key gate; the full targeted catalog regression
passes. Initial unused-import/helper warnings were removed from the new tests.

## Independent root verification

Root ran these checks separately from the implementation worker:

```sh
cargo test --offline -p opencanvas-storage --test states \
  --test references --test style_aliases -- --test-threads=2
# 8 passed

rtk bypass cargo test --offline -p opencanvas-storage --test states \
  --test prop_catalog --test stroke_props -- --test-threads=2
# 12 passed after the final page key-validation correction

rtk bypass cargo test --offline -p opencanvas-server --test states_context \
  --test states_render --test tool_schemas --test compact_context --test render \
  -- --test-threads=1
# 13 passed; approved loopback run after the sandbox denied bind with EPERM

rtk bypass cargo test --offline -p opencanvas-server --test tools_budget \
  -- --test-threads=1 --nocapture
# 1 passed; 16 tools, 25,056 bytes

rtk bypass cargo test --offline -p opencanvas-server --lib -- --test-threads=2
# 35 passed, including the unchanged traversal golden byte comparisons

rtk bypass cargo clippy --offline -p opencanvas-storage -p opencanvas-server \
  --lib --bin opencanvas-server -- -D warnings
cargo fmt --all --check
git diff --check
# passed
```

The golden comparison uses committed pre-P27 expected bytes, not a second
baseline backend process. One root storage invocation named nonexistent test
target `stroke`; it executed no tests and was corrected to `stroke_props` above.

## Module sizes and remaining scope

New files: storage validator 104 lines, storage tests 194; server context helper
36, job paint helper 53, context tests 144, real render tests 121. Existing
modified Rust/skill files are 22–216 lines. The server guide split preserves all
existing sections and links to both operations and the focused state contract.
No renderer/worker/viewer or TS files changed, so no JS build/typecheck was needed.

Outside scope: per-node state targets, inherited/combined states, pointer-driven
viewer activation, implicit animation targets and layout-changing states.
Outside/center strokes retain their normal ability to paint outside bounds.
Mixed-version backends, hot pipeline replacement and arbitrary history remain
unsupported. No full workspace suite/build or viewer browser E2E was run.
