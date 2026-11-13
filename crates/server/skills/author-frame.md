# Author an OpenCanvas frame

## Project design memory

Before reading, understanding, inspecting, editing or authoring an OpenCanvas design,
read the selected project's `AGENTS.md` using authorized client filesystem access
if available. Its location is `<configured-data-root>/<project-slug>/AGENTS.md`,
defaulting to `~/.opencanvas/<project>/AGENTS.md`. Use the selected project's confirmed
data root and slug; a project ID or viewer link is not a filesystem path. Do not
guess a different root, access arbitrary paths or create another project to get memory.

If the file is missing and authorized write access is available, create it from the
user's confirmed design requirements and decisions. Record only concise confirmed
design intent, tokens, styles, fonts, components, layout rules, constraints and
exceptions. Never invent a palette, font or unconfirmed decision to fill it out.
Update it when the user confirms a design change; keep it current rather than
appending an activity log. Do not put coding, build, test or repository rules in it.

Prefer this shared project memory over session-private memory for design decisions,
subject to the user's current instructions and higher-priority directives. Its scope
is only reading, understanding, inspecting and editing/authoring designs in OpenCanvas;
it does not govern code implementation, builds, tests, repository work or other tasks.

This is client instruction policy, not a backend validation requirement. The current
MCP resources are static skill documents; there is no project-memory read/write tool.
If the client lacks authorized filesystem access or a suitable tool to read or create
the file, explain the limitation and use the design context actually available. Never
pretend that memory has been read, initialized or saved.

## Authoring checklist

1. Discover `tools/list` and inspect the tool's `inputSchema`; use MCP tools with
   structured JSON for OpenCanvas design mutations. Do not write/run a JS, shell
   or HTTP mutation client to replace tools already available. This is client
   guidance, not enforcement of a client's system prompt or external programs.
   It does not restrict application code, test fixtures or the authorized design
   memory filesystem workflow above. Valid repeated operations remain accepted.
2. Read `get_outline` and the scoped context before editing an existing project.
   To locate a node you know by name, visible text, kind or ID, use `find_nodes`
   instead of scanning the outline; each match carries its ancestor `path`.
   `get_outline` `depth` counts from the scope root (`{nodeId:<page>, depth:1}` is the
   page and its root frames); `find_nodes {parentId}` lists one node's direct children.
   For a new project use `create_project`, then create a page and a root frame.
   If no current read or successful commit supplies the revision, read it before
   writing. Use `committedRevision` after success; do not add revision-only reads
   after each successful commit. Read props/capabilities and refresh cached
   guidance when its fingerprint/pipeline version changes.
3. Plan containers and sizing before placing nodes. Default to `row`/`column`
   stacks with explicit padding/gap. Use `free` for intentional overlays or
   decoration, not as the default for page content.
   Reuse tokens, styles, component master frames and instances before copying
   trees. Named `theme` on a frame/instance selects a token scheme in that scope;
   the nearest scope wins, with render/export theme as the global fallback.
   Confirm the theme exists on at least one token. Keep scheme differences in
   tokens and text/style overrides where possible; inspect expanded resolved values.
4. Submit ordered `apply_batch` operations (1–500 per atomic batch) with caller-chosen IDs,
   `expectedRevision`, and one stable `idempotencyKey` per exact request body.
   For edits inside existing pages, also supply `pageIds:["page-id"]`, using the
   revision from the **actual authored read**. Storage derives all page and
   transitive shared-dependency guards, so unrelated pages/resources can commit
   concurrently. `get_edit_context` reports guard/dependency metadata; never use
   a newer guard to relabel an older design read. After scoped success, reuse
   `editGuard` for the next batch; after a valid preview it remains provisional.
   Cross-page moves require both pages. Omit `pageIds` for strict revision mode
   when writing shared tokens/styles/fonts/assets, notes, page structure or
   `flowStart`, creating the first root frame, or deleting/reparenting/reordering
   root frames. A later root frame may be appended without an explicit index.
   Scoped deletion that cascades notes is rejected. Read again before switching
   a rejected scoped batch to strict mode; never just raise its revision.
   After a lost response replay that body; never change a replay body under the
   same key.
