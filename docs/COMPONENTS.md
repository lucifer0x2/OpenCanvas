# Components and instances (P24)

Reusable components use a frame with `component:{name}` and ordinary authored
children. Place an `instance` leaf with `componentId` and optional flat `overrides`
keyed by master child ID. Allowed overrides are `text`, `fill`, `visible`, `styleId`,
`assetId`, `name`; layout and structure remain inherited. Full authoring rules are
in `skill://opencanvas/props.md`. Render, inspect and image export use expanded nodes
with stable virtual IDs such as `row3/cmpRow-q`; writes to those IDs fail with
`INSTANCE_CHILD_READONLY`. Change the instance's whole `props.overrides` object.

`get_design_context` defaults to `instances:"reference"`: each referenced master
appears once in first-page `components` (including nested dependencies), while
instance leaves expose `componentId`, `overrides`, `childCount` and, when computed,
`computed:{bounds,children:{"row3/cmpRow-q":{bounds,lines}}}`. Choose
`instances:"expand"` for all virtual nodes. Compact resources cover definitions and
overrides, including style/token/font/asset dependencies across themes. Outline and
subtree keep instances as leaves; a virtual-child scope reads that child directly.
`find_nodes` supports `kinds:["instance"]`, `properties:["componentId"]` and effective
overridden child text. Snapshot/bundle storage remains authored. Cycles/depth >8,
invalid override kinds/refs and an expanded project >20,000 nodes are rejected
atomically; deleting an in-use master returns `COMPONENT_IN_USE` with `count` and
up to 20 `instanceIds`. No new MCP tool is added.

## Interaction states

Expansion preserves the master root's `states` on each placed instance and each
master descendant's own declarations on its virtual child. Theme refs resolve
after expansion. A child never receives a parent's state override implicitly.
The minimal instance whitelist does not accept its own `states` or
`overrides.<childId>.states`; edit the master declarations instead.

`statesMode:"resolved"` exposes differing variants on instance roots, virtual
nodes in expand mode, and nodes inside reference `components` definitions.
Compact resource reads retain tokens/styles referenced only by these variants.
An override `visible:false` remains hidden in every rendered state, while keeping
layout space. State preview and geometry semantics follow
[Interaction states](INTERACTION-STATES.md).
