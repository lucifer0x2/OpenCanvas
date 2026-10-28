# Design token resolution (M3)

The authored `Token` Rust DTO and `value: serde_json::Value` remain compatible.
`type` is authoritative: `color`, `dimension`, or `number`. Supported `value`:

```json
"#112233"
```

```json
{"aliasId":"brand"}
```

```json
{"default":{"aliasId":"brand"},"themes":{"dark":"#eeeeee"}}
```

An envelope requires exactly `default` and `themes`. Each branch is a literal or
an exact `aliasId` object; nested envelopes and other bindings are rejected.
Aliases target existing tokens of the same type. Ordered batch operations do not
permit forward references. Primitive fill/text styles may bind these tokens;
style-to-style aliases remain unsupported.

Concrete themes are `default` plus the union of names declared by all tokens.
A token without an override for the requested concrete theme uses its default
branch. Alias chains resolve each target using that same concrete theme.
Unrecognized themes return structured `UNSUPPORTED_THEME` with field `theme`.
The default theme is available even in an empty project. UI theme is separate.

Limits: 4,096 tokens/project, 32 named themes/project and per token, 32 alias
edges/path, 256-byte token names. Theme names contain 1–64 ASCII letters, digits,
underscores or hyphens; `default` is reserved and cannot be an override name.
Literal ranges and supported node/style bindings retain existing storage limits.

Before committing, validation checks every branch of every token, including
unused tokens/themes. Missing targets, type mismatch, malformed shapes, cycles
and excessive depth fail atomically. The alias graph is the union of all theme
branches, so cross-theme cycles are rejected even if one selected theme would
not loop. Every concrete theme also passes node/style property validation, so a
valid number token cannot create an invalid themed opacity or font size.
Graph recursion is bounded; selected-theme lookup uses an iterative cycle guard.

Public API, exported by `opencanvas-storage`:

```rust
resolved_nodes(&Snapshot) -> Result<Vec<Node>> // default compatibility wrapper
resolved_nodes_for_theme(&Snapshot, &str) -> Result<Vec<Node>>
resolved_tokens_for_theme(&Snapshot, &str)
    -> Result<BTreeMap<String, serde_json::Value>>
```

The APIs leave snapshots and authored references unchanged. Resolved nodes use
literal properties and retain the existing public `fit` to renderer `fill`
conversion. Token readback returns literals keyed by token ID at the snapshot's
revision. A token update follows the existing atomic revision/idempotency path;
callers must include revision and concrete theme in scene/cache keys. Scene jobs
and design context must call the theme-aware APIs rather than silently default.

## Verification — 2026-10-28 resumed run

Actual checks on the shared working tree, using temporary storage roots:

| Command | Result |
| --- | --- |
| `cargo test -p opencanvas-storage --offline --test resolution -- --test-threads=2` | 9 passed, 0 failed, 0.25s |
| `cargo test -p opencanvas-storage --offline --test references --test style_aliases -- --test-threads=2` | 5 passed across 2 suites, 0.17s |
| `node --test --test-concurrency=1 spikes/pipeline/constraints.test.mjs` | 2 passed, 0 failed/skipped, 505.48ms, real Chromium |
| `rustfmt --edition 2021 crates/storage/tests/resolution.rs` | Success; includes its three test modules |
| `npx prettier --write spikes/pipeline/constraints-fixture.mjs spikes/pipeline/constraints.test.mjs` | Success |

The Node fixture first encountered sandbox `listen EPERM` on loopback. The same
targeted command passed with approved sandbox escalation; no mocked browser or
skipped assertion was substituted. Runtime: Node v24.21.0, Chromium 156.0.8078.4,
bundled Noto Sans SHA-256
`bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d`.
No production resolver fix was necessary on resume.

The resolution suite covers multihop default/dark aliases, per-token fallback,
authored/reference immutability, dependent fill/text styles and node overrides,
revision/idempotent replay, missing/type/forward references, unused/cross-theme
cycles, corrupt snapshot cycle guards and structured unknown-theme errors on
both APIs. Invalid node/style ranges and malformed unused branches reject without
a durable commit. Boundaries tested: 32 alias edges accepted/33 rejected;
32 project themes accepted/33 rejected, including their union across tokens;
4,096 tokens accepted/4,097 rejected by snapshot readback validation; names and
malformed envelopes rejected. The 4,096-token mutation workload is not separately
load-tested. Existing literal resolution and style-alias rejection remain green.

The focused [layout fixture](../spikes/pipeline/constraints-fixture.mjs) and
[assertions](../spikes/pipeline/constraints.test.mjs) supplement M0's width/wrap,
cache, clip/crop and resource tests. On a 360×320 frame, exact nested bounds are:
row `(10,10,340,90)`, fixed child `(16,16,80,30)`, max-width fill column
`(104,16,200,78)`, min-height/max-width text `(108,20,150,70)`, footer
`(10,108,40,18)`. These prove fixed/fill/hug, min/max, scalar padding and gap.
Text `Một\nHai` uses 28px line height: local baselines 22/50 and widths
29.96875/24.953125. The exact line list is asserted, and computation leaves all
authored input unchanged. A 120px child in an 80px container retains its extent; exact RGBA
samples prove overflow paints outside only when clip is disabled. Generated
scene evidence is `spikes/pipeline/output/constraints.scene.json` (ignored and
recreated by the test).

All files changed for this resumed task are below 250 lines; the formerly
303-line contract test is split into scenario, limits, safety and style modules.
The M3 production validator modules remain under 250 lines (maximum 199 among
the M3-modified modules). No DB/store/server/layout/renderer implementation was
changed by the resumed worker.

This verifies the resolver and focused layout seam. Server pagination/context,
revision/theme cache invalidation and live viewer integration are owned by the
server/E2E tasks and are not acceptance claims from these tests. Main will run
the full storage suite after M4 stabilizes. Unsupported: style aliases, nested
token envelopes, forward aliases, undeclared themes, percentage/baseline/rich-text
layout and ambiguous fill in a hug axis; existing layout limits remain in the
[pipeline contract](../spikes/pipeline/README.md).