5. On explicit `REVISION_CONFLICT`, inspect `error.changedSince` (`fromRevision`,
   `toRevision`, `changedIds` ≤200, `truncated`). Re-read relevant nodes/ancestors
   and token/style dependencies with scoped reads (`find_nodes {ids}`), or catch
   up with `get_changes {sinceRevision}`. Rebuild from that evidence with the new
   revision and a **new** key; never blindly retry. Changed IDs are a hint, not
   proof that token/style or component dependencies are unaffected. Validation
   runs again. A lost response is different: retry the **identical** body/key.

6. Author only the `props` keys that `skill://opencanvas/props.md` lists for the
   node's kind (generated from the validator). Text color is `fill`, corner
   rounding is `radius`. A wrong key returns `VALIDATION_FAILED` with
   `field: "props.<key>"`, `nodeId`, `nodeKind`, `allowedKeys` and, when close,
   `suggestion`; fix that key instead of bisecting the batch.
   Borders are one `stroke` prop (`{"color":"#D0D5DD"}`; per side
   `{"color":[t,r,b,l],"width":[1,1,2,1]}`, CSS order); never fake a border with
   stacked or edge rectangles. An `inside` stroke (default) takes stack space like a
   CSS border; use `align:"outside"` to keep layout unchanged. Text cannot carry one.

## Structured JSON examples

Prefer `apply_batch` `source` (OCD, `skill://opencanvas/ocd.md`): it compiles
to these same operations. JSON `operations` stay supported; send one of the two.

These are ordered `tools/call` parameter objects for a fresh project whose
`create_project` response supplied revision 0. Replace `PROJECT_ID` with its ID;
sample IDs/colors/text are illustrative, not new confirmed design requirements.
The first batch creates one reusable master, a shared text style, two instances
and a named scheme. The second edits text with `update_node`, `id` and
`props.text`/`fontSize`; there is no `update_text`, `nodeId` or `props.content`.

<!-- structured-authoring-calls -->
```json
[
  {"name":"apply_batch","arguments":{"projectId":"PROJECT_ID","expectedRevision":0,"idempotencyKey":"author-setup","operations":[
    {"op":"create_node","id":"page","kind":"page","parentId":null,"props":{}},
    {"op":"upsert_token","token":{"id":"ink","name":"Ink","type":"color","value":{"default":"#112233","themes":{"a2":"#553311"}}}},
    {"op":"upsert_style","style":{"id":"body","name":"Body","kind":"text","value":{"fontId":"font_bundled","fontSize":16,"lineHeight":24}}},
    {"op":"create_node","id":"card","kind":"frame","parentId":"page","props":{"component":{"name":"Card"},"width":240,"height":80,"layout":{"mode":"column","padding":12}}},
    {"op":"create_node","id":"label","kind":"text","parentId":"card","props":{"text":"Hello","styleId":"body","width":"fill","height":"hug","fill":{"tokenId":"ink"}}},
    {"op":"create_node","id":"screen","kind":"frame","parentId":"page","props":{"width":640,"height":480,"layout":{"mode":"column","gap":16}}},
    {"op":"create_node","id":"first","kind":"instance","parentId":"screen","props":{"componentId":"card"}},
    {"op":"create_node","id":"second","kind":"instance","parentId":"screen","props":{"componentId":"card","theme":"a2","overrides":{"label":{"text":"Welcome"}}}}
  ]}},
  {"name":"apply_batch","arguments":{"projectId":"PROJECT_ID","expectedRevision":1,"idempotencyKey":"author-edit","operations":[
    {"op":"update_node","id":"label","props":{"text":"Hello again","fontSize":18}}
  ]}},
  {"name":"apply_batch","arguments":{"projectId":"PROJECT_ID","expectedRevision":2,"idempotencyKey":"author-token","dryRun":true,"operations":[
    {"op":"upsert_token","token":{"id":"ink","name":"Ink","type":"color","value":{"default":"#223344","themes":{"a2":"#664422"}}}}
  ]}}
]
```

