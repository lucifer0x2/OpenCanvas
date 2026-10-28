# Task06 — `find_nodes` and the shared tree index

Completed by the Task06 worker, 2026-10-28. Shared workspace, branch `main`; no
commit, reset, checkout, push, subagent, new dependency, or change to storage, layout,
web, `PLAN.md` or `AGENTS.md`. Port 3847 and `~/.opencanvas` were not touched: every
check used a temporary data root and a random loopback port.

## What changed

- `crates/server/src/reads/index.rs` (new, 77 lines): `TreeIndex` builds parent→children
  and id→node maps in one pass. Siblings keep storage order, then a stable
  `sort_by_key(sort_key)`, the same rule as the old `visit()`. It provides `get`,
  iterative `preorder(root)` and `ancestors(id, scope)`.
- `reads/tree.rs` (152 → 128 lines): `visit()` (O(n²)) and the per-node
  `resolved.iter().find` (also O(n²)) are replaced by the index and an id map.
  Responses are unchanged byte for byte (see below).
- `reads/find.rs` (new, 177 lines): criteria, matching, match entries, paging.
- `reads.rs`: three new `ReadRequest` fields (`kinds`, `nameContains`, `textContains`),
  each `#[serde(default, skip_serializing_if = "Option::is_none")]`, plus the
  `find_nodes` branch. `schemas.rs`: the tool schema. `mcp.rs`: one dispatch name.
- `instructions.rs`, both skills and `docs/SERVER.md` (section "Finding nodes").

## Request and response, as shipped

```text
request   projectId (required); nodeId | frameId (exclusive; frameId must be a frame);
          kinds[1–6 of page|frame|group|rectangle|text|image]; nameContains 1–128;
          textContains 1–256; ids[1–200]; properties[≤20 keys];
          limit 1–200 (default 50); budget 1024–65536 (default 65536); cursor; revision.
          additionalProperties:false: no depth, theme or viewport.
response  schemaVersion, projectId, revision, scope, criteria, scannedNodes, totalMatches,
          matches[], truncated, nextCursor, limits{nodes, bytes}
match     id, kind, name, parentId, depth, path[{id,kind,name}], pathTruncated?,
          matchedOn["name"|"text"|"kind"|"id"], textPreview?, props?
```

If no criterion is given the tool returns `INVALID_REQUEST`. Criteria are ANDed;
`kinds` is OR within its list. Matching runs `str::to_lowercase` on both sides and
then a substring test. There is no accent folding or NFC/NFD normalization, so
"Tong quan" returns `totalMatches:0`; the tool description and SERVER.md say so.

## Decisions and deviations from the task file

1. **Response key rename.** `pagination::page` (left unchanged as the task requires)
   caps every array except `nodes` at 100 items per page. With a `matches` key,
   `limit:200` would silently stop at 100. So matches page under `nodes`, and the key
   is renamed to `matches` afterwards. `budget - 2` is passed so the 2-byte longer key
   still fits the caller's budget. This is explained in a comment at `RENAME_BYTES`.
2. **`criteria.ids` is echoed as `idCount`.** Echoing 200 IDs of up to 128 characters
   would repeat ~26 KB on every page and fail small budgets.
3. **`textPreview`** shows up to 120 characters (counted as chars, not bytes), placed
   around the first match. It starts up to 30 characters before the match and is
   clamped to the end of the text. Without this, a match past character 120 would
   not appear in its own preview.
4. **`depth`** counts from the scope root; a page has depth 0 for project scope.
   **`path`** goes from the scope root to the parent and keeps the 8 nearest ancestors.
5. **Tool count assertions.** `tests/transport.rs` and `tests/skill_layer.rs` assert
   the advertised tool count. Both changed 13 → 14 (one line each); they are outside
   the ownership list, but the existing suite fails without the change.
6. **Files outside the ownership list.** The byte-identity test needs `tree::read`,
   which is private, so it is a crate unit test: `src/reads/traversal_tests.rs`
   (195 lines) with golden `tests/fixtures/traversal_golden.jsonl`. `find_nodes.rs`
   reached 276 lines after `cargo fmt`, so its fixture and helpers moved to
   `tests/find_support/mod.rs` (66 lines).
7. `spikes/agent-cost/scenarios/s1-find-text.mjs` gains a `findNodes` strategy,
   with the coordinator's permission.

## Byte-identical outline/subtree/context

The golden file was **written by the old `visit()` code** before `tree.rs` changed
(`OPENCANVAS_WRITE_GOLDEN=1`, sha256 `380b17b5…67cc1`). After the refactor the file is
unchanged and `outline_subtree_and_context_bytes_match_golden` passes. The fixture is a
validated stored design whose storage order is reversed, so every parent comes after
its children. It also has duplicate sortKeys among root pages, among frame children
and among nested children. The golden holds 29 pages of serialized output, cursors
included, from 11 reads:

- `get_outline` with and without scope, at depth 0 and 1, and with `limit:2`
  continuations
- `get_subtree` with a properties filter and with depth/limit
- `get_design_context` with the default and `night` themes, with a 2 KiB budget plus
  computed bounds, and with a stack frame plus `reflow:false` (`UNCHANGED_BOUNDS`)

The cursors embed the scope hash, so the golden also shows that the new
`skip_serializing_if` fields leave the old tools' hashes unchanged.
`index_preorder_matches_legacy_visit_for_every_scope` compares the index with a
copy of the old algorithm for every node as a root.

At harness level, S1, S5 and S6 response bytes are identical before and after
(85,986 / 925,442 / 2,592,867 B).

