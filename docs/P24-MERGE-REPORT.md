# P24 merge of P27 interaction states

Date: 2026-10-29. Branch: `codex/p24-components-instances`.
Merge parents: P24 `236d11627a920fdeabb4a274d683a4263112f432` and
`origin/main` / P27 `f901a313998cf3566615b967c8950cbc0acfbd3c`.
The requested operation is a merge, with no rebase. This report describes the
resolved working tree before the coordinating agent creates the merge commit.

## Resolution and integration behavior

The five conflicts retain both feature contracts: instance/reference schema and
`statesMode`, request normalization/cursor identity, instance authoring docs and
state docs, boolean visibility validation and visual state validation. Database
schema remains P24's version 4; scene/API schema remains version 1.

Compute input continues to call `resolved_expanded_nodes`: master root states
survive on placed instance roots, descendant states survive on virtual children,
and state token/style references resolve after expansion at the requested theme.
Compact context resource collection closes over expanded nodes and definitions,
including resources referenced only by states. Instance and descendant override
whitelists remain unchanged; neither accepts authored state overrides.

Two integration seams needed explicit fixes:

- Reference component definitions now include differing `resolvedStates` under
  the same opt-in mode/property filter as scoped nodes. Default mode remains
  unchanged; expanded context already exposes the virtual nodes' variants.
- The post-layout state painter preserves `visible:false` opacity zero even if
  that node's selected state declares opacity 1. State previews keep layout
  bounds and text lines unchanged.

The new `instances_states` regressions cover reference/expand, both themes,
compact state-only token resources, differing/default text fill behavior,
mode/filter omission, rejection of new state override keys, real PNG rendering,
scene/cache identity, hidden virtual children and geometry preservation.

P27's `SERVER-OPERATIONS.md` split is preserved without duplicated sections.
Its database-schema note is updated from 3 to P24's version 4.
P24's component contract moved to [COMPONENTS.md](COMPONENTS.md), linked from
`SERVER.md`, `CONTRACT.md` and `INTERACTION-STATES.md`. The historical P24 report's
556-line server-guide size describes the earlier tree; the merged guide is 347.

## Verification

All Cargo commands below used `CARGO_TARGET_DIR=/private/tmp/opencanvas-p24-target`.
Render tests also used the installed isolated Chromium via
`PLAYWRIGHT_BROWSERS_PATH=/Users/luis/Workspace/OpenCanvas/.runtime/browsers`.
Loopback servers and authored data roots were temporary test fixtures.

```sh
rtk bypass cargo test --offline -p opencanvas-storage --test instances \
  --test instances_boundaries --test instances_legacy --test migration \
  --test states -j 2 -- --test-threads=2
# 16 passed: 4 + 3 + 2 + 4 + 3

rtk bypass cargo test --offline -p opencanvas-server --test instances_states \
  --test instances_mcp --test instances_resources --test states_context \
  --test states_render --test tool_schemas --test tools_budget \
  -j 2 -- --test-threads=1 --nocapture
# 13 passed: 2 + 3 + 1 + 3 + 1 + 2 + 1

rtk bypass cargo test --offline -p opencanvas-server --lib \
  -j 2 -- --test-threads=2
# 35 passed, including traversal golden bytes

rtk bypass cargo clippy --offline -p opencanvas-storage -p opencanvas-server \
  --lib -j 2 -- -D warnings
cargo fmt --all --check
rtk bypass git diff --check
wc -l -c AGENTS.md
# passed; AGENTS.md 64 lines / 7,861 bytes
```

The coordinator independently reran server lib + instance MCP + new integration
tests + P27 context/render tests: 44/44 passed with actual Chromium. That rerun
is separate evidence and is not counted again in the worker's 64 tests above.
The coordinator also independently reran all 16 targeted storage tests: passed.

Existing P24 checks still measure 19,951 → 4,964 bytes of authored operations,
8,694 → 33,386 bytes reference versus expanded context, byte-identical PNGs
against manual nodes, all 20 virtual cells updating after one master edit, and
instance/virtual-child exports ready. Existing P27 pixel confinement, no-layout-
shift, undeclared-state diagnostics and cache tests remain green.

The coordinator's isolated `f901a31` catalog baseline is 25,066 bytes for compact
`{tools}` JSON with 16 tools. The merged catalog is 25,161 bytes: **+95 bytes**, no
new tool. `tools_budget` prints the array-only 25,151 bytes; the enclosing
`{tools}` adds 10 bytes. The former 24,871-byte pre-P27 baseline is historical.

## Module sizes and limits

Focused merge modules: `reads/tree.rs` 192 lines, `reads/instances.rs` 98,
`reads/states.rs` 44, `jobs/states.rs` 60; new regression file 168 lines.
The unchanged instance expander is 120 lines. Rust files touched by this merge
remain at or below 250 lines. New component guide: 37 lines; interaction guide:
81; server guide: 347; server operations: 199. The two server guides remain
reference documents, with responsibilities separated by topic.

No full workspace suite, large performance rerun, viewer E2E or JS build was
needed for this focused Rust merge; renderer/viewer sources did not change.
No push, merge continuation, server-3847 restart or real-data mutation was
performed by the merge worker. Temporary dependency/runtime symlinks are for
verification only and must be removed by the coordinator before committing.