The dry run performs full candidate validation and returns `dryRun:true`,
`baseRevision`, would-be `changedIds` and `diagnostics`, without writes/events,
a revision increment or key consumption. To commit this same token update, call
the last object again with `dryRun:false` (or omit the field), keeping revision 2
and the same key. Its success supplies revision 3. Dry run is not a reservation:
a concurrent write can still cause a conflict before commit.

For a large existing payload, opt in to `apply_batch_file` only with authorized
filesystem access to the selected project's confirmed inbox. Save this **pure
JSON array** as `<configured-data-root>/<project-slug>/inbox/ops.json`:

<!-- structured-authoring-file-operations -->
```json
[{"op":"update_node","id":"label","props":{"text":"Saved JSON","fontSize":18}}]
```

Then submit this MCP call (it never evaluates JS or other executable code):

<!-- structured-authoring-file-call -->
```json
{"name":"apply_batch_file","arguments":{"projectId":"PROJECT_ID","expectedRevision":3,"idempotencyKey":"author-file","path":"ops.json","dryRun":true}}
```

Commit by setting `dryRun:false`. Keep file contents, revision and key stable
after a lost response; replay uses canonical operation contents, not just path.
File calls share validation/atomicity and the 500-op limit, plus a 2 MiB byte cap;
they do not auto-chunk. Split larger input into explicit atomic batches, each
using the last successful revision and its own key. Paths outside the inbox,
traversal and symlink escapes fail. Without authorized filesystem access, use
inline batches; never claim a file was written. Payload savings come from
master/token/style reuse, not from changing JS transport to MCP. The U07
~4 KB/instance estimate and golden-image rebuild remain to be measured separately.

## Stack template

Under an existing page, create a root frame with numeric `width` and `height`,
for example `{"width":960,"height":720,"layout":{"mode":"column","padding":24,"gap":16}}`.
Use a child group with `width:"fill",height:"hug"` for a content section, and a
`row` or `column` layout for its children. Choose widths deliberately: fixed for
an icon, fill for the flexible content area, hug for content-driven dimensions.
Text needs a valid font (for example `fontId:"font_bundled"`), `fontSize`, and
`lineHeight`, or a valid text style providing those fields.

Keep UI copy semantic: one text node per paragraph or coherent label with its full
source string, meaningful spaces and intentional hard breaks. Use fixed/fill width
and hug height for wrapping paragraphs. Measured line breaks are output for one
viewport; do not turn them into authored line or character nodes to match a capture.
Read `skill://opencanvas/props.md` and `opencanvas://capabilities` before a workaround;
cache capabilities by its fingerprint and pipeline versions. Support differs between
authored data, Canvas/PNG, HTML preview and SVG. Runtime readiness and glyph coverage
still require job evidence. Record unsupported behavior and approximations with
`create_note`; never invent a property or use positioned glyphs to hide engine errors.

Character/line layers remain valid for intentional lettering, wordmarks, monograms
and per-letter motion. Preserve grapheme clusters and source copy. A note beginning
`Intentional lettering:` on the group records the exception for the advisory heuristic.
Use `Intentional overflow:` for deliberately overflowing text; measured overflow is
then informational. These are note conventions, not new props. Single-character
labels are valid. Do not merge existing layers without evidence of source and intent.
Inspect `TEXT_FRAGMENTATION_SUSPECTED` and `TEXT_CONTENT_OUTSIDE_BOX` diagnostics:
they describe bounded examples from the complete scope before paging/filtering,
not a rejection or an instruction to change the design automatically.
Check the same revision, fonts, theme and state at the intended width and a second
relevant width; compare text lines, ink/advance evidence and pixels. State which
parts reflow, which are intentional fixed art, and which are approximations.

