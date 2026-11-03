# OpenCanvas: proposal to make instances carry a full colour scheme

Source: U07-test (project `p_74bc09524dc44cd99cc1c33fa6bad25c`, page `u07-test-page`, rev 206).
For the session that implements this in the OpenCanvas MCP. After it lands, the U07 design worker rebuilds U07-test with the new API and re-measures.

## 1. Evidence

Four colour-scheme tiles (`u07-a1..a4-tile`) share one 784-node tree; only colours, a few labels and 6 font weights differ.

| Build | Ops | Payload |
| --- | --- | --- |
| v2: four full trees | 3 140 | 652.8 KB |
| U07-test: master (A1) + 3 instances | 792 | 224.4 KB (master 163.7 KB, each instance ~20 KB, 455–509 override entries) |

All four test renders are clean (no diagnostics). But on 57 nodes per instance, the instances cannot reproduce their scheme, because `overrides` accepts only `text, fill, visible, styleId, assetId, name`:

| Prop that differs per scheme | Nodes | What it is |
| --- | --- | --- |
| `stroke` (colour only, same width) | 37 | hairline borders, input border / focus / error, tab underline, radio rings, banner left bars (`width:[0,0,0,4]`), focus ring, outline button |
| `shadow` (colour only, same geometry) | 14 | cards and elevation samples, shadow tinted with each scheme's ink colour |
| `states` (fill / stroke colours) | 5 | button hover/pressed, focus ring, input focus |
| `layout.padding` | 1 | danger button 19 vs 20 px (A2 has a 1 px outline, the others are filled) |
| master root `fill` | 1 | tile background. Worked around with an extra "paper" child, because the root cannot be overridden |

`fontId` (6 nodes) already works: text styles plus a `styleId` override (validated at rev 204).

Workflow costs seen along the way:
- `apply_batch` is capped at 500 ops, and payloads are 70–160 KB. Sending that through the agent's tool call means the model has to type every byte. That is why the U07 builds go through a local script client instead of the session's MCP tools.
- There is no dry run: checking whether a fill style can hold `stroke` took a real write (it fails with `fill.type`: a fill style object is only a gradient).

## 2. Proposals (priority order)

### P1. Theme scoped to a subtree or instance (the root fix)
Add `theme` (string; a theme name from the tokens) to `frame` and `instance` props. Token refs inside that subtree resolve with that theme; the nearest ancestor wins, and render/export `theme` remains the global default. This works like variable modes on frames in Figma.
- With it, the master is authored once with colour tokens in `fill`, `stroke.color`, `shadow[].color` and `states.*`, all of which already accept `{tokenId}`. Each instance is then `{componentId, theme:"a3"}` plus about 88 `text` overrides. That would leave about 4 KB per scheme instead of 20 KB, and nothing lost.
- Validation: the theme must exist on at least one token, otherwise `UNKNOWN_THEME`. `get_design_context` resolvedProps and `render_frame` must resolve per subtree. Add the effective theme to scene nodes so that inspect_layout can report it.

### P2. Wider override allowlist (paint, not structure)
Allow `stroke`, `shadow` and `states` in `overrides`, validated exactly like the authored prop for the target kind.
- `stroke`: a partial merge (`{color}` alone keeps the master width and align).
- `shadow`: a whole-array replace; layer-count changes are allowed.
- `states`: a per-state merge (`{hover:{fill}}` keeps the master's `pressed`/`focus`).
- Optionally `opacity` and `radius`. Keep `layout`, sizing, children and `component` non-overridable. P1 covers the colour case more cheaply; P2 covers one-off tweaks.

### P3. Paint overrides on the instance itself
Accept `fill`, `stroke`, `shadow`, `radius` and `opacity` on the `instance` node, applied to the expanded master root. This removes the "paper" wrapper workaround, since the root currently cannot be an override target.

### P4. Bulk input without the model typing the payload
Add `apply_batch_file {projectId, expectedRevision, idempotencyKey, path}` (or `operationsPath` on `apply_batch`). It reads a JSON op array from an allowed directory only (`<data-root>/<project>/inbox/`), rejects paths that leave that directory (symlinks, `..`), and has the same validation, atomicity, idempotency and 500-op chunking semantics (or allows several files in order). The batch then stays a visible, approvable tool call in the session while a generator writes the ops.

### P5. `dryRun: true` on `apply_batch`
This runs full validation and returns the would-be changedIds and diagnostics without committing. It lets an agent probe a prop shape (for example a style value) without a real write.

### P6 (optional). Server-side contrast check
`check_contrast {projectId, frameId, theme?, state?}` would return, per text node, its resolved fill against the nearest opaque ancestor fill (instances expanded), the WCAG ratio, and whether it passes as large or normal text. Adding listed UI pairs is optional. This replaces the client-side `contrast2.mjs`, which today reads the generated ops rather than the canvas.

## 3. Acceptance

1. U07-test rebuilt with P1: one master with tokens and 4 instances using `theme`. Each instance's render is pixel-identical (fuzz ≤ 3 %) to `.company/design/export/U07/A1–A4.png`, and each instance has only `text`/`styleId` overrides.
2. P2: an instance overriding `stroke.color` of `in1-in` keeps the master width 2. Overriding `states.hover.fill` of `b-primary` keeps the master `focus` ring. Invalid values fail with `VALIDATION_FAILED` and `field: overrides.<id>.stroke.color`.
3. P3: an instance `fill` paints the expanded root. `get_design_context instances:"expand"` shows it in resolvedProps.
4. P4: a path outside the inbox fails with a clear error code. The same body with the same key replays idempotently.
5. Docs: update `skill://opencanvas/props.md` (Components and instances, the theme key) and `opencanvas://capabilities`. Bump the catalog fingerprint.

## 4. Current state of the test (for the follow-up)
- Page `u07-test-page` holds master `u07t-m-tile` (component "U07 colour tile", with a `u07t-m-paper` child carrying the page fill) and instances `u07t-a2/a3/a4`.
- Two text styles, `u07t-scale-500` and `u07t-scale-700`.
- Generator: `.company/design/work/U07/scripts/test/{ops,emit,send}.mjs`.
- `u07-page` and other pages are untouched.
