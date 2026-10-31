# Implement an OpenCanvas frame

## Design context memory

Before reading or inspecting the design, follow the project design memory checklist
in `skill://opencanvas/author-frame.md`. Read the selected project's
`<configured-data-root>/<project-slug>/AGENTS.md` (default
`~/.opencanvas/<project>/AGENTS.md`) only through authorized client filesystem access.
Prefer it over session-private design memory for understanding design intent, subject
to user and higher-priority directives. It governs design context only, not the code,
build, test or repository work used to implement that design. MCP has no project-memory
read/write tool: explain unavailable access and never claim a read, initialization or
save that did not happen. Do not create another project or access arbitrary paths.

## Element addresses copied from the viewer

`http://127.0.0.1:<port>/?project=<projectId>&page=<pageId>&frame=<frameId>&node=<nodeId>`
identifies one element using stable design IDs. Parse URL query values with a URL
parser (percent-decode once; a virtual node can contain `/`). The port is the local
server address, not design identity. No revision or viewport is encoded: read the
latest authored revision intentionally.

1. Call `find_nodes {projectId, ids:[nodeId]}` to confirm identity. Use the exact ID;
   never infer identity from a label, viewport position or a cached scene.
2. Call `get_design_context {projectId, frameId}` for surrounding authored structure,
   resources and notes; use `get_subtree {projectId, nodeId}` for a smaller authored read. For virtual instance
   children use `instances:"expand"` when expanded surrounding structure is needed.
3. Confirm page/frame ancestry. If an ID was deleted or moved, explain the mismatch
   and ask for a current link; do not silently substitute the first frame.

The browser restores project/page/frame/selection before its stored local selection.
This address does not grant access to any remote system or filesystem path.

## Implement the frame