Stacks align like CSS flexbox, so centering and right edges need no x/y math:

- Centered content: `{"mode":"column","padding":[24,32],"gap":16,"align":"center","justify":"center"}`.
- Toolbar, label … shortcut row: `{"width":"fill","height":48,"layout":{"mode":"row","padding":[0,12],"gap":8,"align":"center","justify":"space-between"}}`.
- Menu whose items match the widest: `{"width":"hug","height":"hug","layout":{"mode":"column","padding":4,"align":"stretch"}}`,
  items `{"height":28,"layout":{"mode":"row","padding":[0,8],"align":"center"}}` with **no** `width`.

`padding` is a number or a CSS shorthand list (`[all]`, `[vertical, horizontal]`,
`[top, horizontal, bottom]`, `[top, right, bottom, left]`). `align` (cross axis:
start, center, end, stretch) and `justify` (main axis: start, center, end,
space-between) default to start and need row or column. `stretch` sizes only
children whose cross size is unset; numbers, percentages and `hug` keep their size. With
`space-between`, `gap` is the minimum spacing. Centered content wider than its
stack overflows both sides (clipped by frames); use start when it may not fit.
`update_node` replaces the whole layout object: resend `mode` and every other
layout key with the one you change.

For an authored mobile stack, set a numeric size on the root frame and declare
the base and narrow layout together.

```json
{
  "mode": "free",
  "breakpoints": [
    { "maxWidth": 600, "layout": { "mode": "column", "padding": 16, "gap": 12 } }
  ]
}
```

At 600 px and below, the stack places the same children in their authored order.
Above 600 px, free layout uses their authored x/y. Up to eight thresholds are
allowed, as increasing unique integers from 1 to 100000. The first inclusive
match wins across the requested frame viewport, including for nested containers.
Each alternate rule supplies its own mode, padding, gap, alignment and wrap as
needed. Defaults apply to omitted properties, and variants cannot nest. Padding
and gap accept dimension refs in every rule.

Size a child with e.g. `width:"50%"` or `height:"37.5%"` when that parent
axis is numeric or follows a fill/percentage chain to a numeric axis. Use literal
decimal values from 0% to 100% and keep min/max in pixels. The percentage basis
is the parent's content box in a stack and padding box under free positioning.
Use separate viewport reads to verify positions and dimensions on both sides
of each breakpoint.

## Constraints and limitations

- `minWidth`/`maxWidth` and the height equivalents are finite numeric bounds.
  Keep minimum no greater than maximum; ensure fixed siblings plus gaps/padding
  can fit the narrow viewport. A minimum can intentionally cause overflow, so
  it is not evidence of responsiveness by itself.
- Use `height:"hug"` for wrapping text and content groups that should grow.
  Do not put fill on an axis whose parent's hug size depends on that fill child;
  the engine rejects ambiguous sizing cycles; for equal-width items in a hug
  column use the parent's `align:"stretch"` instead. Root frames require fixed numbers.
- Fade a whole control (disabled button, keycap) with `opacity` on its group or
  frame: like CSS, the group renders as one image and then fades, so overlapping
  layers inside do not show through. Do not pre-mix colors layer by layer.
- Prefer named typed tokens and styles for repeated values. Validate referenced
  records before use. Read back authored props and resolved values separately.
- Opt in to wrapping with `layout:{mode:"row",wrap:true,gap:16}`. A wrapping
  base or alternate rule needs positive numeric width or a positive
  fill/percentage chain to a definite positive width. Each
  direct width-fill child needs explicit positive `minWidth` (dimension refs are
  supported); text fill inside a card column does not need this extra minimum.
  Use row/card height hug for content-driven rows. Gap applies between rows too;
  the last fill card may expand across the entire row. Oversized minima overflow.
