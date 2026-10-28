# Author an OpenCanvas frame

1. Read `get_outline` and the scoped context before editing an existing project.
   To locate a node you know by name, visible text, kind or ID, use `find_nodes`
   instead of scanning the outline; each match carries its ancestor `path`.
   `get_outline` `depth` counts from the scope root (`{nodeId:<page>, depth:1}` is the
   page and its root frames); `find_nodes {parentId}` lists one node's direct children.
   For a new project use `create_project`, then create a page and a root frame.
2. Plan containers and sizing before placing nodes. Default to `row`/`column`
   stacks with explicit padding/gap. Use `free` for intentional overlays or
   decoration, not as the default for page content.
3. Submit ordered `apply_batch` operations with caller-chosen IDs,
   `expectedRevision`, and one stable `idempotencyKey` per exact request body.
   After a lost response replay that body; never change a replay body under the
   same key.
4. On `REVISION_CONFLICT` read `error.changedSince` (`fromRevision`,
   `toRevision`, `changedIds` ≤200, `truncated`). If `truncated` is false and no
   changed ID is a node you edit or one of its ancestors, resend the same
   operations with `expectedRevision: toRevision` and a **new** `idempotencyKey`.
   Otherwise re-read exactly those IDs with `find_nodes {ids}` (or page through
   `get_changes {sinceRevision}`) and rebuild the batch. Every batch is validated
   again, so a wrong rebase returns `VALIDATION_FAILED`, never corrupt data.

5. Author only the `props` keys that `skill://opencanvas/props.md` lists for the
   node's kind (generated from the validator). Text color is `fill`, corner
   rounding is `radius`. A wrong key returns `VALIDATION_FAILED` with
   `field: "props.<key>"`, `nodeId`, `nodeKind`, `allowedKeys` and, when close,
   `suggestion`; fix that key instead of bisecting the batch.
   Borders are one `stroke` prop (`{"color":"#D0D5DD"}`; per side
   `{"color":[t,r,b,l],"width":[1,1,2,1]}`, CSS order); never fake a border with
   stacked or edge rectangles. An `inside` stroke (default) takes stack space like a
   CSS border; use `align:"outside"` to keep layout unchanged. Text cannot carry one.

## Stack template

Under an existing page, create a root frame with numeric `width` and `height`,
for example `{"width":960,"height":720,"layout":{"mode":"column","padding":24,"gap":16}}`.
Use a child group with `width:"fill",height:"hug"` for a content section, and a
`row` or `column` layout for its children. Choose widths deliberately: fixed for
an icon, fill for the flexible content area, hug for content-driven dimensions.
Text needs a valid font (for example `fontId:"font_bundled"`), `fontSize`, and
`lineHeight`, or a valid text style providing those fields.

Stacks align like CSS flexbox, so centering and right edges need no x/y math:

- Centered content: `{"mode":"column","padding":[24,32],"gap":16,"align":"center","justify":"center"}`.
- Toolbar, label … shortcut row: `{"width":"fill","height":48,"layout":{"mode":"row","padding":[0,12],"gap":8,"align":"center","justify":"space-between"}}`.
- Menu whose items match the widest: `{"width":"hug","height":"hug","layout":{"mode":"column","padding":4,"align":"stretch"}}`,
  items `{"height":28,"layout":{"mode":"row","padding":[0,8],"align":"center"}}` with **no** `width`.

`padding` is a number or a CSS shorthand list (`[all]`, `[vertical, horizontal]`,
`[top, horizontal, bottom]`, `[top, right, bottom, left]`). `align` (cross axis:
start, center, end, stretch) and `justify` (main axis: start, center, end,
space-between) default to start and need row or column. `stretch` sizes only
children whose cross size is unset; numbers and `hug` keep their size. With
`space-between`, `gap` is the minimum spacing. Centered content wider than its
stack overflows both sides (clipped by frames); use start when it may not fit.
`update_node` replaces the whole layout object: resend `mode` and every other
layout key with the one you change.

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
- Opt in to wrapping with `layout:{mode:"row",wrap:true,gap:16}`. The row needs
  positive numeric width or a fill chain to a finite ancestor/root frame. Each
  direct width-fill child needs explicit positive `minWidth` (dimension refs are
  supported); text fill inside a card column does not need this extra minimum.
  Use row/card height hug for content-driven rows. Gap applies between rows too;
  the last fill card may expand across the entire row. Oversized minima overflow.
- Missing/false wrap preserves no-wrap. True is only for row containers; wrap
  must be a literal boolean. In a wrapping row `justify` and `align` apply per
  line. No column wrap, percentage dimensions, breakpoint variants, `alignSelf`,
  `space-around`/`space-evenly` or general CSS/Grid. A fixed sidebar/body minimum still
  needs deliberate authoring; row wrap alone does not make a dashboard mobile.
- Mark icons and illustrations that code should use as files with
  `exportSettings`, e.g. `[{"scale":1},{"scale":2}]` (`Search.png`,
  `Search@2x.png` from the node name); `[]` clears it. `export_nodes` returns
  them as transparent PNGs cropped to the node's bounds.

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
  renderer lacks (blur, gradient) in `create_note` on the node they concern; never as a text node.
- Notes are revisioned like nodes: `delete_subtree` drops the notes of deleted nodes.