1. Choose the frame: `find_nodes` with `kinds:["frame"]`, a `nameContains` or a
   `textContains` when you know what to look for; `get_outline` when you need the
   structure (`depth:1` on a page lists its root frames; `find_nodes {parentId:<page>,
   kinds:["frame"]}` does the same). Then read `get_design_context` for that frame
   without `depth`: a depth leaves deeper nodes out (cut nodes carry `omittedChildren`).
   Follow every `nextCursor` using identical parameters; one page is not the whole design.
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
| `layout.padding`, `layout.gap` | Explicit padding and gap; a padding list is the CSS shorthand (`[8,16]` → `padding:8px 16px`) |
| `layout.align` | `align-items` (`start`/`end` → `flex-start`/`flex-end`; missing = `flex-start`, not CSS's stretch) |
| `layout.justify` | `justify-content` (`space-between` keeps `gap` as the minimum spacing) |
| Child of an `align:"stretch"` stack | Unset cross size → `auto` (stretches); `hug` → `fit-content` / `align-self:flex-start`; number → fixed |
| Numeric width/height | Fixed dimension in the design's logical pixels |
| `fill` on a stack's main axis | Flexible share of remaining space (`flex:1 1 0`) |
| `fill` on the cross axis | Stretch to the available cross size |
| `hug` | Content-driven size; do not freeze the measured bound |
| `minWidth`/`maxWidth`, `minHeight`/`maxHeight` | Corresponding size constraints |
| Children of `free` | Authored x/y relative to that parent; useful for overlays |
| `opacity` on a frame or group | CSS `opacity` on that element; the group fades as one image, so do not multiply it into children |
| `stroke` `align:"inside"` (default) | `border` (`box-sizing:border-box`); width/color arrays are [top, right, bottom, left] |
| `stroke` `align:"outside"` or `"center"` | Paint only (`outline`/`box-shadow` spread); does not move children |
| `radius` 1–4 values | `border-radius` shorthand [top-left, top-right, bottom-right, bottom-left] |
| `fill` `{type:"linear"\|"radial"}` | `linear-gradient(<angle>deg, …)` / `radial-gradient(… at x% y%, …)`, stops `[position, color]` → `color position%` |
| `shadow` layers | `box-shadow`, first layer on top; `inset:true` → `inset` |

A free child's x/y counts from its parent's outer edge; CSS absolute offsets count from
the padding box, so subtract the parent's inside `border-left`/`border-top`.

These are starting rules, not a CSS equivalence guarantee. Without `align`/`justify`
the engine uses start alignment, and it never shrinks fixed siblings. Alignment
overflow is unsafe like CSS: a centered child wider than its stack spills on both
sides. `align`/`justify` move boxes, not the lines inside a text box. Respect explicit fixed sizes
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
- Keep paragraphs from their full authored source strings and semantic styles/layout.
  Read `skill://opencanvas/props.md` and `opencanvas://capabilities` before choosing
  an approximation; cache static capabilities by fingerprint/pipeline version.
  For character/line layers, read source and notes, grouping and intent first.
  Measured positions alone do not require hundreds of positioned spans. Preserve
  intentional lettering/animation; do not automatically merge art. If spaces/order
  or source are missing, report the limitation instead of inventing copy.
- Text node y is its box position, not a browser baseline. Computed line positions
  are rendering output for that font/runtime/viewport, not a CSS layout contract.
  Avoid copying line y values into CSS or offsetting text
  by eye to compensate for an unloaded/different font.
- Render at the same revision, viewport, theme and state; compare `render_frame`
  pixels with `inspect_layout` bounds/text lines for that scene key. The
  `render_frame` response omits the computed scene to stay small; read it through
  `inspect_layout` (paged, `fields:["bounds"]` is smallest) rather than passing
  `includeScene:true`, which returns the whole scene inline. Wait for a
  pending job with `render_frame {jobId, waitMs}` (up to 10000) instead of
  polling in a tight loop; it returns as soon as the job is ready or failed. Use a
  second viewport to check declared stacks. Their presence alone does not promise that
  the entire design fits every width.
- Check horizontal overflow and content wrapping. Row wrap is available; column
  wrap, percent dimensions and media-query breakpoints remain unsupported. Ask about required behavior when the
  authored rules cannot express it; do not invent design intent.
- Declared motion is `props.animations`; `get_design_context` lists every one in
  `animated` (`{ref: "<nodeId>:<name>", on}`, absent when the frame has none). Map
  each to CSS `@keyframes`/`animation` or `el.animate()`: keyframes are WAAPI-shaped
  (`offset`, per-keyframe `easing`), times are milliseconds, `translateX/Y` is CSS
  `translate` from the laid-out position, `scale`/`rotate` turn around the center,
  and a missing 0/1 keyframe is the authored props. `fillMode` defaults to `both`
  (CSS default is `none`), so write it out. `on:"hover"` is a transition to its one
  keyframe while hovered; `on:"click"` restarts on each click. Use the token
  reference in authored `animations`, not the resolved literal.
- A node whose authored props carry `exportSettings` is an image asset: call
  `export_nodes {projectId, nodeIds}` and download each file `url` over HTTP
  right away instead of rebuilding it in code or cropping a frame PNG. Pass
  `settings` (e.g. `[{"scale":2}]`) to export any other node the same way.
  Prefer `[{"format":"svg"}]` for icons, logos and shapes: one self-contained
  vector file. Text in SVG is not outlined (`SVG_TEXT_NOT_OUTLINED`);
  production export embeds the used font bytes. Compare the actual job diagnostics
  and capabilities; SVG text remains font-shaped rather than outlined. Use semantic
  text for UI labels and PNG when a raster reference is needed.
- Navigation is declared: `get_design_context.navigation` lists `out` links (from
  nodes in the frame) and `in` links (from other screens), plus `flowStart`.
  `navigate` → go to the route built from frame `to`; `overlay` → a modal or
  popover built from frame `to`; `back` → history back; `close` → close the
  enclosing overlay. `in` tells you where a screen opens from, so choose route
  or overlay. A `flowStart` frame is the initial route of that flow.
- Route paths, transitions, guards, motion beyond `animations` and real data are not in the
  design and require user requirements. Request `statesMode:"resolved"` for declared
  `props.states` variants; map hover/pressed/focus/disabled to CSS :hover/:active/
  :focus-visible/[disabled]. Never invent undeclared variants. `render_frame state`
  previews all declaring nodes at once; layout and children retain resting geometry.
  Named token themes may be requested. P07a hover animations require one explicit
  keyframe; there is no `to` property or implicit target from `states.hover`.

## Finding nodes

- `find_nodes` ANDs `kinds`, `nameContains`, `textContains`, `ids` and `parentId`
  (direct children of that node only); at least one is required. Matching is case-insensitive substring without accent folding, so
  "Tong quan" does not find "Tổng quan"; search with the accents the design uses.
- `totalMatches: 0` with `matches: []` means nothing matched; it is not an error.
  `truncated: true` with `nextCursor` means more pages of matches.
- Position ("the card on the right") is not searchable. Narrow candidates by name,
  text or kind, then compare bounds from `get_design_context` `viewports` or
  `inspect_layout` `nodeIds`.

## Notes

- `get_design_context` lists the open `notes` of the frame, its descendants and its
  ancestors; read them with the props. After implementing one, `reply_note` how, then
  `update_note {resolved:true}`. Do not resolve a question nobody has answered.
