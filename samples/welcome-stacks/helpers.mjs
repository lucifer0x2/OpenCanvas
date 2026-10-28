import { welcomeOperations } from "../welcome/design.mjs";

export const FRAME = "welcome-dashboard-stacks";
export const DESKTOP = { width: 1040, height: 720 };
export const NARROW = { width: 390, height: 844 };
// Declared before rendering/comparing: strict enough to catch a shifted text line.
export const THRESHOLD = { changedPixelRatio: 0.005, meanAbsoluteChannelError: 0.25 };

export function builder() {
  const original = welcomeOperations();
  const source = new Map(original.filter((o) => o.op === "create_node").map((o) => [o.id, o]));
  const operations = original.filter((o) => o.op !== "create_node");
  let sequence = 0;
  function node(id, kind, parentId, props) {
    operations.push({ op: "create_node", id, kind, parentId, props: { name: id, ...props } });
    return id;
  }
  function old(id, parentId, overrides = {}) {
    const sourceNode = source.get(id);
    const { x, y, layout, ...props } = sourceNode.props;
    return node(id, sourceNode.kind, parentId, { ...props, ...overrides });
  }
  function stack(id, parentId, mode, width, height, extra = {}) {
    return node(id, "group", parentId, {
      width, height, layout: { mode, padding: 0, gap: 0 }, ...extra,
    });
  }
  function gap(parent, size, mode = "column") {
    return node(`space-${++sequence}`, "group", parent, {
      width: mode === "row" ? size : "fill", height: mode === "row" ? 1 : size,
      layout: { mode: "column", padding: 0, gap: 0 },
    });
  }
  function text(id, parent, extra = {}) { return old(id, parent, { width: "fill", ...extra }); }
  function inset(id, parent, width, height, left, top, extra = {}) {
    const outer = stack(id, parent, "row", width, height, extra);
    gap(outer, left, "row");
    const inner = stack(`${id}-content`, outer, "column", "fill", "fill");
    if (top) gap(inner, top);
    return inner;
  }
  return { operations, node, old, stack, gap, text, inset, source };
}
