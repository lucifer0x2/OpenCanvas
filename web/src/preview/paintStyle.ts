/**
 * CSS declarations (kebab-case) for the paint of one scene node, shared by the DOM builder
 * (resting inline styles) and the state stylesheet (overrides). Pure.
 */
import type { SceneNode, SceneStroke } from "../../../renderer/types.ts";
import { corners, clampedCorners } from "../../../renderer/corners.ts";
import { strokeEdges } from "../../../renderer/stroke.ts";

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
    ? { color: typeof fill === "string" ? fill : "transparent", "background-color": "transparent" }
    : { "background-color": typeof fill === "string" ? fill : "transparent",
      "background-image": typeof fill === "string" ? "none" : paintCSS(fill),
      "border-radius": corners(node.radius).map(px).join(" "),
      "box-shadow": shadowCSS(node, false) };
}
export function paintCSS(paint: Exclude<SceneNode["fill"],string>) {
  const stops=paint.stops.map(([at,color])=>`${color} ${at*100}%`).join(",");
  if (paint.type === "linear") return `linear-gradient(${paint.angle ?? 180}deg,${stops})`;
  const [x,y]=paint.center ?? [0.5,0.5], r=paint.radius;
  const shape=typeof r === "number" ? `circle ${px(r)}` : r ? `${px(r[0])} ${px(r[1])}` : "ellipse farthest-corner";
  return `radial-gradient(${shape} at ${x*100}% ${y*100}%,${stops})`;
}
function shadowCSS(n: SceneNode,inset: boolean) {
  return n.shadow?.filter(s=>s.inset === inset).map(s=>`${inset ? "inset " : ""}${px(s.x)} ${px(s.y)} ${px(s.blur)} ${px(s.spread)} ${s.color}`).join(",") || "none";
}
/** Separate overlay keeps image inset shadows above the image and below descendants. */
export function insetDeclarations(n: SceneNode): Declarations {
  const e=n.stroke?.align === "inside" ? strokeEdges(n.bounds,clampedCorners(n.bounds,n.radius),n.stroke.width,"inside") : null;
  const [t,r,b,l]=n.stroke?.align === "inside" ? n.stroke.width : [0,0,0,0];
  return { position:"absolute", top:px(t),right:px(r),bottom:px(b),left:px(l),"pointer-events":"none",
    "border-radius": e ? `${e.iR.map(v=>px(v.x)).join(" ")} / ${e.iR.map(v=>px(v.y)).join(" ")}` : corners(n.radius).map(px).join(" "),
    "box-shadow":shadowCSS(n,true) };
}

/**
 * A stroke as a CSS border on an overlay box grown by width·f per side (inside 0, center ½,
 * outside 1), with per-axis corner radii like renderer/stroke.ts `strokeEdges`. The overlay is
 * a sibling of the paint box, so the node's own clip never cuts its stroke. Radii reduce like
 * CSS on the node's bounds first, so outside/center rings follow the clamped fill shape.
 */
export function strokeDeclarations(
  stroke: SceneStroke | undefined,
  radius: SceneNode["radius"],
  bounds: SceneNode["bounds"],
): Declarations {
  if (!stroke) return { display: "none" };
  const f = ALIGN[stroke.align] ?? 0;
  const [t, r, b, l] = stroke.width;
  // Inside, the overlay is the border box itself: CSS reduces the authored radii exactly.
  const base = f ? clampedCorners(bounds, radius) : corners(radius);
  const corner = (h: number, v: number, i:number) =>
    base[i] > 0 ? [base[i] + h * f, base[i] + v * f] : [0, 0];
  const radii = [corner(l, t,0), corner(r, t,1), corner(r, b,2), corner(l, b,3)];
  return {
    display: "block",
    top: px(-t * f),
    right: px(-r * f),
    bottom: px(-b * f),
    left: px(-l * f),
    "border-style": "solid",
    "border-width": stroke.width.map(px).join(" "),
    "border-color": stroke.color.join(" "),
    "border-radius": `${radii.map((c) => px(c[0])).join(" ")} / ${radii
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
