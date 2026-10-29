/**
 * CSS declarations (kebab-case) for the paint of one scene node, shared by the DOM builder
 * (resting inline styles) and the state stylesheet (overrides). Pure.
 */
import type { SceneNode, SceneStroke } from "../../../renderer/types.ts";

export type Declarations = Record<string, string>;

const ALIGN: Record<string, number> = { inside: 0, center: 0.5, outside: 1 };
const px = (value: number) => `${Math.round(value * 1000) / 1000}px`;

/** Declarations of the node element: group opacity (CSS composites it like the renderer). */
export function nodeDeclarations(node: SceneNode): Declarations {
  return { opacity: String(node.opacity ?? 1) };
}

/** Declarations of the paint box: fill (background, or text colour) and corner radius. */
export function boxDeclarations(node: SceneNode): Declarations {
  const fill = node.fill || "transparent";
  return node.kind === "text"
    ? { color: fill, "background-color": "transparent" }
    : { "background-color": fill, "border-radius": px(node.radius ?? 0) };
}

/**
 * A stroke as a CSS border on an overlay box grown by width·f per side (inside 0, center ½,
 * outside 1), with per-axis corner radii like renderer/stroke.ts `strokeEdges`. The overlay is
 * a sibling of the paint box, so the node's own clip never cuts its stroke.
 */
export function strokeDeclarations(
  stroke: SceneStroke | undefined,
  radius: number,
): Declarations {
  if (!stroke) return { display: "none" };
  const f = ALIGN[stroke.align] ?? 0;
  const [t, r, b, l] = stroke.width;
  const corner = (h: number, v: number) =>
    radius > 0 ? [radius + h * f, radius + v * f] : [0, 0];
  const corners = [corner(l, t), corner(r, t), corner(r, b), corner(l, b)];
  return {
    display: "block",
    top: px(-t * f),
    right: px(-r * f),
    bottom: px(-b * f),
    left: px(-l * f),
    "border-style": "solid",
    "border-width": stroke.width.map(px).join(" "),
    "border-color": stroke.color.join(" "),
    "border-radius": `${corners.map((c) => px(c[0])).join(" ")} / ${corners
      .map((c) => px(c[1]))
      .join(" ")}`,
  };
}

/** Declarations that differ between two declaration sets (keys of `next` only). */
export function changed(base: Declarations, next: Declarations): Declarations {
  const out: Declarations = {};
  for (const [key, value] of Object.entries(next))
    if (base[key] !== value) out[key] = value;
  return out;
}
