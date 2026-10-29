//! Minimal component authoring contract, outside the compact tool catalog.
pub(super) fn section() -> &'static str {
    "\n## Components and instances\n\n\
     A reusable master is a `frame` with `component: {name: \"TableRow\"}`. Its children \
     are authored normally. Masters may use fill/hug sizing; a rendered root frame \
     still needs fixed dimensions. Create an `instance` leaf with `componentId` pointing \
     to the master. It inherits the master's layout and paint; its own name, visibility, \
     position and sizing replace the corresponding root properties.\n\n\
     `overrides` is an object keyed by authored master descendant IDs. Each value is a \
     partial props object accepting only `text`, `fill`, `visible`, `styleId`, `assetId` \
     and `name`, validated for the target's kind and every theme. Image references use \
     `assetId`; token references use `{tokenId: \"...\"}`. The target cannot be the master \
     root. Layout, structure and component references cannot be overridden. \
     `update_node` replaces the complete `overrides` object, so resend entries to retain.\n\n\
     Expanded children have stable IDs `<instanceId>/<masterChildId>` and are read-only: \
     update the instance's `props.overrides` instead (`INSTANCE_CHILD_READONLY`). \
     Instances cannot have authored children. Nesting is limited to 8 component levels; \
     cycles fail with `INSTANCE_CYCLE`. Deleting a referenced master fails with \
     `COMPONENT_IN_USE`, `count` and up to 20 `instanceIds`. Component subtrees cannot \
     carry links, flow starts or animations.\n\n\
     `get_design_context` defaults to `instances: \"reference\"`: one definition per \
     referenced master in `components`, instances as leaves and optional computed \
     child bounds/lines. Choose `instances: \"expand\"` for the full virtual tree. \
     `get_outline` and `get_subtree` show instance leaves with `componentId` and \
     `childCount`; scope a virtual child directly to read it. `find_nodes` accepts \
     `kinds: [\"instance\"]` and searches overridden child text using virtual IDs.\n"
}