## Bytes on the stacks sample (real MCP, production binary, temp root)

Searching `textContains:"Tổng quan"` returns `overview-nav` (text, depth 7 in frame
scope), with a path of 7 ancestors from `welcome-dashboard-stacks`, in one call:

| Scope | find_nodes structured / JSON-RPC response | full get_outline structured / response (pages) |
| --- | ---: | ---: |
| frame `welcome-dashboard-stacks` (181 nodes scanned) | 911 B / 2,078 B | 17,787 B / 38,968 B (1) |
| project (182 nodes scanned) | 954 B / 2,174 B | 18,161 B / 39,866 B (2) |

`kinds:["frame"]` on the same project returns the frame in one call. Error codes
checked over MCP: `INVALID_REQUEST` (no criteria, both scopes, kind `button`) and
`REVISION_UNAVAILABLE`.

## Agent-cost harness (`compare.mjs`)

`before-06` is a same-machine run of the pre-change binary that I made first; the
coordinator's `baseline.json` predates it.
`node spikes/agent-cost/compare.mjs baseline after-06-find` (S1 with `--opt s1.strategy=findNodes`):

| S1 | baseline (outline → subtree) | findNodes (find_nodes → get_subtree depth 0) |
| --- | ---: | ---: |
| calls | 4 | 3 |
| bytes (req+resp) | 85,986 | 4,239 (−95.1%) |
| structuredBytes | 38,782 | 1,605 (−95.9%) |
| callMs | 74.19 | 10.88 (−85.3%) |

`compare.mjs baseline after-06` (default strategies, the same reads):

- S1: bytes are unchanged; latency moved by less than 1 ms.
- S5: bytes are unchanged; one-project p50 is 20.98 → 19.61 ms.
- S6: bytes are unchanged.

`tools/list` grew from 15,167 → 17,174 B (+2,007 B per MCP session). `initialize`
grew from 2,736 → 3,068 B (+332 B for the instructions). The task estimated ~1 KB;
about half the new entry is the input schema.

## 5,000-node latency (harness S6 fixture, debug build, client wall time over loopback)

| Read | before-06 p50 / p95 | after-06 p50 / p95 |
| --- | ---: | ---: |
| get_outline page (25 pages, all 5,000 nodes) | 310.2 / 332.1 ms | 89.1 / 98.4 ms |
| get_design_context page of frame-a (13 pages) | 483.6 / 520.1 ms | 381.2 / 480.0 ms |

find_nodes after the change, measured on the same fixture: 1 warm-up and 20 samples
each, scanning 5,000 nodes. The script is in the session scratchpad
(`measure/find-large.mjs`, using the harness `env`/`seed`/`meter` modules). Cleanup
was true for both server stop and temp-root removal.

| Query | totalMatches | returned | structured B | p50 / p95 ms |
| --- | ---: | ---: | ---: | ---: |
| `kinds:["frame"]` | 2 | 2 | 557 | 25.3 / 25.5 |
| `textContains:"tiếng việt"` (limit 50) | 1,662 | 50 | 13,664 | 48.6 / 48.8 |
| same, `limit:200` | 1,662 | 200 | 53,601 | 185.9 / 186.5 |
| `frameId` + `nameContains` + `kinds:["rectangle"]` | 1,668 | 50 | 8,506 | 39.5 / 39.9 |
| `ids` (200) | 200 | 200 | 41,460 | 149.6 / 155.1 |
| no match | 0 | 0 | 259 | 26.1 / 26.4 |

Every read stays inside the published large-snapshot p95 of 500 ms. The 200-item pages
are dominated by `pagination::page` serializing the whole response after each pushed
item, which is existing behavior shared with the outline. Before this change the
context p95 was 520 ms on this machine.

## Commands run

| Command | Result |
| --- | --- |
| `cargo test -p opencanvas-server --offline` | pass: 26 tests (2 unit, 24 integration across 10 test binaries), 0 failed |
| `cargo clippy -p opencanvas-server --offline -- -D warnings` | pass |
| `cargo fmt --all --check` | pass |
| `git diff --check` | pass; new untracked files checked separately for trailing whitespace: none |
| `node spikes/agent-cost/run.mjs --label before-06 --only s1,s5,s6` (pre-change binary) | 3 scenarios OK |
| `node spikes/agent-cost/run.mjs --label after-06 --only s1,s5,s6` | 3 scenarios OK |
| `node spikes/agent-cost/run.mjs --label after-06-find --only s1 --opt s1.strategy=findNodes` | OK, 3 calls |
| `node spikes/agent-cost/compare.mjs baseline after-06`, `before-06 after-06`, `baseline after-06-find` | numbers above |
| `node samples/welcome-stacks/verify.mjs` | exit 0, `success:true`, `cleanup:true`, desktop 0 changed pixels. It rewrites the gitignored `artifacts/sample-stacks/*` QA files, not `live/`. |
| scratch real-MCP stacks check and 5,000-node find_nodes measurement | numbers above; cleanup true |

`cargo clippy --all-targets` still reports lints that predate this task, in
`tests/support`, `tests/multi_support` and `tests/render.rs`. The new test binary adds
none: it uses `#[allow(dead_code)]` on its `multi_support` import.

## Limits and open points

- There is no accent folding or Unicode normalization, and position or bounds cannot
  be searched. Both are documented as known limits; neither is a bug.
- The concrete numbers come from a debug build on one machine.
- If Task07D adds `get_changes`, the two tool-count assertions move to 15.
