# Implement an OpenCanvas frame

1. Choose the frame: `find_nodes` with `kinds:["frame"]`, a `nameContains` or a
   `textContains` when you know what to look for; `get_outline` when you need the
   structure. Then read `get_design_context` for that frame. Follow every
   `nextCursor` using identical parameters; one page is not the whole design.
2. Keep authored `props` distinct from `resolvedProps`. Map token/style IDs to named
   code tokens; resolved values describe the requested theme at this revision.
   `compact:true` makes the read smaller: with `resolvedPropsMode:"differing"` a
   key missing from `resolvedProps` resolves to its authored value (`styleId` itself
   is never resolved), and with `resources:"referenced"` the token/style/font/asset
   lists hold only what this frame uses, alias and theme targets included. Use
   `list_tokens` when you need the rest of the design system.
3. Read `diagnostics` before choosing a layout. `NO_REFLOW_INFORMATION` means the
   whole authored scope uses `free` or its default. Its `authoredSize` is the frame's
   authored width/height, not the requested viewport. Unchanged bounds at another
   viewport can mean cropping, not responsive behavior. State proposed responsive
   assumptions explicitly or ask the user. Do not silently deliver absolute
   positioning as a complete responsive translation.

## Structure and sizing

| Authored rule | Implementation starting point |
| --- | --- |
| `layout.mode: "row"` | `display:flex; flex-direction:row` |
| `layout.mode: "column"` | `display:flex; flex-direction:column` |
| Row `layout.wrap:true` | `flex-wrap:wrap; align-content:flex-start; align-items:flex-start` |
| `layout.padding`, `layout.gap` | Explicit padding and gap |
| Numeric width/height | Fixed dimension in the design's logical pixels |
| `fill` on a stack's main axis | Flexible share of remaining space (`flex:1 1 0`) |
| `fill` on the cross axis | Stretch to the available cross size |
| `hug` | Content-driven size; do not freeze the measured bound |
| `minWidth`/`maxWidth`, `minHeight`/`maxHeight` | Corresponding size constraints |
| Children of `free` | Authored x/y relative to that parent; useful for overlays |

These are starting rules, not a CSS equivalence guarantee. The engine defaults to
start alignment and does not shrink fixed siblings. Respect explicit fixed sizes
and test fill/min/max combinations rather than assuming browser flex defaults.
In particular, text with `height:"hug"` grows as its authored width changes and
lines wrap. Keep its height automatic; a measured line count is not a height rule.
Wrap is opt-in (missing/false means nowrap). Preserve positive `minWidth` on each
direct fill child of a wrapping row and the row's definite width/fill chain.
Gap applies both across and between lines; fill cards share each line separately,
so the final card may take the full line. Hug row height sums each line's maximum
height plus gaps/padding. Fixed row heights and oversized minima may overflow.

## Text and verification

- Load the declared font and use the authored font size and line height. Read
  `list_fonts`, `list_styles`, and `list_tokens` when mapping the design system.
- Text node y is its box position, not a browser baseline. Computed line positions
  come from font metrics. Avoid copying line y values into CSS or offsetting text
  by eye to compensate for an unloaded/different font.
- Render at the same revision, viewport, theme and state; compare `render_frame`
  pixels with `inspect_layout` bounds/text lines for that scene key. Wait for a
  pending job with `render_frame {jobId, waitMs}` (up to 10000) instead of
  polling in a tight loop; it returns as soon as the job is ready or failed. Use a
  second viewport to check declared stacks. Their presence alone does not promise that
  the entire design fits every width.
- Check horizontal overflow and content wrapping. Row wrap is available; column
  wrap, percent dimensions and media-query breakpoints remain unsupported. Ask about required behavior when the
  authored rules cannot express it; do not invent design intent.
- Interaction, routing, animation and real data require user requirements. Only
  the default interaction state is supported; named token themes may be requested.

## Finding nodes

- `find_nodes` ANDs `kinds`, `nameContains`, `textContains` and `ids`; at least one
  is required. Matching is case-insensitive substring without accent folding, so
  "Tong quan" does not find "Tổng quan"; search with the accents the design uses.
- `totalMatches: 0` with `matches: []` means nothing matched; it is not an error.
  `truncated: true` with `nextCursor` means more pages of matches.
- Position ("the card on the right") is not searchable. Narrow candidates by name,
  text or kind, then compare bounds from `get_design_context` `viewports` or
  `inspect_layout` `nodeIds`.