- Missing/false wrap preserves no-wrap. True is only for row containers; wrap
  must be a literal boolean. In a wrapping row `justify` and `align` apply per
  line. No column wrap, `alignSelf`,
  `space-around`/`space-evenly` or general CSS/Grid. A fixed sidebar/body minimum still
  needs deliberate authoring; row wrap alone does not make a dashboard mobile.
- Mark icons and illustrations that code should use as files with
  `exportSettings`, e.g. `[{"scale":1},{"scale":2}]` (`Search.png`,
  `Search@2x.png` from the node name); `[]` clears it. `export_nodes` returns
  them as transparent PNGs cropped to the node's bounds.

## Links between screens

- Put `links:[{to:"<root frame ID>"}]` on the clickable node (button, row), not on
  the whole frame unless the entire screen is clickable. One link per `on`.
- Menus, dialogs and sheets are their own root frames, opened with
  `action:"overlay"`; `on:"hover"` only opens an overlay. Back buttons use
  `action:"back"`, dismiss buttons `action:"close"` (no `to`).
- Mark the first screen of a flow with `flowStart:"Main"` on its root frame.
- Deleting a target frame removes links to it; their sources are in `changedIds`.

## Animation

- Author props as the resting state (after load). `animations` only says how a
  node reaches or leaves it: keyframes of `opacity`, `translateX`, `translateY`,
  `scale`, `rotate` and `fill`; nothing that changes layout. A key missing from
  the 0 or 1 keyframe takes the authored value, so never repeat it.
- `duration`/`delay` are milliseconds. A lone keyframe without `offset` is the
  end (animate *to* it); write `offset:0` to animate *from* it.
- Each property belongs to one animation per node (translateX/Y count as one);
  to combine a load fade with a hover change of the same property, animate a
  wrapping group. When an animation sets both translate axes, set them at the
  same keyframes. `on:"hover"` takes exactly one keyframe and no iterations.
  Set `triggerNodeId` on a hover track to use an authored group or frame ancestor
  in the same root frame. Omit it for self-hover. For example, give the icon
  inside `card` the `lift` track below and reuse `card` on any other child track
  that should respond to the same hover region.
  A `fill` keyframe requires a solid resting fill; on a gradient box animate
  opacity or transform instead.

```json
{"name":"enter","duration":400,"easing":"ease-out","keyframes":[{"offset":0,"opacity":0,"translateY":12}]}
{"name":"lift","on":"hover","triggerNodeId":"card","duration":200,"keyframes":[{"scale":1.02}]}
{"name":"pulse","duration":800,"iterations":"infinite","direction":"alternate","keyframes":[{"fill":{"tokenId":"color-brand-soft"}}]}
```

## Read back and check

- Read `get_design_context` for the complete frame, following pagination. An
  all-free frame emits `NO_REFLOW_INFORMATION`; inspect its `authoredSize`.
- Render and inspect at the authored size and a second, narrower viewport using
  the same revision; wait for each job with `render_frame {jobId, waitMs}` rather
  than tight polling. Check content bounds, horizontal overflow, wrapping and
  clipping. Root viewport resizing does not rearrange children of a free frame.
- Compare images as well as bounds. Text baseline/line metrics depend on the
  actual font; never replace missing measurements with guessed positions.
- A stack somewhere in the tree suppresses the all-free diagnostic but does not
  certify responsive behavior. Record which widths were checked and any fixed
  sizes or overflows that remain intentional.

## Notes for intent the props cannot carry

- Put behavior (sticky, hover, scroll), constraints ("fixed width per spec") and effects the
  renderer lacks (blur, conic gradient, gradient text) in `create_note` on the node they concern; never as a text node.
- Notes are revisioned like nodes: `delete_subtree` drops the notes of deleted nodes.
