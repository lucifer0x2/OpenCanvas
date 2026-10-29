import type { Bounds, Resources, Scene, SceneNode } from "./types";
import { PipelineError } from "./error";
import { strokeOuter } from "./stroke";

/** An axis-aligned rectangle by its edges; in scene units or device pixels depending on use. */
export interface LogicalRect {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

/** The scene as a tree, with what each subtree paints and where, for group opacity layers. */
export interface SceneTree {
  roots: SceneNode[];
  children: Map<string, SceneNode[]>;
  /** Paint commands a layer would merge: own fill/content, own stroke, each child subtree that paints. */
  units: Map<string, number>;
  /** Scene-unit bounds of everything the subtree paints; null when it paints nothing. */
  extent: Map<string, LogicalRect | null>;
}

export const UNBOUNDED: LogicalRect = {
  x0: -Infinity,
  y0: -Infinity,
  x1: Infinity,
  y1: Infinity,
};

export const rectOf = (b: Bounds, pad = 0): LogicalRect => ({
  x0: b.x - pad,
  y0: b.y - pad,
  x1: b.x + b.width + pad,
  y1: b.y + b.height + pad,
});

export function union(
  a: LogicalRect | null,
  b: LogicalRect | null,
): LogicalRect | null {
  if (!a) return b;
  if (!b) return a;
  return {
    x0: Math.min(a.x0, b.x0),
    y0: Math.min(a.y0, b.y0),
    x1: Math.max(a.x1, b.x1),
    y1: Math.max(a.y1, b.y1),
  };
}

export function intersect(
  a: LogicalRect | null,
  b: LogicalRect | null,
): LogicalRect | null {
  if (!a || !b) return null;
  const r = {
    x0: Math.max(a.x0, b.x0),
    y0: Math.max(a.y0, b.y0),
    x1: Math.min(a.x1, b.x1),
    y1: Math.min(a.y1, b.y1),
  };
  return r.x0 < r.x1 && r.y0 < r.y1 ? r : null;
}

/** Matches paintNode: text and image always paint; anything else only with a fill. */
const paintsItself = (n: SceneNode) =>
  n.kind === "text" || n.kind === "image" || n.fill !== "transparent";

/** Where the node's own paint can land. Glyphs overhang bounds; placeholders draw unbounded text. */
function ownExtent(n: SceneNode, resources: Resources): LogicalRect | null {
  if (!paintsItself(n)) return null;
  if (
    (n.kind === "text" && !resources.fonts.has(n.fontId ?? "")) ||
    (n.kind === "image" && !resources.images.has(n.assetId ?? ""))
  )
    return UNBOUNDED;
  if (n.kind !== "text") return rectOf(n.bounds);
  let extent: LogicalRect | null = rectOf(
    n.bounds,
    Math.max(n.fontSize, n.lineHeight),
  );
  const { x, y } = n.bounds;
  for (const line of n.lines)
    extent = union(extent, {
      x0: x + line.x - n.fontSize,
      y0: y + line.y - 2 * n.fontSize,
      x1: x + line.x + line.width + n.fontSize,
      y1: y + line.y + n.fontSize,
    });
  return extent;
}

/**
 * Builds the tree from pre-order scene nodes, keeping sibling order. A node whose parent is
 * missing is a root. Throws SCENE_INVALID when a node is unreachable from a root (a cycle) or an
 * id repeats.
 */
export function indexScene(scene: Scene, resources: Resources): SceneTree {
  const byId = new Map(scene.nodes.map((n) => [n.id, n]));
  if (byId.size !== scene.nodes.length)
    throw new PipelineError("SCENE_INVALID", "Duplicate scene node id");
  const tree: SceneTree = {
    roots: [],
    children: new Map(),
    units: new Map(),
    extent: new Map(),
  };
  for (const n of scene.nodes) {
    const parent = n.parentId ? byId.get(n.parentId) : undefined;
    if (!parent) tree.roots.push(n);
    else {
      const list = tree.children.get(parent.id);
      if (list) list.push(n);
      else tree.children.set(parent.id, [n]);
    }
  }
  // Depth is validated at 128 upstream, so recursion is bounded.
  const visit = (n: SceneNode): LogicalRect | null => {
    const stroke = strokeOuter(n);
    let units = (paintsItself(n) ? 1 : 0) + (stroke ? 1 : 0);
    // The node's own clip cuts its children, never its stroke.
    let extent = union(ownExtent(n, resources), stroke && rectOf(stroke));
    for (const child of tree.children.get(n.id) ?? []) {
      const inner = visit(child);
      if (inner) units++;
      extent = union(
        extent,
        n.clip ? intersect(inner, rectOf(n.bounds)) : inner,
      );
    }
    tree.units.set(n.id, units);
    tree.extent.set(n.id, extent);
    return extent;
  };
  for (const root of tree.roots) visit(root);
  if (tree.units.size !== scene.nodes.length)
    throw new PipelineError("SCENE_INVALID", "Scene cycle");
  return tree;
}
